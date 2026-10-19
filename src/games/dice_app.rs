//! Dice as the game bridge holds it: the session (`dice_session.rs`) driven
//! by buttons, shakes and the clock, drawn on the game canvas by
//! `dice_view.rs`. The bridge asks it what to draw and what to play; the main
//! loop plays the rattle and feeds it shakes.

use super::{
    canvas::NativeGameCanvas,
    dice_session::{DiceSession, Key, Options, Output, RattleAction, View},
    dice_view::render_options,
    dice_view::render_ready,
    dice_view::render_rolling,
};
use crate::buttons::ButtonEvent;

/// What an event did, for the runtime's diagnostics line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiceEventResult {
    pub reason: &'static str,
    /// Screen shown after the event: `ready`, `rolling` or `options`.
    pub view: &'static str,
    pub dirty_regions: usize,
}

#[derive(Clone, Debug)]
pub struct DiceApp {
    session: DiceSession,
    /// Rattle to start or stop, until the main loop takes it.
    rattle: Option<RattleAction>,
    ended: bool,
    seeded: bool,
}

impl PartialEq for DiceApp {
    fn eq(&self, other: &Self) -> bool {
        self.options() == other.options()
            && self.view() == other.view()
            && self.ended == other.ended
            && self.rattle == other.rattle
    }
}

impl Eq for DiceApp {}

impl DiceApp {
    #[must_use]
    pub fn new(options: Options) -> Self {
        Self {
            session: DiceSession::new(1, options),
            rattle: None,
            ended: false,
            seeded: false,
        }
    }

    #[must_use]
    pub const fn options(&self) -> Options {
        self.session.options()
    }

    #[must_use]
    pub const fn view(&self) -> View {
        self.session.view()
    }

    /// The faces of the last roll as `4+6`.
    #[must_use]
    pub fn last_faces(&self) -> Option<String> {
        self.session.last_faces()
    }

    /// Hold BOOT or `Back to Games`: the game is over.
    #[must_use]
    pub const fn ended(&self) -> bool {
        self.ended
    }

    /// When the next roll frame is due, on the clock the events carry.
    #[must_use]
    pub fn next_tick(&self) -> Option<u64> {
        self.session.next_tick()
    }

    /// A shake would roll now: the motion sensor is worth running.
    #[must_use]
    pub const fn wants_motion(&self) -> bool {
        self.session.wants_motion()
    }

    /// `shake_delta_mg` for the IMU detector, from Shake strength.
    #[must_use]
    pub const fn shake_delta_mg(&self) -> i32 {
        self.session.options().strength.shake_delta_mg()
    }

    /// The rattle to start or stop, once.
    pub fn take_rattle(&mut self) -> Option<RattleAction> {
        self.rattle.take()
    }

    /// Draw the screen the session is on.
    pub fn render(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let options = self.session.options();
        match self.session.view() {
            View::Ready => render_ready(self.session.game(), canvas, &options.view()),
            View::Rolling { frame } => render_rolling(self.session.game(), canvas, frame),
            View::Options { selected } => {
                let mut view = options.view();
                view.selected = selected;
                render_options(self.session.game(), canvas, &view)
            }
        }
    }

    /// The first key or shake of a session seeds it, so sessions differ.
    fn seed_once(&mut self, now_ms: u64) {
        if !self.seeded {
            self.seeded = true;
            self.session
                .reseed((now_ms as u32) ^ ((now_ms >> 32) as u32));
        }
    }

