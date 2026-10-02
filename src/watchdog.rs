//! Cooperative pauses for long CPU-bound work on FreeRTOS tasks.

use std::time::{Duration, Instant};

/// Longest stretch of work between pauses, well inside the 5 s task watchdog.
pub const YIELD_INTERVAL: Duration = Duration::from_millis(200);
/// One FreeRTOS tick at 100 Hz; ESP-IDF busy-waits on shorter sleeps.
pub const YIELD_SLEEP: Duration = Duration::from_millis(10);

/// Sleeps for one tick after every `YIELD_INTERVAL` of work so the idle task
/// can run and feed the task watchdog.
pub struct Pacer {
    last_yield: Instant,
}

impl Pacer {
    #[must_use]
    pub fn start() -> Self {
        Self {
            last_yield: Instant::now(),
        }
    }

    pub fn pace(&mut self) {
        if self.last_yield.elapsed() >= YIELD_INTERVAL {
            std::thread::sleep(YIELD_SLEEP);
            self.last_yield = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{YIELD_INTERVAL, YIELD_SLEEP};

    #[test]
    fn pauses_for_a_full_tick_well_within_the_watchdog() {
        assert!(YIELD_SLEEP >= Duration::from_millis(10));
        assert!(YIELD_INTERVAL < Duration::from_secs(5));
    }
}
