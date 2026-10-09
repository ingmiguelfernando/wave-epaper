//! Polling button adapters for the active-low onboard keys and GPIO0 BOOT back action.

use core::fmt::Debug;

use anyhow::{anyhow, Result};
use embedded_hal::{delay::DelayNs, digital::InputPin};

const DEBOUNCE_MS: u32 = 25;
/// Hold duration required for GPIO0 BOOT to navigate one hierarchy level back.
pub const BOOT_BACK_LONG_PRESS_MS: u32 = 900;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonEvent {
    Up,
    Select,
    Down,
}

/// Small polling adapter. The first milestone deliberately avoids interrupt
/// callbacks and global mutable state; the product UI can add an event queue
/// behind this interface later.
pub struct Buttons<UP, SELECT, DOWN> {
    up: UP,
    select: SELECT,
    down: DOWN,
    repeat: KeyRepeat,
}

impl<UP, SELECT, DOWN> Buttons<UP, SELECT, DOWN>
where
    UP: InputPin,
    UP::Error: Debug,
    SELECT: InputPin,
    SELECT::Error: Debug,
    DOWN: InputPin,
    DOWN::Error: Debug,
{
    #[must_use]
    pub fn new(up: UP, select: SELECT, down: DOWN) -> Self {
        Self {
            up,
            select,
            down,
            repeat: KeyRepeat::default(),
        }
    }

    /// Return one debounced press, or a repeat of a held ▲ or ▼ that is due.
    /// Keys are active low on the Waveshare board. `now_ms` is any clock that
    /// keeps running; `repeats` says whether the current screen repeats arrows.
    pub fn poll<D: DelayNs>(
        &mut self,
        delay: &mut D,
        now_ms: u64,
        repeats: bool,
    ) -> Result<Option<ButtonEvent>> {
        if self.is_pressed(ButtonEvent::Up)? {
            return self.arrow(delay, ButtonEvent::Up, now_ms, repeats);
        }
        if self.is_pressed(ButtonEvent::Select)? {
            self.repeat.step(None, now_ms, false);
            return self.confirm(delay, ButtonEvent::Select);
        }
        if self.is_pressed(ButtonEvent::Down)? {
            return self.arrow(delay, ButtonEvent::Down, now_ms, repeats);
        }
        self.repeat.step(None, now_ms, false);
        Ok(None)
    }

    /// ▲ and ▼ report at the press, after the debounce, and repeat while held.
    fn arrow<D: DelayNs>(
        &mut self,
        delay: &mut D,
        event: ButtonEvent,
        now_ms: u64,
        repeats: bool,
    ) -> Result<Option<ButtonEvent>> {
        if !self.repeat.is_held(event) {
            delay.delay_ms(DEBOUNCE_MS);
            if !self.is_pressed(event)? {
                return Ok(None);
            }
        }
        Ok(self.repeat.step(Some(event), now_ms, repeats))
    }

    fn confirm<D: DelayNs>(
        &mut self,
        delay: &mut D,
        event: ButtonEvent,
    ) -> Result<Option<ButtonEvent>> {
        delay.delay_ms(DEBOUNCE_MS);
        if !self.is_pressed(event)? {
            return Ok(None);
        }

        // Do not generate repeated UI events while the panel is refreshing.
        while self.is_pressed(event)? {
            delay.delay_ms(10);
        }
        Ok(Some(event))
    }

    fn is_pressed(&mut self, event: ButtonEvent) -> Result<bool> {
        match event {
            ButtonEvent::Up => self
                .up
                .is_low()
                .map_err(|error| anyhow!("GPIO4 UP read failed: {error:?}")),
            ButtonEvent::Select => self
                .select
                .is_low()
                .map_err(|error| anyhow!("GPIO5 SELECT read failed: {error:?}")),
            ButtonEvent::Down => self
                .down
                .is_low()
                .map_err(|error| anyhow!("GPIO6 DOWN read failed: {error:?}")),
        }
    }
}

/// First repeat after a held ▲ or ▼, in milliseconds.
pub const REPEAT_FIRST_MS: u64 = 500;
/// Gap between repeats while the arrow stays down.
pub const REPEAT_EVERY_MS: u64 = 200;

/// Hold-to-repeat timer for ▲ and ▼. A press reports at once; while the key
/// stays down, a repeat is due `REPEAT_FIRST_MS` after the press and every
/// `REPEAT_EVERY_MS` after the last one. The next repeat counts from when it
/// is reported, so a long pause gives one repeat, not a burst.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyRepeat {
    held: Option<ButtonEvent>,
    next_at_ms: u64,
}

impl KeyRepeat {
    #[must_use]
    pub fn is_held(&self, event: ButtonEvent) -> bool {
        self.held == Some(event)
    }

