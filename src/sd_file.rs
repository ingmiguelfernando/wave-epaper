//! Power-safe text writes for settings files on SD storage.
//!
//! FAT cannot rename onto an existing file (FatFs `f_rename` returns
//! `FR_EXIST`), and a power cut during a plain `fs::write` can leave an empty
//! file. [`replace`] follows the steps proven by `atomic_replace_text` in
//! `reader.rs`: the new text is written and synced to `.TMP`, the old file
//! waits as `.BAK` while `.TMP` renames into place, and [`read_to_string`]
//! accepts the `.BAK` copy when a write was interrupted between the renames.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Replace the text file at `path` with `text` so a power cut cannot lose the
/// previous contents.
///
/// Steps of `atomic_replace_text` in `reader.rs`: write and sync `PATH.TMP`;
/// move the old file to `PATH.BAK`; rename `.TMP` into place; delete `.BAK`.
/// When the final rename fails, the `.BAK` copy returns to its original name
/// and the error is returned. Stale `.TMP` and `.BAK` siblings of older saves
/// are removed first, and `.TMP` is cleaned up after a failure. `path` must
/// differ from its own `.TMP` and `.BAK` paths.
pub fn replace(path: &Path, text: &str) -> io::Result<()> {
    replace_with(path, text, |from, to| fs::rename(from, to))
}

/// Read the text file at `path`, falling back to its `.BAK` sibling when the
/// main file is missing, as happens after a write interrupted between the two
/// renames of [`replace`]. When both are missing, the `NotFound` error of the
/// main path is returned.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    let backup = sidecar(path, "BAK").filter(|backup| backup.is_file());
    match fs::read_to_string(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => match backup {
            Some(backup) => fs::read_to_string(backup),
            None => Err(error),
        },
        result => result,
    }
}

/// [`replace`] with `rename` for every move, so tests can fail one move and
/// cover the recovery steps.
fn replace_with<F>(path: &Path, text: &str, rename: F) -> io::Result<()>
where
    F: Fn(&Path, &Path) -> io::Result<()>,
{
    let (Some(temporary), Some(backup)) = (sidecar(path, "TMP"), sidecar(path, "BAK")) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} must differ from its own .TMP and .BAK paths",
                path.display()
            ),
        ));
    };
    let saved = (|| -> io::Result<()> {
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_file(&backup);
        write_and_sync(&temporary, text).map_err(|error| step_error("write", &temporary, error))?;
        let replacing = path.is_file();
        if replacing {
            rename(path, &backup).map_err(|error| step_error("backup", path, error))?;
        }
        if let Err(error) = rename(&temporary, path) {
            if backup.exists() {
                let _ = rename(&backup, path);
            }
            return Err(step_error("replace", path, error));
        }
        let _ = fs::remove_file(&backup);
        Ok(())
    })();
    if saved.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    saved
}

/// The `.TMP` or `.BAK` sibling of `path`; `None` when `path` already uses
/// that extension, where the sibling would be `path` itself.
fn sidecar(path: &Path, extension: &str) -> Option<PathBuf> {
    let mut sibling = path.to_path_buf();
    sibling.set_extension(extension);
    (sibling != path).then_some(sibling)
}

/// Create `path` with `text` and flush it to storage before returning.
fn write_and_sync(path: &Path, text: &str) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

/// Wrap `error` with the failing step and path, keeping its kind.
fn step_error(step: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{step} {}: {error}", path.display()))
}

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
struct TempRoot(PathBuf);

