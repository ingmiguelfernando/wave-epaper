//! Power-key event interpretation.
//!
//! The Waveshare board wires the Power key to the PMIC's key input and, through
//! a BSS138 inverter (T1), to GPIO1 (`PWR_OUT` in the schematic). GPIO1 is
//! the main source; the PMIC's latched key interrupt is a backup that only
//! counts until GPIO1 shows a press. The register-level I2C access remains in
//! [`crate::power`]; this module keeps the short-press menu and long-press
//! sleep policy host-testable.

/// Polling cadence for PMIC power-key status bits.
pub const POWER_KEY_POLL_MS: u64 = 100;

/// GPIO1 carries the Power key: T1 holds it low at rest and R24 pulls it high
/// while the key is held. An RTC alarm pulls the same key line low through D4,
/// so GPIO1 is only the key while the alarm line (GPIO45) is idle.
pub const POWER_KEY_GPIO: i32 = 1;
/// Holding the key this long puts the device to sleep.
pub const POWER_KEY_HOLD_MS: u64 = 1_000;
/// Shorter blips are contact bounce, not presses.
pub const POWER_KEY_MIN_PRESS_MS: u64 = 30;
/// A line high this long is stuck, not held, and stops waking light sleep.
pub const POWER_KEY_STUCK_MS: u64 = 15_000;

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

/// What reported a Power-key event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerKeySource {
    /// GPIO1, the board's `PWR_OUT` line.
    Gpio,
    /// The PMIC's latched key interrupt.
    Pmic,
    /// The idle timer, which takes the long-press path.
    AutoSleep,
}

impl PowerKeySource {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Gpio => "gpio1",
            Self::Pmic => "axp2101-pek",
            Self::AutoSleep => "auto-sleep",
        }
    }
}

/// Turns GPIO1 samples into presses. A release before a second is a short
/// press; a second held is a long press, reported once while still held.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PowerKeyPresses {
    down_since: Option<u64>,
    reported: bool,
    seen: bool,
}

impl PowerKeyPresses {
    /// `down` is the line at boot: a key still held from powering on is not a
    /// press.
    #[must_use]
    pub fn new(down: bool) -> Self {
        Self {
            down_since: down.then_some(0),
            reported: down,
            seen: false,
        }
    }

    /// Feed one sample of the line; returns the press it completes, if any.
    pub fn update(&mut self, down: bool, now_ms: u64) -> Option<PowerKeyEvent> {
        match (down, self.down_since) {
            (true, None) => {
                self.down_since = Some(now_ms);
                self.reported = false;
                self.seen = true;
                None
            }
            (true, Some(since)) => {
                if self.reported || now_ms.saturating_sub(since) < POWER_KEY_HOLD_MS {
                    return None;
                }
                self.reported = true;
                Some(PowerKeyEvent::LongPress)
            }
            (false, Some(since)) => {
                self.down_since = None;
                let held = now_ms.saturating_sub(since);
                if self.reported || held < POWER_KEY_MIN_PRESS_MS {
                    None
                } else if held >= POWER_KEY_HOLD_MS {
                    // The loop was busy when the hold passed a second.
                    Some(PowerKeyEvent::LongPress)
                } else {
                    Some(PowerKeyEvent::ShortPress)
                }
            }
            (false, None) => None,
        }
    }

    /// True while the key is held.
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.down_since.is_some()
    }

    /// True once GPIO1 has shown a press, which proves the line works; the
    /// PMIC's own key events then repeat what GPIO1 already reported.
    #[must_use]
    pub const fn seen_press(&self) -> bool {
        self.seen
    }

    /// True when the line has been high too long to be a finger.
    #[must_use]
    pub fn is_stuck(&self, now_ms: u64) -> bool {
        self.down_since
            .is_some_and(|since| now_ms.saturating_sub(since) >= POWER_KEY_STUCK_MS)
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
    gpio: bool,
    failures_in_row: u32,
    errors: u32,
    last_error: Option<String>,
}

impl PowerKeyLink {
    /// GPIO1 reads the key, so it works even when the PMIC does not answer.
    pub fn use_gpio_line(&mut self) {
        self.gpio = true;
    }

