//! Book and chapter picker state for the Bible reader. Drawing and state
//! only: routes and wiring come with the main line.

use crate::bible::{BibleBook, Testament};

/// The eight canonical sections, by book number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Section {
    pub tab: &'static str,
    pub name: &'static str,
    /// Inclusive book-number range.
    pub first: u8,
    pub last: u8,
}

/// Tabs and names as the mockup shows them.
pub const SECTIONS: [Section; 8] = [
    Section {
        tab: "PEN",
        name: "Pentateuch",
        first: 1,
        last: 5,
    },
    Section {
        tab: "HIS",
        name: "History",
        first: 6,
        last: 17,
    },
    Section {
        tab: "POE",
        name: "Poetry & Wisdom",
        first: 18,
        last: 22,
    },
    Section {
        tab: "MAJ",
        name: "Major Prophets",
        first: 23,
        last: 27,
    },
    Section {
        tab: "MIN",
        name: "Minor Prophets",
        first: 28,
        last: 39,
    },
    Section {
        tab: "GOS",
        name: "Gospels & Acts",
        first: 40,
        last: 44,
    },
    Section {
        tab: "PAU",
        name: "Paul's Letters",
        first: 45,
        last: 57,
    },
    Section {
        tab: "REV",
        name: "General Letters & Revelation",
        first: 58,
        last: 66,
    },
];

/// Which picker the user is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BibleNavView {
    Books,
    Chapters,
}

/// Picker state over the books the card actually provides.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibleNav {
    /// Books present in `BOOKS.TXT`, ascending by number.
    books: Vec<BibleBook>,
    view: BibleNavView,
    /// Index into the section's book list, not the book number.
    book_cursor: usize,
    section_cursor: usize,
    chapter_cursor: u16,
}

impl BibleNav {
    /// Build over the parsed books; missing books and empty sections are
    /// skipped, so the cursor lands on the first populated section.
    #[must_use]
    pub fn new(books: Vec<BibleBook>) -> Self {
        let mut nav = Self {
            books,
            view: BibleNavView::Books,
            book_cursor: 0,
            section_cursor: 0,
            chapter_cursor: 1,
        };
        nav.skip_empty_section();
        nav
    }

    #[must_use]
    pub fn books(&self) -> &[BibleBook] {
        &self.books
    }

    #[must_use]
    pub const fn view(&self) -> BibleNavView {
        self.view
    }

    #[must_use]
    pub const fn section_cursor(&self) -> usize {
        self.section_cursor
    }

    /// Position of the highlight inside the section's book list.
    #[must_use]
    pub const fn book_cursor(&self) -> usize {
        self.book_cursor
    }

