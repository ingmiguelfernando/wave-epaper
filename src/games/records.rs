//! Best scores for the SD games, kept in one SD file and saved only when a
//! value changes: every write costs battery.

use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::sudoku_puzzles::SudokuDifficulty;
use crate::sd_file;

/// SD location of the games' best-score file.
pub const RECORDS_PATH: &str = "/sdcard/RUSTMIX/GAMES/RECORDS.TXT";

/// Best scores, one key per game mode.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameRecords {
    pub tetris_zen: u32,
    /// Best Tetris Classic score; slow gravity, its own record.
    pub tetris_classic: u32,
    /// Best Race Classic distance in metres; one step a second at first.
    pub race_classic: u32,
    /// Best Race Zen distance in metres; the road steps on a key.
    pub race_zen: u32,
    /// Best solve time in seconds per difficulty, 0 = none yet.
    pub sudoku_easy: u32,
    pub sudoku_medium: u32,
    pub sudoku_hard: u32,
    /// Last Dice faces as `4+6` (one to three faces). `None` when never rolled.
    pub dice_last: Option<String>,
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
            let key = key.trim();
            let value = value.trim();
            if key == "dice_last" {
                // A bad value is dropped, so the card simply has no last roll
                // and the rest of the file still loads.
                if parse_dice_faces(value).is_some() {
                    records.dice_last = Some(value.to_string());
                }
                continue;
            }
            let parsed = value
                .parse::<u32>()
                .with_context(|| format!("records line {} has a bad number", number + 1))?;
            match key {
                "tetris_zen" => records.tetris_zen = parsed,
                "tetris_classic" => records.tetris_classic = parsed,
                "race_classic" => records.race_classic = parsed,
                "race_zen" => records.race_zen = parsed,
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
        let mut text = format!(
            "# Wave game records v1\ntetris_zen={}\ntetris_classic={}\nrace_classic={}\nrace_zen={}\nsudoku_easy={}\nsudoku_medium={}\nsudoku_hard={}\n",
            self.tetris_zen,
            self.tetris_classic,
            self.race_classic,
            self.race_zen,
            self.sudoku_easy,
            self.sudoku_medium,
            self.sudoku_hard
        );
        if let Some(faces) = &self.dice_last {
            text.push_str(&format!("dice_last={faces}\n"));
        }
        text
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

    /// Raise the Tetris Classic best score; `true` when the file must be saved.
    pub fn observe_tetris_classic(&mut self, best: u32) -> bool {
        if best > self.tetris_classic {
            self.tetris_classic = best;
            true
        } else {
            false
        }
    }

    /// Raise the Race Classic best distance in metres; `true` when the file
    /// must be saved.
    pub fn observe_race_classic(&mut self, best: u32) -> bool {
        if best > self.race_classic {
            self.race_classic = best;
            true
        } else {
            false
        }
    }

    /// Raise the Race Zen best distance in metres; `true` when the file must
    /// be saved.
    pub fn observe_race_zen(&mut self, best: u32) -> bool {
        if best > self.race_zen {
            self.race_zen = best;
            true
        } else {
            false
        }
    }

    /// Record a Sudoku solve; `true` when it beat the difficulty's best
    /// (a 0 stored best means none yet, so any solve improves it).
    pub fn observe_sudoku(&mut self, difficulty: SudokuDifficulty, seconds: u32) -> bool {
        let best = self.sudoku_slot(difficulty);
        if *best != 0 && seconds >= *best {
            return false;
        }
        // 0 means no best, so even an instant solve stores a second.
        *best = seconds.max(1);
        true
    }

    /// The stored best for `difficulty`, 0 = none yet.
    #[must_use]
    pub const fn sudoku_best(&self, difficulty: SudokuDifficulty) -> u32 {
        match difficulty {
            SudokuDifficulty::Easy => self.sudoku_easy,
            SudokuDifficulty::Medium => self.sudoku_medium,
            SudokuDifficulty::Hard => self.sudoku_hard,
        }
    }

    /// The fastest solve of any difficulty, for the Games card when no game
    /// is saved.
    #[must_use]
    pub fn sudoku_fastest(&self) -> Option<(SudokuDifficulty, u32)> {
        SudokuDifficulty::ALL
            .into_iter()
            .map(|difficulty| (difficulty, self.sudoku_best(difficulty)))
            .filter(|&(_, seconds)| seconds != 0)
            .min_by_key(|&(_, seconds)| seconds)
    }

    fn sudoku_slot(&mut self, difficulty: SudokuDifficulty) -> &mut u32 {
        match difficulty {
            SudokuDifficulty::Easy => &mut self.sudoku_easy,
            SudokuDifficulty::Medium => &mut self.sudoku_medium,
            SudokuDifficulty::Hard => &mut self.sudoku_hard,
        }
    }

    /// Store the last Dice roll's faces (`4+6`); `true` when the file must be
    /// saved. A value that is not one to three faces of 1..=6 is refused.
    pub fn observe_dice_faces(&mut self, faces: &str) -> bool {
        if parse_dice_faces(faces).is_none() {
            return false;
        }
        if self.dice_last.as_deref() == Some(faces) {
            return false;
        }
        self.dice_last = Some(faces.to_string());
        true
    }
}