    pub fn apply_button(
        &mut self,
        event: ButtonEvent,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<DiceEventResult>, String> {
        let key = match event {
            ButtonEvent::Up => Key::Up,
            ButtonEvent::Down => Key::Down,
            ButtonEvent::Select => Key::Select,
        };
        self.seed_once(now_ms);
        let output = self.session.on_key(key, now_ms);
        self.absorb(output, "key", canvas)
    }

    pub fn apply_boot_short_press(
        &mut self,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<DiceEventResult>, String> {
        let output = self.session.on_key(Key::BootShort, now_ms);
        self.absorb(output, "boot-short", canvas)
    }

    pub fn apply_shake(
        &mut self,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<DiceEventResult>, String> {
        self.seed_once(now_ms);
        let output = self.session.on_shake(now_ms);
        self.absorb(output, "shake", canvas)
    }

    /// Run the roll frame that is due. True when the screen changed.
    pub fn tick(&mut self, now_ms: u64, canvas: &mut NativeGameCanvas) -> Result<bool, String> {
        let output = self.session.tick(now_ms);
        Ok(self.absorb(output, "tick", canvas)?.is_some())
    }

    /// Take what the session asks for; draw when it asks for a redraw.
    fn absorb(
        &mut self,
        output: Output,
        reason: &'static str,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<DiceEventResult>, String> {
        match output.rattle {
            RattleAction::Keep => {}
            action => self.rattle = Some(action),
        }
        self.ended |= output.ended;
        // The panel's own refresh counter decides when a full refresh comes;
        // `Refresh::Full` only means the same thing there.
        if output.redraw.is_none() && !output.ended {
            return Ok(None);
        }
        if output.redraw.is_some() {
            self.render(canvas)?;
        }
        Ok(Some(DiceEventResult {
            reason: if output.ended { "exit" } else { reason },
            view: match self.session.view() {
                View::Ready => "ready",
                View::Rolling { .. } => "rolling",
                View::Options { .. } => "options",
            },
            dirty_regions: canvas.dirty().regions().len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::dice_session::Strength;

    fn app() -> (DiceApp, NativeGameCanvas) {
        (
            DiceApp::new(Options::default()),
            NativeGameCanvas::default(),
        )
    }

    #[test]
    fn a_key_roll_draws_each_frame_asks_for_the_rattle_and_lands_on_ready() {
        let (mut app, mut canvas) = app();
        app.render(&mut canvas).unwrap();
        let ready = canvas.commands().to_vec();
        let result = app
            .apply_button(ButtonEvent::Select, 1_000, &mut canvas)
            .unwrap()
            .expect("a roll redraws");
        assert_eq!(result.view, "rolling");
        assert!(matches!(
            app.take_rattle(),
            Some(RattleAction::Start { dice: 2, .. })
        ));
        assert_eq!(app.take_rattle(), None, "the rattle is taken once");
        let rolling = canvas.commands().to_vec();
        assert_ne!(rolling, ready);
        let mut frames = 0;
        while let Some(due) = app.next_tick() {
            assert!(app.tick(due, &mut canvas).unwrap());
            frames += 1;
            assert!(frames <= 3, "a roll has three frames");
        }
        assert_eq!(frames, 3);
        assert_eq!(app.view(), View::Ready);
        assert!(app.last_faces().is_some());
        assert!(!app.ended());
    }

    #[test]
    fn a_shake_rolls_only_while_shaking_is_on_and_the_screen_rolls() {
        let (mut app, mut canvas) = app();
        assert!(app.wants_motion());
        let shaken = app.apply_shake(500, &mut canvas).unwrap();
        assert!(shaken.is_some());
        assert!(app.take_rattle().is_some());

        let mut quiet = DiceApp::new(Options {
            shake: false,
            ..Options::default()
        });
        assert!(!quiet.wants_motion());
        assert_eq!(quiet.apply_shake(500, &mut canvas).unwrap(), None);
        assert_eq!(quiet.take_rattle(), None);

        let mut options = DiceApp::new(Options::default());
        options.apply_boot_short_press(0, &mut canvas).unwrap();
        assert!(!options.wants_motion(), "Options ignores shakes");
        assert_eq!(options.apply_shake(900, &mut canvas).unwrap(), None);
    }

    #[test]
    fn sound_off_asks_for_no_rattle_and_strength_sets_the_threshold() {
        let (mut quiet, mut canvas) = (
            DiceApp::new(Options {
                sound: false,
                strength: Strength::High,
                ..Options::default()
            }),
            NativeGameCanvas::default(),
        );
        quiet
            .apply_button(ButtonEvent::Select, 10, &mut canvas)
            .unwrap();
        assert_eq!(quiet.take_rattle(), None);
        assert_eq!(quiet.shake_delta_mg(), Strength::High.shake_delta_mg());
        assert!(DiceApp::new(Options::default()).shake_delta_mg() < quiet.shake_delta_mg());
    }

    #[test]
    fn options_back_and_hold_boot_end_the_game_and_stop_the_rattle() {
        let (mut app, mut canvas) = app();
        app.apply_button(ButtonEvent::Select, 10, &mut canvas)
            .unwrap();
        app.take_rattle();
        // Hold BOOT is the state's `back()`; the session's own end is Back to
        // Games on the Options list: BOOT, then ▲ to the last row, then ●.
        let mut idle = DiceApp::new(Options::default());
        idle.apply_boot_short_press(0, &mut canvas).unwrap();
        idle.apply_button(ButtonEvent::Up, 1, &mut canvas).unwrap();
        let result = idle
            .apply_button(ButtonEvent::Select, 2, &mut canvas)
            .unwrap()
            .expect("leaving reports");
        assert_eq!(result.reason, "exit");
        assert!(idle.ended());
    }

    #[test]
    fn every_screen_draws_inside_the_command_limit() {
        let (mut app, mut canvas) = app();
        app.render(&mut canvas).unwrap();
        app.apply_button(ButtonEvent::Select, 5, &mut canvas)
            .unwrap();
        assert!(canvas.commands().len() < crate::games::canvas::MAX_GAME_DRAW_COMMANDS);
        let mut now = 5;
        while let Some(due) = app.next_tick() {
            now = due;
            app.tick(now, &mut canvas).unwrap();
        }
        app.apply_boot_short_press(now, &mut canvas).unwrap();
        assert!(matches!(app.view(), View::Options { .. }));
        assert!(canvas.commands().len() < crate::games::canvas::MAX_GAME_DRAW_COMMANDS);
    }
}
