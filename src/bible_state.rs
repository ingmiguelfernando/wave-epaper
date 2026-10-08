//! Bible app state: the translation on the card, the book picker, and the
//! place being read. The root is a constructor argument, so tests use a temp
//! folder the same way the photos state does.

use std::path::{Path, PathBuf};

use crate::{
    bible::{self, BibleBook, VerseRef},
    bible_nav::BibleNav,
    bible_place::BiblePlace,
    bible_reader::{self, BookShape, Position, Turn},
    sd_file,
};

/// What the Bible app found on the card.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BibleCard {
    /// No translation folder with an `index.tsv`.
    Missing,
    /// The translation whose books are loaded.
    Ready { translation: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibleUiState {
    root: PathBuf,
    card: BibleCard,
    books: Vec<BibleBook>,
    nav: BibleNav,
    place: Option<BiblePlace>,
    position: Option<Position>,
    place_changed: bool,
}

impl BibleUiState {
    /// Scan `root` for the first translation with an `index.tsv`; no card or
    /// no translation leaves the state in `Missing`.
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        // An empty root is "no card": never scan the current directory.
        let scanned = if root.as_os_str().is_empty() {
            None
        } else {
            bible::translations(&root).ok()
        };
        let (card, books) = match scanned.and_then(|list| list.into_iter().next()) {
            Some(translation) => {
                let text = std::fs::read_to_string(root.join(&translation).join("index.tsv"));
                match text.ok().and_then(|text| bible::parse_index(&text).ok()) {
                    Some(books) if !books.is_empty() => (BibleCard::Ready { translation }, books),
                    _ => (BibleCard::Missing, Vec::new()),
                }
            }
            None => (BibleCard::Missing, Vec::new()),
        };
        let place = BiblePlace::load(&root);
        Self {
            root,
            card,
            nav: BibleNav::new(books.clone()),
            books,
            place,
            position: None,
            place_changed: false,
        }
    }

    pub fn card(&self) -> &BibleCard {
        &self.card
    }

    pub fn books(&self) -> &[BibleBook] {
        &self.books
    }

    pub fn nav(&self) -> &BibleNav {
        &self.nav
    }

    pub fn nav_mut(&mut self) -> &mut BibleNav {
        &mut self.nav
    }

    pub fn place(&self) -> Option<&BiblePlace> {
        self.place.as_ref()
    }

    /// The place is usable only when its book is on the card and its chapter
    /// is inside that book. Otherwise the picker opens.
    pub fn usable_place(&self) -> Option<&BiblePlace> {
        let place = self.place.as_ref()?;
        let book = self.books.iter().find(|book| book.usfm == place.usfm)?;
        (place.chapter <= book.chapters).then_some(place)
    }

    pub fn translation(&self) -> Option<&str> {
        match &self.card {
            BibleCard::Ready { translation } => Some(translation),
            BibleCard::Missing => None,
        }
    }

    /// Open the reading view at `position`. The place is marked changed so
    /// the first close saves it, even when nothing was turned.
    pub fn open_at(&mut self, position: Position) {
        self.position = Some(position);
        self.place_changed = true;
    }

    pub fn position(&self) -> Option<Position> {
        self.position
    }

    /// The book shapes in canonical order, for crossing book boundaries.
    pub fn shapes(&self) -> Vec<BookShape> {
        self.books
            .iter()
            .map(|book| BookShape {
                chapters: book.chapters,
            })
            .collect()
    }

    /// Move one page; `pages` is the open chapter's page count. Returns the
    /// outcome so the caller can reload the chapter when it changed.
    pub fn turn_page(&mut self, forward: bool, pages: usize) -> Turn {
        let shapes = self.shapes();
        let Some(position) = self.position.as_mut() else {
            return Turn::Edge;
        };
        let outcome = bible_reader::turn(position, forward, pages, &shapes);
        if outcome != Turn::Edge {
            self.place_changed = true;
        }
        outcome
    }

    /// Resolve the "last page" sentinel of a backward turn against the page
    /// count of the chapter now open. `pages_of` pages a chapter by its
    /// position, so the caller decides how a chapter is laid out.
    pub fn resolve_last_page(&mut self, pages_of: impl FnOnce(Position) -> usize) {
        let Some(position) = self.position.as_mut() else {
            return;
        };
        if position.page == usize::MAX {
            position.page = pages_of(*position).saturating_sub(1);
        }
    }

    /// Record the place in memory; it reaches the card only through `save`,
    /// called when the view closes or before sleep, never per page.
    pub fn remember_place(&mut self) {
        let (Some(position), Some(translation)) =
            (self.position, self.translation().map(str::to_owned))
        else {
            return;
        };
        let Some(book) = self.books.get(position.book) else {
            return;
        };
        self.place = Some(BiblePlace {
            translation,
            usfm: book.usfm.clone(),
            chapter: position.chapter,
            page: position.page.min(u16::MAX as usize) as u16,
        });
    }

    /// Save the place to the card if it changed. Returns whether it saved.
    pub fn save_if_changed(&mut self) -> bool {
        if !self.place_changed {
            return false;
        }
        self.remember_place();
        let Some(place) = self.place.clone() else {
            return false;
        };
        match place.save(&self.root) {
            Ok(()) => {
                self.place_changed = false;
                true
            }
            Err(_) => false,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `VERSES.TXT` sits in the Bible root, next to the translation folders.
    pub fn verses_path(&self) -> PathBuf {
        self.root.join(VERSES_FILE)
    }

    /// The verse-of-the-day list; a missing or malformed file is empty.
    pub fn verse_list(&self) -> Vec<VerseRef> {
        sd_file::read_to_string(&self.verses_path())
            .ok()
            .and_then(|text| bible::parse_verse_list(&text).ok())
            .unwrap_or_default()
    }

    /// The menu offers the verse only when the file is on the card.
    pub fn has_verse_list(&self) -> bool {
        self.verses_path().is_file()
    }

    /// Translations on the card; the menu offers a switch only with two or more.
    pub fn translation_count(&self) -> usize {
        bible::translations(&self.root).map_or(0, |list| list.len())
    }
}

const VERSES_FILE: &str = "VERSES.TXT";

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("wave-bible-state-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn an_empty_card_reports_missing() {
        let root = temp_root("empty");
        std::fs::create_dir_all(&root).unwrap();
        let state = BibleUiState::with_root(&root);
        assert_eq!(state.card(), &BibleCard::Missing);
        assert!(state.books().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
