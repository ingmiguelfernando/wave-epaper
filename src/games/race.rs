//! Race: a lane game that suits e-paper, one screen change per step.
//!
//! The road runs from right to left: obstacles arrive in the last column and
//! move one column toward the player each step, 20 metres a step. The engine
//! holds no drawing; `tetris.rs` renders the sibling game the same way.

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

#[cfg(test)]
mod tests {
    use super::{
        RaceAction, RaceGame, RaceMode, RaceObstacle, PLAYER_COLUMN, RACE_COLUMNS, RACE_LANES,
    };

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
                    Some(target) => {
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
}
