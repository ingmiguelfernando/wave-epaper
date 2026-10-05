//! Tetris rules and the native Zen view for the SD game path.
//!
//! The rules stay free of drawing and I/O. Row 0 is the top row of the
//! 10 x 20 board. Pieces rotate inside their bounding box (SRS-like states);
//! queries speak in (column, row) pairs.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::{DirtyRect, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH},
};

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

impl TetrisMode {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Zen => "zen",
            Self::Classic => "classic",
        }
    }
}

impl TetrisAction {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Rotate => "rotate",
            Self::Drop => "drop",
        }
    }
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

/// Cells of one rotation state, turned clockwise on screen inside the bounding
/// box: (column, row) becomes (size - 1 - row, column).
fn piece_cells(piece: TetrisPiece, rotation: u8) -> [(u8, u8); 4] {
    let size = box_size(piece);
    let mut cells = spawn_cells(piece);
    for _ in 0..(rotation % 4) {
        for cell in cells.iter_mut() {
            *cell = (size - 1 - cell.1, cell.0);
        }
    }
    cells
}

pub const TETRIS_CELL: i32 = 30;
/// Top-left cell origin inside the 3 px mockup frame.
pub const TETRIS_BOARD_X: i32 = 19;
pub const TETRIS_BOARD_Y: i32 = 59;
/// Locked pieces between full-frame refreshes that clear ghosting.
pub const TETRIS_REFRESH_LOCKS: u32 = 20;

const TETRIS_BAR_HEIGHT: i32 = 44;
/// Average advance used to place right-aligned status-bar text.
const TETRIS_CHAR_WIDTH: i32 = 11;
/// Ring commands allowed on locked blocks; keeps the frame under the limit.
const TETRIS_RING_BUDGET: usize = 120;
const TETRIS_SIDE_X: i32 = 336;
const TETRIS_NEXT_X: i32 = 336;
const TETRIS_NEXT_Y: i32 = 78;
const TETRIS_NEXT_CELL: i32 = 24;
const TETRIS_NEXT_COLUMNS: usize = 4;
const TETRIS_NEXT_ROWS: usize = 2;
const TETRIS_BOARD_RECT: DirtyRect = DirtyRect::new(
    16,
    56,
    TETRIS_WIDTH as i32 * TETRIS_CELL + 6,
    TETRIS_HEIGHT as i32 * TETRIS_CELL + 6,
);
const TETRIS_FULL_RECT: DirtyRect = DirtyRect::new(0, 0, GAME_CANVAS_WIDTH, GAME_CANVAS_HEIGHT);
const TETRIS_BAR_RECT: DirtyRect = DirtyRect::new(0, 0, GAME_CANVAS_WIDTH, TETRIS_BAR_HEIGHT);
const TETRIS_SIDE_RECT: DirtyRect = DirtyRect::new(TETRIS_SIDE_X, 56, 128, 430);
const TETRIS_NEXT_RECT: DirtyRect = DirtyRect::new(
    TETRIS_NEXT_X,
    TETRIS_NEXT_Y,
    TETRIS_NEXT_COLUMNS as i32 * TETRIS_NEXT_CELL,
    TETRIS_NEXT_ROWS as i32 * TETRIS_NEXT_CELL,
);
const TETRIS_STATUS_RECT: DirtyRect = DirtyRect::new(TETRIS_SIDE_X, 336, 128, 110);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TetrisEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub mode: TetrisMode,
    pub action: Option<TetrisAction>,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub completed: bool,
    pub dirty_regions: Vec<DirtyRect>,
}

/// Zen view over one `TetrisGame`: redraws, key mapping and the in-memory
/// best score that Phase 6 will persist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TetrisApp {
    game: TetrisGame,
    seed: u32,
    best: u32,
    locks: u32,
}

impl TetrisApp {
    #[must_use]
    pub fn new(mode: TetrisMode, seed: u32) -> Self {
        Self {
            game: TetrisGame::new(mode, seed),
            seed,
            best: 0,
            locks: 0,
        }
    }

    #[must_use]
    pub const fn game(&self) -> &TetrisGame {
        &self.game
    }

    #[must_use]
    pub const fn best(&self) -> u32 {
        self.best
    }

