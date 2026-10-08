//! Native Sudoku reference game for the SD Lua event bridge.
//!
//! `sudoku.init()` opens a start list: continue the saved game, generate a
//! new puzzle by difficulty, or play the card's own puzzle when the init
//! line declares one. Rust owns board state, conflict checks, the play
//! clock, dirty-cell invalidation and all redraw commands. Scripts never
//! receive panel, framebuffer or SPI access.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::{DirtyRect, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH},
    records::GameRecords,
    sudoku_puzzles::{generate, SudokuDifficulty},
    sudoku_save::SudokuSave,
};

pub const SUDOKU_CELL_COUNT: usize = 81;
/// Play seconds added for one key gap, at most; a game left open does not
/// run up the clock.
pub const MAX_EVENT_GAP_SECONDS: u32 = 60;
pub const SUDOKU_CELL_SIZE: i32 = 48;
pub const SUDOKU_GRID_X: i32 = (GAME_CANVAS_WIDTH - 9 * SUDOKU_CELL_SIZE) / 2;
pub const SUDOKU_GRID_Y: i32 = 100;
const SUDOKU_BAR_HEIGHT: i32 = 44;
/// The mockup's row band: a 5 px frame drawn 7 px outside the row.
const BAND_OUTSET: i32 = 7;
const BAND_WIDTH: i32 = 5;
const PICK_TOP: i32 = 548;
const PICK_WIDTH: i32 = 40;
const PICK_HEIGHT: i32 = 52;
const PICK_GAP: i32 = 4;
const PICK_LEFT: i32 = (GAME_CANVAS_WIDTH - (10 * PICK_WIDTH + 9 * PICK_GAP)) / 2;
/// Canvas text has no metrics, so chips are sized from a generous advance.
const CHAR_WIDTH: i32 = 11;
const SUDOKU_PICK_RECT: DirtyRect = DirtyRect::new(
    PICK_LEFT,
    PICK_TOP,
    10 * PICK_WIDTH + 9 * PICK_GAP,
    PICK_HEIGHT,
);
const SUDOKU_STATUS_RECT: DirtyRect = DirtyRect::new(16, 612, 448, 96);
const SUDOKU_FULL_RECT: DirtyRect = DirtyRect::new(0, 0, GAME_CANVAS_WIDTH, GAME_CANVAS_HEIGHT);
/// The play clock on the bar's right.
const SUDOKU_TIME_RECT: DirtyRect =
    DirtyRect::new(GAME_CANVAS_WIDTH - 140, 0, 140, SUDOKU_BAR_HEIGHT);
/// Mixed with the press time, so every `New` game differs.
const GENERATOR_SEED: u32 = 20_260_108;

/// The screens of one Sudoku app: the start list, then the three-step
/// entry over the board.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SudokuStep {
    Start,
    Row,
    Cell,
    Number,
}

impl SudokuStep {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Row => "row",
            Self::Cell => "cell",
            Self::Number => "number",
        }
    }

    /// The mockup's chip caption, numbered like `1 · ROW`; the start list
    /// has no chip.
    #[must_use]
    pub const fn chip_label(self) -> &'static str {
        match self {
            Self::Start => "START",
            Self::Row => "1 · ROW",
            Self::Cell => "2 · CELL",
            Self::Number => "3 · NUMBER",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SudokuEventResult {
    pub reason: &'static str,
    pub row: usize,
    pub column: usize,
    pub step: SudokuStep,
    pub candidate: u8,
    pub completed: bool,
    pub dirty_regions: Vec<DirtyRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SudokuGame {
    board: [u8; SUDOKU_CELL_COUNT],
    givens: [bool; SUDOKU_CELL_COUNT],
    step: SudokuStep,
    /// Highlighted row of the Row and Cell steps.
    row: usize,
    /// Position within the row's editable cells while in the Cell step.
    cell_choice: usize,
    /// Highlighted cell of the grid, derived from row and cell choice.
    cursor: usize,
    /// 1 to 9, or 0 for the erase entry.
    candidate: u8,
    status: String,
    completed: bool,
    /// Difficulty of a generated or resumed puzzle; `None` for the card's
    /// own puzzle, which keeps no save and no best time.
    difficulty: Option<SudokuDifficulty>,
    /// The original puzzle grid, 0 for empty cells, saved for resume.
    puzzle: [u8; SUDOKU_CELL_COUNT],
    /// Accumulated play time in seconds.
    seconds: u32,
    /// Last event's clock, in milliseconds; None until the first press.
    last_event_ms: Option<u64>,
    /// Best times the runtime handed in, for the solved message.
    records: GameRecords,
    /// Highlighted row of the start list.
    start_cursor: usize,
    /// The resumable save the runtime handed in, until a game starts.
    resumed: Option<SudokuSave>,
    /// The fixed puzzle of a `sudoku.init("...")` card, if any.
    sd_puzzle: Option<String>,
}

/// One row of the start list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartOption {
    pub label: String,
    pub kind: StartKind,
}

/// What SELECT does on a start-list row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartKind {
    /// Resume the saved game.
    Continue,
    /// Generate a new puzzle of this difficulty.
    New(SudokuDifficulty),
    /// Play the card's fixed puzzle.
    SdPuzzle,
}

impl SudokuGame {
    /// The start list over a save and the card's init line. Opening Sudoku
    /// lands here.
    #[must_use]
    pub fn start_list(save: Option<SudokuSave>, sd_puzzle: Option<String>) -> Self {
        let mut game = Self::new(
            [0; SUDOKU_CELL_COUNT],
            [false; SUDOKU_CELL_COUNT],
            [0; SUDOKU_CELL_COUNT],
            None,
            0,
        );
        game.step = SudokuStep::Start;
        game.resumed = save;
        game.sd_puzzle = sd_puzzle;
        game
    }

    /// Hand in the runtime's save and best times before the first frame.
    pub fn prepare(&mut self, save: Option<SudokuSave>, records: GameRecords) {
        self.resumed = save;
        self.records = records;
    }

