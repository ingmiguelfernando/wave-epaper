//! Book and chapter picker state for the Bible reader. Drawing and state
//! only: routes and wiring come with the main line.

use crate::bible::BibleBook;

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
        name: "Pentateuco",
        first: 1,
        last: 5,
    },
    Section {
        tab: "HIS",
        name: "Libros históricos",
        first: 6,
        last: 17,
    },
    Section {
        tab: "POE",
        name: "Poesía y sabiduría",
        first: 18,
        last: 22,
    },
    Section {
        tab: "MAJ",
        name: "Profetas mayores",
        first: 23,
        last: 27,
    },
    Section {
        tab: "MIN",
        name: "Profetas menores",
        first: 28,
        last: 39,
    },
    Section {
        tab: "GOS",
        name: "Evangelios y Hechos",
        first: 40,
        last: 44,
    },
    Section {
        tab: "PAU",
        name: "Cartas de Pablo",
        first: 45,
        last: 57,
    },
    Section {
        tab: "REV",
        name: "Cartas generales y Apocalipsis",
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
    /// Books present in `index.tsv`, ascending by number.
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
        self.books_in(self.section_cursor).collect()
    }

    /// The highlighted book of the current section.
    #[must_use]
    pub fn selected_book(&self) -> Option<&BibleBook> {
        self.books_in(self.section_cursor).nth(self.book_cursor)
    }

    /// The next populated section after the cursor, wrapping; `None` when the
    /// card only carries the current one.
    #[must_use]
    pub fn next_section(&self) -> Option<(usize, &'static str)> {
        (1..SECTIONS.len())
            .map(|distance| (self.section_cursor + distance) % SECTIONS.len())
            .find(|&cursor| self.has_books(cursor))
            .map(|cursor| (cursor, SECTIONS[cursor].name))
    }

    /// Names of the books section `cursor` offers.
    #[must_use]
    pub fn next_section_books(&self, cursor: usize) -> Vec<String> {
        self.books_in(cursor)
            .map(|book| book.name.clone())
            .collect()
    }

    /// Books of section `cursor` that exist on the card.
    fn books_in(&self, cursor: usize) -> impl Iterator<Item = &BibleBook> + '_ {
        let section = SECTIONS[cursor];
        self.books
            .iter()
            .filter(move |book| (section.first..=section.last).contains(&book.number))
    }

    fn has_books(&self, cursor: usize) -> bool {
        self.books_in(cursor).next().is_some()
    }

    /// Land on a populated section, keeping the cursor inside its books.
    fn skip_empty_section(&mut self) {
        if !self.has_books(self.section_cursor) {
            self.next_section_cyclic();
        }
        self.book_cursor = self
            .book_cursor
            .min(self.section_books().len().saturating_sub(1));
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
        if let Some((cursor, _)) = self.next_section() {
            self.section_cursor = cursor;
            self.book_cursor = 0;
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

/// The 66 Reina-Valera names as `index.tsv` text, one line per book:
/// `usfm<TAB>name<TAB>chapters<TAB>file`. Test sample shared with the screen
/// previews so both draw the real data.
#[cfg(test)]
pub(crate) fn sample_books_txt() -> String {
    const NAMES: [(&str, u16); 66] = [
        ("Génesis", 50),
        ("Éxodo", 40),
        ("Levítico", 27),
        ("Números", 36),
        ("Deuteronomio", 34),
        ("Josué", 24),
        ("Jueces", 21),
        ("Rut", 4),
        ("1 Samuel", 31),
        ("2 Samuel", 24),
        ("1 Reyes", 22),
        ("2 Reyes", 25),
        ("1 Crónicas", 29),
        ("2 Crónicas", 36),
        ("Esdras", 10),
        ("Nehemías", 13),
        ("Ester", 10),
        ("Job", 42),
        ("Salmos", 150),
        ("Proverbios", 31),
        ("Eclesiastés", 12),
        ("Cantares", 8),
        ("Isaías", 66),
        ("Jeremías", 52),
        ("Lamentaciones", 5),
        ("Ezequiel", 48),
        ("Daniel", 12),
        ("Oseas", 14),
        ("Joel", 3),
        ("Amós", 9),
        ("Obadías", 1),
        ("Jonás", 4),
        ("Miqueas", 7),
        ("Nahúm", 3),
        ("Habacuc", 3),
        ("Sofonías", 3),
        ("Hageo", 2),
        ("Zacarías", 14),
        ("Malaquías", 4),
        ("Mateo", 28),
        ("Marcos", 16),
        ("Lucas", 24),
        ("Juan", 21),
        ("Hechos", 28),
        ("Romanos", 16),
        ("1 Corintios", 16),
        ("2 Corintios", 13),
        ("Gálatas", 6),
        ("Efesios", 6),
        ("Filipenses", 4),
        ("Colosenses", 4),
        ("1 Tesalonicenses", 5),
        ("2 Tesalonicenses", 3),
        ("1 Timoteo", 6),
        ("2 Timoteo", 4),
        ("Tito", 3),
        ("Filemón", 1),
        ("Hebreos", 13),
        ("Santiago", 5),
        ("1 Pedro", 5),
        ("2 Pedro", 3),
        ("1 Juan", 5),
        ("2 Juan", 1),
        ("3 Juan", 1),
        ("Judas", 1),
        ("Apocalipsis", 22),
    ];
    let mut text = String::from("# Wave Bible books sample\n");
    for (index, (name, chapters)) in NAMES.iter().enumerate() {
        let code = crate::bible::USFM_BOOKS[index];
        text.push_str(&format!("{code}\t{name}\t{chapters}\t{code}.txt\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use crate::bible::{parse_index, Testament};

    use super::{sample_books_txt, BibleNav, BibleNavView, SECTIONS};

    /// The 66 Reina-Valera names, matching the module sample.
    fn sample_books() -> Vec<crate::bible::BibleBook> {
        crate::bible::parse_index(&sample_books_txt()).unwrap()
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
        let text = "GEN\tGénesis\t50\tGEN.txt\nJOB\tJob\t42\tJOB.txt\nPSA\tSalmos\t150\tPSA.txt\nISA\tIsaías\t66\tISA.txt\n";
        let mut nav = BibleNav::new(parse_index(text).unwrap());
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
        assert_eq!(nav.open(), Some((19, 12)), "Psalms 12");
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
    }
}
