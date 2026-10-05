//! Native Sudoku reference game for the SD Lua event bridge.
//!
//! The SD script declares one bounded puzzle through `sudoku.init(...)`.
//! Rust owns board state, conflict checks, dirty-cell invalidation and all
//! redraw commands. Scripts never receive panel, framebuffer or SPI access.

use crate::buttons::ButtonEvent;

use super::{
    canvas::{CanvasTextStyle, NativeGameCanvas},
    dirty_regions::{DirtyRect, GAME_BOTTOM_BAR_RECT},
};

pub const SUDOKU_CELL_COUNT: usize = 81;
pub const SUDOKU_GRID_X: i32 = 51;
pub const SUDOKU_GRID_Y: i32 = 176;
pub const SUDOKU_CELL_SIZE: i32 = 42;
const SUDOKU_STATUS_RECT: DirtyRect = DirtyRect::new(16, 612, 448, 96);

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
                "STEP 1 ROW: UP/DOWN move  SELECT choose".into()
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
                self.status = "STEP 1 ROW: UP/DOWN move  SELECT choose".into();
                "back-to-row"
            }
            SudokuStep::Number => {
                self.step = SudokuStep::Cell;
                self.status = "STEP 2 CELL: UP/DOWN move  SELECT choose".into();
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
        let mut dirty_regions = vec![
            cell_rect(old),
            cell_rect(&self.snapshot()),
            SUDOKU_STATUS_RECT,
        ];
        dirty_regions.dedup();
        if old.2 != self.step {
            dirty_regions.push(GAME_BOTTOM_BAR_RECT);
        }
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
                self.status = "STEP 1 ROW: UP/DOWN move  SELECT choose".into();
                "row-move"
            }
            ButtonEvent::Down => {
                self.row = self.neighbour_row(1);
                self.sync_cursor();
                self.status = "STEP 1 ROW: UP/DOWN move  SELECT choose".into();
                "row-move"
            }
            ButtonEvent::Select => {
                self.step = SudokuStep::Cell;
                self.cell_choice = 0;
                self.sync_cursor();
                self.status = "STEP 2 CELL: UP/DOWN move  SELECT choose".into();
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
                self.status = "STEP 2 CELL: UP/DOWN move  SELECT choose".into();
                "cell-move"
            }
            ButtonEvent::Down => {
                self.cell_choice = if editable.is_empty() {
                    0
                } else {
                    (self.cell_choice + 1) % editable.len()
                };
                self.sync_cursor();
                self.status = "STEP 2 CELL: UP/DOWN move  SELECT choose".into();
                "cell-move"
            }
            ButtonEvent::Select => {
                self.step = SudokuStep::Number;
                self.candidate = 1;
                self.status = "STEP 3 NUMBER: UP/DOWN value  SELECT place".into();
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
                self.status = format!(
                    "STEP 3 NUMBER: value {}  SELECT place",
                    display_candidate(self.candidate)
                );
                "number-change"
            }
            ButtonEvent::Down => {
                self.candidate = if self.candidate >= 9 {
                    0
                } else {
                    self.candidate + 1
                };
                self.status = format!(
                    "STEP 3 NUMBER: value {}  SELECT place",
                    display_candidate(self.candidate)
                );
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
        canvas.text(24, 66, "Sudoku".into(), CanvasTextStyle::Heading)?;
        self.draw_step_strip(canvas)?;
        canvas.grid(
            SUDOKU_GRID_X,
            SUDOKU_GRID_Y,
            9,
            9,
            SUDOKU_CELL_SIZE,
            SUDOKU_CELL_SIZE,
        )?;
        for offset in [3, 6] {
            let x = SUDOKU_GRID_X + offset * SUDOKU_CELL_SIZE;
            let y = SUDOKU_GRID_Y + offset * SUDOKU_CELL_SIZE;
            canvas.line(
                x - 1,
                SUDOKU_GRID_Y,
                x - 1,
                SUDOKU_GRID_Y + 9 * SUDOKU_CELL_SIZE,
            )?;
            canvas.line(
                x + 1,
                SUDOKU_GRID_Y,
                x + 1,
                SUDOKU_GRID_Y + 9 * SUDOKU_CELL_SIZE,
            )?;
            canvas.line(
                SUDOKU_GRID_X,
                y - 1,
                SUDOKU_GRID_X + 9 * SUDOKU_CELL_SIZE,
                y - 1,
            )?;
            canvas.line(
                SUDOKU_GRID_X,
                y + 1,
                SUDOKU_GRID_X + 9 * SUDOKU_CELL_SIZE,
                y + 1,
            )?;
        }
        for (index, value) in self.board.iter().copied().enumerate() {
            if value == 0 {
                continue;
            }
            let row = index / 9;
            let column = index % 9;
            canvas.text(
                SUDOKU_GRID_X + column as i32 * SUDOKU_CELL_SIZE + 14,
                SUDOKU_GRID_Y + row as i32 * SUDOKU_CELL_SIZE + 30,
                value.to_string(),
                if self.givens[index] {
                    CanvasTextStyle::Heading
                } else {
                    CanvasTextStyle::Body
                },
            )?;
        }
        match self.step {
            SudokuStep::Row => {
                // A band around the whole highlighted row.
                canvas.rect(
                    SUDOKU_GRID_X,
                    SUDOKU_GRID_Y + self.row as i32 * SUDOKU_CELL_SIZE,
                    9 * SUDOKU_CELL_SIZE,
                    SUDOKU_CELL_SIZE,
                    false,
                )?;
            }
            SudokuStep::Cell | SudokuStep::Number => {
                let cursor = cell_rect(&self.snapshot());
                canvas.rect(cursor.x, cursor.y, cursor.width, cursor.height, false)?;
                canvas.rect(
                    cursor.x + 2,
                    cursor.y + 2,
                    cursor.width - 4,
                    cursor.height - 4,
                    false,
                )?;
            }
        }
        if self.step == SudokuStep::Number {
            self.draw_pick_strip(canvas)?;
        }
        canvas.text(
            24,
            648,
            format!(
                "Row {} · Col {} · options: {}",
                self.cursor_row() + 1,
                self.cursor_column() + 1,
                self.options_line(),
            ),
            CanvasTextStyle::Body,
        )?;
        canvas.text(24, 690, self.status.clone(), CanvasTextStyle::Detail)?;
        canvas.request_refresh();
        Ok(())
    }

    /// `1 · ROW ▸ 2 · CELL ▸ 3 · NUMBER`, the current chip inverted.
    fn draw_step_strip(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        let mut x = 24;
        for step in [SudokuStep::Row, SudokuStep::Cell, SudokuStep::Number] {
            let label = step.chip_label();
            let width = label.len() as i32 * 11 + 16;
            if step == self.step {
                canvas.rect(x, 84, width, 30, true)?;
                canvas.text(x + 8, 106, label.into(), CanvasTextStyle::Inverse)?;
            } else {
                canvas.rect(x, 84, width, 30, false)?;
                canvas.text(x + 8, 106, label.into(), CanvasTextStyle::Detail)?;
            }
            x += width + 8;
            if step != SudokuStep::Number {
                canvas.text(x, 106, ">".into(), CanvasTextStyle::Detail)?;
                x += 20;
            }
        }
        Ok(())
    }

    /// The mockup's number strip: 1..9 then the erase entry, the choice
    /// inverted and values already placed around the cell struck through.
    fn draw_pick_strip(&self, canvas: &mut NativeGameCanvas) -> Result<(), String> {
        for (index, value) in (1..=9).chain([0]).enumerate() {
            let left = 12 + index as i32 * 46;
            let label = display_candidate(value);
            if value == self.candidate {
                canvas.rect(left, 560, 38, 34, true)?;
                canvas.text(left + 15, 584, label, CanvasTextStyle::Inverse)?;
            } else {
                canvas.text(left + 15, 584, label, CanvasTextStyle::Body)?;
                if value != 0 && conflicts(&self.board, self.cursor, value) {
                    canvas.line(left + 8, 582, left + 30, 558)?;
                }
            }
        }
        Ok(())
    }

    /// The options line payload: free values, or `none` when there are none.
    fn options_line(&self) -> String {
        let options = self
            .options()
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" · ");
        if options.is_empty() {
            "none".into()
        } else {
            options
        }
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