    #[must_use]
    pub const fn locks(&self) -> u32 {
        self.locks
    }

    pub fn render_initial(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        canvas.invalidate_rect(TETRIS_FULL_RECT);
        canvas.request_refresh();
        Ok(())
    }

    /// ▲ moves left, ▼ right and ● rotates. After game over ● starts a new
    /// game and every other key is ignored.
    pub fn apply_button_and_render(
        &mut self,
        event: ButtonEvent,
        canvas: &mut NativeGameCanvas,
    ) -> Result<TetrisEventResult, String> {
        let before = (
            self.game.active_cells(),
            self.game.ghost_cells(),
            self.game.lines(),
        );
        if self.game.is_over() {
            let reason = match event {
                ButtonEvent::Select => {
                    self.seed = self.seed.wrapping_add(1);
                    self.game = TetrisGame::new(self.game.mode(), self.seed);
                    "new-game"
                }
                _ => "game-finished",
            };
            return self.finish_and_render(reason, None, before, canvas);
        }
        let action = match event {
            ButtonEvent::Up => TetrisAction::Left,
            ButtonEvent::Down => TetrisAction::Right,
            ButtonEvent::Select => TetrisAction::Rotate,
        };
        let reason = if self.game.apply(action) {
            match action {
                TetrisAction::Left => "move-left",
                TetrisAction::Right => "move-right",
                TetrisAction::Rotate => "rotate",
                TetrisAction::Drop => "drop",
            }
        } else {
            "blocked"
        };
        self.finish_and_render(reason, Some(action), before, canvas)
    }

    /// A short BOOT press drops the piece to the landing row and locks it.
    pub fn apply_boot_short_press_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<TetrisEventResult, String> {
        let before = (
            self.game.active_cells(),
            self.game.ghost_cells(),
            self.game.lines(),
        );
        if self.game.is_over() {
            return self.finish_and_render("game-finished", None, before, canvas);
        }
        self.game.apply(TetrisAction::Drop);
        self.locks += 1;
        self.best = self.best.max(self.game.score());
        let reason = if self.game.is_over() {
            "game-over"
        } else {
            "drop"
        };
        self.finish_and_render(reason, Some(TetrisAction::Drop), before, canvas)
    }