    /// The start list rows, in order.
    #[must_use]
    pub fn start_options(&self) -> Vec<StartOption> {
        let mut options = Vec::new();
        if let Some(save) = &self.resumed {
            let filled = save.board.iter().filter(|&&cell| cell != 0).count();
            options.push(StartOption {
                label: format!("Continue · {} · {filled}/81", save.difficulty.label()),
                kind: StartKind::Continue,
            });
        }
        for difficulty in SudokuDifficulty::ALL {
            options.push(StartOption {
                label: format!("New · {}", difficulty.label()),
                kind: StartKind::New(difficulty),
            });
        }
        if self.sd_puzzle.is_some() {
            options.push(StartOption {
                label: "SD puzzle".into(),
                kind: StartKind::SdPuzzle,
            });
        }
        options
    }

    #[must_use]
    pub const fn start_cursor(&self) -> usize {
        self.start_cursor
    }

    pub fn from_puzzle(source: &str) -> Result<Self, String> {
        if source.len() != SUDOKU_CELL_COUNT {
            return Err(format!(
                "sudoku puzzle must contain {SUDOKU_CELL_COUNT} cells"
            ));
        }
        let mut board = [0_u8; SUDOKU_CELL_COUNT];
        let mut givens = [false; SUDOKU_CELL_COUNT];
        for (index, byte) in source.bytes().enumerate() {
            board[index] = match byte {
                b'0' | b'.' => 0,
                b'1'..=b'9' => byte - b'0',
                _ => return Err("sudoku puzzle accepts only digits, zero or dot".into()),
            };
            givens[index] = board[index] != 0;
        }
        validate_initial_board(&board)?;
        Ok(Self::new(board, givens, board, None, 0))
    }

    /// Build a game over a board, its original puzzle, difficulty and the
    /// played seconds.
    #[must_use]
    pub fn new(
        board: [u8; SUDOKU_CELL_COUNT],
        givens: [bool; SUDOKU_CELL_COUNT],
        puzzle: [u8; SUDOKU_CELL_COUNT],
        difficulty: Option<SudokuDifficulty>,
        seconds: u32,
    ) -> Self {
        let completed = is_complete(&board);
        let (row, cell_choice, cursor) = Self::entry_point(&givens);
        Self {
            board,
            givens,
            step: SudokuStep::Row,
            row,
            cell_choice,
            cursor,
            candidate: 1,
            status: if completed {
                "Puzzle complete".into()
            } else {
                String::new()
            },
            completed,
            difficulty,
            puzzle,
            seconds,
            last_event_ms: None,
            records: GameRecords::default(),
            start_cursor: 0,
            resumed: None,
            sd_puzzle: None,
        }
    }

    /// The original puzzle grid, 0 for empty cells.
    #[must_use]
    pub const fn puzzle(&self) -> &[u8; SUDOKU_CELL_COUNT] {
        &self.puzzle
    }

    /// The game's difficulty, for the title bar and the records.
    #[must_use]
    pub const fn difficulty(&self) -> Option<SudokuDifficulty> {
        self.difficulty
    }

    /// Accumulated play time in seconds.
    #[must_use]
    pub const fn seconds(&self) -> u32 {
        self.seconds
    }

    /// Add the elapsed time since the previous press, capped at 60 s, so a
    /// game left open does not run up the clock. Call once per event.
    pub fn on_event_clock(&mut self, now_ms: u64) {
        if let Some(previous) = self.last_event_ms {
            let gap_seconds = ((now_ms.saturating_sub(previous)) / 1000) as u32;
            self.seconds += gap_seconds.min(MAX_EVENT_GAP_SECONDS);
        }
        self.last_event_ms = Some(now_ms);
    }

    /// The first row and cell with something to fill.
    fn entry_point(givens: &[bool; SUDOKU_CELL_COUNT]) -> (usize, usize, usize) {
        let row = (0..9)
            .find(|&row| !Self::editable_columns(givens, row).is_empty())
            .unwrap_or(0);
        let column = Self::editable_columns(givens, row)
            .first()
            .copied()
            .unwrap_or(0);
        (row, 0, row * 9 + column)
    }

    /// Columns of one row the player may still write.
    fn editable_columns(givens: &[bool; SUDOKU_CELL_COUNT], row: usize) -> Vec<usize> {
        (0..9).filter(|&column| !givens[row * 9 + column]).collect()
    }

    #[must_use]
    pub const fn step(&self) -> SudokuStep {
        self.step
    }

    #[must_use]
    pub const fn cursor_row(&self) -> usize {
        self.cursor / 9
    }

    #[must_use]
    pub const fn cursor_column(&self) -> usize {
        self.cursor % 9
    }

    #[must_use]
    pub const fn candidate(&self) -> u8 {
        self.candidate
    }

    #[must_use]
    pub const fn completed(&self) -> bool {
        self.completed
    }

    #[must_use]
    pub fn board(&self) -> &[u8; SUDOKU_CELL_COUNT] {
        &self.board
    }

    /// Values the highlighted cell could still take: absent from its row,
    /// column and box.
    #[must_use]
    pub fn options(&self) -> Vec<u8> {
        cell_options(&self.board, self.cursor)
    }

    pub fn render_initial(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        canvas.invalidate_rect(DirtyRect::new(0, 0, 480, 800));
        canvas.request_refresh();
        Ok(())
    }

    pub fn apply_button_and_render(
        &mut self,
        event: ButtonEvent,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let old = self.snapshot();
        let old_seconds = self.seconds;
        self.on_event_clock(now_ms);
        let reason = match self.step {
            SudokuStep::Start => self.apply_start_button(event, now_ms),
            SudokuStep::Row => self.apply_row_button(event),
            SudokuStep::Cell => self.apply_cell_button(event),
            SudokuStep::Number => self.apply_number_button(event),
        };
        self.finish_and_render(reason, &old, old_seconds, canvas)
    }