/// `4+6` or `5`: one to three faces of 1..=6 joined by `+`. `None` otherwise.
#[must_use]
pub fn parse_dice_faces(value: &str) -> Option<Vec<u8>> {
    let mut faces = Vec::new();
    for part in value.split('+') {
        let face = part.trim().parse::<u8>().ok()?;
        if !(1..=6).contains(&face) {
            return None;
        }
        faces.push(face);
    }
    if faces.is_empty() || faces.len() > 3 {
        return None;
    }
    Some(faces)
}

#[cfg(test)]
mod tests {
    use super::GameRecords;

    #[test]
    fn round_trips_the_file_format() {
        let records = GameRecords {
            tetris_zen: 18_950,
            tetris_classic: 4_200,
            race_classic: 5_100,
            race_zen: 2_400,
            sudoku_easy: 0,
            sudoku_medium: 0,
            sudoku_hard: 0,
            dice_last: None,
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
    fn classic_and_zen_keep_separate_best_scores() {
        let mut records = GameRecords::default();
        assert!(records.observe_tetris_classic(4_200));
        assert_eq!(records.tetris_classic, 4_200);
        assert_eq!(records.tetris_zen, 0, "a Classic score is not a Zen best");
        assert!(
            !records.observe_tetris_classic(4_000),
            "lower saves nothing"
        );
        let parsed = GameRecords::parse(&records.serialized()).unwrap();
        assert_eq!(parsed.tetris_classic, 4_200, "the Classic best round-trips");
    }

    /// Race keeps its best distances in metres, one per mode, in the same file.
    #[test]
    fn race_best_distances_round_trip_per_mode() {
        let mut records = GameRecords::default();
        assert!(records.observe_race_classic(5_100));
        assert!(records.observe_race_zen(2_400));
        assert!(!records.observe_race_classic(4_000), "lower saves nothing");
        assert_eq!(records.race_zen, 2_400, "a Classic run is not a Zen best");
        let parsed = GameRecords::parse(&records.serialized()).unwrap();
        assert_eq!(parsed.race_classic, 5_100);
        assert_eq!(parsed.race_zen, 2_400);
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
    fn dice_last_round_trips_and_a_bad_value_leaves_the_card_without_it() {
        // Missing: no line.
        let none = GameRecords::parse("# Wave game records v1\n").unwrap();
        assert_eq!(none.dice_last, None);
        // Empty: still no line.
        let empty = GameRecords::parse("dice_last=\n").unwrap();
        assert_eq!(empty.dice_last, None, "empty dice_last is ignored");
        // Malformed: ignored, the rest of the file loads.
        let bad = GameRecords::parse("tetris_zen=7\ndice_last=4+7\ndice_last=nope\n").unwrap();
        assert_eq!(bad.tetris_zen, 7, "the score still loads");
        assert_eq!(bad.dice_last, None, "a bad dice_last is dropped");
        // Good values round-trip.
        let mut records = GameRecords::default();
        assert!(records.observe_dice_faces("4+6"));
        assert!(
            !records.observe_dice_faces("4+6"),
            "same faces save nothing"
        );
        assert!(!records.observe_dice_faces("9"), "face 9 is refused");
        assert!(records.observe_dice_faces("5"));
        let parsed = GameRecords::parse(&records.serialized()).unwrap();
        assert_eq!(parsed.dice_last.as_deref(), Some("5"));
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