    fn finish_and_render(
        &mut self,
        reason: &'static str,
        action: Option<TetrisAction>,
        before: ([(usize, usize); 4], [(usize, usize); 4], u32),
        canvas: &mut NativeGameCanvas,
    ) -> Result<TetrisEventResult, String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = self.dirty_regions(reason, &before);
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        let (column, row) = self.game.active_cells()[0];
        Ok(TetrisEventResult {
            reason,
            row,
            column,
            mode: self.game.mode(),
            action,
            score: self.game.score(),
            lines: self.game.lines(),
            level: self.game.level(),
            completed: self.game.is_over(),
            dirty_regions,
        })
    }

    fn dirty_regions(
        &self,
        reason: &'static str,
        before: &([(usize, usize); 4], [(usize, usize); 4], u32),
    ) -> Vec<DirtyRect> {
        let (old_active, old_ghost, old_lines) = before;
        match reason {
            "new-game" => vec![TETRIS_FULL_RECT],
            "game-finished" => vec![TETRIS_STATUS_RECT],
            "blocked" => Vec::new(),
            "move-left" | "move-right" | "rotate" => vec![cells_span(&[
                *old_active,
                *old_ghost,
                self.game.active_cells(),
                self.game.ghost_cells(),
            ])],
            _ => {
                // one locked piece: every 20th lock asks for a full refresh
                if self.locks % TETRIS_REFRESH_LOCKS == 0 {
                    return vec![TETRIS_FULL_RECT];
                }
                if reason == "game-over" {
                    return vec![TETRIS_BOARD_RECT, TETRIS_SIDE_RECT];
                }
                if self.game.lines() > *old_lines {
                    return vec![TETRIS_BOARD_RECT, TETRIS_SIDE_RECT, TETRIS_BAR_RECT];
                }
                let active = self.game.active_cells();
                let ghost = self.game.ghost_cells();
                let mut regions = vec![cells_span(&[*old_active, *old_ghost]), cells_rect(&active)];
                if ghost != active {
                    regions.push(cells_rect(&ghost));
                }
                regions.push(TETRIS_NEXT_RECT);
                regions
            }
        }
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.rect(0, 0, GAME_CANVAS_WIDTH, TETRIS_BAR_HEIGHT, true)?;
        canvas.text(16, 30, "Tetris · Zen".into(), CanvasTextStyle::Inverse)?;
        let level = format!("Level {}", self.game.level());
        let level_x = GAME_CANVAS_WIDTH - 16 - level.len() as i32 * TETRIS_CHAR_WIDTH;
        canvas.text(level_x, 30, level, CanvasTextStyle::Inverse)?;

        canvas.rect(
            TETRIS_BOARD_RECT.x,
            TETRIS_BOARD_RECT.y,
            TETRIS_BOARD_RECT.width,
            TETRIS_BOARD_RECT.height,
            true,
        )?;
        canvas.paper_rect(
            TETRIS_BOARD_X,
            TETRIS_BOARD_Y,
            TETRIS_WIDTH as i32 * TETRIS_CELL,
            TETRIS_HEIGHT as i32 * TETRIS_CELL,
            true,
        )?;
        self.draw_locked(canvas)?;
        if !self.game.is_over() {
            let active = self.game.active_cells();
            for (column, row) in active {
                draw_hatched_cell(canvas, column, row)?;
            }
            let ghost = self.game.ghost_cells();
            if ghost != active {
                for (column, row) in ghost {
                    draw_ghost_cell(canvas, column, row)?;
                }
            }
        }

        canvas.text(TETRIS_SIDE_X, 68, "NEXT".into(), CanvasTextStyle::Detail)?;
        let next =
            piece_cells(self.game.next(), 0).map(|(column, row)| (column as i32, row as i32));
        for (column, row) in next {
            let x = TETRIS_NEXT_X + column * TETRIS_NEXT_CELL;
            let y = TETRIS_NEXT_Y + row * TETRIS_NEXT_CELL;
            draw_block(canvas, x, y, TETRIS_NEXT_CELL)?;
        }
        for (label, label_y, value) in [
            ("SCORE", 153, grouped(self.game.score())),
            ("LINES", 219, self.game.lines().to_string()),
            ("BEST", 285, grouped(self.best)),
        ] {
            canvas.text(
                TETRIS_SIDE_X,
                label_y,
                label.into(),
                CanvasTextStyle::Detail,
            )?;
            canvas.text(TETRIS_SIDE_X, label_y + 30, value, CanvasTextStyle::Heading)?;
        }
        canvas.text(TETRIS_SIDE_X, 351, "MODE".into(), CanvasTextStyle::Detail)?;
        if self.game.is_over() {
            canvas.text(
                TETRIS_SIDE_X,
                379,
                "Game over".into(),
                CanvasTextStyle::Body,
            )?;
            canvas.text(
                TETRIS_SIDE_X,
                403,
                format!("Score {}", grouped(self.game.score())),
                CanvasTextStyle::Detail,
            )?;
            canvas.text(
                TETRIS_SIDE_X,
                424,
                "SELECT: new".into(),
                CanvasTextStyle::Detail,
            )?;
        } else {
            canvas.text(TETRIS_SIDE_X, 379, "Zen".into(), CanvasTextStyle::Body)?;
            for (index, line) in ["No gravity: the", "piece moves only", "when you press."]
                .into_iter()
                .enumerate()
            {
                canvas.text(
                    TETRIS_SIDE_X,
                    403 + index as i32 * 21,
                    line.into(),
                    CanvasTextStyle::Detail,
                )?;
            }
        }

        // The key-hint bar is drawn by the screen renderer with the shared
        // key-cap widget, so the canvas leaves that band untouched.
        Ok(())
    }

    /// Locked blocks: one rectangle per row run, then the paper rings. Rings
    /// go on every cell while the command budget allows, on every run next,
    /// and are dropped on the busiest boards.
    fn draw_locked(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let mut runs = Vec::new();
        for row in 0..TETRIS_HEIGHT {
            for (start, end) in filled_runs(
                |column, row| self.game.cell(column, row).is_some(),
                row,
                TETRIS_WIDTH,
            ) {
                runs.push((row, start, end));
            }
        }
        let cells: usize = runs.iter().map(|(_, start, end)| end - start + 1).sum();
        for (row, start, end) in &runs {
            canvas.rect(
                TETRIS_BOARD_X + *start as i32 * TETRIS_CELL,
                TETRIS_BOARD_Y + *row as i32 * TETRIS_CELL,
                (end - start + 1) as i32 * TETRIS_CELL,
                TETRIS_CELL,
                true,
            )?;
        }
        let ring = |canvas: &mut NativeGameCanvas, row: usize, start: usize, end: usize| {
            canvas.paper_rect(
                TETRIS_BOARD_X + start as i32 * TETRIS_CELL + 5,
                TETRIS_BOARD_Y + row as i32 * TETRIS_CELL + 5,
                (end - start + 1) as i32 * TETRIS_CELL - 10,
                TETRIS_CELL - 10,
                false,
            )
        };
        if runs.len() + cells <= TETRIS_RING_BUDGET {
            for (row, start, end) in &runs {
                for column in *start..=*end {
                    ring(canvas, *row, column, column)?;
                }
            }
        } else if runs.len() * 2 <= TETRIS_RING_BUDGET {
            for (row, start, end) in &runs {
                ring(canvas, *row, *start, *end)?;
            }
        }
        Ok(())
    }
}

