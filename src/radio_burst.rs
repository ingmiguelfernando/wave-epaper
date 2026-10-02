//! Wi-Fi stays off except for short bursts that refresh the weather, sync the
//! clock or serve a Wi-Fi transfer. The firmware loop drives the driver; this
//! module only decides when a burst starts, gives up and ends.

use std::time::{Duration, Instant};

/// A burst that has not connected by then gives up.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// After connecting, wait at most this long for the clock sync.
pub const TIME_SYNC_WAIT: Duration = Duration::from_secs(10);
/// Longest burst; only a running Wi-Fi transfer keeps the radio on longer.
pub const BURST_LIMIT: Duration = Duration::from_secs(90);
/// Clock re-sync period when no weather refresh brings the radio up sooner.
pub const TIME_SYNC_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);
/// Scheduled bursts wait this long after the last button press.
pub const IDLE_GRACE: Duration = Duration::from_secs(8);
/// Wait after a burst that failed or left the clock unsynced; doubles on
/// consecutive failures up to `RETRY_GAP_MAX`.
pub const RETRY_GAP: Duration = Duration::from_secs(15 * 60);
pub const RETRY_GAP_MAX: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RadioPhase {
    #[default]
    Off,
    Connecting,
    Connected,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RadioBurst {
    phase: RadioPhase,
    started_at: Option<Instant>,
    connected_at: Option<Instant>,
    synced_this_burst: bool,
    last_time_sync: Option<Instant>,
    retry_after: Option<Instant>,
    failures: u32,
}

impl RadioBurst {
    #[must_use]
    pub const fn phase(&self) -> RadioPhase {
        self.phase
    }

    pub fn begin(&mut self, now: Instant) {
        self.phase = RadioPhase::Connecting;
        self.started_at = Some(now);
        self.connected_at = None;
        self.synced_this_burst = false;
    }

    pub fn connected(&mut self, now: Instant) {
        self.phase = RadioPhase::Connected;
        self.connected_at = Some(now);
        self.failures = 0;
        self.retry_after = None;
    }

    pub fn time_synced(&mut self, now: Instant) {
        self.synced_this_burst = true;
        self.last_time_sync = Some(now);
    }

    /// The radio is off again after a burst that could not connect.
    pub fn failed(&mut self, now: Instant) {
        self.phase = RadioPhase::Off;
        self.failures = self.failures.saturating_add(1);
        let doublings = (self.failures - 1).min(8);
        let gap = RETRY_GAP.saturating_mul(1 << doublings).min(RETRY_GAP_MAX);
        self.retry_after = Some(now + gap);
    }

    /// The radio is off again after a connected burst.
    pub fn finished(&mut self, now: Instant) {
        self.phase = RadioPhase::Off;
        if !self.synced_this_burst {
            self.retry_after = Some(now + RETRY_GAP);
        }
    }

    #[must_use]
    pub fn connect_timed_out(&self, now: Instant) -> bool {
        self.phase == RadioPhase::Connecting
            && self
                .started_at
                .is_some_and(|started| now.duration_since(started) >= CONNECT_TIMEOUT)
    }

    /// True once a connected burst has nothing left to wait for.
    #[must_use]
    pub fn work_done(&self, now: Instant, weather_settled: bool) -> bool {
        if self.phase != RadioPhase::Connected {
            return false;
        }
        let elapsed_since = |at: Option<Instant>| at.map_or(Duration::ZERO, |at| now - at);
        let sync_settled =
            self.synced_this_burst || elapsed_since(self.connected_at) >= TIME_SYNC_WAIT;
        elapsed_since(self.started_at) >= BURST_LIMIT || (weather_settled && sync_settled)
    }

    /// When scheduled work may switch the radio on. `weather_due` is when the
    /// next weather refresh falls due, if weather is configured.
    #[must_use]
    pub fn next_burst(
        &self,
        now: Instant,
        weather_due: Option<Instant>,
        last_input: Instant,
    ) -> Instant {
        let retry = self.retry_after.unwrap_or(now);
        let Some(synced) = self.last_time_sync else {
            // Never synced: the clock may be wrong, so skip the idle grace.
            return retry;
        };
        let time_due = synced + TIME_SYNC_INTERVAL;
        let work_due = weather_due.map_or(time_due, |weather| weather.min(time_due));
        work_due.max(last_input + IDLE_GRACE).max(retry)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        RadioBurst, RadioPhase, BURST_LIMIT, CONNECT_TIMEOUT, IDLE_GRACE, RETRY_GAP, RETRY_GAP_MAX,
        TIME_SYNC_INTERVAL, TIME_SYNC_WAIT,
    };

    const MINUTE: Duration = Duration::from_secs(60);

    fn synced_burst(now: Instant) -> RadioBurst {
        let mut radio = RadioBurst::default();
        radio.begin(now);
        radio.connected(now);
        radio.time_synced(now);
        radio.finished(now);
        radio
    }

    #[test]
    fn first_burst_starts_at_once() {
        let now = Instant::now();
        let radio = RadioBurst::default();
        assert_eq!(radio.next_burst(now, Some(now + MINUTE), now), now);
    }

    #[test]
    fn weather_refresh_waits_for_quiet_buttons() {
        let start = Instant::now();
        let radio = synced_burst(start);
        let now = start + 30 * MINUTE;
        assert_eq!(radio.next_burst(now, Some(now), now), now + IDLE_GRACE);
        let quiet = now - IDLE_GRACE;
        assert_eq!(radio.next_burst(now, Some(now), quiet), now);
    }

    #[test]
    fn without_weather_the_clock_sync_schedules_bursts() {
        let start = Instant::now();
        let radio = synced_burst(start);
        assert_eq!(
            radio.next_burst(start, None, start),
            start + TIME_SYNC_INTERVAL
        );
    }

    #[test]
    fn failures_back_off_up_to_the_cap() {
        let now = Instant::now();
        let mut radio = RadioBurst::default();
        let mut gaps = Vec::new();
        for _ in 0..7 {
            radio.begin(now);
            assert!(!radio.connect_timed_out(now));
            assert!(radio.connect_timed_out(now + CONNECT_TIMEOUT));
            radio.failed(now);
            assert_eq!(radio.phase(), RadioPhase::Off);
            gaps.push(radio.next_burst(now, Some(now), now) - now);
        }
        assert_eq!(gaps[0], RETRY_GAP);
        assert_eq!(gaps[1], RETRY_GAP * 2);
        assert_eq!(gaps[6], RETRY_GAP_MAX);
        radio.begin(now);
        radio.connected(now);
        radio.time_synced(now);
        radio.finished(now);
        let later = now + TIME_SYNC_INTERVAL;
        assert_eq!(radio.next_burst(later, None, now), later);
    }

    #[test]
    fn unsynced_clock_retries_after_a_gap() {
        let now = Instant::now();
        let mut radio = RadioBurst::default();
        radio.begin(now);
        radio.connected(now);
        radio.finished(now);
        assert_eq!(radio.next_burst(now, Some(now), now), now + RETRY_GAP);
    }

    #[test]
    fn burst_ends_after_weather_and_clock_or_the_limit() {
        let now = Instant::now();
        let mut radio = RadioBurst::default();
        radio.begin(now);
        assert!(!radio.work_done(now, true));
        radio.connected(now);
        assert!(!radio.work_done(now, true));
        assert!(radio.work_done(now + TIME_SYNC_WAIT, true));
        assert!(!radio.work_done(now + TIME_SYNC_WAIT, false));
        assert!(radio.work_done(now + BURST_LIMIT, false));
        radio.time_synced(now);
        assert!(radio.work_done(now, true));
    }
}
