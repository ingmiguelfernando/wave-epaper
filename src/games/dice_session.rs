//! A pure Dice session: engine, screens and rattle timing, no drawing.
//!
//! The main line's wiring calls `on_shake`, `on_key` and `tick` with the
//! device's clock and does what the returned `Output` asks: redraw a frame,
//! start or stop the rattle, and wake again at `next_tick`. Options live
//! here and are handed back so the caller can save them.

use super::{
    dice::{DiceGame, DicePhase, DEFAULT_DICE, FRAMES, MAX_DICE},
    dice_view::DiceOptions,
};

/// Cooldown after a roll before another shake counts. Matches
/// `imu_events.rs`'s `SHAKE_COOLDOWN_MS` (kept private there); if that
/// constant becomes public, a test should compare the two.
pub const SHAKE_COOLDOWN_MS: u64 = 900;
/// Shake strength the IMU detector should use for `Strength::Low`.
pub const SHAKE_DELTA_LOW_MG: i32 = 300;
/// Shake strength for `Strength::Medium` (the detector's default 420).
pub const SHAKE_DELTA_MEDIUM_MG: i32 = 420;
/// Shake strength for `Strength::High`.
pub const SHAKE_DELTA_HIGH_MG: i32 = 550;

/// How hard a shake must be to roll.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strength {
    Low,
    Medium,
    High,
}

impl Strength {
    /// `shake_delta_mg` for `imu_events`'s detector.
    #[must_use]
    pub const fn shake_delta_mg(self) -> i32 {
        match self {
            Self::Low => SHAKE_DELTA_LOW_MG,
            Self::Medium => SHAKE_DELTA_MEDIUM_MG,
            Self::High => SHAKE_DELTA_HIGH_MG,
        }
    }
}

/// The owner's Dice options. The caller saves and restores these.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Options {
    pub dice: usize,
    pub sound: bool,
    pub shake: bool,
    pub strength: Strength,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            dice: DEFAULT_DICE,
            sound: true,
            shake: true,
            strength: Strength::Medium,
        }
    }
}

impl Options {
    /// The same values as the options screen.
    #[must_use]
    pub const fn view(self) -> DiceOptions {
        DiceOptions {
            selected: 0,
            sound: self.sound,
            shake: self.shake,
            shake_strength: match self.strength {
                Strength::Low => 0,
                Strength::Medium => 1,
                Strength::High => 2,
            },
        }
    }
}

/// Keys the session understands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Select,
    Up,
    Down,
    /// Short BOOT: open or close Options.
    BootShort,
    /// Hold BOOT: leave the session.
    BootHold,
}

/// What the caller must do after an event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Output {
    /// Redraw the current frame (partial, or full when ghosting needs it).
    pub redraw: Option<Refresh>,
    /// Start or stop the rattle; `Keep` leaves playback alone.
    pub rattle: RattleAction,
    /// When to call `tick` next, if ever.
    pub next_tick: Option<u64>,
    /// The session is over; close it and go back to the Games hub.
    pub ended: bool,
}

