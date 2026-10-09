//! Bible app state: the translation on the card, the book picker, and the
//! place being read. The root is a constructor argument, so tests use a temp
//! folder the same way the photos state does.

use std::path::{Path, PathBuf};

use crate::{
    bible::{self, BibleBook, VerseRef},
    bible_nav::BibleNav,
    bible_place::BiblePlace,
    bible_reader::{self, BookShape, Position, Turn},
    hyphenation::Language,
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

/// The chapter open in the reading view, read once when the place moves to it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ChapterCache {
    translation: String,
    book: usize,
    chapter: u16,
    /// `None` when the chapter file cannot be read.
    items: Option<Vec<bible::ChapterItem>>,
}

/// A translation on the card for the picker: its folder and its title.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationOption {
    pub folder: String,
    pub title: String,
}

/// What choosing a translation did: the place stayed, or the picker must open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchOutcome {
    Kept,
    Picker,
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
    language: Option<Language>,
    chapter: Option<ChapterCache>,
    loads: usize,
    translation_list: Vec<TranslationOption>,
    translation_selected: usize,
}

impl BibleUiState {
    /// Scan `root` for the saved place's translation, else the first with an
    /// `index.tsv`; no card or no translation leaves the state in `Missing`.
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        // An empty root is "no card": never scan the current directory.
        let scanned = if root.as_os_str().is_empty() {
            None
        } else {
            bible::translations(&root).ok()
        };
        let place = BiblePlace::load(&root);
        let saved = place.as_ref().map(|place| place.translation.as_str());
        let (card, books) = match scanned.and_then(|list| pick_translation(list, saved)) {
            Some(translation) => {
                let text = std::fs::read_to_string(root.join(&translation).join("index.tsv"));
                match text.ok().and_then(|text| bible::parse_index(&text).ok()) {
                    Some(books) if !books.is_empty() => (BibleCard::Ready { translation }, books),
                    _ => (BibleCard::Missing, Vec::new()),
                }
            }
            None => (BibleCard::Missing, Vec::new()),
        };
        let language = match &card {
            BibleCard::Ready { translation } => read_language(&root, translation),
            BibleCard::Missing => None,
        };
        Self {
            root,
            card,
            nav: BibleNav::new(books.clone()),
            books,
            place,
            position: None,
            place_changed: false,
            language,
            chapter: None,
            loads: 0,
            translation_list: Vec::new(),
            translation_selected: 0,
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
        self.refresh_chapter();
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
            self.refresh_chapter();
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

    /// The reading position of `verse` and its first verse number, when its
    /// book and chapter are on this card.
    #[must_use]
    pub fn verse_position(&self, verse: &bible::VerseRef) -> Option<(Position, u16)> {
        let book = self
            .books
            .iter()
            .position(|book| book.number == verse.book)?;
        if verse.chapter > self.books[book].chapters {
            return None;
        }
        let position = Position {
            book,
            chapter: verse.chapter,
            page: 0,
        };
        Some((position, verse.first))
    }

    /// Today's verse of the day from `VERSES.TXT`, as a reading position and
    /// its first verse number.
    #[must_use]
    pub fn verse_of_day(&self, epoch_day: u32) -> Option<(Position, u16)> {
        let list = self.verse_list();
        let verse = bible::verse_of_the_day(&list, epoch_day)?;
        self.verse_position(verse)
    }

    /// Translations on the card; the menu offers a switch only with two or more.
    /// The open translation's language, for hyphenation.
    pub fn language(&self) -> Option<Language> {
        self.language
    }

    /// The kept items of the chapter at `position`; `None` when another chapter
    /// is kept or it could not be read.
    pub fn chapter_items_at(&self, position: Position) -> Option<&[bible::ChapterItem]> {
        let cache = self.chapter.as_ref()?;
        let same = cache.translation == self.translation()?
            && cache.book == position.book
            && cache.chapter == position.chapter;
        if same {
            cache.items.as_deref()
        } else {
            None
        }
    }

    /// Chapters read from the card so far; a page turn inside a chapter adds none.
    pub fn loads(&self) -> usize {
        self.loads
    }

    /// Read the open chapter when the place has moved off the kept one.
    fn refresh_chapter(&mut self) {
        let (Some(position), Some(translation)) =
            (self.position, self.translation().map(str::to_owned))
        else {
            self.chapter = None;
            return;
        };
        let Some(book) = self.books.get(position.book).cloned() else {
            self.chapter = None;
            return;
        };
        let kept = self.chapter.as_ref().is_some_and(|cache| {
            cache.translation == translation
                && cache.book == position.book
                && cache.chapter == position.chapter
        });
        if kept {
            return;
        }
        self.loads += 1;
        let items = bible::load_chapter(&self.root, &translation, &book, position.chapter).ok();
        self.chapter = Some(ChapterCache {
            translation,
            book: position.book,
            chapter: position.chapter,
            items,
        });
    }

    /// The translations on the card with their titles, the one in use selected.
    pub fn open_translation_picker(&mut self) {
        let current = self.translation().map(str::to_owned);
        self.translation_list = bible::translations(&self.root)
            .unwrap_or_default()
            .into_iter()
            .map(|folder| {
                let title = std::fs::read_to_string(self.root.join(&folder).join("meta.txt"))
                    .ok()
                    .map(|text| bible::parse_meta(&text).title)
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| folder.clone());
                TranslationOption { folder, title }
            })
            .collect();
        self.translation_selected = current
            .and_then(|current| {
                self.translation_list
                    .iter()
                    .position(|option| option.folder == current)
            })
            .unwrap_or(0);
    }

