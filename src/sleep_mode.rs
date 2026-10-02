//! Hardware-independent power-key sleep-image mode state.
//!
//! While the sleep image is visible the ESP32-S3 light-sleeps between events:
//! the Power key and the GPIO45 RTC alarm line wake it, and the event loop then
//! handles the wake exactly as before. Optional Wi-Fi, SNTP and weather
//! services pause while a static sleep image is visible and resume after the
//! wake frame has rendered.

use crate::app::ScreenRoute;

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
        let minutes = self.seconds / 60;
        let duration = if minutes >= 60 {
            format!("{}h {}m", minutes / 60, minutes % 60)
        } else {
            format!("{minutes}m")
        };
        match (self.battery_before, self.battery_after) {
            (Some(before), Some(after)) => format!("{duration} · {before}% to {after}%"),
            _ => duration,
        }
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
    use super::{SleepModeState, SleepReport, SleepWakeCause};
    use crate::app::ScreenRoute;

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
