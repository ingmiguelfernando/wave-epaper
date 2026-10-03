//! Offline Bible text data; chapter reads retain only the requested chapter.

use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Component, Path},
};

use anyhow::{bail, ensure, Context, Result};

pub const BIBLE_ROOT: &str = "/sdcard/RUSTMIX/BIBLE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Testament {
    Old,
    New,
}

impl Testament {
    pub const fn from_book_number(number: u8) -> Option<Self> {
        match number {
            1..=39 => Some(Self::Old),
            40..=66 => Some(Self::New),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibleBook {
    pub number: u8,
    pub name: String,
    pub short_name: String,
    pub chapters: u16,
}

pub fn parse_books(text: &str) -> Result<Vec<BibleBook>> {
    let mut books = Vec::new();
    let mut seen = [false; 67];
    for (index, raw) in text.lines().enumerate() {
        let line = clean_line(raw);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        ensure!(
            fields.len() == 4,
            "line {number}: expected four book fields"
        );
        ensure!(
            fields.iter().all(|field| !field.is_empty()),
            "line {number}: missing book field"
        );
        let book = parse_book_number(fields[0], number)?;
        ensure!(
            !seen[book as usize],
            "line {number}: duplicate book number {book}"
        );
        let chapters = positive_number(fields[3], "chapters", number)?;
        seen[book as usize] = true;
        books.push(BibleBook {
            number: book,
            name: fields[1].into(),
            short_name: fields[2].into(),
            chapters,
        });
    }
    Ok(books)
}

pub fn book_file_name(number: u8) -> String {
    format!("{number:02}.TXT")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verse {
    pub number: u16,
    pub text: String,
}

pub fn read_chapter(mut reader: impl BufRead, chapter: u16) -> Result<Vec<Verse>> {
    ensure!(chapter > 0, "chapter must be positive");
    let mut verses = Vec::new();
    let mut buffer = String::new();
    let mut line_number = 0;
    let mut previous_chapter = 0;
    loop {
        buffer.clear();
        let number = line_number + 1;
        if reader
            .read_line(&mut buffer)
            .with_context(|| format!("line {number}: reading Bible text"))?
            == 0
        {
            break;
        }
        line_number = number;
        let line = clean_line(&buffer);
        if line.is_empty() {
            continue;
        }
        let (reference, text) = line
            .split_once('\t')
            .with_context(|| format!("line {number}: missing verse text separator"))?;
        let (current, verse) = reference
            .split_once(':')
            .with_context(|| format!("line {number}: expected chapter:verse"))?;
        let current = positive_number(current, "chapter", number)?;
        let verse = positive_number(verse, "verse", number)?;
        ensure!(!text.trim().is_empty(), "line {number}: missing verse text");
        ensure!(
            current >= previous_chapter,
            "line {number}: chapters are not sorted"
        );
        previous_chapter = current;
        // One boundary line establishes that the requested chapter has ended.
        if current > chapter {
            break;
        }
        if current == chapter {
            verses.push(Verse {
                number: verse,
                text: text.into(),
            });
        }
    }
    Ok(verses)
}

pub fn load_chapter(root: &Path, code: &str, book: u8, chapter: u16) -> Result<Vec<Verse>> {
    let mut components = Path::new(code).components();
    ensure!(
        matches!(components.next(), Some(Component::Normal(name)) if name == code)
            && components.next().is_none()
            && !code.contains(['/', '\\', '\0']),
        "translation code must be one normal path component"
    );
    ensure!(
        Testament::from_book_number(book).is_some(),
        "book must be in 1..=66"
    );
    ensure!(chapter > 0, "chapter must be positive");
    let path = root.join(code).join(book_file_name(book));
    let file =
        File::open(&path).with_context(|| format!("opening Bible book {}", path.display()))?;
    read_chapter(BufReader::new(file), chapter)
}

pub fn translations(root: &Path) -> io::Result<Vec<String>> {
    let mut codes = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.as_encoded_bytes().starts_with(b".") {
            continue;
        }
        if !entry.file_type()?.is_dir() {
            continue;
        }
        match fs::metadata(entry.path().join("BOOKS.TXT")) {
            Ok(metadata) if metadata.is_file() => {
                let code = name.into_string().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 translation code")
                })?;
                codes.push(code);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    codes.sort();
    Ok(codes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerseRef {
    pub book: u8,
    pub chapter: u16,
    pub first: u16,
    pub last: u16,
}

impl VerseRef {
    pub fn label(&self, books: &[BibleBook]) -> String {
        let name = books
            .iter()
            .find(|book| book.number == self.book)
            .map(|book| book.short_name.clone())
            .unwrap_or_else(|| format!("Book {}", self.book));
        if self.first == self.last {
            format!("{name} {}:{}", self.chapter, self.first)
        } else {
            format!("{name} {}:{}-{}", self.chapter, self.first, self.last)
        }
    }
}

pub fn parse_verse_list(text: &str) -> Result<Vec<VerseRef>> {
    let mut list = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = clean_line(raw);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let mut fields = line.split_whitespace();
        let book = parse_book_number(fields.next().unwrap_or_default(), number)?;
        let reference = fields
            .next()
            .with_context(|| format!("line {number}: missing verse reference"))?;
        ensure!(
            fields.next().is_none(),
            "line {number}: unexpected reference field"
        );
        let (chapter, verses) = reference
            .split_once(':')
            .with_context(|| format!("line {number}: expected chapter:verse"))?;
        let chapter = positive_number(chapter, "chapter", number)?;
        let (first, last) = verses.split_once('-').unwrap_or((verses, verses));
        let first = positive_number(first, "first verse", number)?;
        let last = positive_number(last, "last verse", number)?;
        ensure!(first <= last, "line {number}: reversed verse range");
        list.push(VerseRef {
            book,
            chapter,
            first,
            last,
        });
    }
    Ok(list)
}

pub fn verse_of_the_day(list: &[VerseRef], epoch_day: u32) -> Option<&VerseRef> {
    if list.is_empty() {
        None
    } else {
        list.get((u64::from(epoch_day) % list.len() as u64) as usize)
    }
}

fn clean_line(line: &str) -> &str {
    line.trim().trim_start_matches('\u{feff}').trim()
}

fn positive_number(text: &str, field: &str, line: usize) -> Result<u16> {
    ensure!(
        !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()),
        "line {line}: bad {field}"
    );
    let value = text
        .parse::<u16>()
        .with_context(|| format!("line {line}: bad {field}"))?;
    ensure!(value > 0, "line {line}: zero {field}");
    Ok(value)
}

fn parse_book_number(text: &str, line: usize) -> Result<u8> {
    let number = positive_number(text, "book number", line)?;
    if number > 66 {
        bail!("line {line}: book number outside 1..=66");
    }
    Ok(number as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Read},
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    const BOOKS: &str =
        "\u{feff}# Books\r\n\n 1 | Génesis | Gén | 50 \r\n19|Salmos|Sal|150\n66|Apocalipsis|Ap|22";
    const CHAPTERS: &str = "\u{feff}1:1\tEn el principio.\r\n1:2\tCreación.\n\n2:1\tAsí terminó.\n2:2\tDescansó.\n3:1\tÚltimo capítulo.";
    const REFERENCES: &str = "\u{feff}# Daily\r\n\n19 23:1\n 19\t23:1-3 \r\n66 22:21";

    fn assert_line_error<T: std::fmt::Debug>(result: Result<T>, line: usize) {
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains(&format!("line {line}:")),
            "{error:#}"
        );
    }

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "wave-bible-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn translation(&self, code: &str) -> PathBuf {
            let path = self.0.join(code);
            fs::create_dir(&path).unwrap();
            fs::write(path.join("BOOKS.TXT"), BOOKS).unwrap();
            path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn testament_classifies_every_valid_book_and_rejects_others() {
        for number in 0..=u8::MAX {
            assert_eq!(
                Testament::from_book_number(number),
                match number {
                    1..=39 => Some(Testament::Old),
                    40..=66 => Some(Testament::New),
                    _ => None,
                }
            );
        }
    }

    #[test]
    fn books_trim_fields_skip_comments_and_keep_accents() {
        let books = parse_books(BOOKS).unwrap();
        assert_eq!(books.len(), 3);
        assert_eq!(
            books[0],
            BibleBook {
                number: 1,
                name: "Génesis".into(),
                short_name: "Gén".into(),
                chapters: 50
            }
        );
        assert_eq!(books[2].number, 66);
    }

    #[test]
    fn empty_books_and_bom_only_are_valid() {
        for text in ["", "\u{feff}", "\n # Comment\n\r\n"] {
            assert!(parse_books(text).unwrap().is_empty());
        }
    }

    #[test]
    fn books_reject_bad_numbers_and_chapter_counts() {
        for field in ["x", "-1", "+1", "1.0", "65536", "0", "67", "256"] {
            assert_line_error(parse_books(&format!("# Header\n{field}|Name|N|1")), 2);
        }
        for field in ["x", "-1", "+1", "65536", "0"] {
            assert_line_error(parse_books(&format!("\n1|Name|N|{field}")), 2);
        }
    }

    #[test]
    fn books_reject_missing_and_extra_fields() {
        for text in [
            "1",
            "1|Name",
            "1|Name|N",
            "|Name|N|1",
            "1||N|1",
            "1|Name||1",
            "1|Name|N|",
            "1|Name|N|1|extra",
        ] {
            assert_line_error(parse_books(text), 1);
        }
    }

    #[test]
    fn books_reject_duplicate_numbers_after_skipped_lines() {
        assert_line_error(parse_books("1|One|O|1\n# Comment\n\n01|Again|A|2"), 4);
    }

    #[test]
    fn books_accept_maximum_chapter_count() {
        assert_eq!(
            parse_books("66|Last|L|65535").unwrap()[0].chapters,
            u16::MAX
        );
    }

    #[test]
    fn file_names_are_two_digits_for_every_valid_book() {
        for number in 1..=66 {
            let name = book_file_name(number);
            assert_eq!(name.len(), 6);
            assert_eq!(name, format!("{number:02}.TXT"));
        }
        assert_eq!(book_file_name(1), "01.TXT");
        assert_eq!(book_file_name(66), "66.TXT");
    }

    #[test]
    fn chapters_select_first_middle_and_final_without_final_newline() {
        assert_eq!(
            read_chapter(Cursor::new(CHAPTERS), 1).unwrap(),
            vec![
                Verse {
                    number: 1,
                    text: "En el principio.".into()
                },
                Verse {
                    number: 2,
                    text: "Creación.".into()
                },
            ]
        );
        let middle = read_chapter(Cursor::new(CHAPTERS), 2).unwrap();
        assert_eq!(middle.len(), 2);
        assert_eq!(middle[1].text, "Descansó.");
        assert_eq!(
            read_chapter(Cursor::new(CHAPTERS), 3).unwrap(),
            vec![Verse {
                number: 1,
                text: "Último capítulo.".into()
            }]
        );
    }

    #[test]
    fn chapters_return_empty_for_missing_and_empty_input() {
        for text in ["", "\u{feff}\n \r\n", "2:1\tLater."] {
            assert!(read_chapter(Cursor::new(text), 1).unwrap().is_empty());
        }
        assert!(read_chapter(Cursor::new(CHAPTERS), 4).unwrap().is_empty());
    }

    #[test]
    fn chapters_reject_zero_target() {
        assert!(read_chapter(Cursor::new(""), 0).is_err());
    }

    #[test]
    fn chapters_reject_malformed_lines_with_physical_line_numbers() {
        for line in [
            "1:1 text",
            "1\tText",
            "x:1\tText",
            "1:x\tText",
            "0:1\tText",
            "1:0\tText",
            "65536:1\tText",
            "1:65536\tText",
            "1:1:2\tText",
            "1:1\t",
            "1:1\t  ",
            "# Not a verse",
            "+1:1\tText",
            "1:-1\tText",
        ] {
            assert_line_error(
                read_chapter(Cursor::new(format!("\u{feff}\n\n{line}")), 1),
                3,
            );
        }
    }

    #[test]
    fn chapters_validate_lines_before_target_and_reject_unsorted_chapters() {
        assert_line_error(read_chapter(Cursor::new("bad\n2:1\tText"), 2), 1);
        assert_line_error(read_chapter(Cursor::new("2:1\tText\n1:1\tEarlier"), 3), 2);
    }

    #[test]
    fn chapters_preserve_embedded_tabs_and_maximum_numbers() {
        let verses = read_chapter(Cursor::new("65535:65535\tCafé\tcon té"), u16::MAX).unwrap();
        assert_eq!(verses[0].number, u16::MAX);
        assert_eq!(verses[0].text, "Café\tcon té");
    }

    struct BoundaryReader {
        prefix: Cursor<Vec<u8>>,
        consumed: usize,
    }

    impl Read for BoundaryReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("chapter reader must use buffered reads")
        }
    }

    impl BufRead for BoundaryReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            assert!(
                (self.prefix.position() as usize) < self.prefix.get_ref().len(),
                "read beyond chapter boundary"
            );
            self.prefix.fill_buf()
        }