    pub fn apply_boot_short_press_and_render(
        &mut self,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let old = self.snapshot();
        let old_seconds = self.seconds;
        self.on_event_clock(now_ms);
        let reason = match self.step {
            SudokuStep::Start => "noop",
            SudokuStep::Row => "noop",
            SudokuStep::Cell => {
                self.step = SudokuStep::Row;
                "back-to-row"
            }
            SudokuStep::Number => {
                self.step = SudokuStep::Cell;
                "back-to-cell"
            }
        };
        self.finish_and_render(reason, &old, old_seconds, canvas)
    }

    fn apply_start_button(&mut self, event: ButtonEvent, now_ms: u64) -> &'static str {
        let count = self.start_options().len().max(1);
        match event {
            ButtonEvent::Up => {
                self.start_cursor = self.start_cursor.checked_sub(1).unwrap_or(count - 1);
                "start-move"
            }
            ButtonEvent::Down => {
                self.start_cursor = (self.start_cursor + 1) % count;
                "start-move"
            }
            ButtonEvent::Select => {
                self.begin_start_option(self.start_cursor, now_ms);
                "start-choose"
            }
        }
    }

    /// Start the highlighted option: resume the save, generate a new puzzle
    /// or play the card's fixed puzzle. The press time seeds the generator.
    fn begin_start_option(&mut self, index: usize, now_ms: u64) {
        let options = self.start_options();
        let Some(option) = options.get(index) else {
            return;
        };
        let started = match option.kind {
            StartKind::Continue => self.resumed.map(|save| {
                let givens = save.puzzle.map(|cell| cell != 0);
                Self::new(
                    save.board,
                    givens,
                    save.puzzle,
                    Some(save.difficulty),
                    save.seconds,
                )
            }),
            StartKind::New(difficulty) => {
                let seed = GENERATOR_SEED ^ now_ms as u32;
                let puzzle = generate(difficulty, seed).puzzle;
                let givens = puzzle.map(|cell| cell != 0);
                Some(Self::new(puzzle, givens, puzzle, Some(difficulty), 0))
            }
            StartKind::SdPuzzle => self
                .sd_puzzle
                .as_deref()
                .and_then(|source| Self::from_puzzle(source).ok()),
        };
        if let Some(mut started) = started {
            started.records = self.records;
            // The clock runs from the starting press.
            started.last_event_ms = Some(now_ms);
            *self = started;
        }
    }

    fn snapshot(&self) -> (usize, usize, SudokuStep, u8) {
        (self.row, self.cursor, self.step, self.candidate)
    }

    fn finish_and_render(
        &mut self,
        reason: &'static str,
        old: &(usize, usize, SudokuStep, u8),
        old_seconds: u32,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if old.2 == self.step && self.step != SudokuStep::Start {
            let mut regions = vec![
                focus_rect(old),
                focus_rect(&self.snapshot()),
                SUDOKU_STATUS_RECT,
            ];
            regions.dedup();
            if self.step == SudokuStep::Number {
                regions.push(SUDOKU_PICK_RECT);
            }
            if self.seconds != old_seconds {
                regions.push(SUDOKU_TIME_RECT);
            }
            regions
        } else {
            // A new step redraws the chips, the highlight, the number strip
            // and the bottom bar.
            vec![SUDOKU_FULL_RECT]
        };
        for rect in &dirty_regions {
            canvas.invalidate_rect(*rect);
        }
        canvas.request_refresh();
        Ok(SudokuEventResult {
            reason,
            row: self.cursor_row(),
            column: self.cursor_column(),
            step: self.step,
            candidate: self.candidate,
            completed: self.completed,
            dirty_regions,
        })
    }

    fn apply_row_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up => {
                self.row = self.neighbour_row(-1);
                self.sync_cursor();
                self.status.clear();
                "row-move"
            }
            ButtonEvent::Down => {
                self.row = self.neighbour_row(1);
                self.sync_cursor();
                self.status.clear();
                "row-move"
            }
            ButtonEvent::Select => {
                self.step = SudokuStep::Cell;
                self.cell_choice = 0;
                self.sync_cursor();
                self.status.clear();
                "cell-enter"
            }
        }
    }

    fn apply_cell_button(&mut self, event: ButtonEvent) -> &'static str {
        let editable = Self::editable_columns(&self.givens, self.row);
        match event {
            ButtonEvent::Up => {
                self.cell_choice = if editable.is_empty() {
                    0
                } else {
                    self.cell_choice
                        .checked_sub(1)
                        .unwrap_or(editable.len() - 1)
                };
                self.sync_cursor();
                self.status.clear();
                "cell-move"
            }
            ButtonEvent::Down => {
                self.cell_choice = if editable.is_empty() {
                    0
                } else {
                    (self.cell_choice + 1) % editable.len()
                };
                self.sync_cursor();
                self.status.clear();
                "cell-move"
            }
            ButtonEvent::Select => {
                self.step = SudokuStep::Number;
                self.candidate = match self.board[self.cursor] {
                    0 => self.options().first().copied().unwrap_or(1),
                    value => value,
                };
                self.status.clear();
                "number-enter"
            }
        }
    }

    fn apply_number_button(&mut self, event: ButtonEvent) -> &'static str {
        match event {
            ButtonEvent::Up => {
                self.candidate = if self.candidate == 0 {
                    9
                } else {
                    self.candidate - 1
                };
                self.status.clear();
                "number-change"
            }
            ButtonEvent::Down => {
                self.candidate = if self.candidate >= 9 {
                    0
                } else {
                    self.candidate + 1
                };
                self.status.clear();
                "number-change"
            }
            ButtonEvent::Select => {
                if self.candidate != 0 && conflicts(&self.board, self.cursor, self.candidate) {
                    self.status = "Conflict: value already used".into();
                    return "conflict";
                }
                self.board[self.cursor] = self.candidate;
                self.givens[self.cursor] = false;
                self.completed = is_complete(&self.board);
                self.status = if self.completed {
                    self.solved_status()
                } else if self.candidate == 0 {
                    "Cell cleared".into()
                } else {
                    "Value placed".into()
                };
                self.step = SudokuStep::Cell;
                self.candidate = self.board[self.cursor];
                "place"
            }
        }
    }

    /// `Solved in 12:41 · best 11:02`, or `new best` when it beat the
    /// stored one. The card's own puzzle keeps no best.
    fn solved_status(&self) -> String {
        let time = time_text(self.seconds);
        let Some(difficulty) = self.difficulty else {
            return format!("Solved in {time}");
        };
        match self.records.sudoku_best(difficulty) {
            best if best != 0 && best <= self.seconds => {
                format!("Solved in {time} · best {}", time_text(best))
            }
            _ => format!("Solved in {time} · new best"),
        }
    }

    /// The editable row above or below, skipping all-given rows.
    fn neighbour_row(&self, direction: i32) -> usize {
        for distance in 1..=9 {
            let row = ((self.row as i32 + direction * distance).rem_euclid(9)) as usize;
            if !Self::editable_columns(&self.givens, row).is_empty() {
                return row;
            }
        }
        self.row
    }

    /// Keep the cursor on the cell the two cursors currently point at.
    fn sync_cursor(&mut self) {
        let editable = Self::editable_columns(&self.givens, self.row);
        if editable.is_empty() {
            self.cursor = self.row * 9;
            return;
        }
        let choice = self.cell_choice.min(editable.len() - 1);
        self.cursor = self.row * 9 + editable[choice];
    }

    fn render_commands(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        canvas.clear_frame();
        canvas.rect(0, 0, GAME_CANVAS_WIDTH, SUDOKU_BAR_HEIGHT, true)?;
        canvas.text(16, 30, self.title_text(), CanvasTextStyle::Inverse)?;
        if self.step == SudokuStep::Start {
            self.draw_start_list(canvas)?;
            canvas.request_refresh();
            return Ok(());
        }
        let time = time_text(self.seconds);
        canvas.text_right(GAME_CANVAS_WIDTH - 16, 30, time, CanvasTextStyle::Inverse)?;
        self.draw_step_strip(canvas)?;
        self.draw_grid(canvas)?;
        if self.step == SudokuStep::Number {
            self.draw_pick_strip(canvas)?;
        }
        canvas.text(24, 644, self.info_line(), CanvasTextStyle::Body)?;
        canvas.text(24, 684, self.status.clone(), CanvasTextStyle::Detail)?;
        canvas.request_refresh();
        Ok(())
    }

    /// `Sudoku · Medium` on the bar's left; the clock is drawn on its right.
    fn title_text(&self) -> String {
        match self.difficulty {
            Some(difficulty) => format!("Sudoku · {}", difficulty.label()),
            None => "Sudoku".into(),
        }
    }

    /// The mockup's start list: one boxed row per option, the selected one
    /// inverted.
    fn draw_start_list(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let options = self.start_options();
        for (index, option) in options.iter().enumerate() {
            let top = 140 + index as i32 * 64;
            let selected = index == self.start_cursor;
            canvas.rect(24, top, 432, 52, selected)?;
            let style = if selected {
                CanvasTextStyle::Inverse
            } else {
                CanvasTextStyle::Body
            };
            canvas.text(40, top + 34, option.label.clone(), style)?;
        }
        Ok(())
    }

    /// Cells, 3 px box lines and border, digits, and the step's highlight:
    /// the row band, or the cursor cell inverted.
    fn draw_grid(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let (left, top, cell) = (SUDOKU_GRID_X, SUDOKU_GRID_Y, SUDOKU_CELL_SIZE);
        let size = 9 * cell;
        canvas.grid(left, top, 9, 9, cell, cell)?;
        canvas.rect(left - 1, top - 1, size + 3, size + 3, false)?;
        canvas.rect(left + 1, top + 1, size - 1, size - 1, false)?;
        for offset in [3, 6] {
            let (x, y) = (left + offset * cell, top + offset * cell);
            for delta in [-1, 1] {
                canvas.line(x + delta, top, x + delta, top + size)?;
                canvas.line(left, y + delta, left + size, y + delta)?;
            }
        }
        let inverted = (self.step != SudokuStep::Row).then_some(self.cursor);
        for (index, value) in self.board.iter().copied().enumerate() {
            if value == 0 || inverted == Some(index) {
                continue;
            }
            let (x, y) = cell_origin(index);
            let style = if self.givens[index] {
                CanvasTextStyle::Heading
            } else {
                CanvasTextStyle::Body
            };
            canvas.text(x + 18, y + 33, value.to_string(), style)?;
        }
        let focus = focus_rect(&self.snapshot());
        if self.step == SudokuStep::Row {
            let DirtyRect {
                x,
                y,
                width,
                height,
            } = focus;
            for (band_x, band_y, band_width, band_height) in [
                (x, y, width, BAND_WIDTH),
                (x, y + height - BAND_WIDTH, width, BAND_WIDTH),
                (x, y, BAND_WIDTH, height),
                (x + width - BAND_WIDTH, y, BAND_WIDTH, height),
            ] {
                canvas.rect(band_x, band_y, band_width, band_height, true)?;
            }
            return Ok(());
        }
        canvas.rect(focus.x + 1, focus.y + 1, cell - 1, cell - 1, true)?;
        // The Number step previews the candidate inside the cell.
        let shown = match self.step {
            SudokuStep::Number if self.candidate != 0 => self.candidate,
            _ => self.board[self.cursor],
        };
        if shown != 0 {
            canvas.text(
                focus.x + 18,
                focus.y + 33,
                shown.to_string(),
                CanvasTextStyle::InverseHeading,
            )?;
        }
        Ok(())
    }

    /// `1 · ROW › 2 · CELL › 3 · NUMBER`, centered, the current chip inverted.
    fn draw_step_strip(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        const STEPS: [SudokuStep; 3] = [SudokuStep::Row, SudokuStep::Cell, SudokuStep::Number];
        const ARROW: i32 = 24;
        let total = STEPS.iter().map(|&step| chip_width(step)).sum::<i32>() + 2 * ARROW;
        let mut x = (GAME_CANVAS_WIDTH - total) / 2;
        for step in STEPS {
            let width = chip_width(step);
            let label = step.chip_label().to_string();
            if step == self.step {
                canvas.rect(x, 56, width, 30, true)?;
                canvas.text(x + 8, 77, label, CanvasTextStyle::Inverse)?;
            } else {
                canvas.rect(x, 56, width, 30, false)?;
                canvas.rect(x + 1, 57, width - 2, 28, false)?;
                canvas.text(x + 8, 77, label, CanvasTextStyle::Body)?;
            }
            x += width;
            if step != SudokuStep::Number {
                canvas.text(x + 8, 77, "\u{203a}".into(), CanvasTextStyle::Body)?;
                x += ARROW;
            }
        }
        Ok(())
    }

    /// The mockup's number strip: boxes for 1 to 9 and erase, the choice
    /// inverted, values already around the cell thin and struck through.
    fn draw_pick_strip(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let (top, width, height) = (PICK_TOP, PICK_WIDTH, PICK_HEIGHT);
        for (index, value) in (1..=9).chain([0]).enumerate() {
            let left = PICK_LEFT + index as i32 * (width + PICK_GAP);
            let label = display_candidate(value);
            let (x, y) = (left + 14, top + 34);
            if value == self.candidate {
                canvas.rect(left, top, width, height, true)?;
                canvas.text(x, y, label, CanvasTextStyle::InverseHeading)?;
            } else if value != 0 && conflicts(&self.board, self.cursor, value) {
                canvas.rect(left, top, width, height, false)?;
                canvas.text(x, y, label, CanvasTextStyle::Body)?;
                canvas.line(left + 8, top + 40, left + 32, top + 12)?;
            } else {
                canvas.rect(left, top, width, height, false)?;
                canvas.rect(left + 1, top + 1, width - 2, height - 2, false)?;
                canvas.text(x, y, label, CanvasTextStyle::Heading)?;
            }
        }
        Ok(())
    }

    /// `Row 5 · 4 to fill` while choosing a row, then
    /// `Row 5 · Col 3 · options: 2 · 6 · 9` as in the mockup.
    fn info_line(&self) -> String {
        let row = self.cursor_row() + 1;
        if self.step == SudokuStep::Row {
            let open = (0..9)
                .filter(|&column| self.board[self.row * 9 + column] == 0)
                .count();
            return format!("Row {row} · {open} to fill");
        }
        let options = self
            .options()
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" · ");
        let options = if options.is_empty() {
            "none".to_string()
        } else {
            options
        };
        format!(
            "Row {row} · Col {} · options: {options}",
            self.cursor_column() + 1
        )
    }
}