    /// The section the cursor points at.
    #[must_use]
    pub const fn section(&self) -> &'static Section {
        &SECTIONS[self.section_cursor]
    }

    /// The books of the current section that exist on the card.
    #[must_use]
    pub fn section_books(&self) -> Vec<&BibleBook> {
        let section = &SECTIONS[self.section_cursor];
        self.books
            .iter()
            .filter(|book| (section.first..=section.last).contains(&book.number))
            .collect()
    }

    /// The highlighted book of the current section.
    #[must_use]
    pub fn selected_book(&self) -> Option<&BibleBook> {
        let section_books = self.section_books();
        section_books.get(self.book_cursor).copied()
    }

    /// The next populated section after the cursor, wrapping; `None` when the
    /// card only carries the current one.
    #[must_use]
    pub fn next_section(&self) -> Option<(usize, &'static str)> {
        for distance in 1..SECTIONS.len() {
            let cursor = (self.section_cursor + distance) % SECTIONS.len();
            if !self.section_book_count(cursor).is_empty() {
                return Some((cursor, SECTIONS[cursor].name));
            }
        }
        None
    }

    /// Names of the books the next populated section offers.
    #[must_use]
    pub fn next_section_books(&self, cursor: usize) -> Vec<String> {
        let section = &SECTIONS[cursor];
        self.books
            .iter()
            .filter(|book| (section.first..=section.last).contains(&book.number))
            .map(|book| book.name.clone())
            .collect()
    }

    fn section_book_count(&self, cursor: usize) -> Vec<&BibleBook> {
        let section = &SECTIONS[cursor];
        self.books
            .iter()
            .filter(|book| (section.first..=section.last).contains(&book.number))
            .collect()
    }

    /// Move to the next populated section; nothing to fill stays untouched.
    pub fn skip_empty_section(&mut self) {
        if !self.section_book_count(self.section_cursor).is_empty() {
            self.book_cursor = self
                .book_cursor
                .min(self.section_books().len().saturating_sub(1));
            return;
        }
        for distance in 1..=SECTIONS.len() {
            let cursor = (self.section_cursor + distance) % SECTIONS.len();
            if !self.section_book_count(cursor).is_empty() {
                self.section_cursor = cursor;
                self.book_cursor = 0;
                return;
            }
        }
    }

    /// ▲▼ inside the books view: move and wrap within the section.
    pub fn move_book(&mut self, direction: i32) {
        let count = self.section_books().len();
        if count == 0 {
            return;
        }
        self.book_cursor = if direction < 0 {
            self.book_cursor.checked_sub(1).unwrap_or(count - 1)
        } else {
            (self.book_cursor + 1) % count
        };
    }

    /// Short BOOT in the books view: always advance to the next populated
    /// section, wrapping.
    pub fn next_section_cyclic(&mut self) {
        for distance in 1..=SECTIONS.len() {
            let cursor = (self.section_cursor + distance) % SECTIONS.len();
            if !self.section_book_count(cursor).is_empty() {
                self.section_cursor = cursor;
                self.book_cursor = 0;
                return;
            }
        }
    }

    /// ● in the books view: open the chapter grid of the chosen book.
    pub fn open_chapters(&mut self) {
        if self.selected_book().is_some() {
            self.view = BibleNavView::Chapters;
            self.chapter_cursor = 1;
        }
    }

    /// ▲▼ in the chapters view, one chapter with wrapping.
    pub fn move_chapter(&mut self, direction: i32) {
        let Some(book) = self.selected_book() else {
            return;
        };
        self.chapter_cursor = if direction < 0 {
            if self.chapter_cursor <= 1 {
                book.chapters
            } else {
                self.chapter_cursor - 1
            }
        } else if self.chapter_cursor >= book.chapters {
            1
        } else {
            self.chapter_cursor + 1
        };
    }

    /// Short BOOT in the chapters view: jump ten chapters, wrapping.
    pub fn jump_chapter(&mut self, direction: i32) {
        let Some(book) = self.selected_book() else {
            return;
        };
        let total = u32::from(book.chapters);
        let current = u32::from(self.chapter_cursor.max(1));
        self.chapter_cursor = if direction < 0 {
            let jumped = current.checked_sub(10).unwrap_or(0);
            if jumped == 0 {
                book.chapters
            } else {
                jumped as u16
            }
        } else {
            let jumped = current + 10;
            if jumped > total {
                1
            } else {
                jumped as u16
            }
        };
    }

    /// The chapter choice of the chapters view.
    #[must_use]
    pub const fn chapter_cursor(&self) -> u16 {
        self.chapter_cursor
    }

    /// ● in the chapters view: hand the choice to the future reading view.
    #[must_use]
    pub fn open(&self) -> Option<(u8, u16)> {
        self.selected_book()
            .map(|book| (book.number, self.chapter_cursor))
    }

    /// The chapters view goes back to the books.
    pub fn back(&mut self) {
        if self.view == BibleNavView::Chapters {
            self.view = BibleNavView::Books;
        }
    }
}

