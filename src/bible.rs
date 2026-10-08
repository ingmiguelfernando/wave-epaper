//! Offline Bible text in the layout of `scripts/bible_json_to_sd.py`
//! (folloup-waveshare). Only the requested chapter stays in RAM.

use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    path::{Component, Path},
};

use anyhow::{bail, ensure, Context, Result};

pub const BIBLE_ROOT: &str = "/sdcard/RUSTMIX/BIBLE";

/// Canonical order; the position plus one is the book number.
pub(crate) const USFM_BOOKS: [&str; 66] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
    "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
    "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
    "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH",
    "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD",
    "REV",
];

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

/// Book number (1 to 66) for a USFM code, `None` for anything else.
pub fn book_number(usfm: &str) -> Option<u8> {
    USFM_BOOKS
        .iter()
        .position(|code| *code == usfm)
        .map(|index| index as u8 + 1)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibleBook {
    pub number: u8,
    pub usfm: String,
    pub name: String,
    pub short_name: String,
    pub chapters: u16,
    pub file: String,
}

/// Short label from a book name: the first three letters of the first word,
/// keeping a leading number (`1 Corintios` gives `1 Cor`).
pub fn short_name(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or_default();
    let (number, word) = match first.chars().next() {
        Some(digit) if digit.is_ascii_digit() => (Some(digit), words.next().unwrap_or_default()),
        _ => (None, first),
    };
    let letters: String = word.chars().take(3).collect();
    match number {
        Some(digit) => format!("{digit} {letters}"),
        None => letters,
    }
}

/// Parse `index.tsv`: `usfm<TAB>name<TAB>chapters<TAB>file` per line. Codes
/// outside the canonical 66 are skipped.
pub fn parse_index(text: &str) -> Result<Vec<BibleBook>> {
    let mut books: Vec<BibleBook> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = clean_line(raw);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        ensure!(
            fields.len() == 4,
            "index line {number}: expected four fields"
        );
        ensure!(
            fields.iter().all(|field| !field.is_empty()),
            "index line {number}: missing field"
        );
        let Some(book) = book_number(fields[0]) else {
            continue;
        };
        ensure!(
            !books.iter().any(|known| known.number == book),
            "index line {number}: duplicate book {}",
            fields[0]
        );
        let chapters = positive_number(fields[2], "chapters", number)?;
        books.push(BibleBook {
            number: book,
            usfm: fields[0].into(),
            name: fields[1].into(),
            short_name: short_name(fields[1]),
            chapters,
            file: fields[3].into(),
        });
    }
    books.sort_by_key(|book| book.number);
    Ok(books)
}

/// One line of `<USFM>.idx`: chapter, byte offset of its `C` record, verses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChapterIndex {
    pub chapter: u16,
    pub offset: u64,
    pub verses: u16,
}

pub fn parse_chapter_index(text: &str) -> Result<Vec<ChapterIndex>> {
    let mut chapters = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = clean_line(raw);
        if line.is_empty() {
            continue;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split('\t').collect();
        ensure!(
            fields.len() == 3,
            "idx line {number}: expected three fields"
        );
        let chapter = positive_number(fields[0], "chapter", number)?;
        let offset = fields[1]
            .parse::<u64>()
            .with_context(|| format!("idx line {number}: bad offset"))?;
        let verses = fields[2]
            .parse::<u16>()
            .with_context(|| format!("idx line {number}: bad verse count"))?;
        chapters.push(ChapterIndex {
            chapter,
            offset,
            verses,
        });
    }
    Ok(chapters)
}

/// One record of a chapter: a heading, or a verse whose label is `3` or
/// `3-4`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChapterItem {
    Heading(String),
    Verse {
        label: String,
        paragraph: bool,
        text: String,
    },
}