/// Values absent from `index`'s row, column and box.
fn cell_options(board: &[u8; SUDOKU_CELL_COUNT], index: usize) -> Vec<u8> {
    (1..=9_u8)
        .filter(|&value| !conflicts(board, index, value))
        .collect()
}

fn validate_initial_board(board: &[u8; SUDOKU_CELL_COUNT]) -> Result<(), String> {
    for (index, value) in board.iter().copied().enumerate() {
        if value != 0 && conflicts(board, index, value) {
            return Err(format!(
                "sudoku puzzle contains a conflict at cell {}",
                index + 1
            ));
        }
    }
    Ok(())
}

fn conflicts(board: &[u8; SUDOKU_CELL_COUNT], index: usize, value: u8) -> bool {
    let row = index / 9;
    let column = index % 9;
    for peer in 0..9 {
        let row_index = row * 9 + peer;
        let column_index = peer * 9 + column;
        if row_index != index && board[row_index] == value {
            return true;
        }
        if column_index != index && board[column_index] == value {
            return true;
        }
    }
    let block_row = (row / 3) * 3;
    let block_column = (column / 3) * 3;
    for block_y in 0..3 {
        for block_x in 0..3 {
            let peer = (block_row + block_y) * 9 + block_column + block_x;
            if peer != index && board[peer] == value {
                return true;
            }
        }
    }
    false
}

