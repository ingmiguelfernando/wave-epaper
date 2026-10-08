//! Sudoku resume file: `/RUSTMIX/GAMES/SUDOKU.TXT`, written once when the
//! player leaves a game, deleted when the puzzle is solved.

use std::path::Path;

use anyhow::{Context, Result};

use crate::games::sudoku_puzzles::SudokuDifficulty;
use crate::sd_file;

/// SD location of the Sudoku resume file.
pub const SUDOKU_SAVE_PATH: &str = "/sdcard/RUSTMIX/GAMES/SUDOKU.TXT";

/// One resumable game: the difficulty, the puzzle, the board and the played
/// seconds. Grids are 81 digits each, 0 for an empty cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SudokuSave {
    pub difficulty: SudokuDifficulty,
    pub puzzle: [u8; 81],
    pub board: [u8; 81],
    pub seconds: u32,
}

impl SudokuSave {
    /// Parse `key=value` lines; a malformed known key names its line.
    pub fn parse(text: &str) -> Result<Self> {
        let mut difficulty = None;
        let mut puzzle = None;
        let mut board = None;
        let mut seconds = None;
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .with_context(|| format!("sudoku save line {} must contain '='", number + 1))?;
            let value = value.trim();
            match key.trim() {
                "difficulty" => {
                    difficulty = Some(SudokuDifficulty::parse(value).with_context(|| {
                        format!("sudoku save line {} has a bad difficulty", number + 1)
                    })?);
                }
                "puzzle" | "board" => {
                    let grid = parse_grid(value).with_context(|| {
                        format!("sudoku save line {} has a bad grid", number + 1)
                    })?;
                    if key.trim() == "puzzle" {
                        puzzle = Some(grid);
                    } else {
                        board = Some(grid);
                    }
                }
                "seconds" => {
                    seconds = Some(value.parse::<u32>().with_context(|| {
                        format!("sudoku save line {} has a bad number", number + 1)
                    })?);
                }
                // Unknown lines stay skipped so v1 keeps loading.
                _ => {}
            }
        }
        Ok(Self {
            difficulty: difficulty.with_context(|| "sudoku save has no difficulty")?,
            puzzle: puzzle.with_context(|| "sudoku save has no puzzle")?,
            board: board.with_context(|| "sudoku save has no board")?,
            seconds: seconds.unwrap_or(0),
        })
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        format!(
            "# Wave sudoku save v1\ndifficulty={}\npuzzle={}\nboard={}\nseconds={}\n",
            self.difficulty.marker(),
            grid_text(&self.puzzle),
            grid_text(&self.board),
            self.seconds
        )
    }

    /// Load through `sd_file`; a missing or malformed file means no save.
    #[must_use]
    pub fn load_from_path(path: impl AsRef<Path>) -> Option<Self> {
        sd_file::read_to_string(path.as_ref())
            .ok()
            .and_then(|text| Self::parse(&text).ok())
    }

    /// Save through `sd_file`, creating the folder first (round-4 lesson:
    /// `/RUSTMIX/GAMES/` exists only after the first write creates it).
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write sudoku save {}", path.display()))
    }

    /// Remove the resume file after a solve, with any `.BAK` or `.TMP` an
    /// interrupted write left, so loading cannot bring the old game back.
    /// Missing files are fine.
    pub fn remove_from_path(path: impl AsRef<Path>) {
        let path = path.as_ref();
        let _ = std::fs::remove_file(path);
        for extension in ["BAK", "TMP"] {
            let _ = std::fs::remove_file(path.with_extension(extension));
        }
    }
}

fn parse_grid(text: &str) -> Result<[u8; 81]> {
    let bytes = text.as_bytes();
    if bytes.len() != 81 {
        anyhow::bail!("grid must contain 81 digits");
    }
    let mut grid = [0_u8; 81];
    for (index, &byte) in bytes.iter().enumerate() {
        grid[index] = match byte {
            b'0' | b'.' => 0,
            b'1'..=b'9' => byte - b'0',
            _ => anyhow::bail!("grid accepts only digits, zero or dot"),
        };
    }
    Ok(grid)
}

fn grid_text(grid: &[u8; 81]) -> String {
    grid.iter()
        .map(|&value| (b'0' + value.min(9)) as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_grid, SudokuSave, SUDOKU_SAVE_PATH};
    use crate::games::sudoku_puzzles::SudokuDifficulty;

    const PUZZLE: &str =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    const BOARD: &str =
        "530070006600195000098000060800060003400803001700020006060000280000419005000080079";

    #[test]
    fn round_trips_the_file_format() {
        let save = SudokuSave {
            difficulty: SudokuDifficulty::Medium,
            puzzle: parse_grid(PUZZLE).unwrap(),
            board: parse_grid(BOARD).unwrap(),
            seconds: 461,
        };
        let parsed = SudokuSave::parse(&save.serialized()).unwrap();
        assert_eq!(parsed, save);
        assert_eq!(SUDOKU_SAVE_PATH, "/sdcard/RUSTMIX/GAMES/SUDOKU.TXT");
    }

    #[test]
    fn missing_fields_are_errors_and_unknown_lines_are_skipped() {
        assert!(SudokuSave::parse("# Wave sudoku save v1\npuzzle=..1\n").is_err());
        let with_extra = format!(
            "# Wave sudoku save v1\nfuture=yes\ndifficulty=hard\npuzzle={PUZZLE}\nboard={BOARD}\nseconds=7\n"
        );
        let parsed = SudokuSave::parse(&with_extra).unwrap();
        assert_eq!(parsed.difficulty, SudokuDifficulty::Hard);
        assert_eq!(parsed.seconds, 7);
    }

    #[test]
    fn bad_grids_and_numbers_name_the_problem() {
        let bad_grid = format!("difficulty=easy\npuzzle={}\nboard=short\n", PUZZLE);
        assert!(SudokuSave::parse(&bad_grid).is_err());
        let bad_number = format!("difficulty=easy\npuzzle={PUZZLE}\nboard={BOARD}\nseconds=many\n");
        assert!(SudokuSave::parse(&bad_number).is_err());
        let bad_difficulty = format!("difficulty=impossible\npuzzle={PUZZLE}\nboard={BOARD}\n");
        assert!(SudokuSave::parse(&bad_difficulty).is_err());
    }

    #[test]
    fn save_reload_and_remove_on_a_temp_folder() {
        let root = std::env::temp_dir().join(format!("wave-sudoku-{}", std::process::id()));
        let path = root.join("GAMES").join("SUDOKU.TXT");
        let save = SudokuSave {
            difficulty: SudokuDifficulty::Easy,
            puzzle: parse_grid(PUZZLE).unwrap(),
            board: parse_grid(BOARD).unwrap(),
            seconds: 12,
        };
        save.save_to_path(&path).unwrap();
        assert_eq!(SudokuSave::load_from_path(&path), Some(save));
        // A backup left by an interrupted write goes too.
        std::fs::copy(&path, path.with_extension("BAK")).unwrap();
        SudokuSave::remove_from_path(&path);
        assert_eq!(SudokuSave::load_from_path(&path), None);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
