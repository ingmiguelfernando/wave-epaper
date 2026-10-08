//! Best scores for the SD games, kept in one SD file and saved only when a
//! value changes: every write costs battery.

use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::sd_file;

/// SD location of the games' best-score file.
pub const RECORDS_PATH: &str = "/sdcard/RUSTMIX/GAMES/RECORDS.TXT";

/// Best scores, one key per game mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GameRecords {
    pub tetris_zen: u32,
    /// Best solve time in seconds per difficulty, 0 = none yet.
    pub sudoku_easy: u32,
    pub sudoku_medium: u32,
    pub sudoku_hard: u32,
}

impl GameRecords {
    /// Parse `key=value` lines under a header comment, as `POWER.TXT` does.
    pub fn parse(text: &str) -> Result<Self> {
        let mut records = Self::default();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .with_context(|| format!("records line {} must contain '='", number + 1))?;
            let parsed = value
                .trim()
                .parse::<u32>()
                .with_context(|| format!("records line {} has a bad number", number + 1))?;
            match key.trim() {
                "tetris_zen" => records.tetris_zen = parsed,
                "sudoku_easy" => records.sudoku_easy = parsed,
                "sudoku_medium" => records.sudoku_medium = parsed,
                "sudoku_hard" => records.sudoku_hard = parsed,
                // Unknown keys stay ignored so older files keep loading.
                _ => {}
            }
        }
        Ok(records)
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        format!(
            "# Wave game records v1\ntetris_zen={}\nsudoku_easy={}\nsudoku_medium={}\nsudoku_hard={}\n",
            self.tetris_zen, self.sudoku_easy, self.sudoku_medium, self.sudoku_hard
        )
    }

    /// Load through `sd_file`, falling back to the `.BAK` copy. A missing file
    /// means no records yet, not an error.
    pub fn load_from_path(path: impl AsRef<Path>) -> Self {
        match sd_file::read_to_string(path.as_ref()) {
            Ok(text) => Self::parse(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save through `sd_file`; the caller decides when, never per piece. The
    /// card has no `GAMES/` folder until the first save creates it.
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write game records {}", path.display()))
    }

    /// Raise the Tetris Zen best score; `true` when the file must be saved.
    pub fn observe_tetris_zen(&mut self, best: u32) -> bool {
        if best > self.tetris_zen {
            self.tetris_zen = best;
            true
        } else {
            false
        }
    }

    /// Record a Sudoku solve; `true` when it beat the difficulty's best
    /// (a 0 stored best means none yet, so any solve improves it).
    pub fn observe_sudoku(
        &mut self,
        difficulty: crate::games::sudoku_puzzles::SudokuDifficulty,
        seconds: u32,
    ) -> bool {
        use crate::games::sudoku_puzzles::SudokuDifficulty;
        let (current, key) = match difficulty {
            SudokuDifficulty::Easy => (self.sudoku_easy, "sudoku_easy"),
            SudokuDifficulty::Medium => (self.sudoku_medium, "sudoku_medium"),
            SudokuDifficulty::Hard => (self.sudoku_hard, "sudoku_hard"),
        };
        if current != 0 && seconds >= current {
            return false;
        }
        match key {
            "sudoku_easy" => self.sudoku_easy = seconds,
            "sudoku_medium" => self.sudoku_medium = seconds,
            _ => self.sudoku_hard = seconds,
        }
        true
    }

    /// The stored best for `difficulty`, 0 = none yet.
    #[must_use]
    pub fn sudoku_best(&self, difficulty: crate::games::sudoku_puzzles::SudokuDifficulty) -> u32 {
        use crate::games::sudoku_puzzles::SudokuDifficulty;
        match difficulty {
            SudokuDifficulty::Easy => self.sudoku_easy,
            SudokuDifficulty::Medium => self.sudoku_medium,
            SudokuDifficulty::Hard => self.sudoku_hard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameRecords;

    #[test]
    fn round_trips_the_file_format() {
        let records = GameRecords {
            tetris_zen: 18_950,
            sudoku_easy: 0,
            sudoku_medium: 0,
            sudoku_hard: 0,
        };
        let parsed = GameRecords::parse(&records.serialized()).unwrap();
        assert_eq!(parsed, records);
    }

    #[test]
    fn reads_examples_with_comments_and_unknown_keys() {
        let parsed =
            GameRecords::parse("# Wave game records v1\ntetris_zen=42\nfuture=7\n").unwrap();
        assert_eq!(parsed.tetris_zen, 42);
    }

    #[test]
    fn malformed_numbers_are_errors_and_missing_files_default() {
        assert!(GameRecords::parse("tetris_zen=many").is_err());
        assert!(GameRecords::parse("tetris_zen").is_err());
        assert_eq!(
            GameRecords::load_from_path("/no/such/RECORDS.TXT"),
            GameRecords::default()
        );
    }

    #[test]
    fn observe_saves_once_on_a_new_best_and_not_after_a_lower_one() {
        let mut records = GameRecords::default();
        assert!(records.observe_tetris_zen(100));
        assert_eq!(records.tetris_zen, 100);
        assert!(!records.observe_tetris_zen(50), "lower score saves nothing");
        assert!(!records.observe_tetris_zen(100), "same score saves nothing");
        assert!(records.observe_tetris_zen(101));
        assert_eq!(records.tetris_zen, 101);
    }

    #[test]
    fn the_first_save_creates_the_games_folder() {
        let root = std::env::temp_dir().join(format!("wave-records-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let games = root.join("GAMES");
        let path = games.join("RECORDS.TXT");
        let mut records = GameRecords::default();
        records.observe_tetris_zen(9001);
        records.save_to_path(&path).unwrap();
        assert_eq!(GameRecords::load_from_path(&path).tetris_zen, 9001);
        assert!(!games.join("RECORDS.TMP").exists());
        assert!(!games.join("RECORDS.BAK").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