    /// True when a Power press can reach the firmware, so sleep can end.
    #[must_use]
    pub const fn can_wake(&self) -> bool {
        self.gpio || self.ready
    }

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

    /// True while the PMIC answers, so its key status is polled.
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
        match (self.can_wake(), self.errors) {
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
        power_key_event_from_irq_status, PowerKeyEvent, PowerKeyLink, PowerKeyPresses,
        PowerKeySource, SleepWakeGuard, SleepWakeGuardDecision, POWER_KEY_EVENT_MASK,
        POWER_KEY_HOLD_MS, POWER_KEY_LONG_PRESS_MASK, POWER_KEY_MIN_PRESS_MS,
        POWER_KEY_SHORT_PRESS_MASK, POWER_KEY_STUCK_MS, POWER_KEY_WAKE_GUARD_QUIET_MS,
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

    #[test]
    fn a_tap_on_gpio1_is_a_short_press() {
        let mut presses = PowerKeyPresses::new(false);
        assert_eq!(presses.update(false, 0), None);
        assert_eq!(presses.update(true, 100), None);
        assert!(presses.is_down() && presses.seen_press());
        assert_eq!(presses.update(true, 160), None);
        let released = presses.update(false, 220);
        assert_eq!(released, Some(PowerKeyEvent::ShortPress));
        assert!(!presses.is_down());
    }

    #[test]
    fn a_second_held_is_one_long_press_while_still_held() {
        let mut presses = PowerKeyPresses::new(false);
        assert_eq!(presses.update(true, 0), None);
        assert_eq!(presses.update(true, POWER_KEY_HOLD_MS - 1), None);
        let held = presses.update(true, POWER_KEY_HOLD_MS);
        assert_eq!(held, Some(PowerKeyEvent::LongPress));
        assert_eq!(presses.update(true, 3_000), None);
        assert_eq!(presses.update(false, 3_200), None);
    }

    #[test]
    fn a_long_hold_seen_only_at_release_still_counts_as_long() {
        let mut presses = PowerKeyPresses::new(false);
        presses.update(true, 0);
        let released = presses.update(false, POWER_KEY_HOLD_MS + 300);
        assert_eq!(released, Some(PowerKeyEvent::LongPress));
    }

    #[test]
    fn bounce_and_a_key_held_since_boot_are_not_presses() {
        let mut presses = PowerKeyPresses::new(false);
        presses.update(true, 0);
        assert_eq!(presses.update(false, POWER_KEY_MIN_PRESS_MS - 1), None);

        let mut held_at_boot = PowerKeyPresses::new(true);
        assert!(held_at_boot.is_down() && !held_at_boot.seen_press());
        assert_eq!(held_at_boot.update(true, 5_000), None);
        assert_eq!(held_at_boot.update(false, 6_000), None);
        held_at_boot.update(true, 7_000);
        let released = held_at_boot.update(false, 7_100);
        assert_eq!(released, Some(PowerKeyEvent::ShortPress));
    }

    #[test]
    fn a_line_high_for_long_is_stuck() {
        let mut presses = PowerKeyPresses::new(false);
        presses.update(true, 1_000);
        assert!(!presses.is_stuck(POWER_KEY_STUCK_MS + 999));
        assert!(presses.is_stuck(POWER_KEY_STUCK_MS + 1_000));
        presses.update(false, POWER_KEY_STUCK_MS + 2_000);
        assert!(!presses.is_stuck(POWER_KEY_STUCK_MS + 2_000));
    }

    #[test]
    fn gpio1_makes_the_key_ready_without_the_pmic() {
        let mut link = PowerKeyLink::default();
        assert!(!link.can_wake());
        link.use_gpio_line();
        assert!(link.can_wake() && !link.is_ready());
        assert_eq!(link.label(), "Ready");
        let _ = link.failed("bus busy");
        assert_eq!(link.label(), "Ready, 1 error");
        assert_eq!(PowerKeySource::Gpio.marker(), "gpio1");
    }
}