#[cfg(test)]
impl TempRoot {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "wave-sd-file-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

#[cfg(test)]
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Existing `.TMP` and `.BAK` siblings of `path`, for cleanup assertions.
#[cfg(test)]
fn leftovers(path: &Path) -> Vec<PathBuf> {
    [path.with_extension("TMP"), path.with_extension("BAK")]
        .into_iter()
        .filter(|sibling| sibling.exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        app::display::{DisplayPreferences, UiFontFamily, UiFontSize},
        battery_log::BatteryLog,
        photos::StarredPhotos,
        power_settings::{AutoSleep, PowerPreferences, WakeKeys},
        reading_stats::ReadingStats,
        weather_config::WeatherConfig,
    };

    #[test]
    fn replace_creates_a_new_file_and_leaves_no_tmp_or_bak() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        replace(&path, "# Wave\nkey=value\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Wave\nkey=value\n");
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn replace_replaces_the_file_in_place_and_leaves_no_tmp_or_bak() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        replace(&path, "one").unwrap();
        replace(&path, "two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn read_falls_back_to_the_bak_of_an_interrupted_write() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        replace(&path, "first").unwrap();
        // Interrupted between the two renames: only the backup remains.
        fs::rename(&path, path.with_extension("BAK")).unwrap();
        assert_eq!(read_to_string(&path).unwrap(), "first");
        // With both in place, the primary wins.
        fs::write(&path, "second").unwrap();
        assert_eq!(read_to_string(&path).unwrap(), "second");
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("BAK")).unwrap();
        assert_eq!(
            read_to_string(&path).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn a_directory_target_blocks_the_rename_and_keeps_the_original() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), "original").unwrap();
        assert!(replace(&path, "new").is_err());
        assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "original");
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn a_failed_backup_rename_keeps_the_original() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        fs::write(&path, "original").unwrap();
        let calls = Cell::new(0);
        let fail_backup = |from: &Path, to: &Path| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Err(io::Error::other("injected backup failure"))
            } else {
                fs::rename(from, to)
            }
        };
        assert!(replace_with(&path, "new", fail_backup).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn a_failed_final_rename_restores_the_original_from_the_bak() {
        let root = TempRoot::new();
        let path = root.path("SETTINGS.TXT");
        fs::write(&path, "original").unwrap();
        let calls = Cell::new(0);
        let fail_replace = |from: &Path, to: &Path| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                Err(io::Error::other("injected replace failure"))
            } else {
                fs::rename(from, to)
            }
        };
        assert!(replace_with(&path, "new", fail_replace).is_err());
        assert_eq!(calls.get(), 3); // backup, failed replace, restore
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn weather_config_round_trips_through_replace() {
        let root = TempRoot::new();
        let path = root.path("WEATHER.TXT");
        let config = WeatherConfig::parse("latitude=0\nlongitude=0\n").unwrap();
        config.save_to_path(&path).unwrap();
        assert_eq!(WeatherConfig::load_from_path(&path).unwrap(), config);
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn power_settings_round_trip_through_replace() {
        let root = TempRoot::new();
        let path = root.path("POWER.TXT");
        let preferences = PowerPreferences {
            auto_sleep: AutoSleep::Hour1,
            wake_keys: WakeKeys::PowerKey,
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(
            PowerPreferences::load_from_path(&path).unwrap(),
            preferences
        );
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn display_preferences_round_trip_through_replace() {
        let root = TempRoot::new();
        let path = root.path("DISPLAY.TXT");
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::AtkinsonHyperlegible,
            font_size: UiFontSize::Compact,
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(
            DisplayPreferences::load_from_path(&path).unwrap(),
            preferences
        );
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn battery_log_round_trips_through_replace() {
        let root = TempRoot::new();
        let path = root.path("BATTERY.TXT");
        let mut log = BatteryLog::default();
        for step in 0..3u32 {
            assert!(log.record(20_000 + step * 15, 90 - step as u8));
        }
        log.save_to_path(&path).unwrap();
        assert_eq!(BatteryLog::load_from_path(&path).unwrap(), log);
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn starred_photos_round_trip_through_replace() {
        let root = TempRoot::new();
        let path = root.path("STARRED.TXT");
        let mut starred = StarredPhotos::default();
        assert!(starred.toggle("A.jpg"));
        assert!(starred.toggle("B.jpg"));
        starred.save_to_path(&path).unwrap();
        assert_eq!(StarredPhotos::load_from_path(&path).unwrap(), starred);
        assert!(leftovers(&path).is_empty());
    }

    #[test]
    fn reading_stats_round_trip_through_replace() {
        let root = TempRoot::new();
        let path = root.path("STATS.TXT");
        let mut stats = ReadingStats::default();
        stats.record(20_729, 60, 1, Some("Café.txt"));
        stats.save_to_path(&path).unwrap();
        assert_eq!(ReadingStats::load_from_path(&path).unwrap(), stats);
        assert!(leftovers(&path).is_empty());
    }
}