fn is_complete(board: &[u8; SUDOKU_CELL_COUNT]) -> bool {
    board.iter().all(|value| *value != 0)
        && board
            .iter()
            .copied()
            .enumerate()
            .all(|(index, value)| !conflicts(board, index, value))
}

/// Top-left corner of a cell, on its grid lines.
fn cell_origin(index: usize) -> (i32, i32) {
    (
        SUDOKU_GRID_X + (index % 9) as i32 * SUDOKU_CELL_SIZE,
        SUDOKU_GRID_Y + (index / 9) as i32 * SUDOKU_CELL_SIZE,
    )
}

fn chip_width(step: SudokuStep) -> i32 {
    step.chip_label().chars().count() as i32 * CHAR_WIDTH + 16
}

/// The highlight of a snapshot: the row band during the Row step, otherwise
/// the single cell.
fn focus_rect(snapshot: &(usize, usize, SudokuStep, u8)) -> DirtyRect {
    let (_row, cursor, step, _candidate) = *snapshot;
    let (x, y) = cell_origin(cursor);
    if step == SudokuStep::Row {
        DirtyRect::new(
            SUDOKU_GRID_X - BAND_OUTSET,
            y - BAND_OUTSET,
            9 * SUDOKU_CELL_SIZE + 2 * BAND_OUTSET + 1,
            SUDOKU_CELL_SIZE + 2 * BAND_OUTSET + 1,
        )
    } else {
        DirtyRect::new(x, y, SUDOKU_CELL_SIZE + 1, SUDOKU_CELL_SIZE + 1)
    }
}

/// The strip caption for one pick: a digit, or the erase entry. The atlas
/// lacks ⌫, so × stands in.
fn display_candidate(candidate: u8) -> String {
    match candidate {
        0 => "\u{d7}".into(),
        value => value.to_string(),
    }
}