    /// The event to report at `now_ms` for the key that is down, if any.
    /// `pressed` is `None` when no arrow is down. Without `repeats`, a held
    /// key reports only its press.
    pub fn step(
        &mut self,
        pressed: Option<ButtonEvent>,
        now_ms: u64,
        repeats: bool,
    ) -> Option<ButtonEvent> {
        let Some(key) = pressed else {
            self.held = None;
            return None;
        };
        if self.held != Some(key) {
            self.held = Some(key);
            self.next_at_ms = now_ms + REPEAT_FIRST_MS;
            return Some(key);
        }
        if repeats && now_ms >= self.next_at_ms {
            self.next_at_ms = now_ms + REPEAT_EVERY_MS;
            return Some(key);
        }
        None
    }
}

/// Dedicated active-low GPIO0 BOOT-button adapter.
///
/// Long presses remain hierarchy-level Back. Short presses are surfaced so
/// route-specific features such as Sudoku axis selection can use BOOT without
/// changing Back behavior elsewhere.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootButtonEvent {
    ShortPress,
    LongPress,
}

pub struct LongPressBackButton<BACK> {
    back: BACK,
}

impl<BACK> LongPressBackButton<BACK>
where
    BACK: InputPin,
    BACK::Error: Debug,
{
    #[must_use]
    pub fn new(back: BACK) -> Self {
        Self { back }
    }

    /// Return one BOOT release classified as short or long. A press made while
    /// `power_key_held` reports the Power key down is left to that key.
    pub fn poll<D: DelayNs>(
        &mut self,
        delay: &mut D,
        mut power_key_held: impl FnMut() -> bool,
    ) -> Result<Option<BootButtonEvent>> {
        if !self.is_pressed()? {
            return Ok(None);
        }
        delay.delay_ms(DEBOUNCE_MS);
        if !self.is_pressed()? || power_key_held() {
            return Ok(None);
        }

        let mut held_ms = DEBOUNCE_MS;
        while self.is_pressed()? {
            if held_ms >= BOOT_BACK_LONG_PRESS_MS {
                while self.is_pressed()? {
                    delay.delay_ms(10);
                }
                return Ok(Some(BootButtonEvent::LongPress));
            }
            delay.delay_ms(10);
            held_ms = held_ms.saturating_add(10);
        }
        Ok(Some(BootButtonEvent::ShortPress))
    }

    fn is_pressed(&mut self) -> Result<bool> {
        self.back
            .is_low()
            .map_err(|error| anyhow!("GPIO0 BOOT read failed: {error:?}"))
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::*;

    #[test]
    fn a_press_reports_at_once_and_holding_waits_before_repeating() {
        let mut keys = KeyRepeat::default();
        assert_eq!(
            keys.step(Some(ButtonEvent::Down), 0, true),
            Some(ButtonEvent::Down)
        );
        assert_eq!(keys.step(Some(ButtonEvent::Down), 499, true), None);
        assert_eq!(
            keys.step(Some(ButtonEvent::Down), 500, true),
            Some(ButtonEvent::Down)
        );
    }

    #[test]
    fn the_repeat_comes_every_200_ms_while_held_and_stops_at_release() {
        let mut keys = KeyRepeat::default();
        assert_eq!(
            keys.step(Some(ButtonEvent::Up), 0, true),
            Some(ButtonEvent::Up)
        );
        assert_eq!(
            keys.step(Some(ButtonEvent::Up), 700, true),
            Some(ButtonEvent::Up)
        );
        assert_eq!(keys.step(Some(ButtonEvent::Up), 899, true), None);
        assert_eq!(
            keys.step(Some(ButtonEvent::Up), 900, true),
            Some(ButtonEvent::Up)
        );
        assert_eq!(keys.step(None, 950, true), None, "released");
        assert_eq!(
            keys.step(Some(ButtonEvent::Up), 1_000, true),
            Some(ButtonEvent::Up),
            "a new press reports"
        );
    }

    #[test]
    fn a_long_refresh_gives_one_repeat_not_a_burst() {
        let mut keys = KeyRepeat::default();
        keys.step(Some(ButtonEvent::Down), 0, true);
        assert_eq!(
            keys.step(Some(ButtonEvent::Down), 2_600, true),
            Some(ButtonEvent::Down)
        );
        assert_eq!(keys.step(Some(ButtonEvent::Down), 2_700, true), None);
        assert_eq!(
            keys.step(Some(ButtonEvent::Down), 2_800, true),
            Some(ButtonEvent::Down)
        );
    }

    #[test]
    fn a_screen_without_repeat_takes_one_press_per_key() {
        let mut keys = KeyRepeat::default();
        assert_eq!(
            keys.step(Some(ButtonEvent::Down), 0, false),
            Some(ButtonEvent::Down)
        );
        assert_eq!(keys.step(Some(ButtonEvent::Down), 500, false), None);
        assert_eq!(keys.step(Some(ButtonEvent::Down), 900, false), None);
    }
}