/// Read one chapter of a book. The `.idx` offset is used when it lands on
/// the chapter's `C` record; otherwise the book file is scanned.
pub fn load_chapter(
    root: &Path,
    translation: &str,
    book: &BibleBook,
    chapter: u16,
) -> Result<Vec<ChapterItem>> {
    ensure!(
        is_single_component(translation),
        "translation code must be one normal path component"
    );
    ensure!(
        chapter > 0 && chapter <= book.chapters,
        "chapter outside the book"
    );
    let folder = root.join(translation);
    let text_path = folder.join(&book.file);
    let mut file = File::open(&text_path)
        .with_context(|| format!("opening Bible text {}", text_path.display()))?;
    let offset = fs::read_to_string(folder.join(index_file_name(&book.file)))
        .ok()
        .and_then(|text| parse_chapter_index(&text).ok())
        .and_then(|entries| entries.into_iter().find(|entry| entry.chapter == chapter))
        .map(|entry| entry.offset);
    if let Some(offset) = offset {
        file.seek(SeekFrom::Start(offset))
            .with_context(|| format!("seeking {}", text_path.display()))?;
        let mut reader = BufReader::new(&mut file);
        let mut first = String::new();
        reader
            .read_line(&mut first)
            .with_context(|| format!("{}: reading", text_path.display()))?;
        if clean_line(&first) == format!("C\t{chapter}") {
            let start = offset + first.len() as u64;
            return collect_records(reader, &text_path, start);
        }
        file.seek(SeekFrom::Start(0))
            .with_context(|| format!("rewinding {}", text_path.display()))?;
    }
    scan_chapter(BufReader::new(file), chapter, &text_path)
}

/// Find the `C` record of `chapter` by reading from the start of the file.
fn scan_chapter(mut reader: impl BufRead, chapter: u16, path: &Path) -> Result<Vec<ChapterItem>> {
    let wanted = format!("C\t{chapter}");
    let mut line = String::new();
    let mut offset: u64 = 0;
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .with_context(|| format!("{}: reading", path.display()))?;
        if read == 0 {
            return Ok(Vec::new());
        }
        offset += read as u64;
        if clean_line(&line) == wanted {
            return collect_records(reader, path, offset);
        }
    }
}

/// Records after the `C` line, up to the next `C` line or the end. `start` is
/// the absolute byte offset of the first record, so errors name a position in
/// the file on both the indexed and the scanned path.
fn collect_records(mut reader: impl BufRead, path: &Path, start: u64) -> Result<Vec<ChapterItem>> {
    let mut items = Vec::new();
    let mut line = String::new();
    let mut offset = start;
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .with_context(|| format!("{}: reading", path.display()))?;
        if read == 0 {
            return Ok(items);
        }
        let record_offset = offset;
        offset += read as u64;
        let record = clean_line(&line);
        if record.is_empty() {
            continue;
        }
        if record.starts_with("C\t") {
            return Ok(items);
        }
        items.push(
            parse_record(record).with_context(|| {
                format!("{}: bad record at byte {record_offset}", path.display())
            })?,
        );
    }
}

fn parse_record(record: &str) -> Result<ChapterItem> {
    let fields: Vec<&str> = record.splitn(4, '\t').collect();
    match fields.as_slice() {
        ["H", text] => {
            ensure!(!text.is_empty(), "empty heading");
            Ok(ChapterItem::Heading((*text).into()))
        }
        ["V", label, paragraph, text] => {
            ensure!(!label.is_empty(), "missing verse label");
            ensure!(
                label
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'-'),
                "bad verse label"
            );
            ensure!(
                matches!(*paragraph, "0" | "1"),
                "paragraph flag must be 0 or 1"
            );
            ensure!(!text.is_empty(), "missing verse text");
            Ok(ChapterItem::Verse {
                label: (*label).into(),
                paragraph: *paragraph == "1",
                text: (*text).into(),
            })
        }
        _ => bail!("unknown record"),
    }
}

/// Folders of the Bible root that hold an `index.tsv`, sorted.
pub fn translations(root: &Path) -> io::Result<Vec<String>> {
    let mut codes = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.as_encoded_bytes().starts_with(b".") || !entry.file_type()?.is_dir() {
            continue;
        }
        match fs::metadata(entry.path().join("index.tsv")) {
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

/// `meta.txt`: `key=value` lines; the fields the Bible screens show.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranslationMeta {
    pub abbreviation: String,
    pub title: String,
    pub language: String,
}

pub fn parse_meta(text: &str) -> TranslationMeta {
    let mut meta = TranslationMeta::default();
    for raw in text.lines() {
        let Some((key, value)) = clean_line(raw).split_once('=') else {
            continue;
        };
        match key.trim() {
            "abbreviation" => meta.abbreviation = value.trim().into(),
            "title" => meta.title = value.trim().into(),
            "language" => meta.language = value.trim().into(),
            _ => {}
        }
    }
    meta
}

/// A verse reference from `VERSES.TXT`: a book number or a USFM code, a
/// chapter, and a verse or range.
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
        let book_text = fields.next().unwrap_or_default();
        let Some(book) = book_number_from_text(book_text) else {
            bail!("line {number}: unknown book {book_text}");
        };
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

/// Book number from `23` or from a USFM code such as `PSA`.
fn book_number_from_text(text: &str) -> Option<u8> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse::<u8>()
            .ok()
            .filter(|number| (1..=66).contains(number))
    } else {
        book_number(&text.to_ascii_uppercase())
    }
}