        fn consume(&mut self, amount: usize) {
            self.consumed += amount;
            self.prefix.consume(amount);
        }
    }

    #[test]
    fn chapters_stop_after_one_boundary_line_without_reading_book_tail() {
        let prefix = b"1:1\tEarlier\n2:1\tTarget\n2:2\tMore\n3:1\tBoundary\n";
        let mut reader = BoundaryReader {
            prefix: Cursor::new(prefix.to_vec()),
            consumed: 0,
        };
        assert_eq!(read_chapter(&mut reader, 2).unwrap().len(), 2);
        assert_eq!(reader.consumed, prefix.len());
    }

    #[test]
    fn chapters_do_not_validate_tail_after_boundary() {
        assert_eq!(
            read_chapter(Cursor::new("1:1\tText\n2:1\tBoundary\nmalformed"), 1)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn missing_chapters_stop_at_the_first_later_chapter() {
        let prefix = b"1:1\tEarlier\n3:1\tBoundary\n";
        let mut reader = BoundaryReader {
            prefix: Cursor::new(prefix.to_vec()),
            consumed: 0,
        };
        assert!(read_chapter(&mut reader, 2).unwrap().is_empty());
        assert_eq!(reader.consumed, prefix.len());
    }

    #[test]
    fn chapters_validate_the_boundary_line() {
        assert_line_error(read_chapter(Cursor::new("1:1\tText\n2:0\tInvalid"), 1), 2);
    }

    #[test]
    fn chapters_report_invalid_utf8_and_io_errors_with_line() {
        assert_line_error(read_chapter(Cursor::new(b"1:1\tText\n1:2\t\xff"), 1), 2);
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::PermissionDenied.into())
            }
        }
        impl BufRead for FailingReader {
            fn fill_buf(&mut self) -> io::Result<&[u8]> {
                Err(io::ErrorKind::PermissionDenied.into())
            }
            fn consume(&mut self, _: usize) {}
        }
        let error = read_chapter(FailingReader, 1).unwrap_err();
        assert!(error.to_string().contains("line 1:"));
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn load_chapter_uses_translation_and_padded_file_name() {
        let root = TempRoot::new();
        let path = root.translation("RV1909");
        fs::write(path.join("01.TXT"), CHAPTERS).unwrap();
        assert_eq!(
            load_chapter(&root.0, "RV1909", 1, 3).unwrap()[0].text,
            "Último capítulo."
        );
    }

    #[test]
    fn load_chapter_rejects_traversal_and_invalid_numbers_before_io() {
        let root = Path::new("/nonexistent-wave-bible");
        for code in [
            "",
            ".",
            "..",
            "../RV",
            "/RV",
            "RV/other",
            "RV/",
            "RV/.",
            "./RV",
            "RV//",
            "RV\\other",
            "RV\0other",
        ] {
            let error = load_chapter(root, code, 1, 1).unwrap_err();
            assert!(
                error.to_string().contains("normal path component"),
                "{code:?}: {error}"
            );
            assert!(error.downcast_ref::<io::Error>().is_none());
        }
        for book in [0, 67, u8::MAX] {
            assert!(load_chapter(root, "RV", book, 1)
                .unwrap_err()
                .to_string()
                .contains("1..=66"));
        }
        assert!(load_chapter(root, "RV", 1, 0)
            .unwrap_err()
            .to_string()
            .contains("positive"));
    }

    #[test]
    fn load_chapter_preserves_open_error_kind() {
        let root = TempRoot::new();
        let error = load_chapter(&root.0, "missing", 1, 1).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn translations_sort_only_visible_directories_with_books_file() {
        let root = TempRoot::new();
        root.translation("ZZ");
        root.translation("AA");
        root.translation("Á");
        root.translation(".hidden");
        root.translation("._resource");
        fs::create_dir(root.0.join("empty")).unwrap();
        fs::create_dir(root.0.join("directory-books")).unwrap();
        fs::create_dir(root.0.join("directory-books/BOOKS.TXT")).unwrap();
        fs::write(root.0.join("not-a-directory"), BOOKS).unwrap();
        assert_eq!(translations(&root.0).unwrap(), vec!["AA", "ZZ", "Á"]);
    }

    #[test]
    fn translations_empty_missing_and_file_root_report_accurately() {
        let root = TempRoot::new();
        assert!(translations(&root.0).unwrap().is_empty());
        assert_eq!(
            translations(&root.0.join("missing")).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        let path = root.0.join("file");
        fs::write(&path, "text").unwrap();
        assert!(translations(&path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn translations_propagate_metadata_errors_not_empty_results() {
        use std::os::unix::fs::symlink;
        let root = TempRoot::new();
        let path = root.translation("loop");
        fs::remove_file(path.join("BOOKS.TXT")).unwrap();
        symlink("BOOKS.TXT", path.join("BOOKS.TXT")).unwrap();
        assert!(translations(&root.0).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn translations_reject_non_utf8_codes_with_invalid_data() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let root = TempRoot::new();
        let path = root.0.join(OsString::from_vec(vec![0xff]));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("BOOKS.TXT"), BOOKS).unwrap();
        assert_eq!(
            translations(&root.0).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn verse_list_parses_singles_ranges_and_retains_duplicates() {
        let list = parse_verse_list(REFERENCES).unwrap();
        assert_eq!(
            list,
            vec![
                VerseRef {
                    book: 19,
                    chapter: 23,
                    first: 1,
                    last: 1
                },
                VerseRef {
                    book: 19,
                    chapter: 23,
                    first: 1,
                    last: 3
                },
                VerseRef {
                    book: 66,
                    chapter: 22,
                    first: 21,
                    last: 21
                },
            ]
        );
        let duplicates = parse_verse_list("19 23:1\n19 23:1").unwrap();
        assert_eq!(duplicates.len(), 2);
        assert_eq!(duplicates[0], duplicates[1]);
    }

    #[test]
    fn verse_list_empty_comments_and_bom_are_valid() {
        for text in ["", "\u{feff}", "\n# Comment\n \r\n"] {
            assert!(parse_verse_list(text).unwrap().is_empty());
        }
    }

    #[test]
    fn verse_list_rejects_invalid_syntax_numbers_and_ranges() {
        for text in [
            "19",
            "19 23",
            "19 23:",
            "19 :1",
            "19 23:1-",
            "19 23:-1",
            "19 23:1-2-3",
            "19 23:3-1",
            "19 0:1",
            "19 23:0",
            "19 23:1-0",
            "0 23:1",
            "67 23:1",
            "256 23:1",
            "x 23:1",
            "19 x:1",
            "19 23:x",
            "19 23:1-x",
            "19 65536:1",
            "19 23:65536",
            "19 23:1-65536",
            "19 23:1 extra",
            "19 23:1:2",
            "+19 23:1",
            "19 +23:1",
            "19 23:+1",
            "19 23:1 - 3",
        ] {
            assert_line_error(parse_verse_list(&format!("\u{feff}# Header\n\n{text}")), 3);
        }
    }

    #[test]
    fn verse_list_accepts_equal_ranges_and_maximum_numbers() {
        let list = parse_verse_list("1 65535:65535-65535").unwrap();
        assert_eq!(
            list[0],
            VerseRef {
                book: 1,
                chapter: u16::MAX,
                first: u16::MAX,
                last: u16::MAX
            }
        );
    }

    #[test]
    fn reference_labels_use_short_names_and_unknown_book_fallback() {
        let books = parse_books(BOOKS).unwrap();
        let list = parse_verse_list(REFERENCES).unwrap();
        assert_eq!(list[0].label(&books), "Sal 23:1");
        assert_eq!(list[1].label(&books), "Sal 23:1-3");
        assert_eq!(list[1].label(&[]), "Book 19 23:1-3");
        assert_eq!(list[0].label(&[]), "Book 19 23:1");
    }

    #[test]
    fn daily_rotation_handles_empty_single_multiple_and_maximum_day() {
        assert_eq!(verse_of_the_day(&[], 0), None);
        assert_eq!(verse_of_the_day(&[], u32::MAX), None);
        let list = parse_verse_list(REFERENCES).unwrap();
        for day in 0..12 {
            assert!(std::ptr::eq(
                verse_of_the_day(&list, day).unwrap(),
                &list[day as usize % list.len()]
            ));
        }
        assert_eq!(
            verse_of_the_day(&list, u32::MAX),
            Some(&list[(u32::MAX as u64 % list.len() as u64) as usize])
        );
        assert_eq!(verse_of_the_day(&list[..1], u32::MAX), Some(&list[0]));
    }
}