/// Play-clock text: `7:41`, or `1:02:03` past an hour.
#[must_use]
pub fn time_text(seconds: u32) -> String {
    let (hours, rest) = (seconds / 3600, seconds % 3600);
    if hours > 0 {
        format!("{hours}:{:02}:{:02}", rest / 60, rest % 60)
    } else {
        format!("{}:{:02}", rest / 60, rest % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cell_options, display_candidate, SudokuGame, SudokuStep, BAND_WIDTH, SUDOKU_CELL_SIZE,
        SUDOKU_FULL_RECT,
    };
    use crate::{
        buttons::ButtonEvent,
        games::{
            canvas::{DrawCommand, NativeGameCanvas, MAX_GAME_DRAW_COMMANDS},
            dirty_regions::MAX_DIRTY_REGIONS,
        },
    };

    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";

    fn game() -> SudokuGame {
        SudokuGame::from_puzzle(PUZZLE).unwrap()
    }

    fn press(game: &mut SudokuGame, event: ButtonEvent) -> super::SudokuEventResult {
        let mut canvas = NativeGameCanvas::default();
        game.apply_button_and_render(event, 1_000, &mut canvas)
            .unwrap()
    }

    fn boot(game: &mut SudokuGame) -> super::SudokuEventResult {
        let mut canvas = NativeGameCanvas::default();
        game.apply_boot_short_press_and_render(2_000, &mut canvas)
            .unwrap()
    }

    #[test]
    fn starts_on_the_first_editable_row() {
        let game = game();
        assert_eq!(game.step(), SudokuStep::Row);
        assert_eq!(game.cursor_column(), 2);
        assert_eq!(game.candidate(), 1);
    }

    #[test]
    fn row_step_moves_down_one_row_in_pinned_direction() {
        let mut game = game();
        let moved = press(&mut game, ButtonEvent::Down);
        assert_eq!(moved.step, SudokuStep::Row);
        assert_eq!((moved.row, moved.column), (1, 1), "Down goes to row 1");
        let up = press(&mut game, ButtonEvent::Up);
        assert_eq!((up.row, up.column), (0, 2), "Up returns to row 0");
    }

    #[test]
    fn row_step_skips_all_given_rows() {
        let mut game = game();
        for column in 0..9 {
            game.board[9 + column] = 1 + column as u8;
            game.givens[9 + column] = true;
        }
        press(&mut game, ButtonEvent::Down);
        assert_eq!(game.cursor_row(), 2, "row 1 is skipped");
    }

    #[test]
    fn row_step_wraps_around() {
        let mut game = game();
        for _ in 0..8 {
            press(&mut game, ButtonEvent::Down);
        }
        let wrapped = press(&mut game, ButtonEvent::Down);
        assert_eq!(wrapped.row, 0, "Down from the last row wraps to row 0");
    }

    #[test]
    fn select_moves_row_to_cell_to_number() {
        let mut game = game();
        let cell = press(&mut game, ButtonEvent::Select);
        assert_eq!(cell.step, SudokuStep::Cell);
        // A new step redraws everything, the bottom bar included.
        assert_eq!(cell.dirty_regions, vec![SUDOKU_FULL_RECT]);
        let number = press(&mut game, ButtonEvent::Select);
        assert_eq!(number.step, SudokuStep::Number);
        assert_eq!(number.dirty_regions, vec![SUDOKU_FULL_RECT]);
    }

    #[test]
    fn cell_step_skips_givens_and_wraps() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        // Row 0 editable columns in PUZZLE: 2, 3, 5, 6, 7, 8.
        assert_eq!(game.cursor_column(), 2);
        let first = press(&mut game, ButtonEvent::Down);
        assert_eq!((first.row, first.column), (0, 3));
        for _ in 0..4 {
            press(&mut game, ButtonEvent::Down);
        }
        let wrapped = press(&mut game, ButtonEvent::Down);
        assert_eq!((wrapped.row, wrapped.column), (0, 2), "wraps to first");
    }

    #[test]
    fn number_step_walks_one_to_nine_then_erase() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        assert_eq!(game.candidate(), 1);
        for expected in [2, 3, 4, 5, 6, 7, 8, 9, 0, 1] {
            let changed = press(&mut game, ButtonEvent::Down);
            assert_eq!(changed.candidate, expected, "Down walks to {expected}");
        }
        let up = press(&mut game, ButtonEvent::Up);
        assert_eq!(up.candidate, 0, "Up goes back to erase");
    }

    #[test]
    fn place_sets_the_value_and_returns_to_cell() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        // Cell (0,2) takes 1 without conflict.
        let placed = press(&mut game, ButtonEvent::Select);
        assert_eq!(placed.reason, "place");
        assert_eq!(placed.step, SudokuStep::Cell);
        assert_eq!(game.board()[2], 1);
    }

    #[test]
    fn erase_clears_a_placed_value() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        assert_eq!(game.board()[2], 1);
        // Re-enter the Number step for the same cell, then walk to erase.
        press(&mut game, ButtonEvent::Select);
        for _ in 0..9 {
            press(&mut game, ButtonEvent::Down);
        }
        assert_eq!(game.candidate(), 0);
        press(&mut game, ButtonEvent::Select);
        assert_eq!(game.board()[2], 0, "erase clears the cell");
    }

    #[test]
    fn conflict_keeps_the_number_step() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        // Cell (0,2): 3 is a given in row 0, so walking to 3 conflicts.
        for _ in 0..2 {
            press(&mut game, ButtonEvent::Down);
        }
        assert_eq!(game.candidate(), 3);
        let conflict = press(&mut game, ButtonEvent::Select);
        assert_eq!(conflict.reason, "conflict");
        assert_eq!(conflict.step, SudokuStep::Number);
        assert_eq!(game.board()[2], 0, "conflicting value is not placed");
    }

    #[test]
    fn boot_back_walks_number_to_cell_to_row() {
        let mut game = game();
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        let to_cell = boot(&mut game);
        assert_eq!(to_cell.step, SudokuStep::Cell);
        assert_eq!(to_cell.dirty_regions, vec![SUDOKU_FULL_RECT]);
        let to_row = boot(&mut game);
        assert_eq!(to_row.step, SudokuStep::Row);
        // Row is the top step: BOOT does nothing and stays on the screen.
        let noop = boot(&mut game);
        assert_eq!(noop.reason, "noop");
        assert_eq!(noop.step, SudokuStep::Row);
    }

    #[test]
    fn options_leave_values_absent_from_row_column_and_box() {
        let game = game();
        // Cell (0,2): row 0 has 5,3,7; column 2 has 8,5,6; the box has
        // 5,3,6,1,9,8 — only 1, 2 and 4 stay.
        assert_eq!(cell_options(game.board(), 2), vec![1, 2, 4]);
    }

    #[test]
    fn busiest_number_render_stays_under_the_command_limit() {
        let mut game = game();
        // Checkerboard fill maximises runs; Number adds the pick strip.
        for row in 0..9_usize {
            for column in 0..9_usize {
                let index = row * 9 + column;
                if (row + column) % 2 == 0 {
                    game.board[index] = ((index % 9) + 1) as u8;
                    game.givens[index] = true;
                }
            }
        }
        while game.step() != SudokuStep::Number {
            press(&mut game, ButtonEvent::Select);
        }
        let mut canvas = NativeGameCanvas::default();
        game.render_commands(&mut canvas).unwrap();
        assert!(canvas.commands().len() < MAX_GAME_DRAW_COMMANDS);
    }

    #[test]
    fn invalid_regions_stay_bounded_on_step_changes() {
        let mut game = game();
        for _ in 0..6 {
            let result = press(&mut game, ButtonEvent::Select);
            assert!(result.dirty_regions.len() <= MAX_DIRTY_REGIONS);
        }
    }

    #[test]
    fn strip_and_options_read_like_the_mockup() {
        let mut game = game();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        let labels = canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(labels.iter().any(|text| text.contains("1 · ROW")));
        assert!(labels.iter().any(|text| text.contains("3 · NUMBER")));
        press(&mut game, ButtonEvent::Select);
        press(&mut game, ButtonEvent::Select);
        let mut canvas = NativeGameCanvas::default();
        game.render_commands(&mut canvas).unwrap();
        let labels = canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(labels.iter().any(|text| text.contains("Row 1 · Col 3")));
        assert!(labels.iter().any(|text| text.contains("options:")));
        assert!(labels.iter().any(|text| text == &display_candidate(1)));
    }

    #[test]
    fn row_band_and_cursor_cell_are_drawn_solid() {
        let mut game = game();
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        let band = super::focus_rect(&game.snapshot());
        assert!(canvas.commands().contains(&DrawCommand::Rect {
            x: band.x,
            y: band.y,
            width: band.width,
            height: BAND_WIDTH,
            filled: true,
        }));
        press(&mut game, ButtonEvent::Select);
        let mut canvas = NativeGameCanvas::default();
        game.render_commands(&mut canvas).unwrap();
        let (x, y) = super::cell_origin(game.cursor);
        assert!(canvas.commands().contains(&DrawCommand::Rect {
            x: x + 1,
            y: y + 1,
            width: SUDOKU_CELL_SIZE - 1,
            height: SUDOKU_CELL_SIZE - 1,
            filled: true,
        }));
    }

    #[test]
    fn number_step_starts_on_the_first_option() {
        let mut game = game();
        press(&mut game, ButtonEvent::Down);
        press(&mut game, ButtonEvent::Select);
        let number = press(&mut game, ButtonEvent::Select);
        assert_eq!((number.row, number.column), (1, 1));
        assert_eq!(number.candidate, 2, "options are 2 · 4 · 7");
    }
}