    pub fn translation_list(&self) -> &[TranslationOption] {
        &self.translation_list
    }

    pub fn translation_selected(&self) -> usize {
        self.translation_selected
    }

    /// Move the picker's selection, wrapping at both ends.
    pub fn move_translation(&mut self, forward: bool) {
        let count = self.translation_list.len();
        if count == 0 {
            return;
        }
        self.translation_selected = if forward {
            (self.translation_selected + 1) % count
        } else {
            (self.translation_selected + count - 1) % count
        };
    }

    /// Read the selected translation. The place keeps its book and chapter when
    /// the new translation has them; otherwise the picker is the next screen.
    pub fn choose_translation(&mut self) -> SwitchOutcome {
        let Some(folder) = self
            .translation_list
            .get(self.translation_selected)
            .map(|option| option.folder.clone())
        else {
            return SwitchOutcome::Picker;
        };
        let index_path = self.root.join(&folder).join("index.tsv");
        let books = match std::fs::read_to_string(index_path)
            .ok()
            .and_then(|text| bible::parse_index(&text).ok())
        {
            Some(books) if !books.is_empty() => books,
            _ => return SwitchOutcome::Picker,
        };
        let language = std::fs::read_to_string(self.root.join(&folder).join("meta.txt"))
            .ok()
            .and_then(|text| Language::from_tag(&bible::parse_meta(&text).language));
        let place = self.position.map(|position| {
            (
                self.books.get(position.book).map(|book| book.usfm.clone()),
                position.chapter,
            )
        });
        self.card = BibleCard::Ready {
            translation: folder,
        };
        self.language = language;
        self.nav = BibleNav::new(books.clone());
        self.books = books;
        self.chapter = None;
        let kept = place.and_then(|(usfm, chapter)| {
            let index = self
                .books
                .iter()
                .position(|book| Some(&book.usfm) == usfm.as_ref())?;
            (chapter <= self.books[index].chapters).then_some(Position {
                book: index,
                chapter,
                page: 0,
            })
        });
        match kept {
            Some(position) => {
                self.position = Some(position);
                self.place_changed = true;
                self.refresh_chapter();
                SwitchOutcome::Kept
            }
            None => {
                self.position = None;
                SwitchOutcome::Picker
            }
        }
    }

    pub fn translation_count(&self) -> usize {
        bible::translations(&self.root).map_or(0, |list| list.len())
    }
}

/// The language of a translation folder from its `meta.txt`, when known.
fn read_language(root: &Path, translation: &str) -> Option<Language> {
    let text = std::fs::read_to_string(root.join(translation).join("meta.txt")).ok()?;
    Language::from_tag(&bible::parse_meta(&text).language)
}

const VERSES_FILE: &str = "VERSES.TXT";

/// The saved translation when the card still has it, else the first one.
fn pick_translation(list: Vec<String>, saved: Option<&str>) -> Option<String> {
    let kept = saved.and_then(|code| list.iter().find(|candidate| *candidate == code));
    kept.cloned().or_else(|| list.into_iter().next())
}

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
    fn a_page_turn_inside_a_chapter_reads_no_chapter() {
        let root = temp_root("one-load");
        let folder = root.join("RVR1960");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("index.tsv"), "GEN\tGénesis\t2\tGEN.txt\n").unwrap();
        std::fs::write(
            folder.join("meta.txt"),
            "format=1\nabbreviation=RVR1960\nlanguage=es\n",
        )
        .unwrap();
        std::fs::write(
            folder.join("GEN.txt"),
            "C\t1\nV\t1\t1\tUno dos tres cuatro\n",
        )
        .unwrap();
        let mut state = BibleUiState::with_root(&root);
        state.open_at(Position {
            book: 0,
            chapter: 1,
            page: 0,
        });
        assert_eq!(state.loads(), 1, "opening reads the chapter once");
        state.turn_page(true, 5);
        assert_eq!(
            state.loads(),
            1,
            "a page turn inside the chapter reads nothing"
        );
        state.turn_page(true, 1);
        assert_eq!(state.loads(), 2, "crossing into chapter 2 reads it once");
        let _ = std::fs::remove_dir_all(&root);
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

    #[test]
    fn the_saved_translation_wins_over_the_first_on_the_card() {
        let list = || vec!["KJV".to_string(), "RVR1960".to_string()];
        assert_eq!(
            pick_translation(list(), Some("RVR1960")).as_deref(),
            Some("RVR1960")
        );
        assert_eq!(
            pick_translation(list(), Some("NVI")).as_deref(),
            Some("KJV")
        );
        assert_eq!(pick_translation(list(), None).as_deref(), Some("KJV"));
        assert_eq!(pick_translation(Vec::new(), Some("KJV")), None);
    }
}