pub fn verse_of_the_day(list: &[VerseRef], epoch_day: u32) -> Option<&VerseRef> {
    if list.is_empty() {
        None
    } else {
        list.get((u64::from(epoch_day) % list.len() as u64) as usize)
    }
}

/// The plain text of one reference, for the sleep screen's verse mode.
/// Verses in a range are joined with a space.
pub fn reference_text(
    root: &Path,
    translation: &str,
    books: &[BibleBook],
    reference: &VerseRef,
) -> Result<String> {
    let book = books
        .iter()
        .find(|book| book.number == reference.book)
        .context("book missing from the index")?;
    let items = load_chapter(root, translation, book, reference.chapter)?;
    let parts: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            ChapterItem::Verse { label, text, .. } => verse_span(label)
                .filter(|(start, end)| *start <= reference.last && *end >= reference.first)
                .map(|_| text.as_str()),
            ChapterItem::Heading(_) => None,
        })
        .collect();
    ensure!(!parts.is_empty(), "verse not found in {}", book.usfm);
    Ok(parts.join(" "))
}

/// `3` gives (3, 3); `3-4` gives (3, 4).
fn verse_span(label: &str) -> Option<(u16, u16)> {
    match label.split_once('-') {
        Some((start, end)) => Some((start.parse().ok()?, end.parse().ok()?)),
        None => label.parse().ok().map(|number| (number, number)),
    }
}

/// The index of `JOB.txt` is `JOB.idx`.
fn index_file_name(text_file: &str) -> String {
    match text_file.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.idx"),
        None => format!("{text_file}.idx"),
    }
}

fn is_single_component(code: &str) -> bool {
    let mut components = Path::new(code).components();
    matches!(components.next(), Some(Component::Normal(name)) if name == code)
        && components.next().is_none()
        && !code.contains(['/', '\\', '\0'])
}

