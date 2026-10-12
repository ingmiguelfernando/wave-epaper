//! Race: a lane game that suits e-paper, one screen change per step.
//!
//! The road runs from right to left: obstacles arrive in the last column and
//! move one column toward the player each step, 20 metres a step. The engine
//! holds no drawing; `tetris.rs` renders the sibling game the same way.

use super::{
    canvas::{CanvasTextStyle, GameSprite, NativeGameCanvas},
    dirty_regions::{DirtyRect, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH},
};
use crate::buttons::ButtonEvent;

/// Lanes of the road, top to bottom.
pub const RACE_LANES: usize = 4;
/// Columns of the road; obstacles arrive in the last one.
pub const RACE_COLUMNS: usize = 6;
/// The player's fixed column: one column behind it is the exit.
pub const PLAYER_COLUMN: usize = 1;
/// Metres travelled by one clear step.
pub const STEP_METRES: u32 = 20;
/// Nitro takes this many steps at once.
const NITRO_STEPS: u32 = 3;
/// Charges the player can hold, and the one the game starts with.
const MAX_NITRO: u8 = 3;
/// Classic steps once a second at speed 1; each level takes 100 ms off.
const BASE_INTERVAL_MS: u64 = 1000;
const LEVEL_DROP_MS: u64 = 100;
const MIN_INTERVAL_MS: u64 = 500;
/// xorshift keeps zero fixed, so a zero seed starts from this constant.
const ZERO_SEED: u32 = 0x9E37_79B9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaceMode {
    Zen,
    Classic,
}

/// What the keys do. In Zen, every one of them also steps the road.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaceAction {
    Up,
    Down,
    Nitro,
}

impl RaceMode {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Zen => "zen",
            Self::Classic => "classic",
        }
    }
}

impl RaceAction {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Up => "lane-up",
            Self::Down => "lane-down",
            Self::Nitro => "nitro",
        }
    }
}

/// What sits in a road cell: another car or a cone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaceObstacle {
    Car,
    Cone,
}

/// One road: four lanes and six columns, the player fixed in `PLAYER_COLUMN`.
/// Every seeded game draws the same road, as `tetris.rs` deals the same pieces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RaceGame {
    mode: RaceMode,
    grid: [[Option<RaceObstacle>; RACE_COLUMNS]; RACE_LANES],
    player_lane: usize,
    /// The lane the column before left free, so the next free lane is in or
    /// next to it and a careful driver can always get through.
    last_free_lane: usize,
    distance_m: u32,
    cars_passed: u32,
    nitro: u8,
    nitro_used: u32,
    /// The kilometres already counted, for the per-kilometre speed and nitro.
    km_seen: u32,
    game_over: bool,
    rng: u32,
}

impl RaceGame {
    #[must_use]
    pub fn new(mode: RaceMode, seed: u32) -> Self {
        Self {
            mode,
            grid: [[None; RACE_COLUMNS]; RACE_LANES],
            player_lane: RACE_LANES / 2,
            last_free_lane: RACE_LANES / 2,
            distance_m: 0,
            cars_passed: 0,
            nitro: 1,
            nitro_used: 0,
            km_seen: 0,
            game_over: false,
            rng: if seed == 0 { ZERO_SEED } else { seed },
        }
    }

    #[must_use]
    pub const fn mode(&self) -> RaceMode {
        self.mode
    }

    /// Applies one action and reports whether anything moved. Every action is
    /// a no-op once the game is over.
    ///
    /// Zen: ▲▼ change a lane and then step the road, ● steps in the same
    /// lane. Classic: ▲▼ only change a lane and ● spends one nitro charge
    /// for three steps at once, each checked.
    pub fn apply(&mut self, action: RaceAction) -> bool {
        if self.game_over {
            return false;
        }
        match (self.mode, action) {
            (RaceMode::Classic, RaceAction::Up) => self.change_lane(-1),
            (RaceMode::Classic, RaceAction::Down) => self.change_lane(1),
            (RaceMode::Classic, RaceAction::Nitro) => {
                if self.nitro == 0 {
                    return false;
                }
                self.nitro -= 1;
                self.nitro_used += 1;
                for _ in 0..NITRO_STEPS {
                    self.step();
                }
                true
            }
            (RaceMode::Zen, RaceAction::Up) => self.change_lane(-1) | self.step(),
            (RaceMode::Zen, RaceAction::Down) => self.change_lane(1) | self.step(),
            (RaceMode::Zen, RaceAction::Nitro) => self.step(),
        }
    }

    /// One timed step. Zen ignores it and a finished game ignores it, so the
    /// caller's timer only runs for Classic.
    pub fn tick(&mut self) -> bool {
        if self.mode != RaceMode::Classic || self.game_over {
            return false;
        }
        self.step()
    }

    /// The timer's interval at the current speed: 1000 ms at speed 1, 100 ms
    /// less each kilometre, down to 500 ms. `None` without a timer (Zen) and
    /// once the game is over.
    #[must_use]
    pub fn tick_interval_ms(&self) -> Option<u64> {
        if self.mode != RaceMode::Classic || self.game_over {
            return None;
        }
        let drop = u64::from(self.speed().saturating_sub(1)) * LEVEL_DROP_MS;
        Some(BASE_INTERVAL_MS.saturating_sub(drop).max(MIN_INTERVAL_MS))
    }

    /// What sits in a cell: `None` when empty or out of bounds. The player's
    /// own car is not an obstacle; `player_lane` draws it.
    #[must_use]
    pub fn cell(&self, lane: usize, column: usize) -> Option<RaceObstacle> {
        self.grid
            .get(lane)
            .and_then(|line| line.get(column))
            .copied()
            .flatten()
    }

