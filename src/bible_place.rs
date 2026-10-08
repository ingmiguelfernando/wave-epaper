//! The last Bible place read: translation, book, chapter and page, kept in
//! `STATE.TXT` under the Bible root through `sd_file`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::sd_file;

/// Where the reader stopped. `usfm` is the book's USFM code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BiblePlace {
    pub translation: String,
    pub usfm: String,
    pub chapter: u16,
    pub page: u16,
}

/// Root-relative file that holds the place.
pub const STATE_FILE: &str = "STATE.TXT";

impl BiblePlace {
    /// `key=value` lines. Unknown keys are ignored when reading.
    pub fn serialized(&self) -> String {
        format!(
            "translation={}\nbook={}\nchapter={}\npage={}\n",
            self.translation, self.usfm, self.chapter, self.page
        )
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut translation = None;
        let mut usfm = None;
        let mut chapter = None;
        let mut page = None;
        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "translation" => translation = Some(value.to_owned()),
                "book" => usfm = Some(value.to_owned()),
                "chapter" => chapter = value.parse().ok(),
                "page" => page = value.parse().ok(),
                _ => {}
            }
        }
        Some(Self {
            translation: translation?,
            usfm: usfm?,
            chapter: chapter.filter(|chapter| *chapter > 0)?,
            page: page.unwrap_or(0),
        })
    }

    /// Load the place from `root`; a missing or malformed file is `None`.
    pub fn load(root: &Path) -> Option<Self> {
        let text = sd_file::read_to_string(&root.join(STATE_FILE)).ok()?;
        Self::parse(&text)
    }

    /// Save through `sd_file`, creating the folder first (the card has none).
    pub fn save(&self, root: &Path) -> Result<()> {
        std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
        let path: PathBuf = root.join(STATE_FILE);
        sd_file::replace(&path, &self.serialized())
            .with_context(|| format!("saving {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("wave-bible-place-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn psalm_23() -> BiblePlace {
        BiblePlace {
            translation: "RVR1960".into(),
            usfm: "PSA".into(),
            chapter: 23,
            page: 1,
        }
    }

    #[test]
    fn place_round_trips_through_the_file() {
        let root = temp_root("round");
        psalm_23().save(&root).unwrap();
        assert_eq!(BiblePlace::load(&root), Some(psalm_23()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn save_creates_the_missing_folder() {
        let root = temp_root("folder");
        assert!(!root.exists());
        psalm_23().save(&root).unwrap();
        assert!(root.join(STATE_FILE).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_or_malformed_place_is_none() {
        let root = temp_root("missing");
        assert_eq!(BiblePlace::load(&root), None);
        assert_eq!(BiblePlace::parse("translation=RVR1960\nbook=PSA\n"), None);
        assert_eq!(
            BiblePlace::parse("translation=RVR1960\nbook=PSA\nchapter=0\n"),
            None
        );
    }

    #[test]
    fn unknown_keys_are_ignored_and_page_defaults_to_first() {
        let place =
            BiblePlace::parse("translation=RVR1960\nbook=JHN\nchapter=3\nfuture=yes\n").unwrap();
        assert_eq!(place.usfm, "JHN");
        assert_eq!(place.page, 0);
    }
}