/// A solid block with the mockup's paper ring, `size` px wide.
fn draw_block(canvas: &mut NativeGameCanvas, x: i32, y: i32, size: i32) -> Result<(), String> {
    canvas.rect(x, y, size, size, true)?;
    canvas.paper_rect(x + 5, y + 5, size - 10, size - 10, false)
}

/// Active piece: ink frame around diagonal stripes.
fn draw_hatched_cell(
    canvas: &mut NativeGameCanvas,
    column: usize,
    row: usize,
) -> Result<(), String> {
    let x = TETRIS_BOARD_X + column as i32 * TETRIS_CELL;
    let y = TETRIS_BOARD_Y + row as i32 * TETRIS_CELL;
    canvas.rect(x, y, TETRIS_CELL, TETRIS_CELL, true)?;
    let inner = TETRIS_CELL - 6;
    let (left, top) = (x + 3, y + 3);
    let mut offset = 4;
    while offset < 2 * inner - 4 {
        let (x1, y1, x2, y2) = if offset < inner {
            (left + offset, top, left, top + offset)
        } else {
            let shift = offset - inner + 1;
            (left + inner - 1, top + shift, left + shift, top + inner - 1)
        };
        canvas.paper_line(x1, y1, x2, y2)?;
        offset += 8;
    }
    Ok(())
}

/// Ghost: a dashed 2 px outline inset 5 px, as in the mockup.
fn draw_ghost_cell(canvas: &mut NativeGameCanvas, column: usize, row: usize) -> Result<(), String> {
    let x = TETRIS_BOARD_X + column as i32 * TETRIS_CELL + 5;
    let y = TETRIS_BOARD_Y + row as i32 * TETRIS_CELL + 5;
    let side = TETRIS_CELL - 10;
    for dash in [0, 8, 16] {
        canvas.rect(x + dash, y, 4, 2, true)?;
        canvas.rect(x + dash, y + side - 2, 4, 2, true)?;
    }
    for dash in [6, 14] {
        canvas.rect(x, y + dash, 2, 4, true)?;
        canvas.rect(x + side - 2, y + dash, 2, 4, true)?;
    }
    Ok(())
}

/// Inclusive (start, end) runs of filled cells in one row.
fn filled_runs(
    mut is_filled: impl FnMut(usize, usize) -> bool,
    row: usize,
    columns: usize,
) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut column = 0;
    while column < columns {
        if !is_filled(column, row) {
            column += 1;
            continue;
        }
        let start = column;
        while column < columns && is_filled(column, row) {
            column += 1;
        }
        runs.push((start, column - 1));
    }
    runs
}

