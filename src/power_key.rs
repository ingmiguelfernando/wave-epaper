//! AXP2101 PMIC power-key event interpretation.
//!
//! The Waveshare board routes the physical power button through the AXP2101
//! PMIC rather than through one of the three application-button GPIOs. The
//! register-level I2C access remains in [`crate::power`]; this module keeps the
//! short-press menu and long-press sleep product policy host-testable and
//! separate from PMIC transport.

/// Polling cadence for PMIC power-key status bits.
pub const POWER_KEY_POLL_MS: u64 = 100;

/// AXP2101 IRQ2 bit used for a POWERON long press.
///
/// XPowers names the source `XPOWERS_AXP2101_PKEY_LONG_IRQ` at global bit 10,
/// which maps to bit 2 of AXP2101 `INTEN2` / `INTSTS2`.
pub const POWER_KEY_LONG_PRESS_MASK: u8 = 1 << 2;

/// AXP2101 IRQ2 bit used for a POWERON short press.
///
/// XPowers names the source `XPOWERS_AXP2101_PKEY_SHORT_IRQ` at global bit 11,
/// which maps to bit 3 of AXP2101 `INTEN2` / `INTSTS2`.
pub const POWER_KEY_SHORT_PRESS_MASK: u8 = 1 << 3;

pub const POWER_KEY_EVENT_MASK: u8 = POWER_KEY_LONG_PRESS_MASK | POWER_KEY_SHORT_PRESS_MASK;

/// Minimum quiet interval after sleep-image entry before a new PMIC Power
/// event can wake the device. This suppresses queued PEK events emitted by the
/// same physical hold that initiated the sleep transition.
pub const POWER_KEY_WAKE_GUARD_QUIET_MS: u64 = 900;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepWakeGuardDecision {
    SuppressStalePress,
    AllowWake,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SleepWakeGuard {
    waiting_for_quiet_window: bool,
    armed: bool,
    suppressed_events: u32,
}

impl SleepWakeGuard {
    pub fn begin_sleep_entry(&mut self) {
        self.waiting_for_quiet_window = true;
        self.armed = false;
    }

    pub fn reset_after_wake(&mut self) {
        self.waiting_for_quiet_window = false;
        self.armed = false;
    }

    #[must_use]
    pub fn arm_after_quiet_window(&mut self, elapsed_ms: u64) -> bool {
        if self.waiting_for_quiet_window
            && !self.armed
            && elapsed_ms >= POWER_KEY_WAKE_GUARD_QUIET_MS
        {
            self.waiting_for_quiet_window = false;
            self.armed = true;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn on_power_press(&mut self, elapsed_ms: u64) -> SleepWakeGuardDecision {
        let _ = self.arm_after_quiet_window(elapsed_ms);
        if self.armed {
            SleepWakeGuardDecision::AllowWake
        } else {
            self.suppressed_events = self.suppressed_events.saturating_add(1);
            SleepWakeGuardDecision::SuppressStalePress
        }
    }

    #[must_use]
    pub const fn suppressed_events(&self) -> u32 {
        self.suppressed_events
    }

    /// True once the quiet window has passed and a Power press would wake.
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.armed
    }
}

/// Product-facing physical Power-key events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerKeyEvent {
    /// Open the global display-maintenance menu while awake.
    ShortPress,
    /// Enter the accepted sleep-image path while awake.
    LongPress,
}

impl PowerKeyEvent {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::ShortPress => "short-press",
            Self::LongPress => "long-press",
        }
    }
}

/// First pause before the Power-key setup is written again after an error.
pub const POWER_KEY_RETRY_FIRST_MS: u64 = 1_000;
/// Longest pause between setup attempts.
pub const POWER_KEY_RETRY_MAX_MS: u64 = 30_000;

/// Whether the PMIC answers Power-key polls. An I2C error never turns the key
/// off for good: polling pauses, then the event setup is written again.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PowerKeyLink {
    ready: bool,
    failures_in_row: u32,
    errors: u32,
    last_error: Option<String>,
}

impl PowerKeyLink {
    /// The event setup worked.
    pub fn succeeded(&mut self) {
        self.ready = true;
        self.failures_in_row = 0;
    }

    /// The event setup or a poll failed. Returns the pause in milliseconds
    /// before the next setup attempt: 1 s, doubling up to 30 s.
    pub fn failed(&mut self, error: impl Into<String>) -> u64 {
        self.ready = false;
        self.failures_in_row = self.failures_in_row.saturating_add(1);
        self.errors = self.errors.saturating_add(1);
        self.last_error = Some(error.into());
        let doublings = (self.failures_in_row - 1).min(5);
        (POWER_KEY_RETRY_FIRST_MS << doublings).min(POWER_KEY_RETRY_MAX_MS)
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.ready
    }

