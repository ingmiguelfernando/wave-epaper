//! Bounded SD Lua event bridge.
//!
//! Static bootstrap drawing remains available. Rust owns mutable Sudoku,
//! Minesweeper and Tetris state, redraw decisions
//! and hardware-facing input. SD declarations never receive raw QMI8658 or
//! panel access.

use crate::{
    buttons::ButtonEvent,
    games::{
        canvas::NativeGameCanvas,
        minesweeper::{MinesweeperEventResult, MinesweeperGame},
        sudoku::{SudokuEventResult, SudokuGame},
        tetris::{TetrisApp, TetrisEventResult, TetrisMode},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LuaGameEventResult {
    Sudoku(SudokuEventResult),
    Minesweeper(MinesweeperEventResult),
    Tetris(TetrisEventResult),
}
impl LuaGameEventResult {
    #[must_use]
    pub const fn bridge_marker(&self) -> &'static str {
        match self {
            Self::Sudoku(_) => "sudoku",
            Self::Minesweeper(_) => "minesweeper",
            Self::Tetris(_) => "tetris",
        }
    }
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Sudoku(r) => r.reason,
            Self::Minesweeper(r) => r.reason,
            Self::Tetris(r) => r.reason,
        }
    }
    #[must_use]
    pub const fn row(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.row,
            Self::Minesweeper(r) => r.row,
            Self::Tetris(r) => r.row,
        }
    }
    #[must_use]
    pub const fn column(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.column,
            Self::Minesweeper(r) => r.column,
            Self::Tetris(r) => r.column,
        }
    }
    #[must_use]
    pub const fn mode_marker(&self) -> &'static str {
        match self {
            Self::Sudoku(r) => r.step.marker(),
            Self::Minesweeper(r) => r.mode.marker(),
            Self::Tetris(r) => r.mode.marker(),
        }
    }
    #[must_use]
    pub const fn axis_marker(&self) -> &'static str {
        match self {
            // Sudoku's former axis log slot now carries the entry step.
            Self::Sudoku(r) => r.step.marker(),
            Self::Minesweeper(r) => r.axis.marker(),
            Self::Tetris(_) => "none",
        }
    }
    #[must_use]
    pub fn detail_marker(&self) -> String {
        match self {
            Self::Sudoku(r) => format!("candidate={}", r.candidate),
            Self::Minesweeper(r) => format!(
                "action={} flags={} safe-left={} outcome={}",
                r.action.marker(),
                r.flags,
                r.safe_left,
                r.outcome.marker()
            ),
            Self::Tetris(r) => format!(
                "action={} score={} lines={} level={}",
                r.action.map_or("none", |action| action.marker()),
                r.score,
                r.lines,
                r.level
            ),
        }
    }
    #[must_use]
    pub const fn completed(&self) -> bool {
        match self {
            Self::Sudoku(r) => r.completed,
            Self::Minesweeper(r) => r.outcome.completed(),
            Self::Tetris(r) => r.completed,
        }
    }
    #[must_use]
    pub fn dirty_regions_len(&self) -> usize {
        match self {
            Self::Sudoku(r) => r.dirty_regions.len(),
            Self::Minesweeper(r) => r.dirty_regions.len(),
            Self::Tetris(r) => r.dirty_regions.len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LuaEventBridge {
    Static,
    Sudoku(SudokuGame),
    Minesweeper(MinesweeperGame),
    Tetris(TetrisApp),
}
impl LuaEventBridge {
    pub fn load(source: &str, canvas: &mut NativeGameCanvas) -> Result<Self, String> {
        if let Some((columns, rows, mines, seed)) = parse_minesweeper_init(source)? {
            let game = MinesweeperGame::from_config(columns, rows, mines, seed)?;
            game.render_initial(canvas)?;
            Ok(Self::Minesweeper(game))
        } else if let Some(puzzle) = parse_sudoku_init(source)? {
            // A declared puzzle is checked now and offered as `SD puzzle`.
            if let Some(puzzle) = &puzzle {
                SudokuGame::from_puzzle(puzzle)?;
            }
            let game = SudokuGame::start_list(None, puzzle);
            game.render_initial(canvas)?;
            Ok(Self::Sudoku(game))
        } else if let Some((mode, seed)) = parse_tetris_init(source)? {
            let game = TetrisApp::new(mode, seed);
            game.render_initial(canvas)?;
            Ok(Self::Tetris(game))
        } else {
            super::bootstrap::execute_bootstrap_script(source, canvas)?;
            Ok(Self::Static)
        }
    }
    #[must_use]
    pub const fn marker(&self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Sudoku(_) => "sudoku",
            Self::Minesweeper(_) => "minesweeper",
            Self::Tetris(_) => "tetris",
        }
    }
    pub fn apply_button(
        &mut self,
        event: ButtonEvent,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<LuaGameEventResult>, String> {
        match self {
            Self::Static => Ok(None),
            Self::Sudoku(g) => g
                .apply_button_and_render(event, now_ms, canvas)
                .map(LuaGameEventResult::Sudoku)
                .map(Some),
            Self::Minesweeper(g) => g
                .apply_button_and_render(event, canvas)
                .map(LuaGameEventResult::Minesweeper)
                .map(Some),
            Self::Tetris(g) => g
                .apply_button_and_render(event, canvas)
                .map(LuaGameEventResult::Tetris)
                .map(Some),
        }
    }
    pub fn apply_boot_short_press(
        &mut self,
        now_ms: u64,
        canvas: &mut NativeGameCanvas,
    ) -> Result<Option<LuaGameEventResult>, String> {
        match self {
            Self::Static => Ok(None),
            Self::Sudoku(g) => g
                .apply_boot_short_press_and_render(now_ms, canvas)
                .map(LuaGameEventResult::Sudoku)
                .map(Some),
            Self::Minesweeper(g) => g
                .apply_boot_short_press_and_render(canvas)
                .map(LuaGameEventResult::Minesweeper)
                .map(Some),
            Self::Tetris(g) => g
                .apply_boot_short_press_and_render(canvas)
                .map(LuaGameEventResult::Tetris)
                .map(Some),
        }
    }
}
/// `sudoku.init()` opens the start list; `sudoku.init("…")` also offers the
/// card's own puzzle. `None` when the script is not a Sudoku card.
fn parse_sudoku_init(source: &str) -> Result<Option<Option<String>>, String> {
    parse_single_quoted_init(source, "sudoku.init(", "sudoku")
}
fn parse_single_quoted_init(
    source: &str,
    prefix: &str,
    label: &str,
) -> Result<Option<Option<String>>, String> {
    let mut value = None;
    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(prefix) || !line.ends_with(')') {
            if line.starts_with(&format!("{label}.")) {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported {label} call"
                ));
            }
            return Ok(None);
        }
        if value.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate {label}.init"
            ));
        }
        let argument = line[prefix.len()..line.len() - 1].trim();
        if argument.is_empty() {
            value = Some(None);
            continue;
        }
        let parsed = parse_quoted(argument, label)
            .map_err(|error| format!("MAIN.LUA line {line_number}: {error}"))?;
        value = Some(Some(parsed));
    }
    Ok(value)
}
fn parse_quoted(value: &str, label: &str) -> Result<String, String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("{label}.init expects one quoted argument"));
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains('"') {
        return Err(format!("escaped {label} strings are not supported"));
    }
    Ok(inner.to_string())
}
fn parse_minesweeper_init(source: &str) -> Result<Option<(usize, usize, usize, u32)>, String> {
    let mut config = None;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("minesweeper.init(") || !line.ends_with(')') {
            if line.starts_with("minesweeper.") {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported minesweeper call"
                ));
            }
            return Ok(None);
        }
        if config.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate minesweeper.init"
            ));
        }
        let arguments = &line["minesweeper.init(".len()..line.len() - 1];
        let values = arguments.split(',').map(str::trim).collect::<Vec<_>>();
        if values.len() != 4 {
            return Err(format!(
                "MAIN.LUA line {line_number}: minesweeper.init expects columns, rows, mines, seed"
            ));
        }
        let parse_usize = |value: &str, label: &str| {
            value
                .parse::<usize>()
                .map_err(|_| format!("MAIN.LUA line {line_number}: invalid minesweeper {label}"))
        };
        config = Some((
            parse_usize(values[0], "columns")?,
            parse_usize(values[1], "rows")?,
            parse_usize(values[2], "mine count")?,
            values[3]
                .parse::<u32>()
                .map_err(|_| format!("MAIN.LUA line {line_number}: invalid minesweeper seed"))?,
        ));
    }
    Ok(config)
}
fn parse_tetris_init(source: &str) -> Result<Option<(TetrisMode, u32)>, String> {
    let mut config = None;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.split("--").next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("tetris.init(") || !line.ends_with(')') {
            if line.starts_with("tetris.") {
                return Err(format!(
                    "MAIN.LUA line {line_number}: unsupported tetris call"
                ));
            }
            return Ok(None);
        }
        if config.is_some() {
            return Err(format!(
                "MAIN.LUA line {line_number}: duplicate tetris.init"
            ));
        }
        let arguments = &line["tetris.init(".len()..line.len() - 1];
        let values = arguments.split(',').map(str::trim).collect::<Vec<_>>();
        if values.len() != 2 {
            return Err(format!(
                "MAIN.LUA line {line_number}: tetris.init expects mode, seed"
            ));
        }
        let mode = unquote(values[0]).ok_or_else(|| {
            format!("MAIN.LUA line {line_number}: tetris.init expects one quoted mode")
        })?;
        if mode != "zen" {
            return Err(format!(
                "MAIN.LUA line {line_number}: tetris supports zen mode only"
            ));
        }
        let seed = values[1]
            .parse::<u32>()
            .map_err(|_| format!("MAIN.LUA line {line_number}: invalid tetris seed"))?;
        config = Some((TetrisMode::Zen, seed));
    }
    Ok(config)
}
fn unquote(value: &str) -> Option<&str> {
    for quote in ['\'', '"'] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            let inner = &value[quote.len_utf8()..value.len() - quote.len_utf8()];
            if !inner.contains(quote) {
                return Some(inner);
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{buttons::ButtonEvent, games::canvas::NativeGameCanvas};
    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    #[test]
    fn loads_sudoku_init_and_routes_native_event() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load(&format!("sudoku.init(\"{PUZZLE}\")"), &mut c).unwrap();
        assert_eq!(b.marker(), "sudoku");
        let LuaEventBridge::Sudoku(game) = &b else {
            unreachable!()
        };
        // The card's puzzle joins the three generated difficulties.
        assert_eq!(game.start_options().len(), 4);
        assert_eq!(
            b.apply_button(ButtonEvent::Down, 1_000, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "start-move"
        );
        // A bare init opens the start list alone; a bad puzzle fails early.
        let bare = LuaEventBridge::load("sudoku.init()", &mut c).unwrap();
        let LuaEventBridge::Sudoku(game) = &bare else {
            unreachable!()
        };
        assert_eq!(game.start_options().len(), 3);
        assert!(LuaEventBridge::load("sudoku.init(\"123\")", &mut c).is_err());
    }
    #[test]
    fn loads_minesweeper_init_and_routes_native_event() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load("minesweeper.init(9, 9, 10, 1803)", &mut c).unwrap();
        assert_eq!(b.marker(), "minesweeper");
        assert_eq!(
            b.apply_button(ButtonEvent::Select, 2_000, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "action-enter"
        );
    }
    #[test]
    fn loads_tetris_init_and_routes_native_event() {
        let mut c = NativeGameCanvas::default();
        let mut b = LuaEventBridge::load("tetris.init('zen', 1803)", &mut c).unwrap();
        assert_eq!(b.marker(), "tetris");
        assert_eq!(
            b.apply_button(ButtonEvent::Down, 1_000, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "move-right"
        );
        assert_eq!(
            b.apply_boot_short_press(3_000, &mut c)
                .unwrap()
                .unwrap()
                .reason(),
            "drop"
        );
    }
    #[test]
    fn preserves_static_hello_grid_bootstrap() {
        let mut c = NativeGameCanvas::default();
        assert_eq!(
            LuaEventBridge::load("ui.grid(80, 220, 4, 4, 64, 64)", &mut c)
                .unwrap()
                .marker(),
            "static"
        );
    }
}