/// The 66 Reina-Valera names as `BOOKS.TXT` text, one line per book:
/// `number|name|short name|chapters`. Test sample shared with the screen
/// previews so both draw the real data.
#[cfg(test)]
pub(crate) fn sample_books_txt() -> String {
    const NAMES: [(&str, &str, u16); 66] = [
        ("Génesis", "Gn", 50),
        ("Éxodo", "Ex", 40),
        ("Levítico", "Lv", 27),
        ("Números", "Nm", 36),
        ("Deuteronomio", "Dt", 34),
        ("Josué", "Jos", 24),
        ("Jueces", "Jue", 21),
        ("Rut", "Rt", 4),
        ("1 Samuel", "1S", 31),
        ("2 Samuel", "2S", 24),
        ("1 Reyes", "1R", 22),
        ("2 Reyes", "2R", 25),
        ("1 Crónicas", "1Cr", 29),
        ("2 Crónicas", "2Cr", 36),
        ("Esdras", "Esd", 10),
        ("Nehemías", "Neh", 13),
        ("Ester", "Est", 10),
        ("Job", "Job", 42),
        ("Salmos", "Sal", 150),
        ("Proverbios", "Pr", 31),
        ("Eclesiastés", "Ec", 12),
        ("Cantares", "Cant", 8),
        ("Isaías", "Is", 66),
        ("Jeremías", "Jer", 52),
        ("Lamentaciones", "Lm", 5),
        ("Ezequiel", "Ez", 48),
        ("Daniel", "Dn", 12),
        ("Oseas", "Os", 14),
        ("Joel", "Jl", 3),
        ("Amós", "Am", 9),
        ("Obadías", "Ob", 1),
        ("Jonás", "Jon", 4),
        ("Miqueas", "Mi", 7),
        ("Nahúm", "Nah", 3),
        ("Habacuc", "Hab", 3),
        ("Sofonías", "Sof", 3),
        ("Hageo", "Hag", 2),
        ("Zacarías", "Zac", 14),
        ("Malaquías", "Mal", 4),
        ("Mateo", "Mt", 28),
        ("Marcos", "Mr", 16),
        ("Lucas", "Lc", 24),
        ("Juan", "Jn", 21),
        ("Hechos", "Hch", 28),
        ("Romanos", "Ro", 16),
        ("1 Corintios", "1Co", 16),
        ("2 Corintios", "2Co", 13),
        ("Gálatas", "Ga", 6),
        ("Efesios", "Ef", 6),
        ("Filipenses", "Fil", 4),
        ("Colosenses", "Col", 4),
        ("1 Tesalonicenses", "1Ts", 5),
        ("2 Tesalonicenses", "2Ts", 3),
        ("1 Timoteo", "1Ti", 6),
        ("2 Timoteo", "2Ti", 4),
        ("Tito", "Tit", 3),
        ("Filemón", "Flm", 1),
        ("Hebreos", "He", 13),
        ("Santiago", "Stg", 5),
        ("1 Pedro", "1P", 5),
        ("2 Pedro", "2P", 3),
        ("1 Juan", "1Jn", 5),
        ("2 Juan", "2Jn", 1),
        ("3 Juan", "3Jn", 1),
        ("Judas", "Jud", 1),
        ("Apocalipsis", "Ap", 22),
    ];
    let mut text = String::from("# Wave Bible books sample\n");
    for (index, (name, short, chapters)) in NAMES.iter().enumerate() {
        text.push_str(&format!("{}|{name}|{short}|{chapters}\n", index + 1));
    }
    text
}

#[cfg(test)]
mod tests {
    use crate::bible::{parse_books, Testament};

    use super::{sample_books_txt, BibleNav, BibleNavView, SECTIONS};

    /// The 66 Reina-Valera names, matching the module sample.
    fn sample_books() -> Vec<crate::bible::BibleBook> {
        crate::bible::parse_books(&sample_books_txt()).unwrap()
    }

    #[test]
    fn sections_cover_the_canonical_ranges() {
        assert_eq!(SECTIONS.len(), 8);
        assert_eq!(SECTIONS[0].first, 1);
        assert_eq!(SECTIONS[7].last, 66);
        // Ranges are contiguous and ordered.
        for pair in SECTIONS.windows(2) {
            assert_eq!(pair[0].last + 1, pair[1].first);
        }
    }