fn cells_rect(cells: &[(usize, usize); 4]) -> DirtyRect {
    let mut min_column = usize::MAX;
    let mut max_column = 0;
    let mut min_row = usize::MAX;
    let mut max_row = 0;
    for (column, row) in cells {
        min_column = min_column.min(*column);
        max_column = max_column.max(*column);
        min_row = min_row.min(*row);
        max_row = max_row.max(*row);
    }
    DirtyRect::new(
        TETRIS_BOARD_X + min_column as i32 * TETRIS_CELL,
        TETRIS_BOARD_Y + min_row as i32 * TETRIS_CELL,
        (max_column - min_column + 1) as i32 * TETRIS_CELL,
        (max_row - min_row + 1) as i32 * TETRIS_CELL,
    )
}

fn cells_span(sets: &[[(usize, usize); 4]]) -> DirtyRect {
    let mut rectangles = sets.iter().map(cells_rect);
    let Some(mut span) = rectangles.next() else {
        return DirtyRect::default();
    };
    for rectangle in rectangles {
        span = span.union(rectangle);
    }
    span
}

/// Thousands-separated value like the mockup ("12,400").
fn grouped(value: u32) -> String {
    let digits = value.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        buttons::ButtonEvent,
        games::{
            canvas::{DrawCommand, NativeGameCanvas, MAX_GAME_DRAW_COMMANDS},
            dirty_regions::MAX_DIRTY_REGIONS,
        },
    };

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
        assert_eq!(game.active_cells(), [(4, 11), (3, 10), (3, 11), (3, 12)]);

        // kicks 0, -1 and +1 are blocked, -2 reaches a free column
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        for column in 3..=5 {
            game.board[12][column] = Some(TetrisPiece::S);
        }
        assert!(game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), [(3, 11), (2, 10), (2, 11), (2, 12)]);

        // kicks 0 to -2 are blocked, +2 reaches a free column
        let mut game = TetrisGame::new(TetrisMode::Zen, 1);
        set_active(&mut game, TetrisPiece::T, 0, 3, 10);
        for column in 2..=5 {
            game.board[12][column] = Some(TetrisPiece::S);
        }
        assert!(game.apply(TetrisAction::Rotate));
        assert_eq!(game.active_cells(), [(7, 11), (6, 10), (6, 11), (6, 12)]);
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

    #[test]
    fn zen_keys_move_left_right_rotate_and_drop() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 4);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();

        set_active(&mut app.game, TetrisPiece::I, 0, 3, 5);
        let left = app
            .apply_button_and_render(ButtonEvent::Up, &mut canvas)
            .unwrap();
        assert_eq!(left.reason, "move-left");
        assert_eq!(left.action, Some(TetrisAction::Left));
        assert_eq!(app.game.active_cells(), [(2, 6), (3, 6), (4, 6), (5, 6)]);

        let right = app
            .apply_button_and_render(ButtonEvent::Down, &mut canvas)
            .unwrap();
        assert_eq!(right.reason, "move-right");
        assert_eq!(right.action, Some(TetrisAction::Right));
        assert_eq!(app.game.active_cells(), [(3, 6), (4, 6), (5, 6), (6, 6)]);

        set_active(&mut app.game, TetrisPiece::T, 0, 3, 5);
        let spawn = app.game.active_cells();
        let rotate = app
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(rotate.reason, "rotate");
        assert_eq!(rotate.action, Some(TetrisAction::Rotate));
        assert_ne!(app.game.active_cells(), spawn);

        let drop = app.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(drop.reason, "drop");
        assert_eq!(drop.action, Some(TetrisAction::Drop));
        assert!(app.game.active_cells().iter().all(|(_, row)| *row < 2));
    }

    #[test]
    fn nearly_full_board_stays_under_the_command_limit() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 4);
        // alternating cells leave one clear column band for the piece and its
        // ghost while packing every other cell with runs
        for row in 0..TETRIS_HEIGHT {
            for column in 0..TETRIS_WIDTH {
                if (2..6).contains(&column) || (row + column) % 2 != 0 {
                    continue;
                }
                app.game.board[row][column] = Some(TetrisPiece::O);
            }
        }
        set_active(&mut app.game, TetrisPiece::I, 0, 2, 0);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        assert!(canvas.commands().len() < MAX_GAME_DRAW_COMMANDS);

        // the most runs one board can hold also fits the budget
        for row in 0..TETRIS_HEIGHT {
            for column in 0..TETRIS_WIDTH {
                if (row + column) % 2 == 0 {
                    app.game.board[row][column] = Some(TetrisPiece::O);
                }
            }
        }
        for (column, row) in app.game.active_cells() {
            app.game.board[row][column] = None;
        }
        app.render_initial(&mut canvas).unwrap();
        assert!(canvas.commands().len() < MAX_GAME_DRAW_COMMANDS);
    }

    #[test]
    fn draws_each_row_run_as_one_rectangle() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 4);
        for column in 0..TETRIS_WIDTH {
            app.game.board[TETRIS_HEIGHT - 1][column] = Some(TetrisPiece::O);
        }
        app.game.game_over = true;
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        // one full row is one filled rectangle, not ten cell rectangles
        assert!(canvas.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Rect {
                x: 19,
                y: 629,
                width: 300,
                height: 30,
                filled: true
            }
        )));
    }

    #[test]
    fn invalidates_only_what_changed_with_bounded_regions() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 4);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();

        set_active(&mut app.game, TetrisPiece::T, 0, 3, 5);
        let moved = app
            .apply_button_and_render(ButtonEvent::Up, &mut canvas)
            .unwrap();
        assert_eq!(moved.dirty_regions.len(), 1, "one span around the move");
        assert!(canvas.dirty().regions().len() <= MAX_DIRTY_REGIONS);
        assert!(!canvas.dirty().full_canvas_fallback());

        let dropped = app.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert!(
            dropped.dirty_regions.len() <= MAX_DIRTY_REGIONS,
            "drop frames stay bounded"
        );
        assert!(canvas.dirty().regions().len() <= MAX_DIRTY_REGIONS);
        assert!(!canvas.dirty().full_canvas_fallback());
    }

    #[test]
    fn requests_a_full_refresh_every_twenty_locks() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 4);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        for lock in 1..=TETRIS_REFRESH_LOCKS {
            let dropped = app.apply_boot_short_press_and_render(&mut canvas).unwrap();
            if lock % TETRIS_REFRESH_LOCKS == 0 {
                assert_eq!(dropped.dirty_regions, vec![TETRIS_FULL_RECT], "lock {lock}");
            } else {
                assert_ne!(dropped.dirty_regions, vec![TETRIS_FULL_RECT], "lock {lock}");
            }
            // keep the stack from ending the game before the cadence point
            app.game.board = [[None; TETRIS_WIDTH]; TETRIS_HEIGHT];
        }
    }

    #[test]
    fn game_over_shows_the_score_and_select_starts_a_new_game() {
        let mut app = TetrisApp::new(TetrisMode::Zen, 9);
        let mut canvas = NativeGameCanvas::default();
        app.render_initial(&mut canvas).unwrap();
        fill_row_except(&mut app.game, TETRIS_HEIGHT - 1, 0);
        set_active(&mut app.game, TetrisPiece::I, 1, -1, 16);
        assert!(app.game.apply(TetrisAction::Drop));
        assert_eq!(app.game.score(), 100);

        let spawn = spawn_column(app.game.next());
        for (column, row) in piece_cells(app.game.next(), 0) {
            app.game.board[row as usize][column as usize + spawn as usize] = Some(TetrisPiece::T);
        }
        set_active(&mut app.game, TetrisPiece::I, 0, 3, 17);
        let over = app.apply_boot_short_press_and_render(&mut canvas).unwrap();
        assert_eq!(over.reason, "game-over");
        assert!(over.completed);
        assert_eq!(over.score, 100);
        assert!(canvas.commands().iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, .. } if text == "Score 100"
        )));

        let ignored = app
            .apply_button_and_render(ButtonEvent::Up, &mut canvas)
            .unwrap();
        assert_eq!(ignored.reason, "game-finished");
        assert!(app.game.is_over());

        let restarted = app
            .apply_button_and_render(ButtonEvent::Select, &mut canvas)
            .unwrap();
        assert_eq!(restarted.reason, "new-game");
        assert_eq!(restarted.score, 0);
        assert!(!app.game.is_over());
    }
}