    #[must_use]
    pub const fn player_lane(&self) -> usize {
        self.player_lane
    }

    /// Distance travelled, in metres: 20 m for every clear step.
    #[must_use]
    pub const fn distance_m(&self) -> u32 {
        self.distance_m
    }

    /// Speed in levels: one more each kilometre.
    #[must_use]
    pub const fn speed(&self) -> u32 {
        1 + self.distance_m / 1000
    }

    #[must_use]
    pub const fn cars_passed(&self) -> u32 {
        self.cars_passed
    }

    #[must_use]
    pub const fn nitro_used(&self) -> u32 {
        self.nitro_used
    }

    #[must_use]
    pub const fn nitro_charges(&self) -> u8 {
        self.nitro
    }

    #[must_use]
    pub const fn is_over(&self) -> bool {
        self.game_over
    }

    /// One step: the road moves a column toward the player and a new column
    /// arrives. A step that brings an obstacle into the player's cell is a
    /// crash, and then nothing else happens.
    fn step(&mut self) -> bool {
        if self.game_over {
            return false;
        }
        self.advance_road();
        if self.grid[self.player_lane][PLAYER_COLUMN].is_some() {
            self.game_over = true;
            return true;
        }
        self.distance_m = self.distance_m.saturating_add(STEP_METRES);
        let km = self.distance_m / 1000;
        if km > self.km_seen {
            self.nitro = self
                .nitro
                .saturating_add((km - self.km_seen) as u8)
                .min(MAX_NITRO);
            self.km_seen = km;
        }
        self.spawn_column();
        true
    }

    /// Move every obstacle one column toward the player. One that reaches the
    /// player's cell is the crash the caller sees; one that reaches the
    /// player's column in another lane has got past and leaves the road — a
    /// car counts as passed — so the road beside the player is always clear
    /// to dodge into and a careful driver gets through.
    fn advance_road(&mut self) {
        for lane in 0..RACE_LANES {
            for column in 0..RACE_COLUMNS {
                let Some(obstacle) = self.grid[lane][column] else {
                    continue;
                };
                self.grid[lane][column] = None;
                match column {
                    0..=PLAYER_COLUMN => {
                        if obstacle == RaceObstacle::Car {
                            self.cars_passed += 1;
                        }
                    }
                    _ if column - 1 == PLAYER_COLUMN => {
                        if lane == self.player_lane {
                            self.grid[lane][PLAYER_COLUMN] = Some(obstacle);
                        } else if obstacle == RaceObstacle::Car {
                            self.cars_passed += 1;
                        }
                    }
                    _ => self.grid[lane][column - 1] = Some(obstacle),
                }
            }
        }
    }

    /// A new column arrives in the last column. It leaves at least one lane
    /// free, and that free lane is in or next to a lane the column before
    /// left free, so the gap the driver follows never jumps. The other lanes
    /// carry an obstacle now and then, as sparse as the mockup's road.
    fn spawn_column(&mut self) {
        let low = self.last_free_lane.saturating_sub(1);
        let high = (self.last_free_lane + 1).min(RACE_LANES - 1);
        let options = high - low + 1;
        let free = low + (self.next_random() % options as u32) as usize;
        self.last_free_lane = free;
        for lane in 0..RACE_LANES {
            if lane == free || self.next_random() % 4 != 0 {
                continue;
            }
            self.grid[lane][RACE_COLUMNS - 1] = Some(if self.next_random() % 2 == 0 {
                RaceObstacle::Car
            } else {
                RaceObstacle::Cone
            });
        }
    }

    /// Change one lane at once. The destination cell must be empty: driving
    /// into a car or a cone is a crash.
    fn change_lane(&mut self, direction: i32) -> bool {
        let target = self.player_lane as i32 + direction;
        if !(0..RACE_LANES as i32).contains(&target) {
            return false;
        }
        let target = target as usize;
        if self.grid[target][PLAYER_COLUMN].is_some() {
            self.game_over = true;
            return true;
        }
        self.player_lane = target;
        true
    }

    /// The same seeded generator `tetris.rs` deals its pieces with.
    fn next_random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    /// Test seam: put an obstacle in a cell.
    #[cfg(test)]
    fn place(&mut self, lane: usize, column: usize, obstacle: RaceObstacle) {
        self.grid[lane][column] = Some(obstacle);
    }
}

/// The road and the player as one action found them, for the redraw that
/// follows: the cells that changed and the player's old lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RoadSnapshot {
    grid: [[Option<RaceObstacle>; RACE_COLUMNS]; RACE_LANES],
    player_lane: usize,
    steps: u32,
}

/// What one key press or step did, for the device log and the redraw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RaceEventResult {
    pub reason: &'static str,
    pub lane: usize,
    pub column: usize,
    pub mode: RaceMode,
    pub action: Option<RaceAction>,
    pub distance_m: u32,
    pub speed: u32,
    pub cars_passed: u32,
    pub nitro_used: u32,
    pub completed: bool,
    pub dirty_regions: Vec<DirtyRect>,
}

/// The mode list Race opens on: Classic and Zen, each with its best distance.
/// Choosing a row starts that mode; the list holds no game of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RaceStart {
    classic_best: u32,
    zen_best: u32,
    cursor: usize,
}

impl RaceStart {
    #[must_use]
    pub const fn new(classic_best: u32, zen_best: u32) -> Self {
        Self {
            classic_best,
            zen_best,
            cursor: 0,
        }
    }

    /// The saved bests, handed in when the list opens.
    pub fn set_bests(&mut self, classic_best: u32, zen_best: u32) {
        self.classic_best = classic_best;
        self.zen_best = zen_best;
    }