#[cfg(test)]
mod d24_tests {
    use super::{time_text, StartKind, SudokuGame, SudokuStep, MAX_EVENT_GAP_SECONDS};
    use crate::{
        buttons::ButtonEvent,
        games::{
            canvas::{CanvasTextStyle, DrawCommand, NativeGameCanvas},
            records::GameRecords,
            sudoku_puzzles::{generate, SudokuDifficulty},
            sudoku_save::SudokuSave,
        },
    };

    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";

    fn press(game: &mut SudokuGame, event: ButtonEvent, now_ms: u64) {
        let mut canvas = NativeGameCanvas::default();
        game.apply_button_and_render(event, now_ms, &mut canvas)
            .unwrap();
    }

    #[test]
    fn the_timer_caps_each_gap_and_accumulates() {
        let mut game = SudokuGame::from_puzzle(PUZZLE).unwrap();
        game.on_event_clock(0);
        // 10 s of real play.
        press(&mut game, ButtonEvent::Down, 10_000);
        assert_eq!(game.seconds(), 10);
        // A 5-minute pause only counts the 60 s cap.
        press(&mut game, ButtonEvent::Down, 10_000 + 300_000);
        assert_eq!(game.seconds(), 70);
        // Sub-second gaps add nothing.
        press(&mut game, ButtonEvent::Down, 70_500);
        assert_eq!(game.seconds(), 70);
        assert_eq!(MAX_EVENT_GAP_SECONDS, 60);
    }

    #[test]
    fn the_clock_text_matches_the_mockup_formats() {
        assert_eq!(time_text(7 * 60 + 41), "7:41");
        assert_eq!(time_text(3600 + 2 * 60 + 3), "1:02:03");
        assert_eq!(time_text(0), "0:00");
    }

    #[test]
    fn the_start_list_lists_continue_new_and_sd_puzzle() {
        let save = SudokuSave {
            difficulty: SudokuDifficulty::Medium,
            puzzle: [0; 81],
            board: [0; 81],
            seconds: 120,
        };
        let game = SudokuGame::start_list(Some(save), Some(PUZZLE.into()));
        assert_eq!(game.step(), SudokuStep::Start);
        let labels: Vec<String> = game
            .start_options()
            .iter()
            .map(|o| o.label.clone())
            .collect();
        assert_eq!(
            labels,
            [
                "Continue · Medium · 0/81",
                "New · Easy",
                "New · Medium",
                "New · Hard",
                "SD puzzle"
            ]
        );
        let kinds: Vec<StartKind> = game.start_options().iter().map(|o| o.kind).collect();
        assert_eq!(kinds[0], StartKind::Continue);
        assert_eq!(kinds[1], StartKind::New(SudokuDifficulty::Easy));
        assert_eq!(kinds[4], StartKind::SdPuzzle);
    }

