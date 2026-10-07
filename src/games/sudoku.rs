//! Native Sudoku reference game for the SD Lua event bridge.
//!
//! The SD script declares one bounded puzzle through `sudoku.init(...)`.
//! Rust owns board state, conflict checks, dirty-cell invalidation and all
//! redraw commands. Scripts never receive panel, framebuffer or SPI access.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::{DirtyRect, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH},
};

pub const SUDOKU_CELL_COUNT: usize = 81;
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

/// The three-step entry: pick a row, then a cell, then a number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SudokuStep {
    Row,
    Cell,
    Number,
}

impl SudokuStep {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Row => "row",
            Self::Cell => "cell",
            Self::Number => "number",
        }
    }

    /// The mockup's chip caption, numbered like `1 · ROW`.
    #[must_use]
    pub const fn chip_label(self) -> &'static str {
        match self {
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
}

impl SudokuGame {
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
        let completed = is_complete(&board);
        let (row, cell_choice, cursor) = Self::entry_point(&givens);
        Ok(Self {
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
        })
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
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let old = self.snapshot();
        let reason = match self.step {
            SudokuStep::Row => self.apply_row_button(event),
            SudokuStep::Cell => self.apply_cell_button(event),
            SudokuStep::Number => self.apply_number_button(event),
        };
        self.finish_and_render(reason, &old, canvas)
    }

    pub fn apply_boot_short_press_and_render(
        &mut self,
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        let old = self.snapshot();
        let reason = match self.step {
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
        self.finish_and_render(reason, &old, canvas)
    }

    fn snapshot(&self) -> (usize, usize, SudokuStep, u8) {
        (self.row, self.cursor, self.step, self.candidate)
    }

    fn finish_and_render(
        &mut self,
        reason: &'static str,
        old: &(usize, usize, SudokuStep, u8),
        canvas: &mut NativeGameCanvas,
    ) -> Result<SudokuEventResult, String> {
        self.render_commands(canvas)?;
        canvas.reset_dirty_regions();
        let dirty_regions = if old.2 == self.step {
            let mut regions = vec![
                focus_rect(old),
                focus_rect(&self.snapshot()),
                SUDOKU_STATUS_RECT,
            ];
            regions.dedup();
            if self.step == SudokuStep::Number {
                regions.push(SUDOKU_PICK_RECT);
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
                    "Puzzle complete".into()
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
        canvas.text(16, 30, "Sudoku".into(), CanvasTextStyle::Inverse)?;
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
        game.apply_button_and_render(event, &mut canvas).unwrap()
    }

    fn boot(game: &mut SudokuGame) -> super::SudokuEventResult {
        let mut canvas = NativeGameCanvas::default();
        game.apply_boot_short_press_and_render(&mut canvas).unwrap()
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