fn clean_line(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
        .trim_start_matches('\u{feff}')
        .trim()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("wave-bible-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const INDEX: &str =
        "PSA\tSalmos\t150\tPSA.txt\r\nGEN\tGénesis\t50\tGEN.txt\r\nXXX\tOther\t1\tXXX.txt\r\n";

    const PSA_TEXT: &str = "C\t1\r\nH\tLibro primero\r\nV\t1\t1\tBienaventurado el varón\r\nV\t2-3\t0\tSino en la ley\r\nC\t23\r\nH\tJehová es mi pastor\r\nV\t1\t1\tJehová es mi pastor; nada me faltará.\r\nV\t3-4\t0\tRestaura mi alma. Aunque ande.\r\nC\t24\r\nV\t1\t1\tDe Jehová es la tierra.\r\n";

    fn write_fixture(root: &Path, with_index: bool) -> PathBuf {
        let folder = root.join("RVR1960");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("index.tsv"), INDEX).unwrap();
        fs::write(
            folder.join("meta.txt"),
            "format=1\nabbreviation=RVR1960\ntitle=Reina-Valera 1960\nlanguage=es\n",
        )
        .unwrap();
        fs::write(folder.join("PSA.txt"), PSA_TEXT).unwrap();
        if with_index {
            let c23 = PSA_TEXT.find("C\t23").unwrap();
            let c24 = PSA_TEXT.find("C\t24").unwrap();
            fs::write(
                folder.join("PSA.idx"),
                format!("1\t0\t3\n23\t{c23}\t2\n24\t{c24}\t1\n"),
            )
            .unwrap();
        }
        folder
    }

    fn psa(root: &Path) -> BibleBook {
        parse_index(&fs::read_to_string(root.join("RVR1960/index.tsv")).unwrap())
            .unwrap()
            .into_iter()
            .find(|book| book.usfm == "PSA")
            .unwrap()
    }

    #[test]
    fn index_maps_canonical_order_and_skips_unknown_codes() {
        let books = parse_index(INDEX).unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].usfm, "GEN");
        assert_eq!(books[0].number, 1);
        assert_eq!(books[1].usfm, "PSA");
        assert_eq!(books[1].number, 19);
        assert_eq!(books[1].chapters, 150);
        assert_eq!(books[1].file, "PSA.txt");
    }

    #[test]
    fn index_rejects_duplicates_and_bad_fields_with_line_numbers() {
        let duplicate = "GEN\tGénesis\t50\tGEN.txt\nGEN\tGénesis\t50\tGEN.txt\n";
        assert!(parse_index(duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate"));
        let missing = "GEN\tGénesis\t50\n";
        assert!(parse_index(missing)
            .unwrap_err()
            .to_string()
            .contains("line 1"));
        let zero = "GEN\tGénesis\t0\tGEN.txt\n";
        assert!(parse_index(zero)
            .unwrap_err()
            .to_string()
            .contains("zero chapters"));
    }

    #[test]
    fn short_names_keep_leading_numbers() {
        assert_eq!(short_name("Salmos"), "Sal");
        assert_eq!(short_name("Génesis"), "Gén");
        assert_eq!(short_name("1 Corintios"), "1 Cor");
    }

    #[test]
    fn usfm_codes_map_to_canonical_numbers() {
        assert_eq!(book_number("GEN"), Some(1));
        assert_eq!(book_number("PSA"), Some(19));
        assert_eq!(book_number("REV"), Some(66));
        assert_eq!(book_number("XXX"), None);
    }

    #[test]
    fn chapter_is_read_through_the_index_offset() {
        let temp = TempRoot::new("indexed");
        write_fixture(&temp.0, true);
        let items = load_chapter(&temp.0, "RVR1960", &psa(&temp.0), 23).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ChapterItem::Heading("Jehová es mi pastor".into()));
        assert_eq!(
            items[2],
            ChapterItem::Verse {
                label: "3-4".into(),
                paragraph: false,
                text: "Restaura mi alma. Aunque ande.".into(),
            }
        );
    }

    #[test]
    fn chapter_is_scanned_without_an_index_and_stops_at_next_chapter() {
        let temp = TempRoot::new("scanned");
        write_fixture(&temp.0, false);
        let first = load_chapter(&temp.0, "RVR1960", &psa(&temp.0), 1).unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(first[0], ChapterItem::Heading("Libro primero".into()));
        let last = load_chapter(&temp.0, "RVR1960", &psa(&temp.0), 24).unwrap();
        assert_eq!(last.len(), 1);
    }

    #[test]
    fn stale_offset_falls_back_to_a_scan() {
        let temp = TempRoot::new("stale");
        let folder = write_fixture(&temp.0, false);
        fs::write(folder.join("PSA.idx"), "23\t3\t2\n").unwrap();
        let items = load_chapter(&temp.0, "RVR1960", &psa(&temp.0), 23).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn chapter_outside_the_book_is_rejected_before_io() {
        let temp = TempRoot::new("range");
        write_fixture(&temp.0, false);
        let book = psa(&temp.0);
        assert!(load_chapter(&temp.0, "RVR1960", &book, 0).is_err());
        assert!(load_chapter(&temp.0, "RVR1960", &book, 151).is_err());
    }

    #[test]
    fn malformed_record_names_its_file_and_line() {
        let temp = TempRoot::new("malformed");
        let folder = write_fixture(&temp.0, false);
        fs::write(folder.join("PSA.txt"), "C\t1\nV\t1\tbad\n").unwrap();
        let error = load_chapter(&temp.0, "RVR1960", &psa(&temp.0), 1).unwrap_err();
        let text = format!("{error:#}");
        assert!(text.contains("PSA.txt"), "{text}");
        // The bad record starts after the 4-byte "C\t1\n" line.
        assert!(text.contains("at byte 4"), "{text}");
    }

    #[test]
    fn translation_code_must_be_one_path_component() {
        let temp = TempRoot::new("traversal");
        write_fixture(&temp.0, false);
        assert!(load_chapter(&temp.0, "../RVR1960", &psa(&temp.0), 1).is_err());
    }

    #[test]
    fn translations_list_folders_with_an_index() {
        let temp = TempRoot::new("list");
        write_fixture(&temp.0, false);
        fs::create_dir_all(temp.0.join("EMPTY")).unwrap();
        fs::create_dir_all(temp.0.join(".hidden")).unwrap();
        assert_eq!(translations(&temp.0).unwrap(), ["RVR1960"]);
    }

    #[test]
    fn meta_reads_title_language_and_abbreviation() {
        let meta = parse_meta(
            "format=1\r\nabbreviation=RVR1960\r\ntitle=Reina-Valera 1960\r\nlanguage=es\r\n",
        );
        assert_eq!(meta.abbreviation, "RVR1960");
        assert_eq!(meta.title, "Reina-Valera 1960");
        assert_eq!(meta.language, "es");
    }

    #[test]
    fn verses_accept_book_numbers_and_usfm_codes() {
        let list = parse_verse_list("19 23:1\nPSA 23:1-3\n# note\n").unwrap();
        assert_eq!(
            list[0],
            VerseRef {
                book: 19,
                chapter: 23,
                first: 1,
                last: 1
            }
        );
        assert_eq!(
            list[1],
            VerseRef {
                book: 19,
                chapter: 23,
                first: 1,
                last: 3
            }
        );
        assert!(parse_verse_list("ZZZ 1:1").is_err());
        assert!(parse_verse_list("19 23:3-1").is_err());
    }

    #[test]
    fn verse_of_the_day_rotates_through_the_list() {
        let list = parse_verse_list("19 23:1\n43 3:16\n").unwrap();
        assert_eq!(verse_of_the_day(&list, 0).map(|r| r.book), Some(19));
        assert_eq!(verse_of_the_day(&list, 1).map(|r| r.book), Some(43));
        assert_eq!(verse_of_the_day(&list, 2).map(|r| r.book), Some(19));
        assert_eq!(verse_of_the_day(&[], 5), None);
    }

    #[test]
    fn reference_text_joins_a_range_from_the_chapter() {
        let temp = TempRoot::new("reference");
        write_fixture(&temp.0, false);
        let book = psa(&temp.0);
        let reference = VerseRef {
            book: 19,
            chapter: 23,
            first: 1,
            last: 3,
        };
        let text =
            reference_text(&temp.0, "RVR1960", std::slice::from_ref(&book), &reference).unwrap();
        assert!(text.starts_with("Jehová es mi pastor;"));
        assert!(text.ends_with("Aunque ande."));
    }

    #[test]
    fn testament_split_is_at_book_forty() {
        assert_eq!(Testament::from_book_number(39), Some(Testament::Old));
        assert_eq!(Testament::from_book_number(40), Some(Testament::New));
        assert_eq!(Testament::from_book_number(67), None);
    }

    /// Reads a folder the script wrote from the real JSON. Run locally with
    /// `BIBLE_FOLDER=/path/to/bible/<abbr> cargo test -- --ignored`; the text is
    /// never stored in the repository.
    #[test]
    #[ignore = "needs BIBLE_FOLDER pointing at a script output folder"]
    fn real_script_output_reads_through_the_device_path() {
        let folder =
            std::path::PathBuf::from(std::env::var("BIBLE_FOLDER").expect("set BIBLE_FOLDER"));
        let text = fs::read_to_string(folder.join("index.tsv")).unwrap();
        let books = parse_index(&text).unwrap();
        assert_eq!(books.len(), 66, "every canonical book is on the index");
        // The reader joins root/translation, so the root is the folder's parent.
        let root = folder.parent().unwrap().to_path_buf();
        let translation = folder.file_name().unwrap().to_string_lossy().into_owned();
        let genesis = &books[0];
        // The script writes a chapter heading before verse 1.
        let first = load_chapter(&root, &translation, genesis, 1).unwrap();
        assert!(matches!(first.first(), Some(ChapterItem::Heading(_))));
        assert!(first.iter().any(|item| matches!(
            item,
            ChapterItem::Verse { label, .. } if label == "1"
        )));
        let psalms = books.iter().find(|book| book.usfm == "PSA").unwrap();
        let twenty_three = load_chapter(&root, &translation, psalms, 23).unwrap();
        assert!(twenty_three
            .iter()
            .any(|item| matches!(item, ChapterItem::Heading(_))));
        assert!(twenty_three
            .iter()
            .any(|item| matches!(item, ChapterItem::Verse { .. })));
    }
}