    /// The rows, in order. A mode without a best yet says so.
    #[must_use]
    pub fn options(&self) -> [(RaceMode, String); 2] {
        [
            (RaceMode::Classic, best_label("Classic", self.classic_best)),
            (RaceMode::Zen, best_label("Zen", self.zen_best)),
        ]
    }

    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// ▲▼ move the highlight, wrapping over the two modes.
    pub fn move_cursor(&mut self, direction: i32) {
        self.cursor = if direction < 0 {
            self.cursor.checked_sub(1).unwrap_or(1)
        } else {
            (self.cursor + 1) % 2
        };
    }

    /// ● on a row: the mode it names.
    #[must_use]
    pub const fn chosen(&self) -> RaceMode {
        if self.cursor == 0 {
            RaceMode::Classic
        } else {
            RaceMode::Zen
        }
    }

    /// The saved best of `mode`, for the game the list starts.
    #[must_use]
    pub const fn best(&self, mode: RaceMode) -> u32 {
        match mode {
            RaceMode::Classic => self.classic_best,
            RaceMode::Zen => self.zen_best,
        }
    }

    /// One boxed row per mode, the highlighted one inverted, as Tetris' list.
    pub fn render(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.rect(0, 0, GAME_CANVAS_WIDTH, RACE_BAR_HEIGHT, true)?;
        canvas.text(16, 30, "Race".into(), CanvasTextStyle::Inverse)?;
        for (index, (_, label)) in self.options().iter().enumerate() {
            let top = 140 + index as i32 * 64;
            let selected = index == self.cursor;
            canvas.rect(24, top, 432, 52, selected)?;
            let style = if selected {
                CanvasTextStyle::Inverse
            } else {
                CanvasTextStyle::Body
            };
            canvas.text(44, top + 34, label.clone(), style)?;
        }
        canvas.text(
            24,
            320,
            "The road runs right to left: change lanes".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.text(
            24,
            344,
            "to pass. Nitro takes three steps at once.".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.text(
            24,
            368,
            "Zen steps only when you press.".into(),
            CanvasTextStyle::Detail,
        )?;
        canvas.request_refresh();
        Ok(())
    }
}

/// The Race game on the canvas: the mockup's playing screen, the crash
/// screen and their redraws. The road is drawn once; a step redraws only
/// the cells whose content changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RaceApp {
    game: RaceGame,
    seed: u32,
    best: u32,
    top_speed: u32,
    steps: u32,
    paused: bool,
    /// Highlighted row of the crash screen's list.
    crash_cursor: usize,
    /// Set by the crash screen's `Back to Games` row.
    exit_requested: bool,
    /// Set by the crash screen's `Mode` row: the mode list opens again.
    mode_requested: bool,
}

impl RaceApp {
    #[must_use]
    pub fn new(mode: RaceMode, seed: u32) -> Self {
        Self {
            game: RaceGame::new(mode, seed),
            seed,
            best: 0,
            top_speed: 1,
            steps: 0,
            paused: false,
            crash_cursor: 0,
            exit_requested: false,
            mode_requested: false,
        }
    }

    #[must_use]
    pub const fn game(&self) -> &RaceGame {
        &self.game
    }

    #[must_use]
    pub const fn best(&self) -> u32 {
        self.best
    }

    /// Seed the in-memory best from the saved records when a session opens.
    pub fn set_best(&mut self, best: u32) {
        self.best = self.best.max(best);
    }

    /// The crash screen's `Back to Games` row asked to leave.
    #[must_use]
    pub const fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// The crash screen's `Mode` row asked for the mode list again.
    #[must_use]
    pub const fn mode_requested(&self) -> bool {
        self.mode_requested
    }

    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn render_initial(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        canvas.invalidate_rect(RACE_FULL_RECT);
        canvas.request_refresh();
        Ok(())
    }