    /// Errors since boot, recovered ones included.
    #[must_use]
    pub const fn errors(&self) -> u32 {
        self.errors
    }

    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// For Device info, for example `Ready` or `Ready, 2 errors`.
    #[must_use]
    pub fn label(&self) -> String {
        match (self.ready, self.errors) {
            (true, 0) => "Ready".into(),
            (true, 1) => "Ready, 1 error".into(),
            (true, errors) => format!("Ready, {errors} errors"),
            (false, 0) => "Not checked".into(),
            (false, _) => "Not answering".into(),
        }
    }
}

/// Interpret one AXP2101 `INTSTS2` byte. Long press wins when both sticky bits
/// are present so one held Power action cannot open the short-press menu first.
#[must_use]
pub const fn power_key_event_from_irq_status(status2: u8) -> Option<PowerKeyEvent> {
    if status2 & POWER_KEY_LONG_PRESS_MASK != 0 {
        Some(PowerKeyEvent::LongPress)
    } else if status2 & POWER_KEY_SHORT_PRESS_MASK != 0 {
        Some(PowerKeyEvent::ShortPress)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        power_key_event_from_irq_status, PowerKeyEvent, PowerKeyLink, SleepWakeGuard,
        SleepWakeGuardDecision, POWER_KEY_EVENT_MASK, POWER_KEY_LONG_PRESS_MASK,
        POWER_KEY_SHORT_PRESS_MASK, POWER_KEY_WAKE_GUARD_QUIET_MS,
    };

    #[test]
    fn errors_pause_polling_and_retry_with_growing_pauses() {
        let mut link = PowerKeyLink::default();
        assert_eq!(link.label(), "Not checked");
        assert_eq!(link.failed("AXP2101 read 0x49 failed"), 1_000);
        assert!(!link.is_ready());
        assert_eq!(link.label(), "Not answering");
        let pauses: Vec<u64> = (0..6).map(|_| link.failed("timeout")).collect();
        assert_eq!(pauses, [2_000, 4_000, 8_000, 16_000, 30_000, 30_000]);
        link.succeeded();
        assert!(link.is_ready());
        assert_eq!(link.label(), "Ready, 7 errors");
        assert_eq!(link.last_error(), Some("timeout"));
        // A new failure after a recovery starts again at the first pause.
        assert_eq!(link.failed("again"), 1_000);
    }

    #[test]
    fn a_link_without_errors_reads_ready() {
        let mut link = PowerKeyLink::default();
        link.succeeded();
        assert_eq!((link.label().as_str(), link.errors()), ("Ready", 0));
        assert_eq!(link.last_error(), None);
        let _ = link.failed("bus busy");
        link.succeeded();
        assert_eq!(link.label(), "Ready, 1 error");
    }

    #[test]
    fn decodes_short_and_long_axp2101_power_key_bits_with_long_priority() {
        assert_eq!(POWER_KEY_LONG_PRESS_MASK, 0x04);
        assert_eq!(POWER_KEY_SHORT_PRESS_MASK, 0x08);
        assert_eq!(POWER_KEY_EVENT_MASK, 0x0C);
        assert_eq!(
            power_key_event_from_irq_status(0x08),
            Some(PowerKeyEvent::ShortPress)
        );
        assert_eq!(
            power_key_event_from_irq_status(0x04),
            Some(PowerKeyEvent::LongPress)
        );
        assert_eq!(
            power_key_event_from_irq_status(0x0C),
            Some(PowerKeyEvent::LongPress)
        );
    }

    #[test]
    fn ignores_unrelated_axp2101_irq2_bits() {
        assert_eq!(power_key_event_from_irq_status(0x00), None);
        assert_eq!(power_key_event_from_irq_status(0x10), None);
        assert_eq!(power_key_event_from_irq_status(0x80), None);
    }

    #[test]
    fn wake_guard_suppresses_entry_press_until_quiet_window() {
        let mut guard = SleepWakeGuard::default();
        guard.begin_sleep_entry();
        assert_eq!(POWER_KEY_WAKE_GUARD_QUIET_MS, 900);
        assert_eq!(
            guard.on_power_press(120),
            SleepWakeGuardDecision::SuppressStalePress
        );
        assert_eq!(guard.suppressed_events(), 1);
        assert!(!guard.arm_after_quiet_window(899));
        assert!(guard.arm_after_quiet_window(900));
        assert!(guard.is_armed());
        assert_eq!(guard.on_power_press(901), SleepWakeGuardDecision::AllowWake);
    }

    #[test]
    fn wake_guard_allows_first_press_after_elapsed_quiet_window() {
        let mut guard = SleepWakeGuard::default();
        guard.begin_sleep_entry();
        assert_eq!(
            guard.on_power_press(1_200),
            SleepWakeGuardDecision::AllowWake
        );
        guard.reset_after_wake();
        guard.begin_sleep_entry();
        assert_eq!(
            guard.on_power_press(0),
            SleepWakeGuardDecision::SuppressStalePress
        );
    }
}