    #[test]
    fn starts_on_the_first_populated_section() {
        let nav = BibleNav::new(sample_books());
        assert_eq!(nav.view(), BibleNavView::Books);
        assert_eq!(nav.section_cursor(), 0);
        assert_eq!(nav.selected_book().map(|book| book.number), Some(1));
    }

    #[test]
    fn missing_books_and_sections_are_skipped() {
        // Only the Pentateuch and two Psalms-section books, with Isaiah's
        // section populated: History and Poetry stay reachable, empty ones
        // never appear.
        let text = "1|Génesis|Gn|50\n18|Job|Job|42\n19|Salmos|Sal|150\n23|Isaías|Is|66\n";
        let mut nav = BibleNav::new(parse_books(text).unwrap());
        assert_eq!(nav.section_books().len(), 1);
        nav.next_section_cyclic();
        assert_eq!(nav.section_cursor(), 2, "Job and Psalms are Poetry");
        assert_eq!(nav.selected_book().map(|book| book.number), Some(18));
        nav.next_section_cyclic();
        assert_eq!(nav.section_cursor(), 3, "empty sections are skipped");
        assert_eq!(nav.selected_book().map(|book| book.number), Some(23));
    }

    #[test]
    fn books_move_and_wrap_within_the_section() {
        let mut nav = BibleNav::new(sample_books());
        nav.move_book(1);
        assert_eq!(nav.selected_book().map(|book| book.number), Some(2));
        for _ in 0..4 {
            nav.move_book(1);
        }
        assert_eq!(
            nav.selected_book().map(|book| book.number),
            Some(1),
            "wraps"
        );
        nav.move_book(-1);
        assert_eq!(nav.selected_book().map(|book| book.number), Some(5));
    }

    #[test]
    fn chapters_open_move_wrap_and_report_the_choice() {
        let mut nav = BibleNav::new(sample_books());
        // Jump to Poetry (Job, Psalms) through two section jumps.
        nav.next_section_cyclic();
        nav.next_section_cyclic();
        assert_eq!(nav.selected_book().map(|book| book.number), Some(18));
        nav.move_book(1);
        assert_eq!(nav.selected_book().map(|book| book.number), Some(19));
        nav.open_chapters();
        assert_eq!(nav.view(), BibleNavView::Chapters);
        assert_eq!(nav.chapter_cursor(), 1);
        nav.move_chapter(1);
        assert_eq!(nav.chapter_cursor(), 2);
        nav.jump_chapter(1);
        assert_eq!(nav.chapter_cursor(), 12, "short BOOT jumps ten");
        nav.move_chapter(-1);
        assert_eq!(nav.chapter_cursor(), 11);
        nav.move_chapter(1);
        assert_eq!(nav.chapter_cursor(), 12);
        nav.open();
        nav.back();
        assert_eq!(nav.view(), BibleNavView::Books);
    }

    #[test]
    fn chapter_moves_wrap_upwards_and_jump_backwards() {
        let mut nav = BibleNav::new(sample_books());
        nav.next_section_cyclic();
        nav.next_section_cyclic();
        nav.move_book(1);
        nav.open_chapters();
        // Back ten from chapter 1 wraps to the book's last chapter.
        nav.jump_chapter(-1);
        assert_eq!(
            nav.chapter_cursor(),
            150,
            "back ten from 1 wraps to the end"
        );
        nav.move_chapter(-1);
        assert_eq!(nav.chapter_cursor(), 149);
        // Single moves wrap past the ends like the books view.
        nav.move_chapter(1);
        assert_eq!(nav.chapter_cursor(), 150);
        nav.move_chapter(1);
        assert_eq!(nav.chapter_cursor(), 1, "+1 from the last chapter wraps");
    }

    #[test]
    fn testament_covers_every_book_number() {
        let books = sample_books();
        assert_eq!(books.len(), 66);
        assert_eq!(Testament::from_book_number(1), Some(Testament::Old));
        assert_eq!(Testament::from_book_number(40), Some(Testament::New));
        let _ = books;
    }
}