    #[test]
    fn without_a_save_the_list_starts_at_new_easy() {
        let game = SudokuGame::start_list(None, None);
        let labels: Vec<String> = game
            .start_options()
            .iter()
            .map(|o| o.label.clone())
            .collect();
        assert_eq!(labels, ["New · Easy", "New · Medium", "New · Hard"]);
        assert_eq!(game.start_cursor(), 0);
    }

    #[test]
    fn select_on_new_generates_a_playable_puzzle_with_the_title_bar() {
        let mut game = SudokuGame::start_list(None, None);
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        press(&mut game, ButtonEvent::Down, 0); // New · Medium
        press(&mut game, ButtonEvent::Select, 500);
        assert_eq!(game.step(), SudokuStep::Row);
        assert_eq!(game.difficulty(), Some(SudokuDifficulty::Medium));
        assert_eq!(game.title_text(), "Sudoku · Medium");
        // The generated puzzle renders within the command budget, with the
        // clock right-aligned on the bar.
        let mut canvas = NativeGameCanvas::default();
        game.render_initial(&mut canvas).unwrap();
        assert!(canvas.commands().len() < crate::games::canvas::MAX_GAME_DRAW_COMMANDS);
        let clock = DrawCommand::TextRight {
            right: 464,
            y: 30,
            text: "0:00".into(),
            style: CanvasTextStyle::Inverse,
        };
        assert!(canvas.commands().contains(&clock));
    }

    #[test]
    fn new_games_are_seeded_by_the_press_time() {
        let start = |now_ms| {
            let mut game = SudokuGame::start_list(None, None);
            press(&mut game, ButtonEvent::Select, now_ms);
            *game.puzzle()
        };
        assert_eq!(start(1_000), start(1_000));
        assert_ne!(start(1_000), start(2_000));
    }

    #[test]
    fn the_clock_runs_from_the_starting_press() {
        let mut game = SudokuGame::start_list(None, None);
        press(&mut game, ButtonEvent::Down, 0);
        press(&mut game, ButtonEvent::Select, 30_000);
        assert_eq!(game.seconds(), 0, "time on the start list is not play");
        press(&mut game, ButtonEvent::Down, 42_000);
        assert_eq!(game.seconds(), 12);
    }

    #[test]
    fn solving_says_the_time_and_the_best() {
        let solve = |best: u32| {
            let generated = generate(SudokuDifficulty::Easy, 7);
            let last = generated.puzzle.iter().position(|&cell| cell == 0).unwrap();
            let mut board = generated.solution;
            board[last] = 0;
            let givens = board.map(|cell| cell != 0);
            let difficulty = Some(SudokuDifficulty::Easy);
            let mut game = SudokuGame::new(board, givens, board, difficulty, 100);
            let records = GameRecords {
                sudoku_easy: best,
                ..GameRecords::default()
            };
            game.prepare(None, records);
            for _ in 0..3 {
                press(&mut game, ButtonEvent::Select, 1_000);
            }
            assert!(game.completed());
            game.status.clone()
        };
        assert_eq!(solve(90), "Solved in 1:40 · best 1:30");
        assert_eq!(solve(0), "Solved in 1:40 · new best");
        assert_eq!(solve(120), "Solved in 1:40 · new best");
    }

    #[test]
    fn select_on_sd_puzzle_plays_the_card_puzzle() {
        let mut game = SudokuGame::start_list(None, Some(PUZZLE.into()));
        // Move to the last row (SD puzzle) and start.
        for _ in 0..3 {
            press(&mut game, ButtonEvent::Down, 1_000);
        }
        press(&mut game, ButtonEvent::Select, 2_000);
        assert_eq!(game.step(), SudokuStep::Row);
        assert_eq!(game.puzzle().len(), 81);
        assert!(game.puzzle()[0] == 5, "the SD puzzle's first given is 5");
    }

    #[test]
    fn resume_restores_the_board_and_seconds() {
        let mut puzzle_text = PUZZLE.to_string();
        puzzle_text.replace_range(..1, "0"); // one empty given to fill
        let save = SudokuSave {
            difficulty: SudokuDifficulty::Hard,
            puzzle: text_grid(&puzzle_text),
            board: text_grid(&puzzle_text),
            seconds: 777,
        };
        let mut game = SudokuGame::start_list(Some(save), None);
        press(&mut game, ButtonEvent::Select, 1_000);
        assert_eq!(game.step(), SudokuStep::Row);
        assert_eq!(game.seconds(), 777);
        assert_eq!(game.difficulty(), Some(SudokuDifficulty::Hard));
    }

    fn text_grid(text: &str) -> [u8; 81] {
        let mut grid = [0_u8; 81];
        for (index, byte) in text.bytes().enumerate() {
            grid[index] = byte - b'0';
        }
        grid
    }

    #[test]
    fn records_gain_the_three_sudoku_keys_and_observe_only_improves() {
        let mut records = GameRecords::default();
        assert!(records.observe_sudoku(SudokuDifficulty::Easy, 600));
        assert_eq!(records.sudoku_easy, 600);
        assert!(!records.observe_sudoku(SudokuDifficulty::Easy, 700));
        assert!(records.observe_sudoku(SudokuDifficulty::Easy, 500));
        assert_eq!(records.sudoku_easy, 500);
        // Other difficulties stay independent.
        assert!(records.observe_sudoku(SudokuDifficulty::Hard, 4_000));
        assert_eq!(records.sudoku_hard, 4_000);
        assert_eq!(records.sudoku_medium, 0);
        let round = GameRecords::parse(&records.serialized()).unwrap();
        assert_eq!(round, records);
    }
}