fn first_editable_cell(givens: &[bool; SUDOKU_CELL_COUNT]) -> Option<usize> {
    givens.iter().position(|given| !*given)
}

/// Grid rectangle of the snapshot: the row band during the Row step,
/// otherwise the single cell.
fn cell_rect(snapshot: &(usize, usize, SudokuStep, u8)) -> DirtyRect {
    let (_row, cursor, step, _candidate) = *snapshot;
    if step == SudokuStep::Row {
        DirtyRect::new(
            SUDOKU_GRID_X,
            SUDOKU_GRID_Y + (cursor / 9) as i32 * SUDOKU_CELL_SIZE,
            9 * SUDOKU_CELL_SIZE + 1,
            SUDOKU_CELL_SIZE + 1,
        )
    } else {
        DirtyRect::new(
            SUDOKU_GRID_X + (cursor % 9) as i32 * SUDOKU_CELL_SIZE,
            SUDOKU_GRID_Y + (cursor / 9) as i32 * SUDOKU_CELL_SIZE,
            SUDOKU_CELL_SIZE + 1,
            SUDOKU_CELL_SIZE + 1,
        )
    }
}

/// The strip caption for one pick: a digit, or the erase entry. The atlas
/// lacks ⌫, so the minus sign stands in.
fn display_candidate(candidate: u8) -> String {
    match candidate {
        0 => "\u{2212}".into(),
        value => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{cell_options, display_candidate, SudokuGame, SudokuStep, SUDOKU_CELL_COUNT};
    use crate::{
        buttons::ButtonEvent,
        games::{
            canvas::{DrawCommand, NativeGameCanvas, MAX_GAME_DRAW_COMMANDS},
            dirty_regions::{GAME_BOTTOM_BAR_RECT, MAX_DIRTY_REGIONS},
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
        assert!(cell.dirty_regions.contains(&GAME_BOTTOM_BAR_RECT));
        let number = press(&mut game, ButtonEvent::Select);
        assert_eq!(number.step, SudokuStep::Number);
        assert!(number.dirty_regions.contains(&GAME_BOTTOM_BAR_RECT));
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
        assert!(to_cell.dirty_regions.contains(&GAME_BOTTOM_BAR_RECT));
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
        let _ = SUDOKU_CELL_COUNT;
    }
}
