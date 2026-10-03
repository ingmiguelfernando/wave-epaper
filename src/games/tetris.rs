//! Pure Tetris rules for the SD game path: no drawing, no timers and no I/O.
//!
//! Row 0 is the top row of the 10 x 20 board. Pieces rotate inside their
//! bounding box (SRS-like states); queries speak in (column, row) pairs.

pub const TETRIS_WIDTH: usize = 10;
pub const TETRIS_HEIGHT: usize = 20;

/// Classic gravity tick, fixed at every level because the e-paper panel
/// cannot refresh faster.
pub const GRAVITY_MS: u32 = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TetrisMode {
    Zen,
    Classic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TetrisAction {
    Left,
    Right,
    Rotate,
    Drop,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TetrisPiece {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

const ALL_PIECES: [TetrisPiece; 7] = [
    TetrisPiece::I,
    TetrisPiece::O,
    TetrisPiece::T,
    TetrisPiece::S,
    TetrisPiece::Z,
    TetrisPiece::J,
    TetrisPiece::L,
];
const LINE_SCORES: [u32; 4] = [100, 300, 500, 800];
/// Column offsets tried when the plain rotation is blocked.
const KICK_OFFSETS: [i32; 5] = [0, -1, 1, -2, 2];
/// xorshift keeps zero fixed, so a zero seed starts from this constant.
const ZERO_SEED: u32 = 0x9E37_79B9;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TetrisGame {
    mode: TetrisMode,
    board: [[Option<TetrisPiece>; TETRIS_WIDTH]; TETRIS_HEIGHT],
    active: TetrisPiece,
    rotation: u8,
    box_column: i32,
    box_row: i32,
    next_piece: TetrisPiece,
    bag: [TetrisPiece; 7],
    bag_next: usize,
    rng: u32,
    score: u32,
    lines: u32,
    game_over: bool,
}

impl TetrisGame {
    #[must_use]
    pub fn new(mode: TetrisMode, seed: u32) -> Self {
        let mut game = Self {
            mode,
            board: [[None; TETRIS_WIDTH]; TETRIS_HEIGHT],
            active: TetrisPiece::I,
            rotation: 0,
            box_column: 0,
            box_row: 0,
            next_piece: TetrisPiece::I,
            bag: ALL_PIECES,
            bag_next: ALL_PIECES.len(), // the first draw refills the bag
            rng: if seed == 0 { ZERO_SEED } else { seed },
            score: 0,
            lines: 0,
            game_over: false,
        };
        game.next_piece = game.draw_from_bag();
        game.spawn_next();
        game
    }

    #[must_use]
    pub const fn mode(&self) -> TetrisMode {
        self.mode
    }

    /// Applies one action and reports whether the state changed. Every action
    /// is a no-op once the game is over.
    pub fn apply(&mut self, action: TetrisAction) -> bool {
        if self.game_over {
            return false;
        }
        match action {
            TetrisAction::Left => self.shift(-1),
            TetrisAction::Right => self.shift(1),
            TetrisAction::Rotate => self.rotate(),
            TetrisAction::Drop => {
                self.box_row = self.landing_row();
                self.lock_active();
                true
            }
        }
    }

    /// Gravity tick: one row down, locking when the piece cannot move. Zen
    /// never calls it; Classic calls it every `GRAVITY_MS`.
    pub fn step(&mut self) -> bool {
        if self.game_over {
            return false;
        }
        if self.fits(
            self.active,
            self.rotation,
            self.box_column,
            self.box_row + 1,
        ) {
            self.box_row += 1;
        } else {
            self.lock_active();
        }
        true
    }

    /// Locked cell state at (column, row); `None` when empty or out of bounds.
    #[must_use]
    pub fn cell(&self, column: usize, row: usize) -> Option<TetrisPiece> {
        self.board
            .get(row)
            .and_then(|line| line.get(column))
            .copied()
            .flatten()
    }

    /// Active piece cells as (column, row) pairs.
    #[must_use]
    pub fn active_cells(&self) -> [(usize, usize); 4] {
        self.cells_at(self.active, self.rotation, self.box_column, self.box_row)
    }

    /// Where Drop would lock the active piece, in the same order as
    /// `active_cells`.
    #[must_use]
    pub fn ghost_cells(&self) -> [(usize, usize); 4] {
        self.cells_at(
            self.active,
            self.rotation,
            self.box_column,
            self.landing_row(),
        )
    }

    #[must_use]
    pub const fn next(&self) -> TetrisPiece {
        self.next_piece
    }

    #[must_use]
    pub const fn score(&self) -> u32 {
        self.score
    }

    #[must_use]
    pub const fn lines(&self) -> u32 {
        self.lines
    }

    #[must_use]
    pub const fn level(&self) -> u32 {
        1 + self.lines / 10
    }

    #[must_use]
    pub const fn is_over(&self) -> bool {
        self.game_over
    }

    fn shift(&mut self, delta: i32) -> bool {
        if self.fits(
            self.active,
            self.rotation,
            self.box_column + delta,
            self.box_row,
        ) {
            self.box_column += delta;
            true
        } else {
            false
        }
    }

    fn rotate(&mut self) -> bool {
        let rotated = (self.rotation + 1) % 4;
        for offset in KICK_OFFSETS {
            let column = self.box_column + offset;
            if self.fits(self.active, rotated, column, self.box_row) {
                self.rotation = rotated;
                self.box_column = column;
                return true;
            }
        }
        false
    }

    fn landing_row(&self) -> i32 {
        let mut row = self.box_row;
        while self.fits(self.active, self.rotation, self.box_column, row + 1) {
            row += 1;
        }
        row
    }

    fn lock_active(&mut self) {
        for (column, row) in self.active_cells() {
            self.board[row][column] = Some(self.active);
        }
        let cleared = self.clear_full_rows();
        if cleared > 0 {
            // the level in play scores these lines, before they are added
            self.score += LINE_SCORES[cleared as usize - 1] * self.level();
            self.lines += cleared;
        }
        self.spawn_next();
    }

    fn clear_full_rows(&mut self) -> u32 {
        let mut cleared = 0_u32;
        let mut write = TETRIS_HEIGHT;
        for read in (0..TETRIS_HEIGHT).rev() {
            if self.board[read].iter().all(Option::is_some) {
                cleared += 1;
                continue;
            }
            write -= 1;
            self.board[write] = self.board[read];
        }
        for row in 0..write {
            self.board[row] = [None; TETRIS_WIDTH];
        }
        cleared
    }

    fn spawn_next(&mut self) {
        self.active = self.next_piece;
        self.next_piece = self.draw_from_bag();
        self.rotation = 0;
        self.box_column = spawn_column(self.active);
        self.box_row = 0;
        if !self.fits(self.active, self.rotation, self.box_column, self.box_row) {
            self.game_over = true;
        }
    }

    fn draw_from_bag(&mut self) -> TetrisPiece {
        if self.bag_next >= self.bag.len() {
            self.refill_bag();
        }
        let piece = self.bag[self.bag_next];
        self.bag_next += 1;
        piece
    }

    fn refill_bag(&mut self) {
        self.bag = ALL_PIECES;
        for index in (1..self.bag.len()).rev() {
            let swap = (self.next_random() % (index as u32 + 1)) as usize;
            self.bag.swap(index, swap);
        }
        self.bag_next = 0;
    }

    fn next_random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn fits(&self, piece: TetrisPiece, rotation: u8, column: i32, row: i32) -> bool {
        piece_cells(piece, rotation).into_iter().all(|(c, r)| {
            let (x, y) = (column + i32::from(c), row + i32::from(r));
            let inside =
                (0..TETRIS_WIDTH as i32).contains(&x) && (0..TETRIS_HEIGHT as i32).contains(&y);
            inside && self.board[y as usize][x as usize].is_none()
        })
    }

    fn cells_at(
        &self,
        piece: TetrisPiece,
        rotation: u8,
        column: i32,
        row: i32,
    ) -> [(usize, usize); 4] {
        // every reachable position is inside the board
        piece_cells(piece, rotation).map(|(c, r)| {
            (
                (column + i32::from(c)) as usize,
                (row + i32::from(r)) as usize,
            )
        })
    }
}

/// Spawn offsets (column, row) inside the piece's bounding box.
const fn spawn_cells(piece: TetrisPiece) -> [(u8, u8); 4] {
    match piece {
        TetrisPiece::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
        TetrisPiece::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
        TetrisPiece::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
        TetrisPiece::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
        TetrisPiece::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
        TetrisPiece::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
        TetrisPiece::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
    }
}

const fn box_size(piece: TetrisPiece) -> u8 {
    match piece {
        TetrisPiece::I => 4,
        TetrisPiece::O => 2,
        _ => 3,
    }
}

const fn spawn_column(piece: TetrisPiece) -> i32 {
    match piece {
        TetrisPiece::I => 3,
        TetrisPiece::O => 4,
        _ => 3,
    }
}

/// Cells of one rotation state, rotated clockwise inside the bounding box.
fn piece_cells(piece: TetrisPiece, rotation: u8) -> [(u8, u8); 4] {
    let size = box_size(piece);
    let mut cells = spawn_cells(piece);
    for _ in 0..(rotation % 4) {
        for cell in cells.iter_mut() {
            *cell = (cell.1, size - 1 - cell.0);
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_active(game: &mut TetrisGame, piece: TetrisPiece, rotation: u8, column: i32, row: i32) {
        game.active = piece;
        game.rotation = rotation;
        game.box_column = column;
        game.box_row = row;
    }

    fn fill_row_except(game: &mut TetrisGame, row: usize, free_column: usize) {
        for column in 0..TETRIS_WIDTH {
            if column != free_column {
                game.board[row][column] = Some(TetrisPiece::O);
            }
        }
    }

    /// Clears the bottom row with a vertical I in its leftmost column.
    fn clear_bottom_row(game: &mut TetrisGame) {
        fill_row_except(game, TETRIS_HEIGHT - 1, 0);
        set_active(game, TetrisPiece::I, 1, -1, 16);
        assert!(game.apply(TetrisAction::Drop));
    }

    #[test]
    fn each_piece_rotates_through_four_states_and_back() {
        for piece in ALL_PIECES {
            let mut game = TetrisGame::new(TetrisMode::Zen, 11);
            set_active(&mut game, piece, 0, 3, 0);
            let spawn = game.active_cells();
            for turn in 1..=4 {
                assert!(game.apply(TetrisAction::Rotate), "{piece:?} turn {turn}");
                // O is rotation-invariant inside its 2 x 2 box
                if turn < 4 && piece != TetrisPiece::O {
                    assert_ne!(game.active_cells(), spawn, "{piece:?} must change state");
                }
            }
            assert_eq!(game.active_cells(), spawn, "{piece:?} must return to spawn");
        }
    }

    #[test]
    fn walls_and_locked_cells_block_moves() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 5);
        set_active(&mut game, TetrisPiece::I, 0, 0, 18);
        let start = game.active_cells();
        assert_eq!(start, [(0, 19), (1, 19), (2, 19), (3, 19)]);
        assert!(!game.apply(TetrisAction::Left));
        assert_eq!(game.active_cells(), start, "left wall blocks");
        assert!(game.apply(TetrisAction::Right));
        assert_eq!(game.active_cells(), [(1, 19), (2, 19), (3, 19), (4, 19)]);

        set_active(&mut game, TetrisPiece::I, 0, 6, 18);
        let start = game.active_cells();
        assert!(!game.apply(TetrisAction::Right));
        assert_eq!(game.active_cells(), start, "right wall blocks");

        set_active(&mut game, TetrisPiece::O, 0, 4, 17);
        game.board[18][6] = Some(TetrisPiece::T);
        let start = game.active_cells();
        assert!(!game.apply(TetrisAction::Right));
        assert_eq!(game.active_cells(), start, "locked cell blocks");
        assert!(game.apply(TetrisAction::Left));
        assert_eq!(game.active_cells(), [(3, 17), (4, 17), (3, 18), (4, 18)]);
    }

    #[test]
    fn rotation_kicks_shift_the_piece_sideways() {
        // kick 0 is blocked at (4, 12), so the kick order prefers -1 over +1
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        game.board[12][4] = Some(TetrisPiece::S);
        assert!(game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), [(2, 11), (3, 12), (3, 11), (3, 10)]);

        // kicks 0, -1 and +1 are blocked, -2 reaches a free column
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        for column in 3..=5 {
            game.board[12][column] = Some(TetrisPiece::S);
        }
        assert!(game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), [(1, 11), (2, 12), (2, 11), (2, 10)]);

        // kicks 0 to -2 are blocked, +2 reaches a free column
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        for column in 2..=5 {
            game.board[12][column] = Some(TetrisPiece::S);
        }
        assert!(game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), [(5, 11), (6, 12), (6, 11), (6, 10)]);
    }

    #[test]
    fn rotation_is_rejected_when_every_kick_is_blocked() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        for column in 1..=6 {
            game.board[12][column] = Some(TetrisPiece::S);
        }
        let start = game.active_cells();
        assert!(!game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), start);
    }

    #[test]
    fn clearing_one_to_four_rows_scores_the_line_table() {
        for cleared in 1..=4_usize {
            let mut game = TetrisGame::new(TetrisMode::Zen, 9);
            for row in (TETRIS_HEIGHT - cleared)..TETRIS_HEIGHT {
                fill_row_except(&mut game, row, 0);
            }
            set_active(&mut game, TetrisPiece::I, 1, -1, 16);
            assert!(game.apply(TetrisAction::Drop));
            assert_eq!(game.lines(), cleared as u32, "cleared {cleared} rows");
            assert_eq!(game.score(), [100, 300, 500, 800][cleared - 1]);
            assert_eq!(game.level(), 1);
        }
    }

    #[test]
    fn score_uses_the_level_and_level_follows_the_lines() {
        let mut at_level_one = TetrisGame::new(TetrisMode::Zen, 9);
        at_level_one.lines = 9;
        clear_bottom_row(&mut at_level_one);
        assert_eq!(at_level_one.score(), 100, "one row at level 1");
        assert_eq!(at_level_one.lines(), 10);
        assert_eq!(at_level_one.level(), 2);

        let mut at_level_two = TetrisGame::new(TetrisMode::Zen, 9);
        at_level_two.lines = 10;
        clear_bottom_row(&mut at_level_two);
        assert_eq!(at_level_two.score(), 200, "one row at level 2");
        assert_eq!(at_level_two.lines(), 11);
        assert_eq!(at_level_two.level(), 2);
    }

    #[test]
    fn each_bag_contains_every_piece_once() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 21);
        // the game already drew the active piece and the next one
        let mut first_bag = vec![game.active, game.next()];
        for _ in 0..5 {
            first_bag.push(game.draw_from_bag());
        }
        first_bag.sort();
        assert_eq!(first_bag, ALL_PIECES);

        let mut second_bag = Vec::new();
        for _ in 0..7 {
            second_bag.push(game.draw_from_bag());
        }
        second_bag.sort();
        assert_eq!(second_bag, ALL_PIECES);
    }

    #[test]
    fn same_seed_gives_the_same_piece_sequence() {
        for seed in [0, 1, 0xDEAD_BEEF] {
            let mut left = TetrisGame::new(TetrisMode::Zen, seed);
            let mut right = TetrisGame::new(TetrisMode::Classic, seed);
            for _ in 0..10 {
                assert_eq!(left.active, right.active, "seed {seed}");
                assert_eq!(left.next(), right.next(), "seed {seed}");
                assert!(left.apply(TetrisAction::Drop));
                assert!(right.apply(TetrisAction::Drop));
            }
        }
    }

    #[test]
    fn ghost_marks_where_the_drop_lands() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 3);
        set_active(&mut game, TetrisPiece::I, 0, 2, 0);
        assert_eq!(game.ghost_cells(), [(2, 19), (3, 19), (4, 19), (5, 19)]);

        game.board[19][3] = Some(TetrisPiece::T);
        let ghost = game.ghost_cells();
        assert_eq!(ghost, [(2, 18), (3, 18), (4, 18), (5, 18)]);
        assert!(game.apply(TetrisAction::Drop));
        for (column, row) in ghost {
            assert_eq!(game.cell(column, row), Some(TetrisPiece::I));
        }
    }

    #[test]
    fn game_over_at_spawn_makes_actions_no_ops() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 13);
        // block every spawn cell of the next piece in the top two rows
        let spawn = spawn_column(game.next());
        for (column, row) in piece_cells(game.next(), 0) {
            game.board[row as usize][column as usize + spawn as usize] = Some(TetrisPiece::T);
        }
        set_active(&mut game, TetrisPiece::I, 0, 3, 17);
        assert!(game.apply(TetrisAction::Drop));
        assert!(game.is_over());

        let frozen = game.clone();
        assert!(!game.apply(TetrisAction::Left));
        assert!(!game.apply(TetrisAction::Right));
        assert!(!game.apply(TetrisAction::Rotate));
        assert!(!game.apply(TetrisAction::Drop));
        assert!(!game.step());
        assert_eq!(game, frozen);
    }

    #[test]
    fn drop_locks_at_once() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 3);
        let next = game.next();
        set_active(&mut game, TetrisPiece::I, 0, 3, 0);
        assert!(game.apply(TetrisAction::Drop));
        for column in 3..7 {
            assert_eq!(game.cell(column, 19), Some(TetrisPiece::I));
        }
        assert_eq!(game.active, next, "Drop spawns the next piece at once");
        assert!(
            game.active_cells().iter().all(|(_, row)| *row < 2),
            "the new piece spawns in the top two rows"
        );
    }

    #[test]
    fn step_moves_down_one_row_and_locks_when_blocked() {
        let mut game = TetrisGame::new(TetrisMode::Zen, 3);
        let next = game.next();
        set_active(&mut game, TetrisPiece::O, 0, 4, 17);
        assert!(game.step());
        assert_eq!(game.active_cells(), [(4, 18), (5, 18), (4, 19), (5, 19)]);
        assert!(game.step(), "blocked step still locks the piece");
        assert_eq!(game.cell(4, 19), Some(TetrisPiece::O));
        assert_eq!(game.cell(5, 18), Some(TetrisPiece::O));
        assert_eq!(game.active, next);
    }
}
