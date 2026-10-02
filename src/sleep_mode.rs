//! Hardware-independent sleep state and light-sleep bookkeeping.
//!
//! While the sleep image is visible the ESP32-S3 light-sleeps between events:
//! the Power key and the GPIO45 RTC alarm line wake it, and the event loop then
//! handles the wake exactly as before. While awake it also light-sleeps between
//! button presses whenever no timed job is due sooner.

use std::time::Duration;

use crate::app::ScreenRoute;

/// Longest light sleep; the timer wake also covers a missed wake line.
pub const LIGHT_SLEEP_MAX: Duration = Duration::from_secs(60);
/// Shorter waits are not worth a light-sleep round trip.
pub const LIGHT_SLEEP_MIN: Duration = Duration::from_millis(50);

/// How long the loop may light-sleep given the time left until each of its
/// timed jobs; `None` when the next job is too close.
#[must_use]
pub fn light_sleep_budget(due_in: &[Option<Duration>]) -> Option<Duration> {
    let next = due_in.iter().flatten().copied().min();
    let budget = next.map_or(LIGHT_SLEEP_MAX, |next| next.min(LIGHT_SLEEP_MAX));
    (budget >= LIGHT_SLEEP_MIN).then_some(budget)
}

/// For example `7h 32m` or `5m`.
fn duration_label(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes >= 60 {
        format!("{}h {}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

/// Battery use across one sleep, shown on the Device Info screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepReport {
    pub seconds: u64,
    pub battery_before: Option<u8>,
    pub battery_after: Option<u8>,
}

impl SleepReport {
    /// For example `7h 32m · 82% to 80%`.
    #[must_use]
    pub fn label(&self) -> String {
        let duration = duration_label(self.seconds);
        match (self.battery_before, self.battery_after) {
            (Some(before), Some(after)) => format!("{duration} · {before}% to {after}%"),
            _ => duration,
        }
    }
}

/// Time spent in light sleep since the last wake, shown on Device Info.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LightSleepShare {
    pub asleep_seconds: u64,
    pub awake_seconds: u64,
}

impl LightSleepShare {
    /// For example `92% of 1h 5m`.
    #[must_use]
    pub fn label(&self) -> String {
        if self.awake_seconds == 0 {
            return "--".into();
        }
        let asleep = self.asleep_seconds.min(self.awake_seconds);
        let percent = asleep * 100 / self.awake_seconds;
        format!("{percent}% of {}", duration_label(self.awake_seconds))
    }
}

/// Why a sleeping display returned to the product UI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepWakeCause {
    /// A second short power-key press toggled the device back to active mode.
    PowerKey,
    /// A validated PCF85063 alarm occurred while the sleep image was visible.
    RtcAlarm,
}

impl SleepWakeCause {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::PowerKey => "power-key",
            Self::RtcAlarm => "rtc-alarm",
        }
    }
}

/// Stateful sleep-image mode boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SleepModeState {
    sleeping: bool,
    restore_route: ScreenRoute,
    last_image: Option<String>,
    entries: u32,
}

impl Default for SleepModeState {
    fn default() -> Self {
        Self {
            sleeping: false,
            restore_route: ScreenRoute::Home,
            last_image: None,
            entries: 0,
        }
    }
}

impl SleepModeState {
    #[must_use]
    pub const fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    #[must_use]
    pub const fn restore_route(&self) -> ScreenRoute {
        self.restore_route
    }

    #[must_use]
    pub fn last_image(&self) -> Option<&str> {
        self.last_image.as_deref()
    }

    #[must_use]
    pub const fn entries(&self) -> u32 {
        self.entries
    }

    /// Record sleep-image mode entry after the selected frame is visible.
    pub fn enter(&mut self, restore_route: ScreenRoute, image_label: impl Into<String>) {
        self.sleeping = true;
        self.restore_route = restore_route;
        self.last_image = Some(image_label.into());
        self.entries = self.entries.saturating_add(1);
    }

    /// Exit sleep-image mode and return the route that should be restored for a
    /// normal power-key wake. Alarm wake deliberately routes to Alarms instead.
    pub fn exit(&mut self, _cause: SleepWakeCause) -> ScreenRoute {
        self.sleeping = false;
        self.restore_route
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        light_sleep_budget, LightSleepShare, SleepModeState, SleepReport, SleepWakeCause,
        LIGHT_SLEEP_MAX,
    };
    use crate::app::ScreenRoute;

    #[test]
    fn light_sleep_lasts_until_the_next_timed_job() {
        let second = Duration::from_secs(1);
        assert_eq!(light_sleep_budget(&[]), Some(LIGHT_SLEEP_MAX));
        assert_eq!(
            light_sleep_budget(&[None, Some(5 * second), Some(90 * second)]),
            Some(5 * second)
        );
        let blink = Duration::from_millis(10);
        assert_eq!(light_sleep_budget(&[Some(blink)]), None);
    }

    #[test]
    fn light_sleep_share_shows_percent_of_awake_time() {
        let share = LightSleepShare {
            asleep_seconds: 3_300,
            awake_seconds: 3_900,
        };
        assert_eq!(share.label(), "84% of 1h 5m");
        assert_eq!(LightSleepShare::default().label(), "--");
    }

    #[test]
    fn sleep_report_shows_duration_and_battery_change() {
        let report = SleepReport {
            seconds: 7 * 3600 + 32 * 60 + 59,
            battery_before: Some(82),
            battery_after: Some(80),
        };
        assert_eq!(report.label(), "7h 32m · 82% to 80%");
        let short = SleepReport {
            seconds: 300,
            battery_before: None,
            battery_after: Some(80),
        };
        assert_eq!(short.label(), "5m");
    }

    #[test]
    fn sleep_mode_remembers_route_and_selected_image() {
        let mut state = SleepModeState::default();
        state.enter(ScreenRoute::Weather, "SLEEP01.BMP");
        assert!(state.is_sleeping());
        assert_eq!(state.restore_route(), ScreenRoute::Weather);
        assert_eq!(state.last_image(), Some("SLEEP01.BMP"));
        assert_eq!(state.entries(), 1);
        assert_eq!(state.exit(SleepWakeCause::PowerKey), ScreenRoute::Weather);
        assert!(!state.is_sleeping());
    }

    #[test]
    fn repeated_entries_keep_ram_only_counter() {
        let mut state = SleepModeState::default();
        state.enter(ScreenRoute::Home, "SLEEP.BMP");
        state.exit(SleepWakeCause::PowerKey);
        state.enter(ScreenRoute::Home, "SLEEP01.BMP");
        assert_eq!(state.entries(), 2);
    }
}