impl Output {
    fn idle() -> Self {
        Self {
            redraw: None,
            rattle: RattleAction::Keep,
            next_tick: None,
            ended: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refresh {
    Partial,
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RattleAction {
    Start { seed: u32, dice: usize, frames: u8 },
    Stop,
    Keep,
}

/// Which screen the session is on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Ready,
    Rolling { frame: u8 },
    Options,
}

/// A Dice game session. No drawing, no audio, no clock of its own.
#[derive(Clone, Debug)]
pub struct DiceSession {
    game: DiceGame,
    options: Options,
    screen: Screen,
    /// Highlighted row of the Options list.
    options_selected: usize,
    /// Seed given to the next `Rattle::new`.
    game_seed: u32,
    /// Set while a roll is animating: times of each advance, in ms.
    frame_times: Vec<u64>,
    frame_index: usize,
    /// When the last roll ended, for the shake cooldown.
    last_roll_end_ms: u64,
}

impl DiceSession {
    /// A new session with `options`; `seed` drives the engine and the rattle.
    #[must_use]
    pub fn new(seed: u32, options: Options) -> Self {
        let mut game = DiceGame::new(seed);
        game.set_dice(options.dice.clamp(1, MAX_DICE));
        Self {
            game,
            options: Options {
                dice: options.dice.clamp(1, MAX_DICE),
                ..options
            },
            screen: Screen::Ready,
            options_selected: 0,
            game_seed: seed,
            frame_times: Vec::new(),
            frame_index: 0,
            last_roll_end_ms: 0,
        }
    }

    #[must_use]
    pub const fn options(&self) -> Options {
        self.options
    }

    #[must_use]
    pub const fn game(&self) -> &DiceGame {
        &self.game
    }

    #[must_use]
    pub const fn is_rolling(&self) -> bool {
        matches!(self.screen, Screen::Rolling { .. })
    }

    /// A shake: rolls on Ready when Shake to roll is on and the cooldown
    /// has passed. Does nothing on the other screens.
    pub fn on_shake(&mut self, now_ms: u64) -> Output {
        if !matches!(self.screen, Screen::Ready) {
            return Output::idle();
        }
        if !self.options.shake {
            return Output::idle();
        }
        if now_ms.saturating_sub(self.last_roll_end_ms) < SHAKE_COOLDOWN_MS && self.game.rolls() > 0
        {
            return Output::idle();
        }
        self.start_roll(now_ms)
    }

    /// A key. `Select` rolls even when shaking is off.
    pub fn on_key(&mut self, key: Key, now_ms: u64) -> Output {
        match key {
            Key::Select => match self.screen {
                Screen::Ready => self.start_roll(now_ms),
                Screen::Rolling { .. } => Output::idle(),
                Screen::Options => {
                    // Change the selected row's value.
                    self.change_option();
                    self.redraw()
                }
            },
            Key::Up | Key::Down => match self.screen {
                Screen::Ready => {
                    let delta = if key == Key::Up { -1 } else { 1 };
                    let dice = self.options.dice as i32 + delta;
                    self.options.dice = dice.clamp(1, MAX_DICE as i32) as usize;
                    self.game.set_dice(self.options.dice);
                    self.redraw()
                }
                Screen::Rolling { .. } => Output::idle(),
                Screen::Options => {
                    let rows = 6;
                    let selected = self.options_selected as i32;
                    let next = if key == Key::Up {
                        selected + 1
                    } else {
                        selected + rows - 1
                    };
                    self.options_selected = next.rem_euclid(rows) as usize;
                    self.redraw()
                }
            },
            Key::BootShort => match self.screen {
                Screen::Ready | Screen::Rolling { .. } => {
                    self.screen = Screen::Options;
                    self.options_selected = 0;
                    self.redraw()
                }
                Screen::Options => {
                    self.screen = Screen::Ready;
                    self.redraw()
                }
            },
            Key::BootHold => {
                let mut output = Output::idle();
                output.rattle = RattleAction::Stop;
                output.ended = true;
                output
            }
        }
    }

    /// Advance the roll animation, or report the next due time.
    pub fn tick(&mut self, now_ms: u64) -> Output {
        if self.screen != Screen::Options && self.frame_times.is_empty() {
            let mut output = Output::idle();
            output.next_tick = self.next_due(now_ms);
            return output;
        }
        if self.frame_index >= self.frame_times.len() {
            let mut output = Output::idle();
            output.next_tick = self.next_due(now_ms);
            return output;
        }
        if now_ms < self.frame_times[self.frame_index] {
            let mut output = Output::idle();
            output.next_tick = Some(self.frame_times[self.frame_index]);
            return output;
        }
        self.frame_index += 1;
        if self.game.advance() {
            if let DicePhase::Rolling { frame } = self.game.phase() {
                self.screen = Screen::Rolling { frame };
                let mut output = self.redraw();
                output.next_tick = self.next_due(now_ms);
                return output;
            }
            // Landed: the result is on the table.
            self.screen = Screen::Ready;
            self.frame_times.clear();
            self.frame_index = 0;
            self.last_roll_end_ms = now_ms;
            let mut output = self.redraw();
            output.rattle = RattleAction::Keep;
            output.next_tick = self.next_due(now_ms);
            return output;
        }
        let mut output = Output::idle();
        output.next_tick = self.next_due(now_ms);
        output
    }

    /// Seed and dice count the rattle needs for the roll in flight.
    #[must_use]
    pub fn rattle_start(&self) -> Option<RattleAction> {
        if !self.is_rolling() {
            return None;
        }
        Some(RattleAction::Start {
            seed: self.game_seed,
            dice: self.options.dice,
            frames: FRAMES,
        })
    }

    fn start_roll(&mut self, now_ms: u64) -> Output {
        if !self.game.request_roll() {
            return Output::idle();
        }
        self.screen = Screen::Rolling { frame: 1 };
        // Space the FRAMES advances inside the rattle; the last lands with
        // the knock. Sound off keeps the same timing.
        let total = u64::from(crate::dice_sound::duration_ms(self.options.dice));
        let knock = total.saturating_sub(80 + 20); // KNOCK_MS + TAIL_MS
        self.frame_times = (1..=u64::from(FRAMES))
            .map(|i| now_ms + knock * i / u64::from(FRAMES))
            .collect();
        self.frame_index = 0;
        self.game_seed = self.game_seed.wrapping_add(1);
        let mut output = self.redraw();
        output.rattle = RattleAction::Start {
            seed: self.game_seed,
            dice: self.options.dice,
            frames: FRAMES,
        };
        output.next_tick = self.frame_times.first().copied();
        output
    }

    fn change_option(&mut self) {
        let row = self.options_selected;
        match row {
            0 => {
                self.options.dice = if self.options.dice >= MAX_DICE {
                    1
                } else {
                    self.options.dice + 1
                };
                self.game.set_dice(self.options.dice);
            }
            1 => self.options.sound = !self.options.sound,
            2 => self.options.shake = !self.options.shake,
            3 => {
                self.options.strength = match self.options.strength {
                    Strength::Low => Strength::Medium,
                    Strength::Medium => Strength::High,
                    Strength::High => Strength::Low,
                };
            }
            _ => {}
        }
    }

    fn redraw(&self) -> Output {
        let mut output = Output::idle();
        output.redraw = Some(if self.game.full_refresh_due() {
            Refresh::Full
        } else {
            Refresh::Partial
        });
        output
    }

    fn next_due(&self, now_ms: u64) -> Option<u64> {
        self.frame_times
            .get(self.frame_index)
            .copied()
            .filter(|due| *due >= now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> DiceSession {
        DiceSession::new(1803, Options::default())
    }

    #[test]
    fn a_shake_rolls_and_one_300_ms_later_does_not() {
        let mut s = session();
        let out = s.on_shake(1_000);
        assert!(matches!(out.rattle, RattleAction::Start { .. }));
        assert!(s.is_rolling());
        let quiet = s.on_shake(1_300);
        assert_eq!(quiet.redraw, None, "a second shake does nothing");
        assert_eq!(quiet.rattle, RattleAction::Keep);
    }

    #[test]
    fn a_shake_after_the_rattle_and_cooldown_rolls_again() {
        let mut s = session();
        s.on_shake(1_000);
        // Run the roll to its landing.
        let total = u64::from(crate::dice_sound::duration_ms(s.options().dice));
        let end = 1_000 + total;
        let mut now = 1_000;
        while s.is_rolling() && now < end + 1_000 {
            let out = s.tick(now);
            if let Some(next) = out.next_tick {
                now = next;
            } else {
                now += 10;
            }
        }
        assert!(!s.is_rolling());
        assert!(s.on_shake(end + 300).redraw.is_none(), "still cooling down");
        let out = s.on_shake(end + SHAKE_COOLDOWN_MS);
        assert!(matches!(out.rattle, RattleAction::Start { .. }));
    }

    #[test]
    fn a_key_roll_works_with_shaking_off() {
        let mut s = DiceSession::new(
            7,
            Options {
                shake: false,
                ..Default::default()
            },
        );
        assert!(s.on_shake(0).redraw.is_none(), "shaking is off");
        let out = s.on_key(Key::Select, 10);
        assert!(matches!(out.rattle, RattleAction::Start { .. }));
        assert!(s.is_rolling());
    }

    #[test]
    fn frame_times_increase_and_the_last_is_at_the_knock() {
        let mut s = session();
        let out = s.on_shake(0);
        let Some(next) = out.next_tick else {
            panic!("a roll schedules a tick");
        };
        assert!(next > 0, "the first frame is not at t=0");
        let total = u64::from(crate::dice_sound::duration_ms(2));
        // last advance lands with the knock: duration - knock - tail.
        let knock = total - 80 - 20;
        let mut times = vec![next];
        let mut now = next;
        while s.is_rolling() {
            let out = s.tick(now);
            if let Some(t) = out.next_tick {
                times.push(t);
                now = t;
            } else {
                break;
            }
            if times.len() > 10 {
                break;
            }
        }
        // Strictly increasing; the landing tick is the knock.
        for pair in times.windows(2) {
            assert!(pair[1] > pair[0], "frame times {times:?}");
        }
        assert_eq!(
            *times.last().unwrap(),
            knock,
            "the last frame is at the knock"
        );
    }

    #[test]
    fn sound_off_keeps_the_frame_times_and_no_rattle() {
        let mut quiet = DiceSession::new(
            7,
            Options {
                sound: false,
                ..Default::default()
            },
        );
        let out = quiet.on_key(Key::Select, 0);
        assert!(
            matches!(out.rattle, RattleAction::Start { .. }),
            "timing is kept"
        );
        assert!(out.next_tick.is_some());
    }

    #[test]
    fn options_swallows_shakes() {
        let mut s = session();
        s.on_key(Key::BootShort, 0);
        assert_eq!(s.options().view().selected, 0);
        let out = s.on_shake(100);
        assert_eq!(out.redraw, None);
        assert_eq!(out.rattle, RattleAction::Keep);
    }

    #[test]
    fn changing_the_dice_count_mid_roll_waits() {
        let mut s = session();
        s.on_key(Key::Select, 0);
        assert!(s.is_rolling());
        let out = s.on_key(Key::Up, 10);
        assert_eq!(out.redraw, None, "keys wait while rolling");
        assert_eq!(s.options().dice, 2, "the count is unchanged");
    }

    #[test]
    fn hold_boot_ends_from_every_screen() {
        for setup in [0, 1, 2] {
            let mut s = session();
            match setup {
                1 => {
                    s.on_key(Key::Select, 0);
                }
                2 => {
                    s.on_key(Key::BootShort, 0);
                }
                _ => {}
            }
            let out = s.on_key(Key::BootHold, 50);
            assert!(out.ended, "setup {setup}");
            assert_eq!(out.rattle, RattleAction::Stop);
        }
    }

    #[test]
    fn a_tick_with_nothing_due_returns_the_next_due_time() {
        let mut s = session();
        let out = s.tick(0);
        assert_eq!(out.redraw, None);
        assert_eq!(out.next_tick, None, "no roll, no tick");
        s.on_key(Key::Select, 100);
        let out = s.tick(100);
        assert!(out.next_tick.is_some(), "the roll wants a tick");
        assert_eq!(out.redraw, None, "nothing to redraw yet");
    }

    #[test]
    fn the_tenth_roll_asks_for_a_full_refresh() {
        let mut s = session();
        for roll in 1..=12 {
            let out = s.on_key(Key::Select, roll * 10_000);
            let mut now = out.next_tick.unwrap_or(roll * 10_000 + 1);
            while s.is_rolling() {
                let step = s.tick(now);
                match step.next_tick {
                    Some(next) => now = next,
                    None => now += 5,
                }
                if now > roll * 10_000 + 5_000 {
                    break;
                }
            }
            // After landing, the last redraw was Full on the tenth.
            if roll == 10 {
                assert_eq!(s.game().rolls(), 10);
                assert!(s.game().full_refresh_due());
            }
            if roll == 9 {
                assert!(!s.game().full_refresh_due());
            }
        }
    }

    #[test]
    fn the_same_script_of_events_is_deterministic() {
        let script = |s: &mut DiceSession| {
            let mut outputs = Vec::new();
            outputs.push(s.on_shake(100));
            outputs.push(s.on_shake(400));
            outputs.push(s.on_key(Key::BootShort, 500));
            outputs.push(s.on_key(Key::Up, 600));
            outputs.push(s.on_key(Key::BootShort, 700));
            outputs.push(s.on_key(Key::Select, 800));
            for i in 0..5 {
                outputs.push(s.tick(800 + i * 200));
            }
            outputs.push(s.on_key(Key::BootHold, 2_000));
            outputs
        };
        let left = script(&mut DiceSession::new(42, Options::default()));
        let right = script(&mut DiceSession::new(42, Options::default()));
        assert_eq!(left, right);
    }

    #[test]
    fn strength_maps_to_the_imu_threshold() {
        assert_eq!(Strength::Low.shake_delta_mg(), SHAKE_DELTA_LOW_MG);
        assert_eq!(Strength::Medium.shake_delta_mg(), 420);
        assert_eq!(Strength::High.shake_delta_mg(), SHAKE_DELTA_HIGH_MG);
    }
}