    /// ▲▼ change a lane, ● spends a nitro charge (Classic) or steps (Zen).
    /// On the crash screen ▲▼ choose a row and ● runs it.
    pub fn apply_button_and_render(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<RaceEventResult, String> {
        let before = self.snapshot();
        if self.game.is_over() {
            return self.apply_crash_menu(event, canvas);
        }
        if self.paused {
            // While paused only ● resumes; the road stands still.
            if event == ButtonEvent::Select {
                self.paused = false;
            }
            return self.finish_and_render("pause-toggle", None, &before, canvas);
        }
        let action = match event {
            ButtonEvent::Up => RaceAction::Up,
            ButtonEvent::Down => RaceAction::Down,
            ButtonEvent::Select => RaceAction::Nitro,
        };
        let moved = self.game.apply(action);
        let road_steps = match (moved, action) {
            (false, _) => 0,
            (true, RaceAction::Nitro) if self.game.mode() == RaceMode::Classic => 3,
            (true, _) if self.game.mode() == RaceMode::Zen => 1,
            (true, _) => 0,
        };
        self.note_progress(road_steps);
        let reason = match (moved, action) {
            (false, RaceAction::Nitro) if self.game.mode() == RaceMode::Classic => "no-nitro",
            (false, _) => "blocked",
            (true, RaceAction::Up) => "lane-up",
            (true, RaceAction::Down) => "lane-down",
            (true, RaceAction::Nitro) if self.game.mode() == RaceMode::Classic => "nitro",
            (true, RaceAction::Nitro) => "step",
        };
        let reason = if self.game.is_over() {
            "game-over"
        } else {
            reason
        };
        self.finish_and_render(reason, Some(action), &before, canvas)
    }

    /// A short BOOT press pauses Classic (the step clock stops and `PAUSED`
    /// shows) and steps Zen one column. On the crash screen it does nothing;
    /// hold BOOT leaves, as in Tetris.
    pub fn apply_boot_short_press_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<RaceEventResult, String> {
        let before = self.snapshot();
        if self.game.is_over() {
            return self.finish_and_render("game-finished", None, &before, canvas);
        }
        match self.game.mode() {
            RaceMode::Classic => {
                self.paused = !self.paused;
                self.finish_and_render("pause-toggle", None, &before, canvas)
            }
            RaceMode::Zen => {
                self.game.tick();
                self.note_progress(1);
                let reason = if self.game.is_over() {
                    "game-over"
                } else {
                    "step"
                };
                self.finish_and_render(reason, None, &before, canvas)
            }
        }
    }

    /// The result reported when the mode list starts this game.
    #[must_use]
    pub fn started_result(&self) -> RaceEventResult {
        RaceEventResult {
            reason: "start-choose",
            lane: self.game.player_lane(),
            column: PLAYER_COLUMN,
            mode: self.game.mode(),
            action: None,
            distance_m: self.game.distance_m(),
            speed: self.game.speed(),
            cars_passed: self.game.cars_passed(),
            nitro_used: self.game.nitro_used(),
            completed: self.game.is_over(),
            dirty_regions: vec![RACE_FULL_RECT],
        }
    }

    /// One timed step, redrawn like a key press. `None` when the tick changed
    /// nothing (Zen, a paused or finished game), so the caller skips a redraw.
    pub fn tick_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<RaceEventResult>, String> {
        if self.paused {
            return Ok(None);
        }
        let before = self.snapshot();
        if !self.game.tick() {
            return Ok(None);
        }
        self.note_progress(1);
        let reason = if self.game.is_over() {
            "game-over"
        } else {
            "step"
        };
        self.finish_and_render(reason, None, &before, canvas)
            .map(Some)
    }

    /// The crash screen's rows: ▲▼ choose, ● runs the highlighted one.
    fn apply_crash_menu(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<RaceEventResult, String> {
        let before = self.snapshot();
        let reason = match event {
            ButtonEvent::Up => {
                self.crash_cursor = self.crash_cursor.checked_sub(1).unwrap_or(2);
                "crash-choose"
            }
            ButtonEvent::Down => {
                self.crash_cursor = (self.crash_cursor + 1) % 3;
                "crash-choose"
            }
            ButtonEvent::Select => match self.crash_cursor {
                0 => {
                    self.seed = self.seed.wrapping_add(1);
                    self.game = RaceGame::new(self.game.mode(), self.seed);
                    self.steps = 0;
                    self.top_speed = 1;
                    self.paused = false;
                    self.crash_cursor = 0;
                    "new-game"
                }
                1 => {
                    self.mode_requested = true;
                    "mode-list"
                }
                _ => {
                    self.exit_requested = true;
                    "back-to-games"
                }
            },
        };
        self.finish_and_render(reason, None, &before, canvas)
    }

    /// Keep the top speed and the best distance as the game runs, and count
    /// the road steps taken, for the refresh policy.
    fn note_progress(&mut self, road_steps: u32) {
        self.steps = self.steps.saturating_add(road_steps);
        self.top_speed = self.top_speed.max(self.game.speed());
        self.best = self.best.max(self.game.distance_m());
    }

    /// The road and the player as one action found them.
    fn snapshot(&self) -> RoadSnapshot {
        let mut grid = [[None; RACE_COLUMNS]; RACE_LANES];
        for lane in 0..RACE_LANES {
            for column in 0..RACE_COLUMNS {
                grid[lane][column] = self.game.cell(lane, column);
            }
        }
        RoadSnapshot {
            grid,
            player_lane: self.game.player_lane(),
            steps: self.steps,
        }
    }

    fn finish_and_render(
        &mut self,
        reason: &'static str,
        action: Option<RaceAction>,
        before: &RoadSnapshot,
        canvas: &mut NativeGameCanvas,
    ) -> Result<RaceEventResult, String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = self.dirty_regions(reason, before);
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(RaceEventResult {
            reason,
            lane: self.game.player_lane(),
            column: PLAYER_COLUMN,
            mode: self.game.mode(),
            action,
            distance_m: self.game.distance_m(),
            speed: self.game.speed(),
            cars_passed: self.game.cars_passed(),
            nitro_used: self.game.nitro_used(),
            completed: self.game.is_over(),
            dirty_regions,
        })
    }

    /// Only the cells whose content changed are redrawn, plus the counters; a
    /// lane change moves the player's car, its streaks and the move hint, and
    /// every 30 road steps a full refresh clears ghosting.
    fn dirty_regions(&self, reason: &'static str, before: &RoadSnapshot) -> Vec<DirtyRect> {
        if matches!(
            reason,
            "new-game" | "mode-list" | "back-to-games" | "start-choose" | "game-over"
        ) {
            return vec![RACE_FULL_RECT];
        }
        if reason == "pause-toggle" {
            return vec![RACE_PAUSE_RECT];
        }
        if before.steps / RACE_REFRESH_STEPS != self.steps / RACE_REFRESH_STEPS {
            return vec![RACE_FULL_RECT];
        }
        let mut regions = vec![RACE_STATS_RECT];
        for lane in 0..RACE_LANES {
            for column in 0..RACE_COLUMNS {
                if before.grid[lane][column] != self.game.cell(lane, column) {
                    regions.push(cell_rect(lane, column));
                }
            }
        }
        if before.player_lane != self.game.player_lane() {
            for lane in [before.player_lane, self.game.player_lane()] {
                regions.push(cell_rect(lane, PLAYER_COLUMN));
                regions.push(cell_rect(lane, PLAYER_COLUMN - 1));
                if lane > 0 {
                    regions.push(cell_rect(lane - 1, PLAYER_COLUMN));
                }
            }
        }
        regions
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        if self.game.is_over() {
            return self.render_crash(canvas);
        }
        canvas.clear_frame();
        canvas.rect(0, 0, GAME_CANVAS_WIDTH, RACE_BAR_HEIGHT, true)?;
        let title = match self.game.mode() {
            RaceMode::Classic => "Race · Classic",
            RaceMode::Zen => "Race · Zen",
        };
        canvas.text(16, 30, title.into(), CanvasTextStyle::Inverse)?;
        let speed = format!("Speed {}", self.game.speed());
        canvas.text_right(464, 30, speed, CanvasTextStyle::Inverse)?;

        self.draw_counters(canvas)?;
        self.draw_road(canvas)?;

        // SPEED 4 AT 3.0 KM and the bar toward the next kilometre.
        let next_km = (self.game.distance_m() / 1000 + 1) * 1000;
        let speed_line = format!(
            "SPEED {} AT {}",
            self.game.speed() + 1,
            distance_label(next_km).to_ascii_uppercase()
        );
        canvas.text(16, 608, speed_line, CanvasTextStyle::Detail)?;
        let into_km = self.game.distance_m() % 1000;
        canvas.rect(16, 616, 448, 14, true)?;
        canvas.paper_rect(18, 618, 444, 10, true)?;
        let fill = 444 * into_km as i32 / 1000;
        if fill > 0 {
            canvas.rect(18, 618, fill, 10, true)?;
        }

        // The hub's info box, wrapped by the screen with the real font.
        let info = match self.game.mode() {
            RaceMode::Classic => {
                "One step a second at first, faster every kilometre. \
                 A full refresh now and then clears ghosting."
            }
            RaceMode::Zen => {
                "The road moves only when you press. \
                 A full refresh now and then clears ghosting."
            }
        };
        canvas.info_box(642, info.into())?;

        if self.paused {
            canvas.rect(120, 300, 240, 80, true)?;
            canvas.text(176, 352, "PAUSED".into(), CanvasTextStyle::InverseHeading)?;
        }
        Ok(())
    }

    /// Distance, best and the charges: three columns as the mockup's, the
    /// labels in capitals at the Detail size and the values in Heading under
    /// them. Zen has no nitro: its third column counts the cars passed.
    fn draw_counters(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.text(16, 66, "DISTANCE".into(), CanvasTextStyle::Detail)?;
        canvas.text(
            16,
            102,
            distance_label(self.game.distance_m()),
            CanvasTextStyle::Heading,
        )?;
        canvas.text(232, 66, "BEST".into(), CanvasTextStyle::Detail)?;
        canvas.text(
            232,
            102,
            distance_label(self.best),
            CanvasTextStyle::Heading,
        )?;
        match self.game.mode() {
            RaceMode::Classic => {
                canvas.text_right(464, 66, "NITRO".into(), CanvasTextStyle::Detail)?;
                for charge in 0..3_u32 {
                    let x = 416 + charge as i32 * 17;
                    if charge < u32::from(self.game.nitro_charges()) {
                        canvas.rect(x, 82, 14, 16, true)?;
                    } else {
                        canvas.rect(x, 82, 14, 16, true)?;
                        canvas.paper_rect(x + 2, 84, 10, 12, true)?;
                    }
                }
            }
            RaceMode::Zen => {
                canvas.text_right(464, 66, "CARS PASSED".into(), CanvasTextStyle::Detail)?;
                canvas.text_right(
                    464,
                    102,
                    self.game.cars_passed().to_string(),
                    CanvasTextStyle::Heading,
                )?;
            }
        }
        Ok(())
    }

    /// The road as the mockup's SVG: rumble strips of 16 px blocks every 32
    /// px, a 3 px edge line inside each, lane lines of 30 × 4 px dashes
    /// every 52 px. Other cars are outlines; the player's car is solid. The
    /// sprites stay inside their cells, so the cells a step dirties cover
    /// what changed.
    fn draw_road(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        for x in (ROAD_X..ROAD_X + ROAD_WIDTH).step_by(32) {
            let width = 16.min(ROAD_X + ROAD_WIDTH - x);
            canvas.rect(x, ROAD_TOP, width, RUMBLE_HEIGHT, true)?;
            canvas.rect(x, ROAD_BOTTOM - RUMBLE_HEIGHT, width, RUMBLE_HEIGHT, true)?;
        }
        canvas.rect(
            ROAD_X,
            ROAD_TOP + RUMBLE_HEIGHT,
            ROAD_WIDTH,
            EDGE_HEIGHT,
            true,
        )?;
        canvas.rect(
            ROAD_X,
            ROAD_BOTTOM - RUMBLE_HEIGHT - EDGE_HEIGHT,
            ROAD_WIDTH,
            EDGE_HEIGHT,
            true,
        )?;
        for y in LANE_LINES {
            for x in (ROAD_X..ROAD_X + ROAD_WIDTH).step_by(52) {
                let width = 30.min(ROAD_X + ROAD_WIDTH - x);
                canvas.rect(x, y, width, 4, true)?;
            }
        }
        for lane in 0..RACE_LANES {
            for column in 0..RACE_COLUMNS {
                match self.game.cell(lane, column) {
                    Some(RaceObstacle::Car) => {
                        let (x, y) = sprite_origin(lane, column, GameSprite::Traffic);
                        canvas.sprite(x, y, GameSprite::Traffic)?;
                    }
                    Some(RaceObstacle::Cone) => {
                        for offset in [2, 38] {
                            let cell = cell_rect(lane, column);
                            canvas.sprite(
                                cell.x + offset,
                                cell.y + (cell.height - 42) / 2,
                                GameSprite::Cone,
                            )?;
                        }
                    }
                    None => {}
                }
            }
        }
        // Where Up would move the car; none in the top lane.
        if self.game.player_lane() > 0 {
            let (x, y) = sprite_origin(
                self.game.player_lane() - 1,
                PLAYER_COLUMN,
                GameSprite::MoveHint,
            );
            canvas.sprite(x, y, GameSprite::MoveHint)?;
        }
        // Speed streaks in the cell behind the player's car.
        let (x, y) = sprite_origin(
            self.game.player_lane(),
            PLAYER_COLUMN - 1,
            GameSprite::Streaks,
        );
        canvas.sprite(x, y, GameSprite::Streaks)?;
        let (x, y) = sprite_origin(
            self.game.player_lane(),
            PLAYER_COLUMN,
            GameSprite::PlayerCar,
        );
        canvas.sprite(x, y, GameSprite::PlayerCar)
    }

    /// The mockup's crash screen: the distance, the best and how far it was,
    /// top speed, cars passed and nitro used, then the three rows.
    fn render_crash(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.rect(0, 0, GAME_CANVAS_WIDTH, RACE_BAR_HEIGHT, true)?;
        let title = match self.game.mode() {
            RaceMode::Classic => "Race · Classic",
            RaceMode::Zen => "Race · Zen",
        };
        canvas.text(16, 30, title.into(), CanvasTextStyle::Inverse)?;
        canvas.text_right(464, 30, "Crash!".into(), CanvasTextStyle::Inverse)?;

        canvas.text(
            16,
            130,
            distance_label(self.game.distance_m()),
            CanvasTextStyle::Heading,
        )?;
        let to_go = self.best.saturating_sub(self.game.distance_m());
        let best_line = if to_go > 0 {
            format!(
                "Best {} · {} to go",
                distance_label(self.best),
                distance_label(to_go)
            )
        } else {
            format!("Best {} · new record!", distance_label(self.best))
        };
        canvas.text(16, 162, best_line, CanvasTextStyle::Body)?;

        for (label, value, x) in [
            ("Top speed", self.top_speed.to_string(), 16),
            ("Cars passed", self.game.cars_passed().to_string(), 176),
            ("Nitro used", self.game.nitro_used().to_string(), 336),
        ] {
            canvas.text(x, 216, label.into(), CanvasTextStyle::Detail)?;
            canvas.text(x, 248, value, CanvasTextStyle::Heading)?;
        }

        let mode = match self.game.mode() {
            RaceMode::Classic => "Classic",
            RaceMode::Zen => "Zen",
        };
        let rows = [
            ("Race again", mode),
            ("Mode", "Classic · Zen"),
            ("Back to Games", ""),
        ];
        for (index, (label, value)) in rows.iter().enumerate() {
            let top = 300 + index as i32 * 64;
            let selected = index == self.crash_cursor;
            canvas.rect(24, top, 432, 52, selected)?;
            let style = if selected {
                CanvasTextStyle::Inverse
            } else {
                CanvasTextStyle::Body
            };
            canvas.text(44, top + 34, (*label).into(), style)?;
            if !value.is_empty() {
                canvas.text_right(436, top + 34, (*value).into(), style)?;
            }
        }
        Ok(())
    }
}

/// The road's geometry as the mockup's SVG: four lanes and six columns
/// between the rumble strips.
const RACE_BAR_HEIGHT: i32 = 44;
const ROAD_X: i32 = 16;
const ROAD_WIDTH: i32 = 448;
const ROAD_TOP: i32 = 118;
const RUMBLE_HEIGHT: i32 = 14;
const EDGE_HEIGHT: i32 = 3;
const ROAD_HEIGHT: i32 = 468;
const ROAD_BOTTOM: i32 = ROAD_TOP + ROAD_HEIGHT;
/// The dashed lane lines, 4 px tall, between the lane bands.
const LANE_LINES: [i32; 3] = [241, 350, 459];
/// The lane bands sprites sit in; the dirty cells use the same rectangles.
const LANE_TOPS: [i32; 4] = [135, 245, 354, 463];
const LANE_HEIGHTS: [i32; 4] = [106, 105, 105, 106];
const CELL_WIDTH: i32 = ROAD_WIDTH / RACE_COLUMNS as i32;
/// A full refresh every 30 road steps clears ghosting.
const RACE_REFRESH_STEPS: u32 = 30;
const RACE_FULL_RECT: DirtyRect = DirtyRect::new(0, 0, GAME_CANVAS_WIDTH, GAME_CANVAS_HEIGHT);
const RACE_STATS_RECT: DirtyRect = DirtyRect::new(0, RACE_BAR_HEIGHT, GAME_CANVAS_WIDTH, 74);
const RACE_PAUSE_RECT: DirtyRect = DirtyRect::new(120, 300, 240, 80);

/// The cell's rectangle on the road.
fn cell_rect(lane: usize, column: usize) -> DirtyRect {
    DirtyRect::new(
        ROAD_X + column as i32 * CELL_WIDTH,
        LANE_TOPS[lane],
        CELL_WIDTH,
        LANE_HEIGHTS[lane],
    )
}

/// Where a sprite goes in a cell: centred, so it stays inside the cell.
fn sprite_origin(lane: usize, column: usize, sprite: GameSprite) -> (i32, i32) {
    let cell = cell_rect(lane, column);
    let (width, height) = sprite.size();
    (
        cell.x + (CELL_WIDTH - width) / 2,
        cell.y + (cell.height - height) / 2,
    )
}

/// `2.4 km` above a kilometre, `350 m` below it.
#[must_use]
pub fn distance_label(metres: u32) -> String {
    if metres >= 1000 {
        format!("{}.{:01} km", metres / 1000, (metres % 1000) / 100)
    } else {
        format!("{metres} m")
    }
}

/// `Classic · best 5.1 km`, or `Classic · no best yet`.
fn best_label(mode: &str, best_m: u32) -> String {
    if best_m == 0 {
        format!("{mode} · no best yet")
    } else {
        format!("{mode} · best {}", distance_label(best_m))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RaceAction, RaceGame, RaceMode, RaceObstacle, PLAYER_COLUMN, RACE_COLUMNS, RACE_LANES,
    };
    use crate::buttons::ButtonEvent;

    /// The free lanes of a column as a bit per lane.
    fn free_mask(game: &RaceGame, column: usize) -> u8 {
        let mut mask = 0;
        for lane in 0..RACE_LANES {
            if game.cell(lane, column).is_none() {
                mask |= 1 << lane;
            }
        }
        mask
    }

    /// Whether some lane of `left` is the same as or next to a lane of `right`.
    fn touches(left: u8, right: u8) -> bool {
        ((left << 1) & right != 0) || (left & right != 0) || ((left >> 1) & right != 0)
    }

    /// The road keeps a gap a driver can follow: 10,000 steps from several
    /// seeds never close every lane, and each column's free lane is in or
    /// next to one the column before left free (so one the player can reach
    /// a lane at a time). The player sits out; the check is of the road.
    #[test]
    fn the_road_never_closes_every_lane_and_the_gap_stays_reachable() {
        for seed in [1_u32, 7, 0xDEAD_BEEF] {
            let mut game = RaceGame::new(RaceMode::Classic, seed);
            for _ in 0..10_000 {
                game.advance_road();
                game.spawn_column();
                for column in 0..RACE_COLUMNS {
                    assert_ne!(free_mask(&game, column), 0, "lane closed at {column}");
                    if column > 0 {
                        assert!(
                            touches(free_mask(&game, column), free_mask(&game, column - 1)),
                            "the gap jumped at {column} with seed {seed:#x}"
                        );
                    }
                }
            }
        }
    }

    /// A step that brings a car into the player's cell is a crash.
    #[test]
    fn a_car_arriving_in_the_player_cell_crashes() {
        let mut game = RaceGame::new(RaceMode::Classic, 7);
        game.place(game.player_lane(), PLAYER_COLUMN + 1, RaceObstacle::Car);
        assert!(game.tick());
        assert!(game.is_over());
    }

    /// Driving into a car is a crash, the same as being hit by one.
    #[test]
    fn a_lane_change_into_a_car_crashes() {
        let mut game = RaceGame::new(RaceMode::Classic, 7);
        game.place(game.player_lane() - 1, PLAYER_COLUMN, RaceObstacle::Car);
        assert!(game.apply(RaceAction::Up));
        assert!(game.is_over());
        assert_eq!(game.distance_m(), 0);
    }

    /// Nitro steps three times, checking each: a car two columns ahead ends
    /// the game on the second step.
    #[test]
    fn nitro_crashes_into_a_car_two_columns_ahead() {
        let mut game = RaceGame::new(RaceMode::Classic, 7);
        game.place(game.player_lane(), PLAYER_COLUMN + 2, RaceObstacle::Car);
        assert!(game.apply(RaceAction::Nitro));
        assert!(game.is_over());
        assert_eq!(game.distance_m(), 20);
    }

    /// On a clear lane nitro adds three steps of 20 m and spends a charge.
    #[test]
    fn nitro_on_a_clear_lane_adds_sixty_metres() {
        let mut game = RaceGame::new(RaceMode::Classic, 7);
        assert_eq!(game.nitro_charges(), 1);
        assert!(game.apply(RaceAction::Nitro));
        assert!(!game.is_over());
        assert_eq!(game.distance_m(), 60);
        assert_eq!(game.nitro_used(), 1);
        assert_eq!(game.nitro_charges(), 0);
        // A second nitro has no charge to spend.
        assert!(!game.apply(RaceAction::Nitro));
        assert_eq!(game.distance_m(), 60);
    }

    /// Drive by the gap: keep the car in a lane that survives the arriving
    /// column, moving only where the road is clear, and tick the world once
    /// the lane is safe. A seed's road is fixed, so the run is repeatable.
    fn careful_drive(game: &mut RaceGame, metres: u32) {
        while game.distance_m() < metres {
            assert!(!game.is_over(), "the careful driver gets through");
            for _ in 0..RACE_LANES {
                if game.cell(game.player_lane(), PLAYER_COLUMN + 1).is_none() {
                    break;
                }
                let target = (0..RACE_LANES)
                    .filter(|lane| {
                        *lane != game.player_lane()
                            && game.cell(*lane, PLAYER_COLUMN).is_none()
                            && game.cell(*lane, PLAYER_COLUMN + 1).is_none()
                    })
                    .min_by_key(|lane| lane.abs_diff(game.player_lane()));
                match target {
                    Some(target) if target < game.player_lane() => {
                        game.apply(RaceAction::Up);
                    }
                    Some(_) => {
                        game.apply(RaceAction::Down);
                    }
                    None => break,
                }
            }
            game.tick();
        }
    }

    /// The step every second at speed 1 loses 100 ms each kilometre, down to
    /// 500 ms after 6 km.
    #[test]
    fn the_interval_falls_with_the_speed() {
        let mut game = RaceGame::new(RaceMode::Classic, 3);
        assert_eq!(game.tick_interval_ms(), Some(1000));
        careful_drive(&mut game, 1_000);
        assert_eq!(game.speed(), 2);
        assert_eq!(game.tick_interval_ms(), Some(900));
        careful_drive(&mut game, 3_000);
        assert_eq!(game.speed(), 4);
        assert_eq!(game.tick_interval_ms(), Some(700));
        careful_drive(&mut game, 6_000);
        assert_eq!(game.speed(), 7);
        assert_eq!(game.tick_interval_ms(), Some(500));
        // And the charges of those kilometres are held, three at most.
        assert_eq!(game.nitro_charges(), 3);
    }

    /// Zen has no timer: the road moves only on a key, ▲▼ also change a lane
    /// and ● just steps. There is no nitro.
    #[test]
    fn zen_moves_only_on_a_key() {
        let mut game = RaceGame::new(RaceMode::Zen, 7);
        assert_eq!(game.tick_interval_ms(), None);
        assert!(!game.tick());
        assert_eq!(game.distance_m(), 0);
        assert!(game.apply(RaceAction::Down));
        assert_eq!(game.player_lane(), RACE_LANES / 2 + 1);
        assert_eq!(game.distance_m(), 20);
        assert!(game.apply(RaceAction::Nitro));
        assert_eq!(game.distance_m(), 40);
        assert_eq!(game.nitro_used(), 0);
    }

    /// A step dirties only the cells whose content changed: the column the
    /// obstacles left, the one they entered and the new column.
    #[test]
    fn a_step_dirties_only_the_cells_that_changed() {
        use super::{NativeGameCanvas, RaceApp, CELL_WIDTH, RACE_FULL_RECT};
        let mut app = RaceApp::new(RaceMode::Zen, 7);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        // One car on the road, so the step has something to move.
        app.game.place(2, 3, RaceObstacle::Car);
        let result = app
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(result.reason, "step");
        let changed: Vec<_> = result
            .dirty_regions
            .iter()
            .filter(|rect| rect.width == CELL_WIDTH)
            .collect();
        assert!(!changed.is_empty(), "the moved cells are redrawn");
        assert!(
            changed.len() <= 2 * RACE_LANES,
            "only the cells that changed, not the road: {}",
            changed.len()
        );
        assert!(
            !result.dirty_regions.contains(&RACE_FULL_RECT),
            "an ordinary step is not a full refresh"
        );
    }

    /// The crash screen's rows: Race again, Mode and Back to Games.
    #[test]
    fn the_crash_menu_races_again_and_leaves() {
        use super::{NativeGameCanvas, RaceApp};
        let mut app = RaceApp::new(RaceMode::Classic, 7);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        // Drive into the first obstacle: no dodging at all.
        for _ in 0..200 {
            if app.game().is_over() {
                break;
            }
            app.tick_and_render(&mut canvas).unwrap();
        }
        assert!(app.game().is_over());
        // Row 0: Race again.
        let result = app
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(result.reason, "new-game");
        assert!(!app.game().is_over());
        assert_eq!(app.game().distance_m(), 0);
        // Crash again, then row 2: Back to Games.
        for _ in 0..200 {
            if app.game().is_over() {
                break;
            }
            app.tick_and_render(&mut canvas).unwrap();
        }
        app.apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        app.apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        let result = app
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(result.reason, "back-to-games");
        assert!(app.exit_requested());
    }

    /// The fullest road — three obstacles in every column — still fits the
    /// command budget: one command per sprite keeps a frame cheap.
    #[test]
    fn the_fullest_road_stays_under_the_command_limit() {
        use super::{NativeGameCanvas, RaceApp};
        use crate::games::canvas::MAX_GAME_DRAW_COMMANDS;
        let mut app = RaceApp::new(RaceMode::Classic, 7);
        let mut canvas = NativeGameCanvas::default();
        // Three obstacles in every column, cones worst-case (two commands).
        for column in 0..RACE_COLUMNS {
            for lane in 0..3 {
                app.game.place(
                    lane,
                    column,
                    if column % 2 == 0 {
                        RaceObstacle::Cone
                    } else {
                        RaceObstacle::Car
                    },
                );
            }
        }
        app.render_initial(&mut canvas).unwrap();
        assert!(
            canvas.commands().len() < MAX_GAME_DRAW_COMMANDS,
            "the fullest road draws {} commands",
            canvas.commands().len()
        );
    }

    /// A Classic lane change dirties the player's old and new cells.
    #[test]
    fn a_lane_change_dirties_both_player_cells() {
        use super::{cell_rect, NativeGameCanvas, RaceApp, PLAYER_COLUMN};
        let mut app = RaceApp::new(RaceMode::Classic, 7);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        let before = app.game().player_lane();
        let result = app
            .apply_button_and_render(ButtonEvent::Up, &mut canvas)
            .unwrap();
        assert_eq!(result.reason, "lane-up");
        assert_eq!(app.game().player_lane(), before - 1);
        assert!(
            result
                .dirty_regions
                .contains(&cell_rect(before, PLAYER_COLUMN)),
            "the old player cell is redrawn"
        );
        assert!(
            result
                .dirty_regions
                .contains(&cell_rect(before - 1, PLAYER_COLUMN)),
            "the new player cell is redrawn"
        );
    }

    /// Zen counts the cars passed instead of nitro charges it cannot spend.
    #[test]
    fn zen_counts_cars_passed_and_draws_no_nitro_slots() {
        use super::{NativeGameCanvas, RaceApp};
        use crate::games::canvas::DrawCommand;
        let mut app = RaceApp::new(RaceMode::Zen, 7);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        let texts = |name: &str| {
            canvas
                .commands()
                .iter()
                .filter(|command| match command {
                    DrawCommand::Text { text, .. } | DrawCommand::TextRight { text, .. } => {
                        text.contains(name)
                    }
                    _ => false,
                })
                .count()
        };
        assert_eq!(texts("CARS PASSED"), 1, "the third column counts cars");
        assert_eq!(texts("NITRO"), 0, "Zen has no nitro column");
        let slots = canvas
            .commands()
            .iter()
            .filter(|command| match command {
                DrawCommand::Rect { width, height, .. } => *width == 14 && *height == 16,
                _ => false,
            })
            .count();
        assert_eq!(slots, 0, "no nitro slots on Zen");
    }
}
