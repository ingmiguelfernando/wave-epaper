//! Offline Reader state, TXT / EPUB pagination and Reader-owned persistence.
//!
//! v0.17.1 adds chapter-aware EPUB page labels, persistent chapter-aware EPUB
//! bookmark labels and OPF-title Library rows while preserving the accepted TXT
//! Reader, FAT 8.3 persistence, per-book resume and staged loading architecture.
// rustmix-wave=epub-watchdog-memory-pressure-repair-ready

use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Instant, UNIX_EPOCH},
};

use crate::{
    app::{reader_typography::reader_body_style, typography::UiTextStyle},
    buttons::ButtonEvent,
    charset::glyph_index,
    epub::{
        open_epub_on_worker, read_epub_title_on_worker, EpubChapter, EpubDocument, EpubTocEntry,
        EPUB_SPINE_LIMIT, EPUB_TEXT_VERSION, EPUB_TOC_LIMIT,
    },
    framebuffer::{HEIGHT, WIDTH},
    hyphenation::{break_offsets, Language},
    watchdog::Pacer,
};

/// SD-card library owned by the Reader subsystem.
pub const READER_BOOKS_DIRECTORY: &str = "/sdcard/RUSTMIX/BOOKS";
/// SD-card state directory owned by the Reader subsystem.
pub const READER_STATE_DIRECTORY: &str = "/sdcard/RUSTMIX/READER";
/// Persistent last-read state file.
pub const READER_STATE_FILE: &str = "STATE.TXT";
/// Persistent per-book last-position map.
pub const READER_POSITIONS_FILE: &str = "POSITS.TXT";
/// Legacy long-name per-book positions file accepted read-only for migration.
pub const LEGACY_READER_POSITIONS_FILE: &str = "POSITIONS.TXT";
/// Persistent recent-book list.
pub const READER_RECENT_FILE: &str = "RECENT.TXT";
/// Persistent bookmark list.
pub const READER_BOOKMARKS_FILE: &str = "MARKS.TXT";
/// Persistent Reader-specific preferences.
pub const READER_PREFS_FILE: &str = "PREFS.TXT";
/// SD-backed TXT anchor-cache directory.
pub const READER_CACHE_DIRECTORY: &str = "CACHE";
/// Number of text lines rendered on one portrait Reader page.
pub const READER_LINES_PER_PAGE: usize = 22;
/// Maximum wrapped characters per line for the current Reader body profile.
pub const READER_CHARS_PER_LINE: usize = 43;
/// Nearby page cache retained in RAM while one book is open.
pub const READER_NEARBY_PAGE_CACHE: usize = 8;
/// Maximum bytes read while generating a single page.
pub const READER_PAGE_READ_BYTES: usize = 16 * 1024;
/// Maximum library rows retained for the embedded product UI.
pub const READER_LIBRARY_LIMIT: usize = 128;
/// Maximum per-book last-position records retained on removable storage.
pub const READER_POSITION_LIMIT: usize = 64;
/// Maximum recent-book records retained on removable storage.
pub const READER_RECENT_LIMIT: usize = 16;
/// Maximum bookmark records retained on removable storage.
pub const READER_BOOKMARK_LIMIT: usize = 128;
/// Maximum page anchors accepted from one SD-backed cache file.
pub const READER_CACHE_OFFSET_LIMIT: usize = 4096;
/// Persist an anchor-cache checkpoint after this many newly indexed pages.
pub const READER_CACHE_CHECKPOINT_PAGES: usize = 4;
/// Maximum page anchors generated for one EPUB chapter.
pub const READER_EPUB_PAGE_ANCHOR_LIMIT: usize = 8192;
/// Horizontal inset of Reader body text from each screen edge.
pub const READER_BODY_INSET: i32 = 24;

const READER_PERSISTENCE_VERSION: &str = "1";
const READER_CACHE_VERSION: &str = "4";
const READER_EPUB_INDEX_VERSION: &str = "2";
const READER_PREFS_VERSION: &str = "1";
const CACHE_FNV_OFFSET: u64 = 0xcbf29ce484222325;
const CACHE_FNV_PRIME: u64 = 0x100000001b3;

/// Reader-supported content types. TXT and bounded reflowable EPUB are active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookFormat {
    Text,
    Epub,
}

impl BookFormat {
    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::Text => "TXT",
            Self::Epub => "EPUB",
        }
    }

    #[must_use]
    const fn marker(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Epub => "epub",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "txt" => Some(Self::Text),
            "epub" => Some(Self::Epub),
            _ => None,
        }
    }
}

/// One Reader library row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderBook {
    pub path: String,
    pub title: String,
    pub format: BookFormat,
    pub size_bytes: u64,
    pub modified_seconds: u64,
}

/// Chapter-relative EPUB page presentation retained with bookmarks so MARKS.TXT
/// remains useful after restart and before the matching book is reopened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderChapterPageLabel {
    pub chapter_number: usize,
    pub page_number: usize,
    pub page_count: usize,
}

impl ReaderChapterPageLabel {
    #[must_use]
    pub fn page_text(&self) -> String {
        format!("{}/{}", self.page_number, self.page_count)
    }
}

/// Stable logical reading position used by STATE.TXT, RECENT.TXT and
/// MARKS.TXT. TXT byte offsets remain valid independently of generated UI page
/// labels. EPUB reuses this byte-offset boundary against its flattened text buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderLocation {
    pub path: String,
    pub title: String,
    pub format: BookFormat,
    pub size_bytes: u64,
    pub modified_seconds: u64,
    pub page_index: usize,
    pub byte_offset: u64,
    pub epub_chapter: Option<ReaderChapterPageLabel>,
}

impl ReaderLocation {
    #[must_use]
    pub fn as_book(&self) -> ReaderBook {
        ReaderBook {
            path: self.path.clone(),
            title: self.title.clone(),
            format: self.format,
            size_bytes: self.size_bytes,
            modified_seconds: self.modified_seconds,
        }
    }

    #[must_use]
    fn matches_book(&self, book: &ReaderBook) -> bool {
        self.size_bytes == book.size_bytes
            && self.modified_seconds == book.modified_seconds
            && self.format == book.format
            && (self.path == book.path || renamed_in_place(&self.path, book))
    }

    /// Same book as `other`, including an older 8.3 spelling of its path.
    #[must_use]
    fn same_book(&self, other: &Self) -> bool {
        self.path == other.path || self.matches_book(&other.as_book())
    }

    #[must_use]
    fn same_position(&self, other: &Self) -> bool {
        self.path == other.path && self.byte_offset == other.byte_offset
    }

    /// Page label in Reader status-bar style; chapter-relative for EPUB.
    #[must_use]
    pub fn display_page_label(&self) -> String {
        let Some(chapter) = self.epub_chapter.as_ref() else {
            return format!("PAGE {}", self.page_index + 1);
        };
        let number = chapter.chapter_number;
        format!("CH {number}  PAGE {}", chapter.page_text())
    }
}

/// One list row rendered by Recent, Books, Files or Bookmarks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderLibraryEntry {
    pub book: ReaderBook,
    pub location: Option<ReaderLocation>,
}

/// Reader Library tab model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReaderLibraryTab {
    Recent,
    #[default]
    Books,
    Files,
    Bookmarks,
}

impl ReaderLibraryTab {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Books => "Books",
            Self::Files => "Files",
            Self::Bookmarks => "Bookmarks",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Recent => Self::Books,
            Self::Books => Self::Files,
            Self::Files => Self::Bookmarks,
            Self::Bookmarks => Self::Recent,
        }
    }
}

/// Text decoding mode detected when a TXT book is opened.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Windows1252,
}

impl TextEncoding {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Windows1252 => "WIN-1252",
        }
    }
}

/// E-paper-friendly Reader page theme.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReadingTheme {
    #[default]
    Classic,
    HighContrast,
}

impl ReadingTheme {
    pub const ALL: [Self; 2] = [Self::Classic, Self::HighContrast];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::HighContrast => "High Contrast",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::HighContrast => "high-contrast",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Classic => Self::HighContrast,
            Self::HighContrast => Self::Classic,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        self.next()
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "classic" => Ok(Self::Classic),
            "high-contrast" | "high_contrast" => Ok(Self::HighContrast),
            other => Err(format!("unsupported theme value {other:?}")),
        }
    }
}

/// Reader-page orientation independent from the portrait system UI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReaderOrientation {
    #[default]
    Portrait,
    Landscape,
}

impl ReaderOrientation {
    pub const ALL: [Self; 2] = [Self::Portrait, Self::Landscape];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Portrait => "Portrait",
            Self::Landscape => "Landscape",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Portrait => Self::Landscape,
            Self::Landscape => Self::Portrait,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        self.next()
    }

    /// Logical screen width of Reader pages in this orientation.
    #[must_use]
    pub const fn screen_width(self) -> i32 {
        match self {
            Self::Portrait => HEIGHT as i32,
            Self::Landscape => WIDTH as i32,
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "portrait" => Ok(Self::Portrait),
            "landscape" => Ok(Self::Landscape),
            other => Err(format!("unsupported orientation value {other:?}")),
        }
    }
}

/// Reader-specific book font size. This is intentionally independent from
/// `/sdcard/RUSTMIX/DISPLAY.TXT`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BookFontSize {
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl BookFontSize {
    pub const ALL: [Self; 4] = [Self::Small, Self::Medium, Self::Large, Self::XLarge];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::XLarge => "XLarge",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::XLarge => "xlarge",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            Self::Large => Self::XLarge,
            Self::XLarge => Self::Small,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Small => Self::XLarge,
            Self::Medium => Self::Small,
            Self::Large => Self::Medium,
            Self::XLarge => Self::Large,
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "xlarge" | "extra-large" | "extra_large" => Ok(Self::XLarge),
            other => Err(format!("unsupported book_font_size value {other:?}")),
        }
    }
}

/// Reader-specific body font family. Reader-only generated bitmap strikes
/// cover the shared `charset`; raw font files are not distributed. Persisted
/// `serif` and `atkinson-hyperlegible` keys remain stable for compatibility.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BookFont {
    Inter,
    AtkinsonHyperlegible,
    #[default]
    Serif,
    Literata,
}

impl BookFont {
    pub const ALL: [Self; 4] = [
        Self::Inter,
        Self::AtkinsonHyperlegible,
        Self::Serif,
        Self::Literata,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::AtkinsonHyperlegible => "Atkinson",
            Self::Serif => "Serif",
            Self::Literata => "Literata",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Inter => "inter",
            Self::AtkinsonHyperlegible => "atkinson-hyperlegible",
            Self::Serif => "serif",
            Self::Literata => "literata",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Inter => Self::AtkinsonHyperlegible,
            Self::AtkinsonHyperlegible => Self::Serif,
            Self::Serif => Self::Literata,
            Self::Literata => Self::Inter,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Inter => Self::Literata,
            Self::AtkinsonHyperlegible => Self::Inter,
            Self::Serif => Self::AtkinsonHyperlegible,
            Self::Literata => Self::Serif,
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "inter" => Ok(Self::Inter),
            "atkinson" | "atkinson-hyperlegible" | "atkinson_hyperlegible" => {
                Ok(Self::AtkinsonHyperlegible)
            }
            "serif" | "dejavu-serif" => Ok(Self::Serif),
            "literata" => Ok(Self::Literata),
            other => Err(format!("unsupported book_font value {other:?}")),
        }
    }
}

/// Reader paragraph alignment. Justified is the default e-book presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParagraphAlignment {
    #[default]
    Justified,
    Left,
    Center,
    Right,
}

impl ParagraphAlignment {
    pub const ALL: [Self; 4] = [Self::Justified, Self::Left, Self::Center, Self::Right];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Justified => "Justified",
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Justified => "justified",
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Justified => Self::Left,
            Self::Left => Self::Center,
            Self::Center => Self::Right,
            Self::Right => Self::Justified,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Justified => Self::Right,
            Self::Left => Self::Justified,
            Self::Center => Self::Left,
            Self::Right => Self::Center,
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "justified" | "justify" => Ok(Self::Justified),
            "left" => Ok(Self::Left),
            "center" | "centred" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            other => Err(format!("unsupported paragraph_alignment value {other:?}")),
        }
    }
}

/// Layout dimensions affecting pagination and cache fingerprints. Lines wrap
/// by pixel width; `chars_per_line` only sizes the text window read per page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReaderLayout {
    pub chars_per_line: usize,
    pub lines_per_page: usize,
    pub orientation: ReaderOrientation,
    pub font_size: BookFontSize,
    pub book_font: BookFont,
    pub paragraph_alignment: ParagraphAlignment,
}

impl ReaderLayout {
    /// Width of one body line in pixels. The page renderer insets text by the
    /// same amount, so lines wrap exactly where they are drawn.
    #[must_use]
    pub const fn line_width(self) -> i32 {
        self.orientation.screen_width() - 2 * READER_BODY_INSET
    }
}

/// Reader-owned preference file persisted as `/RUSTMIX/READER/PREFS.TXT`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReaderPreferences {
    pub theme: ReadingTheme,
    pub orientation: ReaderOrientation,
    pub font_size: BookFontSize,
    pub book_font: BookFont,
    pub paragraph_alignment: ParagraphAlignment,
    pub show_progress: bool,
}

impl Default for ReaderPreferences {
    fn default() -> Self {
        Self {
            theme: ReadingTheme::Classic,
            orientation: ReaderOrientation::Portrait,
            font_size: BookFontSize::Medium,
            book_font: BookFont::Serif,
            paragraph_alignment: ParagraphAlignment::Justified,
            show_progress: true,
        }
    }
}

impl ReaderPreferences {
    #[must_use]
    pub const fn layout(self) -> ReaderLayout {
        // Reader pages share one bounded body viewport across Classic and
        // High Contrast. Lines wrap by pixel width; the character budgets only
        // bound how much text is read for one page.
        let (chars_per_line, lines_per_page) =
            match (self.orientation, self.font_size, self.book_font) {
                (
                    ReaderOrientation::Portrait,
                    BookFontSize::Small,
                    BookFont::Serif | BookFont::Literata,
                ) => (39, 25),
                (
                    ReaderOrientation::Portrait,
                    BookFontSize::Medium,
                    BookFont::Serif | BookFont::Literata,
                ) => (35, 22),
                (
                    ReaderOrientation::Portrait,
                    BookFontSize::Large,
                    BookFont::Serif | BookFont::Literata,
                ) => (30, 19),
                (
                    ReaderOrientation::Portrait,
                    BookFontSize::XLarge,
                    BookFont::Serif | BookFont::Literata,
                ) => (25, 16),
                (ReaderOrientation::Portrait, BookFontSize::Small, _) => (43, 25),
                (ReaderOrientation::Portrait, BookFontSize::Medium, _) => (38, 22),
                (ReaderOrientation::Portrait, BookFontSize::Large, _) => (33, 19),
                (ReaderOrientation::Portrait, BookFontSize::XLarge, _) => (27, 16),
                (
                    ReaderOrientation::Landscape,
                    BookFontSize::Small,
                    BookFont::Serif | BookFont::Literata,
                ) => (68, 13),
                (
                    ReaderOrientation::Landscape,
                    BookFontSize::Medium,
                    BookFont::Serif | BookFont::Literata,
                ) => (58, 11),
                (
                    ReaderOrientation::Landscape,
                    BookFontSize::Large,
                    BookFont::Serif | BookFont::Literata,
                ) => (49, 10),
                (
                    ReaderOrientation::Landscape,
                    BookFontSize::XLarge,
                    BookFont::Serif | BookFont::Literata,
                ) => (41, 8),
                (ReaderOrientation::Landscape, BookFontSize::Small, _) => (72, 13),
                (ReaderOrientation::Landscape, BookFontSize::Medium, _) => (64, 11),
                (ReaderOrientation::Landscape, BookFontSize::Large, _) => (55, 10),
                (ReaderOrientation::Landscape, BookFontSize::XLarge, _) => (45, 8),
            };
        ReaderLayout {
            chars_per_line,
            lines_per_page,
            orientation: self.orientation,
            font_size: self.font_size,
            book_font: self.book_font,
            paragraph_alignment: self.paragraph_alignment,
        }
    }

    #[must_use]
    pub fn serialized(self) -> String {
        let show_progress = if self.show_progress { "true" } else { "false" };
        format!(
            "version={}\ntheme={}\norientation={}\nfont_size={}\nbook_font={}\nparagraph_alignment={}\nshow_progress={}\n",
            READER_PREFS_VERSION,
            self.theme.marker(),
            self.orientation.marker(),
            self.font_size.marker(),
            self.book_font.marker(),
            self.paragraph_alignment.marker(),
            show_progress,
        )
    }

    fn parse(text: &str) -> Result<Self, String> {
        let mut prefs = Self::default();
        let mut version = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| "Reader preference line must contain '='".to_string())?;
            match key.trim() {
                "version" => version = Some(value.trim().to_string()),
                "theme" => prefs.theme = ReadingTheme::parse(value)?,
                "orientation" => prefs.orientation = ReaderOrientation::parse(value)?,
                "font_size" => prefs.font_size = BookFontSize::parse(value)?,
                "book_font" => prefs.book_font = BookFont::parse(value)?,
                "paragraph_alignment" => {
                    prefs.paragraph_alignment = ParagraphAlignment::parse(value)?
                }
                "show_progress" => {
                    prefs.show_progress = match value.trim() {
                        "true" => true,
                        "false" => false,
                        _ => return Err("show_progress must be true or false".into()),
                    }
                }
                other => return Err(format!("unsupported Reader preference key {other:?}")),
            }
        }
        if version.as_deref() != Some(READER_PREFS_VERSION) {
            return Err("unsupported Reader preference version".into());
        }
        Ok(prefs)
    }
}

/// Stages shown by the e-paper loading screen. A book opens within a single
/// tick: each loading-screen refresh costs about as much as the work itself,
/// so the screen is drawn once and the next refresh shows the page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderLoadingStage {
    OpeningFile,
    UpdatingLayout,
    Failed,
}

impl ReaderLoadingStage {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpeningFile => "Opening file",
            Self::UpdatingLayout => "Updating layout cache",
            Self::Failed => "Unable to open book",
        }
    }

    #[must_use]
    pub const fn progress(self) -> u8 {
        match self {
            Self::OpeningFile => 10,
            Self::UpdatingLayout => 45,
            Self::Failed => 100,
        }
    }
}

/// Pending staged book open retained while the loading screen is visible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingReaderOpen {
    pub book: ReaderBook,
    pub stage: ReaderLoadingStage,
    pub encoding: Option<TextEncoding>,
    pub epub_document: Option<EpubDocument>,
    pub resume: Option<ReaderLocation>,
    pub message: String,
}

/// One wrapped Reader line. `paragraph_end` prevents Justified rendering from
/// stretching the final line of a paragraph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderPageLine {
    pub text: String,
    pub paragraph_end: bool,
}

/// One cached portrait page and its byte anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderCachedPage {
    /// Absolute book-page index, independent of a cache-recovery base offset.
    pub page_index: usize,
    pub byte_offset: u64,
    pub next_byte_offset: u64,
    pub lines: Vec<ReaderPageLine>,
}

/// SD-backed page-anchor cache. The cache is intentionally text-based and
/// bounded so corrupt records can be rejected without blocking book opening.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ReaderAnchorCache {
    fingerprint: u64,
    base_page: usize,
    offsets: Vec<u64>,
    indexed_through: u64,
    complete: bool,
}

/// Layout-specific page anchors of the open EPUB chapter, rebuilt whenever a
/// chapter opens or the Reader layout changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderEpubChapterPages {
    pub chapter_number: usize,
    pub text_offset: u64,
    pub text_end_offset: u64,
    pub page_offsets: Vec<u64>,
}

/// Text of the one EPUB chapter whose pages are open; the rest of the book
/// stays in the archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedEpubChapter {
    /// Index into `EpubDocument::chapters`.
    pub index: usize,
    pub text_offset: u64,
    pub text: String,
}

impl LoadedEpubChapter {
    fn end_offset(&self) -> u64 {
        self.text_offset + self.text.len() as u64
    }
}

/// Where to land when an EPUB chapter opens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EpubPageTarget {
    FirstPage,
    LastPage,
    Offset(u64),
}

/// Active Reader session. Generated page anchors and nearby rendered pages remain
/// bounded in RAM and are rebuilt lazily when the reader advances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderSession {
    pub book: ReaderBook,
    pub encoding: TextEncoding,
    pub epub_document: Option<EpubDocument>,
    pub layout: ReaderLayout,
    /// Hyphenation language; `None` breaks lines at spaces and hyphens only.
    pub language: Option<Language>,
    /// Local index within page_offsets.
    pub current_page: usize,
    /// Absolute page index represented by page_offsets[0]. Normally zero. A
    /// non-zero value is allowed when STATE.TXT survives but a cache is absent.
    pub page_number_base: usize,
    pub page_offsets: Vec<u64>,
    pub indexed_through: u64,
    pub index_complete: bool,
    pub cache: Vec<ReaderCachedPage>,
    pub epub_chapter_pages: Vec<ReaderEpubChapterPages>,
    pub epub_chapter: Option<LoadedEpubChapter>,
}

impl ReaderSession {
    #[must_use]
    pub fn current_absolute_page(&self) -> usize {
        self.page_number_base.saturating_add(self.current_page)
    }

    #[must_use]
    pub fn source_size_bytes(&self) -> u64 {
        self.epub_document
            .as_ref()
            .map_or(self.book.size_bytes, EpubDocument::text_size_bytes)
    }

    #[must_use]
    pub fn content_badge(&self) -> &'static str {
        self.book.format.badge()
    }

    #[must_use]
    pub fn toc_entries(&self) -> &[EpubTocEntry] {
        self.epub_document
            .as_ref()
            .map_or(&[], |document| document.toc.as_slice())
    }

    #[must_use]
    pub fn current_cached_page(&self) -> Option<&ReaderCachedPage> {
        let absolute = self.current_absolute_page();
        self.cache.iter().find(|page| page.page_index == absolute)
    }

    #[must_use]
    pub fn current_location(&self) -> ReaderLocation {
        let byte_offset = self
            .page_offsets
            .get(self.current_page)
            .copied()
            .or_else(|| self.current_cached_page().map(|page| page.byte_offset))
            .unwrap_or(0);
        ReaderLocation {
            path: self.book.path.clone(),
            title: self.book.title.clone(),
            format: self.book.format,
            size_bytes: self.book.size_bytes,
            modified_seconds: self.book.modified_seconds,
            page_index: self.current_absolute_page(),
            byte_offset,
            epub_chapter: self.epub_chapter_page_label_for_offset(byte_offset),
        }
    }

    #[must_use]
    pub fn progress_percent(&self) -> u8 {
        let source_size = self.source_size_bytes();
        if source_size == 0 {
            return 100;
        }
        ((self.indexed_through.saturating_mul(100) / source_size).min(100)) as u8
    }

    #[must_use]
    pub fn page_label(&self) -> String {
        if self.index_complete {
            format!(
                "{}/{}",
                self.current_absolute_page() + 1,
                self.page_number_base + self.page_offsets.len()
            )
        } else {
            format!("{}+", self.current_absolute_page() + 1)
        }
    }

    /// Product-facing page label. TXT keeps the accepted book-relative label;
    /// EPUB uses a chapter-relative label as requested by the Reader UI.
    #[must_use]
    pub fn display_page_label(&self) -> String {
        self.current_epub_chapter_page_label().map_or_else(
            || format!("PAGE {}", self.page_label()),
            |chapter| {
                format!(
                    "CH {}  PAGE {}",
                    chapter.chapter_number,
                    chapter.page_text()
                )
            },
        )
    }

    #[must_use]
    pub fn current_epub_chapter_page_label(&self) -> Option<ReaderChapterPageLabel> {
        let offset = self
            .page_offsets
            .get(self.current_page)
            .copied()
            .or_else(|| self.current_cached_page().map(|page| page.byte_offset))?;
        self.epub_chapter_page_label_for_offset(offset)
    }

    #[must_use]
    pub fn epub_chapter_page_label_for_offset(
        &self,
        offset: u64,
    ) -> Option<ReaderChapterPageLabel> {
        let chapter = self.epub_chapter_pages.iter().find(|chapter| {
            offset >= chapter.text_offset
                && (offset < chapter.text_end_offset
                    || (offset == chapter.text_end_offset
                        && chapter.text_end_offset == self.source_size_bytes()))
        })?;
        let page_number = chapter
            .page_offsets
            .partition_point(|anchor| *anchor <= offset)
            .max(1);
        Some(ReaderChapterPageLabel {
            chapter_number: chapter.chapter_number,
            page_number,
            page_count: chapter.page_offsets.len().max(1),
        })
    }

    fn push_cached_page(&mut self, page: ReaderCachedPage) {
        if let Some(existing) = self
            .cache
            .iter_mut()
            .find(|cached| cached.page_index == page.page_index)
        {
            *existing = page;
            return;
        }
        self.cache.push(page);
        self.cache.sort_by_key(|page| page.page_index);
        while self.cache.len() > READER_NEARBY_PAGE_CACHE {
            let current = self.current_absolute_page();
            let remove = if current.saturating_sub(self.cache[0].page_index)
                > self
                    .cache
                    .last()
                    .map_or(0, |page| page.page_index.saturating_sub(current))
            {
                0
            } else {
                self.cache.len() - 1
            };
            self.cache.remove(remove);
        }
    }

    fn ensure_page_cached(&mut self, local_page_index: usize) -> Result<(), String> {
        let absolute = self.page_number_base.saturating_add(local_page_index);
        if self.cache.iter().any(|page| page.page_index == absolute) {
            return Ok(());
        }
        let offset = *self
            .page_offsets
            .get(local_page_index)
            .ok_or_else(|| "page anchor is not indexed yet".to_string())?;
        let page = self.read_page(offset, absolute)?;
        self.push_cached_page(page);
        Ok(())
    }

    fn index_one_page(&mut self) -> Result<bool, String> {
        if self.index_complete {
            return Ok(false);
        }
        let absolute_page = self
            .page_number_base
            .saturating_add(self.page_offsets.len());
        let offset = self.indexed_through;
        let source_size = self.source_size_bytes();
        if offset >= source_size {
            self.index_complete = true;
            return Ok(false);
        }
        let page = self.read_page(offset, absolute_page)?;
        if page.next_byte_offset <= offset {
            self.index_complete = true;
            return Ok(false);
        }
        self.page_offsets.push(offset);
        self.indexed_through = page.next_byte_offset;
        self.index_complete = self.indexed_through >= source_size;
        self.push_cached_page(page);
        Ok(true)
    }

    pub fn next_page(&mut self) -> Result<(), String> {
        let target = self.current_page.saturating_add(1);
        if let Some(index) = self.epub_chapter_index() {
            if target >= self.page_offsets.len() {
                if index + 1 < self.epub_chapter_count() {
                    return self.open_epub_chapter(index + 1, EpubPageTarget::FirstPage);
                }
                return Ok(());
            }
        }
        while target >= self.page_offsets.len() && !self.index_complete {
            self.index_one_page()?;
        }
        if target < self.page_offsets.len() {
            self.current_page = target;
            self.ensure_page_cached(target)?;
        }
        Ok(())
    }

    pub fn previous_page(&mut self) -> Result<(), String> {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.ensure_page_cached(self.current_page)?;
        } else if let Some(index) = self.epub_chapter_index().filter(|index| *index > 0) {
            self.open_epub_chapter(index - 1, EpubPageTarget::LastPage)?;
        }
        Ok(())
    }

    fn read_page(&self, offset: u64, index: usize) -> Result<ReaderCachedPage, String> {
        let setter = Typesetter::new(self.layout, self.language);
        match (&self.epub_chapter, self.book.format) {
            (Some(chapter), _) => read_epub_chapter_page(chapter, &setter, offset, index),
            (None, BookFormat::Text) => {
                read_txt_page(&self.book, self.encoding, &setter, offset, index)
            }
            (None, BookFormat::Epub) => Err("EPUB chapter is not loaded".into()),
        }
    }

    fn epub_chapter_index(&self) -> Option<usize> {
        self.epub_chapter.as_ref().map(|chapter| chapter.index)
    }

    fn epub_chapter_count(&self) -> usize {
        self.epub_document
            .as_ref()
            .map_or(0, |document| document.chapters.len())
    }

    /// Show the page holding `offset`, opening its chapter when needed.
    fn show_epub_offset(&mut self, offset: u64) -> Result<(), String> {
        let index = self
            .epub_document
            .as_ref()
            .ok_or_else(|| "EPUB document is unavailable".to_string())?
            .chapter_index_for_offset(offset);
        if self.epub_chapter_index() == Some(index) {
            self.current_page = page_containing(&self.page_offsets, offset);
            return self.ensure_page_cached(self.current_page);
        }
        self.open_epub_chapter(index, EpubPageTarget::Offset(offset))
    }

    /// Load one EPUB chapter, paginate it and show the page for `target`.
    fn open_epub_chapter(&mut self, index: usize, target: EpubPageTarget) -> Result<(), String> {
        let document = self
            .epub_document
            .as_ref()
            .ok_or_else(|| "EPUB document is unavailable".to_string())?;
        let chapter = document
            .chapters
            .get(index)
            .ok_or_else(|| "EPUB chapter is out of range".to_string())?;
        let mut pages = ReaderEpubChapterPages {
            chapter_number: chapter.number,
            text_offset: chapter.text_offset,
            text_end_offset: chapter.text_end_offset,
            page_offsets: Vec::new(),
        };
        let loaded = LoadedEpubChapter {
            index,
            text_offset: chapter.text_offset,
            text: document.chapter_text(index)?,
        };
        let setter = Typesetter::new(self.layout, self.language);
        pages.page_offsets = paginate_epub_chapter(&loaded, &setter)?;
        self.current_page = match target {
            EpubPageTarget::FirstPage => 0,
            EpubPageTarget::LastPage => pages.page_offsets.len() - 1,
            EpubPageTarget::Offset(offset) => page_containing(&pages.page_offsets, offset),
        };
        self.page_offsets = pages.page_offsets.clone();
        self.epub_chapter_pages = vec![pages];
        self.epub_chapter = Some(loaded);
        self.cache.clear();
        self.ensure_page_cached(self.current_page)
    }

    #[must_use]
    fn anchor_cache(&self) -> Option<ReaderAnchorCache> {
        if self.book.format != BookFormat::Text {
            return None;
        }
        Some(ReaderAnchorCache {
            fingerprint: book_fingerprint(&self.book, self.layout),
            base_page: self.page_number_base,
            offsets: self.page_offsets.clone(),
            indexed_through: self.indexed_through,
            complete: self.index_complete,
        })
    }
}

/// Reader Options action rows. Editable values live on the separate
/// Reading Preferences editor so menu controls match the rest of the firmware.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderOption {
    Bookmark,
    Bookmarks,
    TableOfContents,
    ReadingPreferences,
    ClearGhosting,
    GoToLibrary,
    GoHome,
}

impl ReaderOption {
    pub const ALL: [Self; 7] = [
        Self::Bookmark,
        Self::Bookmarks,
        Self::TableOfContents,
        Self::ReadingPreferences,
        Self::ClearGhosting,
        Self::GoToLibrary,
        Self::GoHome,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bookmark => "Add / Remove Bookmark",
            Self::Bookmarks => "Bookmarks",
            Self::TableOfContents => "Table of Contents",
            Self::ReadingPreferences => "Reading Preferences",
            Self::ClearGhosting => "Clear Ghosting",
            Self::GoToLibrary => "Go to Library",
            Self::GoHome => "Go Home",
        }
    }

    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::Bookmark => "TOGGLE",
            Self::Bookmarks => "LIST",
            Self::TableOfContents => "NONE",
            Self::ReadingPreferences => ">>>",
            Self::ClearGhosting => "RUN",
            Self::GoToLibrary | Self::GoHome => ">>>",
        }
    }
}

/// Reading Preferences editor rows. MOVE highlights a row and SELECT opens an
/// option list of every value for it, matching the Settings pickers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingPreference {
    ReadingTheme,
    Orientation,
    BookFontSize,
    BookFont,
    ParagraphAlignment,
    ShowProgress,
}

impl ReadingPreference {
    pub const ALL: [Self; 6] = [
        Self::ReadingTheme,
        Self::Orientation,
        Self::BookFontSize,
        Self::BookFont,
        Self::ParagraphAlignment,
        Self::ShowProgress,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadingTheme => "Reading Theme",
            Self::Orientation => "Orientation",
            Self::BookFontSize => "Book Font Size",
            Self::BookFont => "Book Font",
            Self::ParagraphAlignment => "Paragraph Alignment",
            Self::ShowProgress => "Show Progress",
        }
    }
}

/// Coarse background tick result used by main.rs to refresh the panel only
/// when the visible Reader screen changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderTickOutcome {
    None,
    FirstPageReady,
    BackgroundCacheAdvanced,
    Failed,
}

/// Non-fatal Reader persistence startup report.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReaderPersistenceReport {
    pub state_loaded: bool,
    pub preferences_loaded: bool,
    pub position_count: usize,
    pub recent_count: usize,
    pub bookmark_count: usize,
    pub warning: Option<String>,
}

/// Hardware-independent Reader UI state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderUiState {
    pub books_root: String,
    pub state_root: String,
    pub books: Vec<ReaderBook>,
    pub positions: Vec<ReaderLocation>,
    pub recent: Vec<ReaderLocation>,
    pub bookmarks: Vec<ReaderLocation>,
    pub resume: Option<ReaderLocation>,
    pub preferences: ReaderPreferences,
    pub library_error: Option<String>,
    pub persistence_warning: Option<String>,
    pub library_tab: ReaderLibraryTab,
    /// Row zero is the explicit tab-control row; book rows begin at one.
    pub library_selected: usize,
    pub bookmarks_selected: usize,
    pub toc_selected: usize,
    pub loading: Option<PendingReaderOpen>,
    pub session: Option<ReaderSession>,
    pub options_selected: usize,
    pub preferences_selected: usize,
    /// Highlighted choice of the open option list, `None` while rows show.
    pub preferences_picker: Option<usize>,
    preferences_layout_dirty: bool,
    pub last_message: Option<String>,
    persistence_event: Option<String>,
    last_persistence_event: Option<String>,
    clear_ghost_requested: bool,
}

impl Default for ReaderUiState {
    fn default() -> Self {
        Self {
            books_root: READER_BOOKS_DIRECTORY.into(),
            state_root: READER_STATE_DIRECTORY.into(),
            books: Vec::new(),
            positions: Vec::new(),
            recent: Vec::new(),
            bookmarks: Vec::new(),
            resume: None,
            preferences: ReaderPreferences::default(),
            library_error: None,
            persistence_warning: None,
            library_tab: ReaderLibraryTab::default(),
            library_selected: 0,
            bookmarks_selected: 0,
            toc_selected: 0,
            loading: None,
            session: None,
            options_selected: 0,
            preferences_selected: 0,
            preferences_picker: None,
            preferences_layout_dirty: false,
            last_message: None,
            persistence_event: None,
            last_persistence_event: None,
            clear_ghost_requested: false,
        }
    }
}

impl ReaderUiState {
    #[must_use]
    pub fn with_books_root(root: impl Into<String>) -> Self {
        Self {
            books_root: root.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_roots(books_root: impl Into<String>, state_root: impl Into<String>) -> Self {
        Self {
            books_root: books_root.into(),
            state_root: state_root.into(),
            ..Self::default()
        }
    }

    /// Load persisted state without making startup dependent on removable
    /// storage. Corrupt records are ignored and reported as a warning.
    pub fn load_persistent_state(&mut self) -> ReaderPersistenceReport {
        let mut warnings = Vec::new();
        let preferences_loaded = match load_preferences(&self.preferences_path()) {
            Ok(Some(preferences)) => {
                self.preferences = preferences;
                true
            }
            Ok(None) => false,
            Err(error) => {
                warnings.push(format!("PREFS.TXT: {error}"));
                false
            }
        };
        self.resume = match load_location_record(&self.state_path()) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("STATE.TXT: {error}"));
                None
            }
        };
        self.positions = match self.load_positions_with_legacy_migration() {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("POSITS.TXT: {error}"));
                Vec::new()
            }
        };
        self.recent = match load_location_list(&self.recent_path(), READER_RECENT_LIMIT) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("RECENT.TXT: {error}"));
                Vec::new()
            }
        };
        self.bookmarks = match load_location_list(&self.bookmarks_path(), READER_BOOKMARK_LIMIT) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("MARKS.TXT: {error}"));
                Vec::new()
            }
        };
        self.bookmarks_selected = self
            .bookmarks_selected
            .min(self.bookmarks.len().saturating_sub(1));
        let warning = if warnings.is_empty() {
            None
        } else {
            Some(warnings.join("; "))
        };
        self.persistence_warning = warning.clone();
        ReaderPersistenceReport {
            state_loaded: self.resume.is_some(),
            preferences_loaded,
            position_count: self.positions.len(),
            recent_count: self.recent.len(),
            bookmark_count: self.bookmarks.len(),
            warning,
        }
    }

    pub fn refresh_library(&mut self) {
        match scan_txt_library(&self.books_root) {
            Ok(books) => {
                self.books = books;
                self.library_error = None;
            }
            Err(error) => {
                self.books.clear();
                self.library_error = Some(error);
            }
        }
        self.library_selected = 0;
    }

    #[must_use]
    pub fn can_continue(&self) -> bool {
        self.session.is_some() || self.resume.is_some() || !self.recent.is_empty()
    }

    pub fn request_continue(&mut self) -> bool {
        let Some(location) = self.resume.clone().or_else(|| self.recent.first().cloned()) else {
            return false;
        };
        self.request_open_book(location.as_book(), Some(location));
        true
    }

    #[must_use]
    pub fn visible_entries(&self) -> Vec<ReaderLibraryEntry> {
        match self.library_tab {
            ReaderLibraryTab::Recent => self
                .recent
                .iter()
                .cloned()
                .map(|location| ReaderLibraryEntry {
                    book: location.as_book(),
                    location: Some(location),
                })
                .collect(),
            ReaderLibraryTab::Books | ReaderLibraryTab::Files => self
                .books
                .iter()
                .cloned()
                .map(|book| ReaderLibraryEntry {
                    location: self.saved_position_for_book(&book),
                    book,
                })
                .collect(),
            ReaderLibraryTab::Bookmarks => self
                .bookmarks
                .iter()
                .cloned()
                .map(|location| ReaderLibraryEntry {
                    book: location.as_book(),
                    location: Some(location),
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn library_row_count(&self) -> usize {
        self.visible_entries().len().saturating_add(1)
    }

    pub fn apply_library_button(&mut self, event: ButtonEvent) -> bool {
        let count = self.library_row_count().max(1);
        match event {
            ButtonEvent::Up => {
                self.library_selected = self.library_selected.checked_sub(1).unwrap_or(count - 1);
                false
            }
            ButtonEvent::Down => {
                self.library_selected = (self.library_selected + 1) % count;
                false
            }
            ButtonEvent::Select if self.library_selected == 0 => {
                self.library_tab = self.library_tab.next();
                self.library_selected = 0;
                false
            }
            ButtonEvent::Select => self.request_open_visible(self.library_selected - 1),
        }
    }

    pub fn apply_bookmarks_button(&mut self, event: ButtonEvent) -> bool {
        if self.bookmarks.is_empty() {
            return false;
        }
        match event {
            ButtonEvent::Up => {
                self.bookmarks_selected = self
                    .bookmarks_selected
                    .checked_sub(1)
                    .unwrap_or(self.bookmarks.len() - 1);
                false
            }
            ButtonEvent::Down => {
                self.bookmarks_selected = (self.bookmarks_selected + 1) % self.bookmarks.len();
                false
            }
            ButtonEvent::Select => self.request_open_bookmark(self.bookmarks_selected),
        }
    }

    pub fn request_open_visible(&mut self, visible_index: usize) -> bool {
        let Some(entry) = self.visible_entries().get(visible_index).cloned() else {
            return false;
        };
        let resume = entry
            .location
            .or_else(|| self.saved_position_for_book(&entry.book))
            .or_else(|| {
                self.resume
                    .clone()
                    .filter(|location| location.matches_book(&entry.book))
            });
        self.request_open_book(entry.book, resume);
        true
    }

    #[must_use]
    fn saved_position_for_book(&self, book: &ReaderBook) -> Option<ReaderLocation> {
        self.positions
            .iter()
            .find(|location| location.matches_book(book))
            .cloned()
    }

    pub fn request_open_bookmark(&mut self, bookmark_index: usize) -> bool {
        let Some(location) = self.bookmarks.get(bookmark_index).cloned() else {
            return false;
        };
        self.request_open_book(location.as_book(), Some(location));
        true
    }

    fn request_open_book(&mut self, book: ReaderBook, resume: Option<ReaderLocation>) {
        self.release_active_session_for_open();
        self.loading = Some(PendingReaderOpen {
            book,
            stage: ReaderLoadingStage::OpeningFile,
            encoding: None,
            epub_document: None,
            resume,
            message: "Preparing reader...".into(),
        });
    }

    /// Persist and drop the previous session before a new book is parsed. EPUB
    /// documents retain flattened text and chapter anchors in RAM; keeping the
    /// old document alive while allocating the next parser-worker stack can
    /// exhaust the embedded heap after repeated book switches.
    fn release_active_session_for_open(&mut self) {
        if self.session.is_none() {
            return;
        }
        self.persist_current_session_best_effort();
        self.session = None;
        log::info!("rustmix-wave=reader-session-memory-release status=completed reason=book-open");
    }

    fn request_layout_rebuild(&mut self) -> bool {
        if self.session.is_none() {
            self.persist_preferences_best_effort();
            return false;
        }
        self.persist_current_session_best_effort();
        let Some(mut session) = self.session.take() else {
            return false;
        };
        let book = session.book.clone();
        let encoding = session.encoding;
        let resume = session.current_location();
        self.loading = Some(PendingReaderOpen {
            book,
            stage: ReaderLoadingStage::UpdatingLayout,
            encoding: Some(encoding),
            epub_document: session.epub_document.take(),
            resume: Some(resume),
            message: "Rebuilding the current page first...".into(),
        });
        log::info!(
            "rustmix-wave=reader-session-memory-release status=completed reason=layout-rebuild"
        );
        self.persist_preferences_best_effort();
        true
    }

    pub fn cancel_loading(&mut self) {
        self.loading = None;
        self.last_message = Some("Book opening cancelled".into());
    }

    #[must_use]
    pub fn loading_stage(&self) -> Option<ReaderLoadingStage> {
        self.loading.as_ref().map(|loading| loading.stage)
    }

    /// True while `tick` still has a book to open or nearby pages to index.
    #[must_use]
    pub fn has_background_work(&self) -> bool {
        if let Some(loading) = self.loading.as_ref() {
            return loading.stage != ReaderLoadingStage::Failed;
        }
        self.session.as_ref().is_some_and(|session| {
            session.cache.len() < READER_NEARBY_PAGE_CACHE && !session.index_complete
        })
    }

    pub fn tick(&mut self) -> ReaderTickOutcome {
        if let Some(mut loading) = self.loading.take() {
            if loading.stage == ReaderLoadingStage::Failed {
                self.loading = Some(loading);
                return ReaderTickOutcome::None;
            }
            return match self.open_pending_book(&mut loading) {
                Ok(session) => {
                    self.session = Some(session);
                    self.last_message =
                        Some("Saved position ready; caching continues lazily".into());
                    self.persist_current_session_best_effort();
                    ReaderTickOutcome::FirstPageReady
                }
                Err(error) => {
                    loading.stage = ReaderLoadingStage::Failed;
                    loading.message = error;
                    self.loading = Some(loading);
                    ReaderTickOutcome::Failed
                }
            };
        }

        let (outcome, checkpoint) = if let Some(session) = self.session.as_mut() {
            if session.cache.len() < READER_NEARBY_PAGE_CACHE && !session.index_complete {
                match session.index_one_page() {
                    Ok(true) => (
                        ReaderTickOutcome::BackgroundCacheAdvanced,
                        session.page_offsets.len() % READER_CACHE_CHECKPOINT_PAGES == 0
                            || session.index_complete,
                    ),
                    Ok(false) => (ReaderTickOutcome::None, session.index_complete),
                    Err(error) => {
                        self.last_message = Some(error);
                        return ReaderTickOutcome::Failed;
                    }
                }
            } else {
                (ReaderTickOutcome::None, false)
            }
        } else {
            (ReaderTickOutcome::None, false)
        };
        if checkpoint {
            self.persist_anchor_cache_best_effort();
        }
        outcome
    }

    pub fn previous_page(&mut self) {
        if let Some(session) = self.session.as_mut() {
            if let Err(error) = session.previous_page() {
                self.last_message = Some(error);
                return;
            }
            self.persist_current_session_best_effort();
        }
    }

    pub fn next_page(&mut self) {
        if let Some(session) = self.session.as_mut() {
            if let Err(error) = session.next_page() {
                self.last_message = Some(error);
                return;
            }
            self.persist_current_session_best_effort();
        }
    }

    pub fn cycle_option_previous(&mut self) {
        self.options_selected = self
            .options_selected
            .checked_sub(1)
            .unwrap_or(ReaderOption::ALL.len() - 1);
    }

    pub fn cycle_option_next(&mut self) {
        self.options_selected = (self.options_selected + 1) % ReaderOption::ALL.len();
    }

    #[must_use]
    pub fn selected_option(&self) -> ReaderOption {
        ReaderOption::ALL[self.options_selected]
    }

    /// Resolve a bookmark's user-facing page label against the active layout
    /// when nearby anchors are available. The persisted byte offset remains the
    /// canonical bookmark authority; the stored page index is a safe fallback.
    #[must_use]
    pub fn bookmark_display_page(&self, bookmark: &ReaderLocation) -> usize {
        self.session
            .as_ref()
            .filter(|session| bookmark.matches_book(&session.book))
            .and_then(|session| {
                session
                    .page_offsets
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, offset)| **offset <= bookmark.byte_offset)
                    .map(|(index, _)| {
                        session
                            .page_number_base
                            .saturating_add(index)
                            .saturating_add(1)
                    })
            })
            .unwrap_or_else(|| bookmark.page_index.saturating_add(1))
    }

    /// Resolve an EPUB bookmark against the active layout when possible and
    /// otherwise use the persisted chapter-relative fallback stored in MARKS.TXT.
    #[must_use]
    pub fn bookmark_display_chapter_page(
        &self,
        bookmark: &ReaderLocation,
    ) -> Option<ReaderChapterPageLabel> {
        if bookmark.format != BookFormat::Epub {
            return None;
        }
        self.session
            .as_ref()
            .filter(|session| bookmark.matches_book(&session.book))
            .and_then(|session| session.epub_chapter_page_label_for_offset(bookmark.byte_offset))
            .or_else(|| bookmark.epub_chapter.clone())
    }

    #[must_use]
    pub fn has_structured_toc(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| !session.toc_entries().is_empty())
    }

    #[must_use]
    pub fn toc_entries(&self) -> &[EpubTocEntry] {
        self.session
            .as_ref()
            .map_or(&[], ReaderSession::toc_entries)
    }

    pub fn apply_toc_button(&mut self, event: ButtonEvent) -> bool {
        let count = self.toc_entries().len();
        if count == 0 {
            return false;
        }
        match event {
            ButtonEvent::Up => {
                self.toc_selected = self.toc_selected.checked_sub(1).unwrap_or(count - 1);
                false
            }
            ButtonEvent::Down => {
                self.toc_selected = (self.toc_selected + 1) % count;
                false
            }
            ButtonEvent::Select => self.open_selected_toc_entry(),
        }
    }

    fn open_selected_toc_entry(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let Some(entry) = session
            .epub_document
            .as_ref()
            .and_then(|document| document.toc.get(self.toc_selected))
            .cloned()
        else {
            return false;
        };
        match session.show_epub_offset(entry.text_offset) {
            Ok(()) => {
                self.last_message = Some(format!("TOC: {}", entry.label));
                self.persist_current_session_best_effort();
                true
            }
            Err(error) => {
                self.last_message = Some(error);
                false
            }
        }
    }

    #[must_use]
    pub fn current_page_is_bookmarked(&self) -> bool {
        let Some(location) = self.session.as_ref().map(ReaderSession::current_location) else {
            return false;
        };
        self.bookmarks
            .iter()
            .any(|bookmark| bookmark.same_position(&location))
    }

    pub fn toggle_current_bookmark(&mut self) {
        let Some(location) = self.session.as_ref().map(ReaderSession::current_location) else {
            self.last_message = Some("Open a Reader page before adding a bookmark".into());
            return;
        };
        if let Some(index) = self
            .bookmarks
            .iter()
            .position(|bookmark| bookmark.same_position(&location))
        {
            self.bookmarks.remove(index);
            self.bookmarks_selected = self
                .bookmarks_selected
                .min(self.bookmarks.len().saturating_sub(1));
            self.last_message = Some("Bookmark removed".into());
        } else {
            self.bookmarks.insert(0, location);
            self.bookmarks.truncate(READER_BOOKMARK_LIMIT);
            self.bookmarks_selected = 0;
            self.last_message = Some("Bookmark saved".into());
        }
        self.persist_bookmarks_best_effort();
    }

    pub fn begin_preferences_edit(&mut self) {
        self.preferences_selected = 0;
        self.preferences_picker = None;
        self.preferences_layout_dirty = false;
    }

    pub fn cycle_preference_previous(&mut self) {
        self.preferences_selected = self
            .preferences_selected
            .checked_sub(1)
            .unwrap_or(ReadingPreference::ALL.len() - 1);
    }

    pub fn cycle_preference_next(&mut self) {
        self.preferences_selected = (self.preferences_selected + 1) % ReadingPreference::ALL.len();
    }

    #[must_use]
    pub fn selected_preference(&self) -> ReadingPreference {
        ReadingPreference::ALL[self.preferences_selected]
    }

    /// Labels of the choices for the highlighted preference and the index of
    /// the value in use. Show Progress offers On and Off.
    #[must_use]
    pub fn preference_options(&self) -> (Vec<&'static str>, usize) {
        match self.selected_preference() {
            ReadingPreference::ReadingTheme => (
                ReadingTheme::ALL
                    .iter()
                    .map(|value| value.label())
                    .collect(),
                ReadingTheme::ALL
                    .iter()
                    .position(|&value| value == self.preferences.theme)
                    .unwrap_or(0),
            ),
            ReadingPreference::Orientation => (
                ReaderOrientation::ALL
                    .iter()
                    .map(|value| value.label())
                    .collect(),
                ReaderOrientation::ALL
                    .iter()
                    .position(|&value| value == self.preferences.orientation)
                    .unwrap_or(0),
            ),
            ReadingPreference::BookFontSize => (
                BookFontSize::ALL
                    .iter()
                    .map(|value| value.label())
                    .collect(),
                BookFontSize::ALL
                    .iter()
                    .position(|&value| value == self.preferences.font_size)
                    .unwrap_or(0),
            ),
            ReadingPreference::BookFont => (
                BookFont::ALL.iter().map(|value| value.label()).collect(),
                BookFont::ALL
                    .iter()
                    .position(|&value| value == self.preferences.book_font)
                    .unwrap_or(0),
            ),
            ReadingPreference::ParagraphAlignment => (
                ParagraphAlignment::ALL
                    .iter()
                    .map(|value| value.label())
                    .collect(),
                ParagraphAlignment::ALL
                    .iter()
                    .position(|&value| value == self.preferences.paragraph_alignment)
                    .unwrap_or(0),
            ),
            ReadingPreference::ShowProgress => (
                vec!["On", "Off"],
                usize::from(!self.preferences.show_progress),
            ),
        }
    }

    /// Open the option list for the highlighted preference at the value in use.
    pub fn open_preference_picker(&mut self) {
        self.preferences_picker = Some(self.preference_options().1);
    }

    /// Apply one option-list choice to the highlighted preference and close
    /// the list. Redraw-only settings persist immediately in place.
    /// Layout-sensitive settings persist immediately and request a staged
    /// current-page rebuild.
    #[must_use]
    pub fn choose_preference(&mut self, index: usize) -> bool {
        let layout_sensitive = match self.selected_preference() {
            ReadingPreference::ReadingTheme => {
                self.preferences.theme = ReadingTheme::ALL[index % ReadingTheme::ALL.len()];
                self.last_message =
                    Some(format!("Reading theme: {}", self.preferences.theme.label()));
                self.persist_preferences_best_effort();
                self.request_clear_ghosting();
                false
            }
            ReadingPreference::Orientation => {
                self.preferences.orientation =
                    ReaderOrientation::ALL[index % ReaderOrientation::ALL.len()];
                self.last_message = Some(format!(
                    "Orientation: {}",
                    self.preferences.orientation.label()
                ));
                true
            }
            ReadingPreference::BookFontSize => {
                self.preferences.font_size = BookFontSize::ALL[index % BookFontSize::ALL.len()];
                self.last_message = Some(format!(
                    "Book font size: {}",
                    self.preferences.font_size.label()
                ));
                true
            }
            ReadingPreference::BookFont => {
                self.preferences.book_font = BookFont::ALL[index % BookFont::ALL.len()];
                self.last_message =
                    Some(format!("Book font: {}", self.preferences.book_font.label()));
                true
            }
            ReadingPreference::ParagraphAlignment => {
                self.preferences.paragraph_alignment =
                    ParagraphAlignment::ALL[index % ParagraphAlignment::ALL.len()];
                self.last_message = Some(format!(
                    "Paragraph alignment: {}",
                    self.preferences.paragraph_alignment.label()
                ));
                true
            }
            ReadingPreference::ShowProgress => {
                self.preferences.show_progress = index == 0;
                self.last_message = Some(format!(
                    "Show progress: {}",
                    if self.preferences.show_progress {
                        "On"
                    } else {
                        "Off"
                    }
                ));
                self.persist_preferences_best_effort();
                false
            }
        };
        self.preferences_picker = None;
        if layout_sensitive {
            self.request_layout_rebuild()
        } else {
            false
        }
    }

    /// Finish the Settings-style editor. A chosen value already persists and
    /// launches any required staged rebuild, so BOOT simply returns to options.
    pub fn finish_preferences_edit(&mut self) -> bool {
        self.preferences_layout_dirty = false;
        false
    }

    pub fn request_clear_ghosting(&mut self) {
        self.clear_ghost_requested = true;
        self.last_message = Some("Global ghost-clearing refresh requested".into());
    }

    #[must_use]
    pub fn take_clear_ghost_request(&mut self) -> bool {
        core::mem::take(&mut self.clear_ghost_requested)
    }

    #[must_use]
    pub fn take_persistence_event(&mut self) -> Option<String> {
        self.persistence_event.take()
    }

    #[must_use]
    fn state_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_STATE_FILE)
    }

    #[must_use]
    fn positions_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_POSITIONS_FILE)
    }

    #[must_use]
    fn legacy_positions_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(LEGACY_READER_POSITIONS_FILE)
    }

    fn load_positions_with_legacy_migration(&mut self) -> Result<Vec<ReaderLocation>, String> {
        let positions = self.positions_path();
        let positions_backup = with_extension(&positions, "BAK");
        if positions.exists() || positions_backup.exists() {
            return load_location_list(&positions, READER_POSITION_LIMIT);
        }

        let legacy = self.legacy_positions_path();
        let legacy_backup = with_extension(&legacy, "BAK");
        if !legacy.exists() && !legacy_backup.exists() {
            return Ok(Vec::new());
        }

        let migrated = load_location_list(&legacy, READER_POSITION_LIMIT)?;
        if !migrated.is_empty() {
            if let Err(error) = atomic_replace_text(&positions, &serialize_location_list(&migrated))
            {
                self.persistence_warning = Some(format!(
                    "legacy POSITIONS.TXT loaded; POSITS.TXT migration deferred: {error}"
                ));
            }
        }
        Ok(migrated)
    }

    #[must_use]
    fn recent_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_RECENT_FILE)
    }

    #[must_use]
    fn bookmarks_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_BOOKMARKS_FILE)
    }

    #[must_use]
    fn preferences_path(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_PREFS_FILE)
    }

    #[must_use]
    fn cache_directory(&self) -> PathBuf {
        Path::new(&self.state_root).join(READER_CACHE_DIRECTORY)
    }

    #[must_use]
    fn cache_file_name_for(book: &ReaderBook, layout: ReaderLayout) -> String {
        format!("{:08X}.CCH", book_fingerprint(book, layout) as u32)
    }

    #[must_use]
    fn cache_path_for(&self, book: &ReaderBook, layout: ReaderLayout) -> PathBuf {
        self.cache_directory()
            .join(Self::cache_file_name_for(book, layout))
    }

    #[must_use]
    fn epub_index_path_for(&self, book: &ReaderBook) -> PathBuf {
        let name = format!("{:08X}.EPX", epub_index_fingerprint(book) as u32);
        self.cache_directory().join(name)
    }

    /// Open the staged book through its first visible page, reusing the
    /// encoding or EPUB index a layout rebuild already holds.
    fn open_pending_book(
        &mut self,
        loading: &mut PendingReaderOpen,
    ) -> Result<ReaderSession, String> {
        match loading.book.format {
            BookFormat::Text => {
                let encoding = match loading.encoding {
                    Some(encoding) => encoding,
                    None => detect_txt_encoding(&loading.book.path)?,
                };
                loading.encoding = Some(encoding);
                self.open_txt_session(&loading.book, encoding, loading.resume.as_ref())
            }
            BookFormat::Epub => {
                let document = match loading.epub_document.take() {
                    Some(document) => document,
                    None => self.load_epub_document(&loading.book)?,
                };
                self.open_epub_session(&loading.book, document, loading.resume.as_ref())
            }
        }
    }

    /// The book's saved chapter index when it is current; otherwise index the
    /// archive once and save the result for the next open.
    fn load_epub_document(&mut self, book: &ReaderBook) -> Result<EpubDocument, String> {
        let index_path = self.epub_index_path_for(book);
        match load_epub_index(&index_path, book) {
            Ok(Some(document)) => return Ok(document),
            Ok(None) => {}
            Err(error) => {
                self.persistence_warning = Some(format!("EPUB index ignored: {error}"));
            }
        }
        let document = open_epub_on_worker(&book.path)?;
        let text = serialize_epub_index(&document, book);
        if let Err(error) = atomic_replace_text(&index_path, &text) {
            self.persistence_warning = Some(format!("EPUB index not saved: {error}"));
        }
        Ok(document)
    }

    fn open_txt_session(
        &mut self,
        book: &ReaderBook,
        encoding: TextEncoding,
        requested: Option<&ReaderLocation>,
    ) -> Result<ReaderSession, String> {
        let cached = match load_anchor_cache(
            &self.cache_path_for(book, self.preferences.layout()),
            book,
            self.preferences.layout(),
        ) {
            Ok(value) => value,
            Err(error) => {
                self.persistence_warning = Some(format!("TXT cache ignored: {error}"));
                None
            }
        };
        let (page_number_base, page_offsets, current_page, indexed_through, index_complete) =
            if let Some(cache) = cached {
                let selected = requested
                    .filter(|location| location.matches_book(book))
                    .and_then(|location| {
                        location
                            .page_index
                            .checked_sub(cache.base_page)
                            .filter(|index| *index < cache.offsets.len())
                    })
                    .unwrap_or(0);
                (
                    cache.base_page,
                    cache.offsets,
                    selected,
                    cache.indexed_through,
                    cache.complete,
                )
            } else if let Some(location) = requested.filter(|location| location.matches_book(book))
            {
                (
                    location.page_index,
                    vec![location.byte_offset.min(book.size_bytes)],
                    0,
                    location.byte_offset.min(book.size_bytes),
                    false,
                )
            } else {
                (0, vec![0], 0, 0, false)
            };
        let offset = page_offsets.get(current_page).copied().unwrap_or(0);
        let absolute_page = page_number_base.saturating_add(current_page);
        let layout = self.preferences.layout();
        let language = detect_txt_language(&book.path, encoding);
        let setter = Typesetter::new(layout, language);
        let page = read_txt_page(book, encoding, &setter, offset, absolute_page)?;
        let indexed_through = indexed_through.max(page.next_byte_offset);
        let index_complete = index_complete || indexed_through >= book.size_bytes;
        Ok(ReaderSession {
            book: book.clone(),
            encoding,
            epub_document: None,
            layout,
            language,
            current_page,
            page_number_base,
            page_offsets,
            indexed_through,
            index_complete,
            cache: vec![page],
            epub_chapter_pages: Vec::new(),
            epub_chapter: None,
        })
    }

    fn open_epub_session(
        &mut self,
        book: &ReaderBook,
        document: EpubDocument,
        requested: Option<&ReaderLocation>,
    ) -> Result<ReaderSession, String> {
        let source_size = document.text_size_bytes();
        let requested = requested.filter(|location| location.matches_book(book));
        let offset = requested.map_or(0, |location| location.byte_offset.min(source_size));
        let mut session_book = book.clone();
        if !document.title.trim().is_empty() {
            session_book.title = document.title.clone();
        }
        let language = Language::from_tag(&document.language);
        let mut session = ReaderSession {
            book: session_book,
            encoding: TextEncoding::Utf8,
            epub_document: Some(document),
            layout: self.preferences.layout(),
            language,
            current_page: 0,
            page_number_base: 0,
            page_offsets: Vec::new(),
            indexed_through: source_size,
            index_complete: true,
            cache: Vec::new(),
            epub_chapter_pages: Vec::new(),
            epub_chapter: None,
        };
        session.show_epub_offset(offset)?;
        Ok(session)
    }

    fn persist_current_session_best_effort(&mut self) {
        let Some(location) = self.session.as_ref().map(ReaderSession::current_location) else {
            return;
        };
        self.resume = Some(location.clone());
        self.positions.retain(|entry| !entry.same_book(&location));
        self.positions.insert(0, location.clone());
        self.positions.truncate(READER_POSITION_LIMIT);
        self.recent.retain(|entry| !entry.same_book(&location));
        self.recent.insert(0, location);
        self.recent.truncate(READER_RECENT_LIMIT);
        let mut errors = Vec::new();
        if let Some(location) = self.resume.as_ref() {
            if let Err(error) =
                atomic_replace_text(&self.state_path(), &serialize_location(location))
            {
                errors.push(format!("STATE.TXT: {error}"));
            }
        }
        if let Err(error) = atomic_replace_text(
            &self.positions_path(),
            &serialize_location_list(&self.positions),
        ) {
            errors.push(format!("POSITS.TXT: {error}"));
        }
        if let Err(error) =
            atomic_replace_text(&self.recent_path(), &serialize_location_list(&self.recent))
        {
            errors.push(format!("RECENT.TXT: {error}"));
        }
        if let Err(error) = self.persist_anchor_cache() {
            errors.push(format!("CACHE: {error}"));
        }
        self.finish_persistence("state-positions-recent-cache", errors);
    }

    fn persist_bookmarks_best_effort(&mut self) {
        let mut errors = Vec::new();
        if let Err(error) = atomic_replace_text(
            &self.bookmarks_path(),
            &serialize_location_list(&self.bookmarks),
        ) {
            errors.push(format!("MARKS.TXT: {error}"));
        }
        self.finish_persistence("bookmarks", errors);
    }

    fn persist_anchor_cache_best_effort(&mut self) {
        let mut errors = Vec::new();
        if let Err(error) = self.persist_anchor_cache() {
            errors.push(format!("CACHE: {error}"));
        }
        self.finish_persistence("anchor-cache", errors);
    }

    fn persist_anchor_cache(&self) -> Result<(), String> {
        let Some(session) = self.session.as_ref() else {
            return Ok(());
        };
        let Some(cache) = session.anchor_cache() else {
            return Ok(());
        };
        atomic_replace_text(
            &self.cache_path_for(&session.book, session.layout),
            &serialize_anchor_cache(&cache),
        )
    }

    fn persist_preferences_best_effort(&mut self) {
        let mut errors = Vec::new();
        if let Err(error) =
            atomic_replace_text(&self.preferences_path(), &self.preferences.serialized())
        {
            errors.push(format!("PREFS.TXT: {error}"));
        }
        self.finish_persistence("preferences", errors);
    }

    fn finish_persistence(&mut self, scope: &str, errors: Vec<String>) {
        let event = if errors.is_empty() {
            format!("status=saved scope={scope}")
        } else {
            let warning = errors.join("; ");
            self.persistence_warning = Some(warning.clone());
            format!("status=degraded scope={scope} error={warning}")
        };
        if self.last_persistence_event.as_deref() != Some(event.as_str()) {
            self.last_persistence_event = Some(event.clone());
            self.persistence_event = Some(event);
        }
    }
}

/// Scan one bounded Reader library. TXT and EPUB/EPU rows open through the
/// shared staged Reader architecture.
pub fn scan_txt_library(root: impl AsRef<Path>) -> Result<Vec<ReaderBook>, String> {
    let root = root.as_ref();
    let mut books = Vec::new();
    let entries =
        fs::read_dir(root).map_err(|error| format!("Books folder unavailable: {error}"))?;
    for entry in entries.flatten() {
        // macOS writes hidden `._name` metadata companions to FAT cards.
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(format) = book_format_from_path(&path) else {
            continue;
        };
        let metadata = entry.metadata().ok();
        let size_bytes = metadata.as_ref().map_or(0, |meta| meta.len());
        let modified_seconds = metadata
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());
        let fallback_title = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Untitled book")
            .to_string();
        let title = if format == BookFormat::Epub {
            read_epub_title_on_worker(&path)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(fallback_title)
        } else {
            fallback_title
        };
        books.push(ReaderBook {
            path: path.to_string_lossy().into_owned(),
            title,
            format,
            size_bytes,
            modified_seconds,
        });
        if books.len() >= READER_LIBRARY_LIMIT {
            break;
        }
    }
    books.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    Ok(books)
}

/// Positions saved before long file names were enabled use 8.3 paths such as
/// `POIROT~1.TXT`; an unchanged file in the same folder is the same book.
fn renamed_in_place(saved_path: &str, book: &ReaderBook) -> bool {
    book.modified_seconds != 0 && folder_key(saved_path) == folder_key(&book.path)
}

fn folder_key(path: &str) -> Option<String> {
    let parent = Path::new(path).parent()?;
    Some(parent.to_string_lossy().to_ascii_uppercase())
}

#[must_use]
pub fn book_format_from_path(path: &Path) -> Option<BookFormat> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "txt" => Some(BookFormat::Text),
        "epub" | "epu" => Some(BookFormat::Epub),
        _ => None,
    }
}

pub fn detect_txt_encoding(path: impl AsRef<Path>) -> Result<TextEncoding, String> {
    let mut file = File::open(path.as_ref()).map_err(|error| format!("Open failed: {error}"))?;
    let mut sample = vec![0_u8; 4096];
    let read = file
        .read(&mut sample)
        .map_err(|error| format!("Read failed: {error}"))?;
    sample.truncate(read);
    if sample.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(TextEncoding::Utf8Bom);
    }
    match std::str::from_utf8(&sample) {
        Ok(_) => Ok(TextEncoding::Utf8),
        Err(error) if error.error_len().is_none() => Ok(TextEncoding::Utf8),
        Err(_) => Ok(TextEncoding::Windows1252),
    }
}

/// Language of a TXT book, guessed from its first few kilobytes.
fn detect_txt_language(path: &str, encoding: TextEncoding) -> Option<Language> {
    let mut sample = Vec::new();
    File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut sample)
        .ok()?;
    let text: String = decode_with_offsets(&sample, encoding, 0)
        .into_iter()
        .map(|(character, _)| character)
        .collect();
    Language::detect(&text)
}

/// Page start offsets of one loaded EPUB chapter.
fn paginate_epub_chapter(
    chapter: &LoadedEpubChapter,
    setter: &Typesetter,
) -> Result<Vec<u64>, String> {
    let started = Instant::now();
    let mut pacer = Pacer::start();
    let mut offsets = Vec::new();
    let mut offset = chapter.text_offset;
    while offset < chapter.end_offset() {
        if offsets.len() >= READER_EPUB_PAGE_ANCHOR_LIMIT {
            return Err(format!(
                "EPUB chapter exceeds {} page anchor limit",
                READER_EPUB_PAGE_ANCHOR_LIMIT
            ));
        }
        offsets.push(offset);
        let page = read_epub_chapter_page(chapter, setter, offset, 0)?;
        if page.next_byte_offset <= offset {
            return Err("EPUB chapter pagination did not advance".into());
        }
        offset = page.next_byte_offset;
        pacer.pace();
    }
    log::info!(
        "rustmix-wave=epub-chapter-index status=completed chapter={} pages={} elapsed-ms={}",
        chapter.index + 1,
        offsets.len(),
        started.elapsed().as_millis()
    );
    if offsets.is_empty() {
        return Err("EPUB chapter has no readable text".into());
    }
    Ok(offsets)
}

/// Index of the last page anchor at or before `offset`.
fn page_containing(offsets: &[u64], offset: u64) -> usize {
    offsets
        .partition_point(|anchor| *anchor <= offset)
        .saturating_sub(1)
}

fn read_epub_chapter_page(
    chapter: &LoadedEpubChapter,
    setter: &Typesetter,
    byte_offset: u64,
    page_index: usize,
) -> Result<ReaderCachedPage, String> {
    let bytes = chapter.text.as_bytes();
    let local = byte_offset
        .checked_sub(chapter.text_offset)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value <= bytes.len())
        .ok_or_else(|| "page offset is outside the open EPUB chapter".to_string())?;
    let start = next_utf8_boundary(bytes, local);
    let base = chapter.text_offset + start as u64;
    let mut window = setter.window_bytes;
    loop {
        let window_end = start.saturating_add(window);
        let end = previous_utf8_boundary(bytes, window_end).max(start);
        let at_end = end == bytes.len();
        let decoded = decode_with_offsets(&bytes[start..end], TextEncoding::Utf8, base);
        let (lines, consumed) = compose_page(&normalize_decoded(&decoded), base, at_end, setter);
        if lines.len() < setter.lines_per_page && !at_end && window < READER_PAGE_READ_BYTES {
            window = READER_PAGE_READ_BYTES;
            continue;
        }
        return Ok(ReaderCachedPage {
            page_index,
            byte_offset: base,
            next_byte_offset: consumed.max(base).min(chapter.end_offset()),
            lines,
        });
    }
}

/// Bytes read for one page: a page of 4-byte characters at the layout's
/// character budget, so indexing cost follows the page size. Text dense
/// enough to need more is retried with `READER_PAGE_READ_BYTES`.
fn page_window_bytes(layout: ReaderLayout) -> usize {
    let bytes = layout.lines_per_page * (layout.chars_per_line + 1) * 4;
    bytes.min(READER_PAGE_READ_BYTES)
}

fn next_utf8_boundary(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() && offset > 0 && bytes[offset] & 0xC0 == 0x80 {
        offset += 1;
    }
    offset.min(bytes.len())
}

fn previous_utf8_boundary(bytes: &[u8], mut offset: usize) -> usize {
    offset = offset.min(bytes.len());
    while offset > 0 && offset < bytes.len() && bytes[offset] & 0xC0 == 0x80 {
        offset -= 1;
    }
    offset
}

fn read_txt_page(
    book: &ReaderBook,
    encoding: TextEncoding,
    setter: &Typesetter,
    byte_offset: u64,
    page_index: usize,
) -> Result<ReaderCachedPage, String> {
    let mut file = File::open(&book.path).map_err(|error| format!("Open failed: {error}"))?;
    let mut window = setter.window_bytes;
    loop {
        file.seek(SeekFrom::Start(byte_offset))
            .map_err(|error| format!("Seek failed: {error}"))?;
        let mut bytes = Vec::with_capacity(window);
        (&mut file)
            .take(window as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("Read failed: {error}"))?;
        let at_end = byte_offset + bytes.len() as u64 >= book.size_bytes;
        let skip = if byte_offset == 0 && bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            3
        } else {
            0
        };
        let base = byte_offset + skip as u64;
        let decoded = decode_with_offsets(&bytes[skip..], encoding, base);
        let (lines, consumed) = compose_page(&normalize_decoded(&decoded), base, at_end, setter);
        if lines.len() < setter.lines_per_page && !at_end && window < READER_PAGE_READ_BYTES {
            window = READER_PAGE_READ_BYTES;
            continue;
        }
        return Ok(ReaderCachedPage {
            page_index,
            byte_offset,
            next_byte_offset: consumed.max(base).min(book.size_bytes),
            lines,
        });
    }
}

fn decode_with_offsets(bytes: &[u8], encoding: TextEncoding, base: u64) -> Vec<(char, u64)> {
    match encoding {
        TextEncoding::Windows1252 => bytes
            .iter()
            .enumerate()
            .map(|(index, byte)| (decode_windows_1252(*byte), base + index as u64 + 1))
            .collect(),
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => {
            let valid = match std::str::from_utf8(bytes) {
                Ok(text) => text,
                Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()]).unwrap_or(""),
            };
            valid
                .char_indices()
                .map(|(index, character)| {
                    (character, base + index as u64 + character.len_utf8() as u64)
                })
                .collect()
        }
    }
}

fn normalize_decoded(decoded: &[(char, u64)]) -> Vec<(char, u64)> {
    let mut normalized = Vec::with_capacity(decoded.len());
    for (index, (character, next_offset)) in decoded.iter().copied().enumerate() {
        if character == '_' {
            let previous = index
                .checked_sub(1)
                .and_then(|value| decoded.get(value))
                .map(|value| value.0);
            let next = decoded.get(index + 1).map(|value| value.0);
            let word_internal =
                previous.is_some_and(is_word_character) && next.is_some_and(is_word_character);
            let repeated_separator = previous == Some('_') || next == Some('_');

            // Project Gutenberg TXT files often wrap emphasis across multiple
            // source lines: `_first line ... last line_`. Remove each bounded
            // delimiter independently so closing markers after punctuation do
            // not leak into rendered pages. Keep filename-style word_internal
            // underscores and repeated separator rows intact.
            if !word_internal && !repeated_separator {
                continue;
            }
        }
        push_normalized_character(&mut normalized, character, next_offset);
    }
    normalized
}

fn push_normalized_character(output: &mut Vec<(char, u64)>, character: char, next_offset: u64) {
    match character {
        // A kept soft hyphen would render as a visible '-' in mid-word.
        '\u{00AD}' => {}
        '\n' | '\r' | '\t' => output.push((character, next_offset)),
        value if glyph_index(value).is_some() => output.push((value, next_offset)),
        _ => output.push(('?', next_offset)),
    }
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric()
}

/// Line-breaking inputs shared by every page of one open book: body font
/// advances, line width, page size and the hyphenation language.
#[derive(Clone, Copy, Debug)]
struct Typesetter {
    style: UiTextStyle,
    line_width: i32,
    lines_per_page: usize,
    window_bytes: usize,
    space: i32,
    hyphen: i32,
    language: Option<Language>,
}

impl Typesetter {
    fn new(layout: ReaderLayout, language: Option<Language>) -> Self {
        // Every theme draws with the same strike, so pagination ignores it.
        let style = reader_body_style(layout.book_font, layout.font_size, ReadingTheme::Classic);
        Self {
            style,
            line_width: layout.line_width(),
            lines_per_page: layout.lines_per_page,
            window_bytes: page_window_bytes(layout),
            space: style.char_advance(' '),
            hyphen: style.char_advance('-'),
            language,
        }
    }

    fn width(&self, text: &[(char, u64)]) -> i32 {
        text.iter()
            .map(|(character, _)| self.style.char_advance(*character))
            .sum()
    }
}

/// Spaces that separate words. A no-break space stays inside its word.
fn is_word_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r')
}

/// Lay out one page of `text`, whose first character starts at byte offset
/// `base`. Lines break greedily by pixel width at collapsed spaces; a word
/// that does not fit is hyphenated when the language allows it, and a word
/// wider than a whole line is split where it overflows. Every `'\n'` ends a
/// paragraph. Unless `at_end`, the last word of `text` may continue past it,
/// so it is left for the next page. Returns the lines and the offset where the
/// next page starts.
fn compose_page(
    text: &[(char, u64)],
    base: u64,
    at_end: bool,
    setter: &Typesetter,
) -> (Vec<ReaderPageLine>, u64) {
    let mut page = PageComposer {
        text,
        base,
        setter,
        lines: Vec::new(),
        line: String::new(),
        width: 0,
    };
    let mut index = 0;
    while index < text.len() {
        let character = text[index].0;
        if character == '\n' {
            if page.end_line(true) {
                return (page.lines, text[index].1);
            }
            index += 1;
        } else if is_word_space(character) {
            index += 1;
        } else {
            let end = text[index..]
                .iter()
                .position(|(character, _)| *character == '\n' || is_word_space(*character))
                .map_or(text.len(), |length| index + length);
            if end == text.len() && !at_end && !page.is_empty() {
                page.end_partial_line();
                let next_page = page.start_of(index);
                return (page.lines, next_page);
            }
            if let Some(next_page) = page.place_word(index, end) {
                return (page.lines, next_page);
            }
            index = end;
        }
    }
    page.end_partial_line();
    (page.lines, text.last().map_or(base, |(_, next)| *next))
}

/// Where a word breaks across lines: before `text[at]`, adding a hyphen or
/// not (after an existing hyphen, or when splitting an over-long word).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WordBreak {
    at: usize,
    hyphen: bool,
}

/// One page being laid out by [`compose_page`].
struct PageComposer<'a> {
    text: &'a [(char, u64)],
    base: u64,
    setter: &'a Typesetter,
    lines: Vec<ReaderPageLine>,
    line: String,
    width: i32,
}

impl PageComposer<'_> {
    /// Byte offset where `text[index]` starts.
    fn start_of(&self, index: usize) -> u64 {
        index
            .checked_sub(1)
            .map_or(self.base, |previous| self.text[previous].1)
    }

    fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.line.is_empty()
    }

    /// Close the current line; true when that fills the page.
    fn end_line(&mut self, paragraph_end: bool) -> bool {
        self.lines.push(ReaderPageLine {
            text: core::mem::take(&mut self.line),
            paragraph_end,
        });
        self.width = 0;
        self.lines.len() >= self.setter.lines_per_page
    }

    /// Close the last line of the text, if it has any content.
    fn end_partial_line(&mut self) {
        if !self.line.is_empty() || self.lines.is_empty() {
            self.end_line(true);
        }
    }

    /// Add `text[start..end]`, `width` pixels wide, after a space.
    fn append(&mut self, start: usize, end: usize, width: i32) {
        if !self.line.is_empty() {
            self.line.push(' ');
            self.width += self.setter.space;
        }
        let word = self.text[start..end]
            .iter()
            .map(|(character, _)| *character);
        self.line.extend(word);
        self.width += width;
    }

    /// Lay out the word `text[start..end]`, moving or breaking it when it does
    /// not fit. Returns where the next page starts once this page is full.
    fn place_word(&mut self, mut start: usize, end: usize) -> Option<u64> {
        loop {
            let gap = if self.line.is_empty() {
                0
            } else {
                self.setter.space
            };
            let room = self.setter.line_width - self.width - gap;
            let width = self.setter.width(&self.text[start..end]);
            if width <= room {
                self.append(start, end, width);
                return None;
            }
            let split = match self.hyphenation_break(start, end, room) {
                Some(split) => split,
                None if self.line.is_empty() => self.overflow_break(start, end),
                None => {
                    // Nothing fits after the words already on this line.
                    if self.end_line(false) {
                        return Some(self.start_of(start));
                    }
                    continue;
                }
            };
            let width = self.setter.width(&self.text[start..split.at]);
            self.append(start, split.at, width);
            if split.hyphen {
                self.line.push('-');
            }
            if self.end_line(false) {
                return Some(self.start_of(split.at));
            }
            start = split.at;
        }
    }

    /// The latest break inside `text[start..end]` whose first part fits in
    /// `room` pixels: after an existing hyphen, or at a syllable boundary of
    /// the book language.
    fn hyphenation_break(&self, start: usize, end: usize, room: i32) -> Option<WordBreak> {
        let word = &self.text[start..end];
        let mut breaks = Vec::new();
        for index in 1..word.len().saturating_sub(1) {
            if word[index].0 == '-'
                && word[index - 1].0.is_alphanumeric()
                && word[index + 1].0.is_alphanumeric()
            {
                breaks.push(WordBreak {
                    at: start + index + 1,
                    hyphen: false,
                });
            }
        }
        if let Some(language) = self.setter.language {
            let spelled: String = word.iter().map(|(character, _)| *character).collect();
            let mut offsets = break_offsets(&spelled, language).into_iter().peekable();
            let mut bytes = 0;
            for (index, (character, _)) in word.iter().enumerate() {
                if offsets.next_if_eq(&bytes).is_some() {
                    breaks.push(WordBreak {
                        at: start + index,
                        hyphen: true,
                    });
                }
                bytes += character.len_utf8();
            }
        }
        breaks
            .into_iter()
            .filter(|split| {
                let hyphen = if split.hyphen { self.setter.hyphen } else { 0 };
                self.setter.width(&self.text[start..split.at]) + hyphen <= room
            })
            .max_by_key(|split| split.at)
    }

    /// Split a word wider than a whole line after its last fitting character.
    fn overflow_break(&self, start: usize, end: usize) -> WordBreak {
        let mut width = 0;
        let mut at = start;
        while at < end {
            width += self.setter.style.char_advance(self.text[at].0);
            if width > self.setter.line_width && at > start {
                break;
            }
            at += 1;
        }
        WordBreak { at, hyphen: false }
    }
}

fn decode_windows_1252(byte: u8) -> char {
    match byte {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        value => char::from(value),
    }
}

fn book_fingerprint(book: &ReaderBook, layout: ReaderLayout) -> u64 {
    let mut hash = CACHE_FNV_OFFSET;
    fn feed(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(CACHE_FNV_PRIME);
        }
    }
    feed(&mut hash, book.path.as_bytes());
    feed(&mut hash, &book.size_bytes.to_le_bytes());
    feed(&mut hash, &book.modified_seconds.to_le_bytes());
    feed(&mut hash, book.format.marker().as_bytes());
    feed(&mut hash, &layout.lines_per_page.to_le_bytes());
    feed(&mut hash, &layout.chars_per_line.to_le_bytes());
    feed(&mut hash, layout.orientation.marker().as_bytes());
    feed(&mut hash, layout.font_size.marker().as_bytes());
    feed(&mut hash, layout.book_font.marker().as_bytes());
    feed(&mut hash, layout.paragraph_alignment.marker().as_bytes());
    feed(&mut hash, READER_CACHE_VERSION.as_bytes());
    hash
}

fn serialize_location(location: &ReaderLocation) -> String {
    format!(
        "version={}\npath={}\ntitle={}\nformat={}\nsize={}\nmodified={}\npage={}\noffset={}\nchapter={}\nchapter_page={}\nchapter_pages={}\n",
        READER_PERSISTENCE_VERSION,
        escape_field(&location.path),
        escape_field(&location.title),
        location.format.marker(),
        location.size_bytes,
        location.modified_seconds,
        location.page_index,
        location.byte_offset,
        optional_usize(location.epub_chapter.as_ref().map(|chapter| chapter.chapter_number)),
        optional_usize(location.epub_chapter.as_ref().map(|chapter| chapter.page_number)),
        optional_usize(location.epub_chapter.as_ref().map(|chapter| chapter.page_count))
    )
}

fn parse_location_record(text: &str) -> Result<ReaderLocation, String> {
    let mut version = None;
    let mut path = None;
    let mut title = None;
    let mut format = None;
    let mut size = None;
    let mut modified = None;
    let mut page = None;
    let mut offset = None;
    let mut chapter = None;
    let mut chapter_page = None;
    let mut chapter_pages = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "version" => version = Some(value),
            "path" => path = Some(unescape_field(value)?),
            "title" => title = Some(unescape_field(value)?),
            "format" => format = BookFormat::parse(value),
            "size" => size = value.parse().ok(),
            "modified" => modified = value.parse().ok(),
            "page" => page = value.parse().ok(),
            "offset" => offset = value.parse().ok(),
            "chapter" => chapter = parse_optional_usize(value),
            "chapter_page" => chapter_page = parse_optional_usize(value),
            "chapter_pages" => chapter_pages = parse_optional_usize(value),
            _ => {}
        }
    }
    if version != Some(READER_PERSISTENCE_VERSION) {
        return Err("unsupported persistence version".into());
    }
    Ok(ReaderLocation {
        path: path.ok_or_else(|| "missing path".to_string())?,
        title: title.ok_or_else(|| "missing title".to_string())?,
        format: format.ok_or_else(|| "missing format".to_string())?,
        size_bytes: size.ok_or_else(|| "missing size".to_string())?,
        modified_seconds: modified.unwrap_or(0),
        page_index: page.ok_or_else(|| "missing page".to_string())?,
        byte_offset: offset.ok_or_else(|| "missing offset".to_string())?,
        epub_chapter: chapter_page_label(chapter, chapter_page, chapter_pages),
    })
}

fn serialize_location_list(locations: &[ReaderLocation]) -> String {
    let mut output = format!("version={}\n", READER_PERSISTENCE_VERSION);
    for location in locations {
        output.push_str("entry=");
        output.push_str(&serialize_location_fields(location));
        output.push('\n');
    }
    output
}

fn serialize_location_fields(location: &ReaderLocation) -> String {
    [
        escape_field(&location.path),
        escape_field(&location.title),
        location.format.marker().into(),
        location.size_bytes.to_string(),
        location.modified_seconds.to_string(),
        location.page_index.to_string(),
        location.byte_offset.to_string(),
        optional_usize(
            location
                .epub_chapter
                .as_ref()
                .map(|chapter| chapter.chapter_number),
        ),
        optional_usize(
            location
                .epub_chapter
                .as_ref()
                .map(|chapter| chapter.page_number),
        ),
        optional_usize(
            location
                .epub_chapter
                .as_ref()
                .map(|chapter| chapter.page_count),
        ),
    ]
    .join("\t")
}

fn parse_location_fields(value: &str) -> Result<ReaderLocation, String> {
    let fields = split_escaped_tabs(value)?;
    if fields.len() != 7 && fields.len() != 10 {
        return Err("invalid location field count".into());
    }
    let epub_chapter = if fields.len() == 10 {
        chapter_page_label(
            parse_optional_usize(&fields[7]),
            parse_optional_usize(&fields[8]),
            parse_optional_usize(&fields[9]),
        )
    } else {
        None
    };
    Ok(ReaderLocation {
        path: fields[0].clone(),
        title: fields[1].clone(),
        format: BookFormat::parse(&fields[2]).ok_or_else(|| "invalid format".to_string())?,
        size_bytes: fields[3].parse().map_err(|_| "invalid size".to_string())?,
        modified_seconds: fields[4]
            .parse()
            .map_err(|_| "invalid modified time".to_string())?,
        page_index: fields[5].parse().map_err(|_| "invalid page".to_string())?,
        byte_offset: fields[6]
            .parse()
            .map_err(|_| "invalid offset".to_string())?,
        epub_chapter,
    })
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn parse_optional_usize(value: &str) -> Option<usize> {
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

fn chapter_page_label(
    chapter_number: Option<usize>,
    page_number: Option<usize>,
    page_count: Option<usize>,
) -> Option<ReaderChapterPageLabel> {
    Some(ReaderChapterPageLabel {
        chapter_number: chapter_number?,
        page_number: page_number?,
        page_count: page_count?,
    })
}

fn parse_location_list(text: &str, limit: usize) -> Result<Vec<ReaderLocation>, String> {
    let mut version = None;
    let mut output = Vec::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("version=") {
            version = Some(value);
        } else if let Some(value) = line.strip_prefix("entry=") {
            if output.len() < limit {
                output.push(parse_location_fields(value)?);
            }
        }
    }
    if version != Some(READER_PERSISTENCE_VERSION) {
        return Err("unsupported persistence version".into());
    }
    Ok(output)
}

fn serialize_anchor_cache(cache: &ReaderAnchorCache) -> String {
    let mut output = format!(
        "version={}\nfingerprint={:016X}\nbase_page={}\nindexed_through={}\ncomplete={}\n",
        READER_CACHE_VERSION,
        cache.fingerprint,
        cache.base_page,
        cache.indexed_through,
        cache.complete
    );
    for offset in cache.offsets.iter().take(READER_CACHE_OFFSET_LIMIT) {
        output.push_str(&format!("offset={offset}\n"));
    }
    output
}

fn parse_anchor_cache(
    text: &str,
    book: &ReaderBook,
    layout: ReaderLayout,
) -> Result<ReaderAnchorCache, String> {
    let mut version = None;
    let mut fingerprint = None;
    let mut base_page = None;
    let mut indexed_through: Option<u64> = None;
    let mut complete = None;
    let mut offsets = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "version" => version = Some(value),
            "fingerprint" => fingerprint = u64::from_str_radix(value, 16).ok(),
            "base_page" => base_page = value.parse().ok(),
            "indexed_through" => indexed_through = value.parse().ok(),
            "complete" => complete = value.parse().ok(),
            "offset" if offsets.len() < READER_CACHE_OFFSET_LIMIT => {
                offsets.push(
                    value
                        .parse()
                        .map_err(|_| "invalid cache offset".to_string())?,
                );
            }
            _ => {}
        }
    }
    if version != Some(READER_CACHE_VERSION) {
        return Err("unsupported cache version".into());
    }
    let fingerprint = fingerprint.ok_or_else(|| "missing cache fingerprint".to_string())?;
    if fingerprint != book_fingerprint(book, layout) {
        return Err("cache fingerprint mismatch".into());
    }
    if offsets.is_empty() {
        return Err("cache contains no offsets".into());
    }
    if offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("cache offsets are not strictly increasing".into());
    }
    if offsets.iter().any(|offset| *offset > book.size_bytes) {
        return Err("cache offset exceeds book size".into());
    }
    Ok(ReaderAnchorCache {
        fingerprint,
        base_page: base_page.ok_or_else(|| "missing base page".to_string())?,
        offsets,
        indexed_through: indexed_through
            .ok_or_else(|| "missing indexed offset".to_string())?
            .min(book.size_bytes),
        complete: complete.ok_or_else(|| "missing complete flag".to_string())?,
    })
}

fn load_preferences(path: &Path) -> Result<Option<ReaderPreferences>, String> {
    load_with_backup(path, ReaderPreferences::parse)
}

fn load_location_record(path: &Path) -> Result<Option<ReaderLocation>, String> {
    load_with_backup(path, parse_location_record)
}

fn load_location_list(path: &Path, limit: usize) -> Result<Vec<ReaderLocation>, String> {
    load_with_backup(path, |text| parse_location_list(text, limit))
        .map(|value| value.unwrap_or_default())
}

fn load_anchor_cache(
    path: &Path,
    book: &ReaderBook,
    layout: ReaderLayout,
) -> Result<Option<ReaderAnchorCache>, String> {
    load_with_backup(path, |text| parse_anchor_cache(text, book, layout))
}

fn load_epub_index(path: &Path, book: &ReaderBook) -> Result<Option<EpubDocument>, String> {
    load_with_backup(path, |text| parse_epub_index(text, book))
}

/// Identity of one EPUB file and of the flattening that produced its offsets.
fn epub_index_fingerprint(book: &ReaderBook) -> u64 {
    let size = book.size_bytes.to_le_bytes();
    let modified = book.modified_seconds.to_le_bytes();
    let parts: [&[u8]; 5] = [
        book.path.as_bytes(),
        &size,
        &modified,
        READER_EPUB_INDEX_VERSION.as_bytes(),
        EPUB_TEXT_VERSION.as_bytes(),
    ];
    let mut hash = CACHE_FNV_OFFSET;
    for byte in parts.into_iter().flatten() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(CACHE_FNV_PRIME);
    }
    hash
}

fn serialize_epub_index(document: &EpubDocument, book: &ReaderBook) -> String {
    let mut output = format!(
        "version={}\nfingerprint={:016X}\ntitle={}\nlanguage={}\nspine_count={}\ntext_size={}\n",
        READER_EPUB_INDEX_VERSION,
        epub_index_fingerprint(book),
        escape_field(&document.title),
        escape_field(&document.language),
        document.spine_count,
        document.text_size
    );
    for chapter in &document.chapters {
        output.push_str(&format!(
            "chapter={}\t{}\t{}\t{}\t{}\t{}\n",
            chapter.number,
            chapter.spine_index,
            chapter.text_offset,
            chapter.text_end_offset,
            escape_field(&chapter.member),
            escape_field(&chapter.label)
        ));
    }
    for entry in &document.toc {
        output.push_str(&format!(
            "toc={}\t{}\t{}\n",
            entry.spine_index,
            entry.text_offset,
            escape_field(&entry.label)
        ));
    }
    output
}

fn parse_epub_index(text: &str, book: &ReaderBook) -> Result<EpubDocument, String> {
    let mut version = None;
    let mut fingerprint = None;
    let mut title = None;
    let mut language = String::new();
    let mut spine_count = None;
    let mut text_size = None;
    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "version" => version = Some(value),
            "fingerprint" => fingerprint = u64::from_str_radix(value, 16).ok(),
            "title" => title = Some(unescape_field(value)?),
            "language" => language = unescape_field(value)?,
            "spine_count" => spine_count = value.parse().ok(),
            "text_size" => text_size = value.parse().ok(),
            "chapter" if chapters.len() < EPUB_SPINE_LIMIT => {
                chapters.push(parse_epub_index_chapter(value)?);
            }
            "toc" if toc.len() < EPUB_TOC_LIMIT => {
                toc.push(parse_epub_index_toc(value)?);
            }
            _ => {}
        }
    }
    if version != Some(READER_EPUB_INDEX_VERSION) {
        return Err("unsupported EPUB index version".into());
    }
    if fingerprint != Some(epub_index_fingerprint(book)) {
        return Err("EPUB index fingerprint mismatch".into());
    }
    let text_size: u64 = text_size.ok_or_else(|| "missing EPUB text size".to_string())?;
    validate_epub_chapters(&chapters, text_size)?;
    if toc.iter().any(|entry| entry.text_offset > text_size) {
        return Err("EPUB index TOC offset exceeds the text".into());
    }
    Ok(EpubDocument {
        path: book.path.clone(),
        title: title.ok_or_else(|| "missing EPUB title".to_string())?,
        language,
        text_size,
        toc,
        chapters,
        spine_count: spine_count.ok_or_else(|| "missing EPUB spine count".to_string())?,
    })
}

fn parse_epub_index_chapter(value: &str) -> Result<EpubChapter, String> {
    let fields: Vec<&str> = value.split('\t').collect();
    let &[number, spine, start, end, member, label] = fields.as_slice() else {
        return Err("invalid EPUB index chapter".into());
    };
    Ok(EpubChapter {
        number: parse_index_number(number)?,
        label: unescape_field(label)?,
        text_offset: parse_index_number(start)?,
        text_end_offset: parse_index_number(end)?,
        spine_index: parse_index_number(spine)?,
        member: unescape_field(member)?,
    })
}

fn parse_epub_index_toc(value: &str) -> Result<EpubTocEntry, String> {
    let fields: Vec<&str> = value.split('\t').collect();
    let &[spine, offset, label] = fields.as_slice() else {
        return Err("invalid EPUB index TOC entry".into());
    };
    Ok(EpubTocEntry {
        label: unescape_field(label)?,
        text_offset: parse_index_number(offset)?,
        spine_index: parse_index_number(spine)?,
    })
}

fn parse_index_number<T: std::str::FromStr>(value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid EPUB index number: {value}"))
}

fn validate_epub_chapters(chapters: &[EpubChapter], text_size: u64) -> Result<(), String> {
    if chapters.is_empty() {
        return Err("EPUB index has no chapters".into());
    }
    let mut previous_end = 0;
    for (index, chapter) in chapters.iter().enumerate() {
        let ordered = chapter.number == index + 1
            && chapter.text_offset >= previous_end
            && chapter.text_offset < chapter.text_end_offset
            && chapter.text_end_offset <= text_size;
        if !ordered {
            return Err("EPUB index chapters are out of order".into());
        }
        previous_end = chapter.text_end_offset;
    }
    Ok(())
}

fn load_with_backup<T>(
    path: &Path,
    parser: impl Fn(&str) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let backup = with_extension(path, "BAK");
    let mut errors = Vec::new();
    for candidate in [path.to_path_buf(), backup] {
        if !candidate.exists() {
            continue;
        }
        match fs::read_to_string(&candidate) {
            Ok(text) => match parser(&text) {
                Ok(value) => return Ok(Some(value)),
                Err(error) => errors.push(format!("{}: {error}", candidate.display())),
            },
            Err(error) => errors.push(format!("{}: {error}", candidate.display())),
        }
    }
    if errors.is_empty() {
        Ok(None)
    } else {
        Err(errors.join("; "))
    }
}

fn is_fat83_safe_file_name(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some((stem, extension)) = file_name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && stem.len() <= 8
        && !extension.is_empty()
        && extension.len() <= 3
        && stem
            .bytes()
            .chain(extension.bytes())
            .all(|value| value.is_ascii_alphanumeric() || value == b'_')
}

/// Power-safe bounded text replacement for Reader-owned state. The previous
/// primary is retained as .BAK until the new .TMP file has been renamed into
/// place. Readers accept the backup if startup observes an interrupted write.
fn atomic_replace_text(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "state path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let temp = with_extension(path, "TMP");
    let backup = with_extension(path, "BAK");
    for candidate in [path, temp.as_path(), backup.as_path()] {
        if !is_fat83_safe_file_name(candidate) {
            return Err(format!(
                "Reader state filename is not FAT 8.3 safe: {}",
                candidate.display()
            ));
        }
    }
    let _ = fs::remove_file(&temp);
    let _ = fs::remove_file(&backup);
    {
        let mut file =
            File::create(&temp).map_err(|error| format!("create {}: {error}", temp.display()))?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("write {}: {error}", temp.display()))?;
        file.sync_all()
            .map_err(|error| format!("sync {}: {error}", temp.display()))?;
    }
    if path.exists() {
        fs::rename(path, &backup).map_err(|error| format!("backup {}: {error}", path.display()))?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!("replace {}: {error}", path.display()));
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

fn with_extension(path: &Path, extension: &str) -> PathBuf {
    let mut output = path.to_path_buf();
    output.set_extension(extension);
    output
}

fn escape_field(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            value => output.push(value),
        }
    }
    output
}

fn unescape_field(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            match character {
                '\\' => output.push('\\'),
                't' => output.push('\t'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                _ => return Err("invalid escape sequence".into()),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        return Err("trailing escape sequence".into());
    }
    Ok(output)
}

fn split_escaped_tabs(value: &str) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push('\\');
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '\t' {
            output.push(unescape_field(&current)?);
            current.clear();
        } else {
            current.push(character);
        }
    }
    if escaped {
        return Err("trailing escape sequence".into());
    }
    output.push(unescape_field(&current)?);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::{
        atomic_replace_text, book_format_from_path, detect_txt_encoding, is_fat83_safe_file_name,
        load_location_record, normalize_decoded, parse_location_fields, parse_location_record,
        scan_txt_library, serialize_location, serialize_location_fields, BookFont, BookFontSize,
        BookFormat, ParagraphAlignment, ReaderBook, ReaderChapterPageLabel, ReaderLoadingStage,
        ReaderLocation, ReaderOrientation, ReaderPreferences, ReaderSession, ReaderTickOutcome,
        ReaderUiState, ReadingPreference, ReadingTheme, TextEncoding, LEGACY_READER_POSITIONS_FILE,
        READER_BOOKMARKS_FILE, READER_POSITIONS_FILE, READER_PREFS_FILE, READER_RECENT_FILE,
        READER_STATE_FILE,
    };
    use crate::{buttons::ButtonEvent, hyphenation::Language};

    fn temp_dir(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("rustmix-reader-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn detects_txt_epub_and_short_epu_aliases() {
        assert_eq!(
            book_format_from_path(PathBuf::from("a.TXT").as_path()),
            Some(BookFormat::Text)
        );
        assert_eq!(
            book_format_from_path(PathBuf::from("a.epub").as_path()),
            Some(BookFormat::Epub)
        );
        assert_eq!(
            book_format_from_path(PathBuf::from("a.EPU").as_path()),
            Some(BookFormat::Epub)
        );
    }

    #[test]
    fn detects_utf8_bom_and_windows_1252() {
        let root = temp_dir("encoding");
        let bom = root.join("bom.txt");
        let cp = root.join("cp.txt");
        fs::write(&bom, [0xEF, 0xBB, 0xBF, b'H', b'i']).unwrap();
        fs::write(&cp, [b'H', 0x92, b'i']).unwrap();
        assert_eq!(detect_txt_encoding(&bom).unwrap(), TextEncoding::Utf8Bom);
        assert_eq!(detect_txt_encoding(&cp).unwrap(), TextEncoding::Windows1252);
    }

    #[test]
    fn scans_txt_and_epub_rows_but_ignores_other_files() {
        let root = temp_dir("scan");
        fs::write(root.join("Dracula.txt"), "hello").unwrap();
        fs::write(root.join("Later.epu"), "zip").unwrap();
        fs::write(root.join("ignore.bin"), "no").unwrap();
        let books = scan_txt_library(&root).unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].title, "Dracula");
        assert_eq!(books[1].format, BookFormat::Epub);
    }

    #[test]
    fn opening_txt_is_staged_first_page_first_and_lazy() {
        let root = temp_dir("open");
        let state = temp_dir("open-state");
        fs::write(root.join("Book.txt"), "hello world ".repeat(600)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(!reader.has_background_work());
        assert!(reader.apply_library_button(ButtonEvent::Select));
        assert!(reader.has_background_work());
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        let session = reader.session.as_ref().unwrap();
        assert_eq!(session.current_page, 0);
        assert!(!session.cache.is_empty());
        assert!(session.indexed_through > 0);
        for _ in 0..16 {
            reader.tick();
        }
        assert!(!reader.has_background_work());
    }

    #[test]
    fn persists_continue_recent_bookmarks_and_anchor_cache() {
        let root = temp_dir("persist-books");
        let state = temp_dir("persist-state");
        fs::write(root.join("Dracula.txt"), "Dracula text ".repeat(1000)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        reader.next_page();
        reader.toggle_current_bookmark();
        assert!(state.join(READER_STATE_FILE).exists());
        assert!(state.join(READER_POSITIONS_FILE).exists());
        assert!(state.join(READER_RECENT_FILE).exists());
        assert!(state.join(READER_BOOKMARKS_FILE).exists());
        assert!(state.join("CACHE").read_dir().unwrap().next().is_some());

        let mut restored = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        let report = restored.load_persistent_state();
        assert!(report.state_loaded);
        assert_eq!(report.recent_count, 1);
        assert_eq!(report.bookmark_count, 1);
        assert!(restored.request_continue());
        assert_eq!(restored.tick(), ReaderTickOutcome::FirstPageReady);
        assert_eq!(
            restored.session.as_ref().unwrap().current_absolute_page(),
            1
        );
    }

    #[test]
    fn invalid_anchor_cache_fingerprint_falls_back_to_saved_offset() {
        let root = temp_dir("fingerprint-books");
        let state = temp_dir("fingerprint-state");
        fs::write(root.join("Book.txt"), "text body ".repeat(1000)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        reader.next_page();
        let cache = state
            .join("CACHE")
            .read_dir()
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let text = fs::read_to_string(&cache).unwrap();
        fs::write(
            &cache,
            text.replace("fingerprint=", "fingerprint=0000000000000000#"),
        )
        .unwrap();

        let mut restored = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        restored.load_persistent_state();
        assert!(restored.request_continue());
        assert_eq!(restored.tick(), ReaderTickOutcome::FirstPageReady);
        assert!(restored
            .persistence_warning
            .as_deref()
            .unwrap_or("")
            .contains("TXT cache ignored"));
        assert_eq!(
            restored.session.as_ref().unwrap().current_absolute_page(),
            1
        );
    }

    #[test]
    fn bookmark_toggle_removes_existing_mark() {
        let root = temp_dir("toggle-books");
        let state = temp_dir("toggle-state");
        fs::write(root.join("Book.txt"), "text ".repeat(100)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        reader.toggle_current_bookmark();
        assert_eq!(reader.bookmarks.len(), 1);
        reader.toggle_current_bookmark();
        assert!(reader.bookmarks.is_empty());
    }

    #[test]
    fn interrupted_atomic_replace_recovers_backup() {
        let root = temp_dir("backup");
        let state = root.join(READER_STATE_FILE);
        atomic_replace_text(
            &state,
            "version=1\npath=a.txt\ntitle=A\nformat=txt\nsize=1\nmodified=0\npage=0\noffset=0\n",
        )
        .unwrap();
        let backup = root.join("STATE.BAK");
        fs::rename(&state, &backup).unwrap();
        let restored = load_location_record(&state).unwrap().unwrap();
        assert_eq!(restored.title, "A");
    }

    #[test]
    fn corrupt_primary_falls_back_to_backup() {
        let root = temp_dir("corrupt");
        let state = root.join(READER_STATE_FILE);
        fs::write(&state, "not-valid").unwrap();
        fs::write(
            root.join("STATE.BAK"),
            "version=1\npath=b.txt\ntitle=B\nformat=txt\nsize=2\nmodified=0\npage=3\noffset=4\n",
        )
        .unwrap();
        let restored = load_location_record(&state).unwrap().unwrap();
        assert_eq!(restored.title, "B");
    }

    #[test]
    fn reader_options_request_manual_clear_ghosting() {
        let mut reader = ReaderUiState::default();
        reader.request_clear_ghosting();
        assert!(reader.take_clear_ghost_request());
        assert!(!reader.take_clear_ghost_request());
    }

    #[test]
    fn keeps_spanish_text_and_punctuation_and_strips_simple_emphasis() {
        let decoded: Vec<(char, u64)> = "“En vérité!” _I_—once… ¿Señor? ¡Ahí! ca\u{ad}sā"
            .chars()
            .enumerate()
            .map(|(index, value)| (value, index as u64 + 1))
            .collect();
        let normalized: String = normalize_decoded(&decoded)
            .into_iter()
            .map(|(value, _)| value)
            .collect();
        assert_eq!(normalized, "“En vérité!” I—once… ¿Señor? ¡Ahí! cas?");
    }

    #[test]
    fn removes_multiline_gutenberg_emphasis_but_preserves_safe_underscores() {
        let decoded: Vec<(char, u64)> =
            "'_You have lost your\ngold pencil-case? Couragez!'_ file_name\n_____"
                .chars()
                .enumerate()
                .map(|(index, value)| (value, index as u64 + 1))
                .collect();
        let normalized: String = normalize_decoded(&decoded)
            .into_iter()
            .map(|(value, _)| value)
            .collect();
        assert_eq!(
            normalized,
            "'You have lost your\ngold pencil-case? Couragez!' file_name\n_____"
        );
    }

    #[test]
    fn theme_switch_keeps_layout_geometry_and_cache_fingerprint_inputs_stable() {
        let classic = ReaderPreferences::default();
        let mut contrast = classic;
        contrast.theme = ReadingTheme::HighContrast;
        assert_eq!(classic.layout(), contrast.layout());
    }

    #[test]
    fn reader_font_cycle_preserves_legacy_keys_and_adds_literata() {
        assert_eq!(
            BookFont::AtkinsonHyperlegible.marker(),
            "atkinson-hyperlegible"
        );
        assert_eq!(BookFont::Serif.marker(), "serif");
        assert_eq!(BookFont::Literata.marker(), "literata");
        assert_eq!(BookFont::Inter.next(), BookFont::AtkinsonHyperlegible);
        assert_eq!(BookFont::AtkinsonHyperlegible.next(), BookFont::Serif);
        assert_eq!(BookFont::Serif.next(), BookFont::Literata);
        assert_eq!(BookFont::Literata.next(), BookFont::Inter);
        assert_eq!(BookFont::Inter.previous(), BookFont::Literata);
        assert_eq!(BookFont::parse("literata").unwrap(), BookFont::Literata);
    }

    #[test]
    fn parses_serializes_and_cycles_reader_preferences() {
        let parsed = ReaderPreferences::parse(
            "version=1\ntheme=high-contrast\norientation=landscape\nfont_size=xlarge\nbook_font=serif\nparagraph_alignment=right\nshow_progress=false\n",
        )
        .unwrap();
        assert_eq!(parsed.theme, ReadingTheme::HighContrast);
        assert_eq!(parsed.orientation, ReaderOrientation::Landscape);
        assert_eq!(parsed.font_size, BookFontSize::XLarge);
        assert_eq!(parsed.book_font, BookFont::Serif);
        assert_eq!(parsed.paragraph_alignment, ParagraphAlignment::Right);
        assert!(!parsed.show_progress);
        assert!(parsed.serialized().contains("font_size=xlarge"));
        assert!(parsed.serialized().contains("book_font=serif"));
        assert!(parsed.serialized().contains("paragraph_alignment=right"));
    }

    #[test]
    fn layout_changes_request_first_page_first_rebuild_and_persist_preferences() {
        let root = temp_dir("prefs-books");
        let state = temp_dir("prefs-state");
        fs::write(root.join("Book.txt"), "hello world ".repeat(800)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        reader.begin_preferences_edit();
        reader.cycle_preference_next();
        reader.cycle_preference_next();
        reader.open_preference_picker();
        let large = BookFontSize::ALL
            .iter()
            .position(|&size| size == BookFontSize::Large)
            .unwrap();
        assert!(reader.choose_preference(large));
        assert_eq!(
            reader.loading_stage(),
            Some(ReaderLoadingStage::UpdatingLayout)
        );
        assert!(state.join(READER_PREFS_FILE).exists());
        assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
        assert_eq!(
            reader.session.as_ref().unwrap().layout,
            reader.preferences.layout()
        );
    }

    #[test]
    fn books_and_files_reopen_from_per_book_positions_while_bookmarks_remain_explicit() {
        let root = temp_dir("positions-books");
        let state = temp_dir("positions-state");
        fs::write(root.join("A.txt"), "alpha body ".repeat(1200)).unwrap();
        fs::write(root.join("B.txt"), "beta body ".repeat(1200)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        for _ in 0..3 {
            reader.tick();
        }
        reader.next_page();
        reader.next_page();
        let saved = reader.session.as_ref().unwrap().current_location();
        assert_eq!(saved.page_index, 2);

        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        for _ in 0..4 {
            reader.tick();
        }
        assert_eq!(reader.session.as_ref().unwrap().current_absolute_page(), 2);

        let mut explicit = saved.clone();
        explicit.page_index = 1;
        explicit.byte_offset = reader.session.as_ref().unwrap().page_offsets[1];
        reader.bookmarks = vec![explicit];
        assert!(reader.request_open_bookmark(0));
        for _ in 0..4 {
            reader.tick();
        }
        assert_eq!(reader.session.as_ref().unwrap().current_absolute_page(), 1);
    }

    #[test]
    fn paragraph_alignment_defaults_to_justified_and_changes_cache_fingerprint_inputs() {
        let justified = ReaderPreferences::default();
        assert_eq!(justified.paragraph_alignment, ParagraphAlignment::Justified);
        let mut left = justified;
        left.paragraph_alignment = ParagraphAlignment::Left;
        assert_ne!(justified.layout(), left.layout());
    }

    #[test]
    fn preference_editor_opens_a_picker_and_applies_the_highlighted_value() {
        let mut reader = ReaderUiState::default();
        reader.begin_preferences_edit();
        assert_eq!(
            reader.selected_preference(),
            ReadingPreference::ReadingTheme
        );
        reader.cycle_preference_next();
        assert_eq!(reader.selected_preference(), ReadingPreference::Orientation);
        reader.cycle_preference_previous();
        assert_eq!(
            reader.selected_preference(),
            ReadingPreference::ReadingTheme
        );
        reader.open_preference_picker();
        assert_eq!(reader.preferences_picker, Some(0));
        assert_eq!(reader.preferences.theme, ReadingTheme::Classic);
        assert!(!reader.choose_preference(1));
        assert_eq!(reader.preferences.theme, ReadingTheme::HighContrast);
        assert_eq!(reader.preferences_picker, None);
    }

    #[test]
    fn reader_owned_runtime_filenames_are_fat83_safe() {
        for name in [
            READER_STATE_FILE,
            READER_POSITIONS_FILE,
            READER_RECENT_FILE,
            READER_BOOKMARKS_FILE,
            READER_PREFS_FILE,
            "ED9B69AF.CCH",
            "ED9B69AF.TMP",
            "ED9B69AF.BAK",
        ] {
            assert!(is_fat83_safe_file_name(Path::new(name)), "{name}");
        }
        assert!(!is_fat83_safe_file_name(Path::new(
            LEGACY_READER_POSITIONS_FILE
        )));
        assert!(!is_fat83_safe_file_name(Path::new("BED9B69AF.CCH")));
    }

    #[test]
    fn cache_filename_uses_exactly_eight_hexadecimal_characters() {
        let root = temp_dir("fat83-cache-books");
        let state = temp_dir("fat83-cache-state");
        fs::write(root.join("Book.txt"), "text body ".repeat(1000)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        let book = reader.books.first().unwrap();
        let cache = reader.cache_path_for(book, reader.preferences.layout());
        let file = cache.file_name().unwrap().to_str().unwrap();
        assert_eq!(file.len(), 12);
        assert_eq!(&file[8..], ".CCH");
        assert!(file[..8].bytes().all(|value| value.is_ascii_hexdigit()));
        assert!(is_fat83_safe_file_name(&cache));
    }

    #[test]
    fn legacy_positions_file_migrates_to_short_name_safe_primary() {
        let root = temp_dir("legacy-positions-books");
        let state = temp_dir("legacy-positions-state");
        fs::write(root.join("Book.txt"), "text body ".repeat(1000)).unwrap();
        let legacy = state.join(LEGACY_READER_POSITIONS_FILE);
        fs::write(
            &legacy,
            "version=1\nentry=Book.txt\tBook\ttxt\t1000\t0\t3\t42\n",
        )
        .unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        let report = reader.load_persistent_state();
        assert_eq!(report.position_count, 1);
        assert!(state.join(READER_POSITIONS_FILE).exists());
        assert_eq!(reader.positions[0].byte_offset, 42);
    }

    #[test]
    fn fat83_runtime_primary_temp_and_backup_paths_are_safe_without_cache_prefix() {
        let root = temp_dir("fat83-runtime-books");
        let state = temp_dir("fat83-runtime-state");
        fs::write(root.join("Book.txt"), "text body ".repeat(1000)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        let book = reader.books.first().unwrap();
        let positions = reader.positions_path();
        let cache = reader.cache_path_for(book, reader.preferences.layout());
        for path in [
            positions.clone(),
            super::with_extension(&positions, "TMP"),
            super::with_extension(&positions, "BAK"),
            cache.clone(),
            super::with_extension(&cache, "TMP"),
            super::with_extension(&cache, "BAK"),
        ] {
            assert!(is_fat83_safe_file_name(&path), "{}", path.display());
        }
        let cache_file = cache.file_name().unwrap().to_str().unwrap();
        assert_eq!(&cache_file[8..], ".CCH");
        assert!(
            !cache_file.starts_with('B')
                || cache_file[..8]
                    .bytes()
                    .all(|value| value.is_ascii_hexdigit())
        );
        assert_eq!(cache_file[..8].len(), 8);
    }

    #[test]
    fn bookmark_page_label_uses_active_layout_offsets_and_stored_fallback() {
        let book = ReaderBook {
            path: "Book.txt".into(),
            title: "Book".into(),
            format: BookFormat::Text,
            size_bytes: 1000,
            modified_seconds: 0,
        };
        let bookmark = ReaderLocation {
            path: book.path.clone(),
            title: book.title.clone(),
            format: book.format,
            size_bytes: book.size_bytes,
            modified_seconds: book.modified_seconds,
            page_index: 8,
            byte_offset: 220,
            epub_chapter: None,
        };
        let mut reader = ReaderUiState::default();
        assert_eq!(reader.bookmark_display_page(&bookmark), 9);
        reader.session = Some(ReaderSession {
            book,
            encoding: TextEncoding::Utf8,
            epub_document: None,
            layout: ReaderPreferences::default().layout(),
            language: None,
            current_page: 0,
            page_number_base: 0,
            page_offsets: vec![0, 100, 200, 300],
            indexed_through: 300,
            index_complete: false,
            cache: Vec::new(),
            epub_chapter_pages: Vec::new(),
            epub_chapter: None,
        });
        assert_eq!(reader.bookmark_display_page(&bookmark), 3);
    }

    #[test]
    fn epub_chapter_labels_use_chapter_relative_page_totals_and_persist() {
        let label = ReaderChapterPageLabel {
            chapter_number: 3,
            page_number: 2,
            page_count: 9,
        };
        let location = ReaderLocation {
            path: "book.epub".into(),
            title: "Book title".into(),
            format: BookFormat::Epub,
            size_bytes: 100,
            modified_seconds: 7,
            page_index: 11,
            byte_offset: 55,
            epub_chapter: Some(label.clone()),
        };
        assert_eq!(
            parse_location_record(&serialize_location(&location)).unwrap(),
            location
        );
        assert_eq!(
            parse_location_fields(&serialize_location_fields(&location)).unwrap(),
            location
        );
        assert_eq!(label.page_text(), "2/9");
    }

    #[test]
    fn legacy_location_fields_without_chapter_metadata_remain_readable() {
        let location = parse_location_fields("book.txt\tBook\ttxt\t10\t0\t2\t5").unwrap();
        assert_eq!(location.format, BookFormat::Text);
        assert_eq!(location.epub_chapter, None);
    }

    #[test]
    fn epub_page_window_fills_a_page_of_four_byte_characters() {
        let setter = typesetter(None);
        let chapter = super::LoadedEpubChapter {
            index: 0,
            text_offset: 0,
            text: "\u{1F4D6}".repeat(4096),
        };
        let page = super::read_epub_chapter_page(&chapter, &setter, 0, 0).unwrap();
        assert_eq!(page.lines.len(), setter.lines_per_page);
    }

    fn typesetter(language: Option<Language>) -> super::Typesetter {
        super::Typesetter::new(ReaderPreferences::default().layout(), language)
    }

    fn characters(text: &str) -> Vec<(char, u64)> {
        super::decode_with_offsets(text.as_bytes(), TextEncoding::Utf8, 0)
    }

    /// Lines joined back into text, removing the hyphens added at line ends.
    fn rejoin(lines: &[super::ReaderPageLine]) -> String {
        let mut text = String::new();
        for line in lines {
            if text.ends_with('-') {
                text.pop();
            } else if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&line.text);
        }
        text
    }

    #[test]
    fn lines_wrap_by_pixel_width_between_whole_words() {
        let setter = typesetter(None);
        let text = "uno dos tres cuatro cinco seis siete ocho nueve diez ".repeat(6);
        let (lines, next) = super::compose_page(&characters(&text), 0, true, &setter);
        assert_eq!(next, text.len() as u64);
        assert_eq!(rejoin(&lines), text.trim_end());
        for line in &lines {
            assert!(setter.style.text_width(&line.text) <= setter.line_width);
        }
        for pair in lines.windows(2) {
            let next_word = pair[1].text.split(' ').next().unwrap();
            let longer = format!("{} {next_word}", pair[0].text);
            assert!(setter.style.text_width(&longer) > setter.line_width);
            assert!(!pair[0].paragraph_end);
        }
        assert!(lines.last().unwrap().paragraph_end);
    }

    #[test]
    fn only_a_known_language_hyphenates_a_word_that_does_not_fit() {
        let long = "extraordinariamente";
        let spanish = typesetter(Some(Language::Spanish));
        let style = spanish.style;
        let mut text = String::new();
        while style.text_width(&text) < spanish.line_width - style.text_width(long) / 2 {
            text.push_str("a ");
        }
        text.push_str(long);
        let (lines, _) = super::compose_page(&characters(&text), 0, true, &spanish);
        assert!(lines[0].text.ends_with('-'), "{lines:?}");
        assert!(style.text_width(&lines[0].text) <= spanish.line_width);
        assert_eq!(rejoin(&lines), text);

        let (lines, _) = super::compose_page(&characters(&text), 0, true, &typesetter(None));
        assert_eq!(lines[1].text, long);
    }

    #[test]
    fn words_wider_than_a_line_split_where_they_overflow() {
        let setter = typesetter(None);
        let token = "x".repeat(200);
        let (lines, _) = super::compose_page(&characters(&token), 0, true, &setter);
        assert!(lines.len() > 1);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<String>(),
            token
        );
        for line in &lines {
            assert!(setter.style.text_width(&line.text) <= setter.line_width);
        }
    }

    #[test]
    fn newlines_end_paragraphs_and_keep_blank_lines() {
        let text = characters("uno\n\ndos");
        let (lines, _) = super::compose_page(&text, 0, true, &typesetter(None));
        let lines: Vec<_> = lines
            .iter()
            .map(|line| (line.text.as_str(), line.paragraph_end))
            .collect();
        assert_eq!(lines, [("uno", true), ("", true), ("dos", true)]);
    }

    #[test]
    fn a_word_cut_by_the_read_window_waits_for_the_next_page() {
        let text = characters("uno dos tres");
        let (lines, next) = super::compose_page(&text[..10], 0, false, &typesetter(None));
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "uno dos");
        assert_eq!(next, 8);
    }

    #[test]
    fn consecutive_pages_resume_at_the_next_word() {
        let setter = typesetter(Some(Language::Spanish));
        let text = "Él dijo: «¿Señor, qué haré?» y respondió con muchísimo cuidado. ".repeat(80);
        let all = characters(&text);
        let mut offset = 0;
        let mut pages = 0;
        let mut lines = Vec::new();
        while offset < text.len() as u64 {
            let start = all.partition_point(|(_, next)| *next <= offset);
            let (page, next) = super::compose_page(&all[start..], offset, true, &setter);
            assert!(next > offset);
            assert!(text.is_char_boundary(next as usize));
            assert!(page.len() <= setter.lines_per_page);
            lines.extend(page);
            offset = next;
            pages += 1;
        }
        assert!(pages > 1);
        assert_eq!(rejoin(&lines), text.trim_end());
    }

    #[test]
    fn epub_index_round_trips_and_rejects_changed_books() {
        let root = temp_dir("epub-index");
        let path = root.join("Libro.epub");
        let bytes = crate::epub::sample_epub(&["uno".into(), "dos".into()]);
        fs::write(&path, bytes).unwrap();
        let document = crate::epub::open_epub(&path).unwrap();
        let book = ReaderBook {
            path: document.path.clone(),
            title: "Libro".into(),
            format: BookFormat::Epub,
            size_bytes: 10,
            modified_seconds: 5,
        };
        let text = super::serialize_epub_index(&document, &book);
        assert_eq!(super::parse_epub_index(&text, &book).unwrap(), document);
        let changed = ReaderBook {
            modified_seconds: 6,
            ..book
        };
        assert!(super::parse_epub_index(&text, &changed).is_err());
    }

    #[test]
    fn epub_pages_cross_chapters_and_the_index_is_saved() {
        let root = temp_dir("epub-books");
        let state = temp_dir("epub-state");
        let bodies = ["uno ".repeat(500), "dos ".repeat(500)];
        fs::write(root.join("Libro.epub"), crate::epub::sample_epub(&bodies)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 1;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        for _ in 0..8 {
            if reader.tick() == ReaderTickOutcome::FirstPageReady {
                break;
            }
        }
        let first_chapter_pages = reader.session.as_ref().unwrap().page_offsets.len();
        assert!(first_chapter_pages > 1);
        assert_eq!(
            reader.session.as_ref().unwrap().language,
            Some(Language::Spanish)
        );
        for _ in 0..first_chapter_pages {
            reader.next_page();
        }
        let label = current_chapter_label(&reader);
        assert_eq!((label.chapter_number, label.page_number), (2, 1));
        reader.previous_page();
        let label = current_chapter_label(&reader);
        assert_eq!(label.chapter_number, 1);
        assert_eq!(label.page_number, first_chapter_pages);
        let index_saved = state
            .join("CACHE")
            .read_dir()
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".EPX"));
        assert!(index_saved);
    }

    fn current_chapter_label(reader: &ReaderUiState) -> ReaderChapterPageLabel {
        reader
            .session
            .as_ref()
            .unwrap()
            .current_epub_chapter_page_label()
            .unwrap()
    }

    #[test]
    fn library_skips_hidden_macos_companion_files() {
        let root = temp_dir("hidden-books");
        fs::write(root.join("Libro.txt"), "Hola").unwrap();
        fs::write(root.join("._Libro.txt"), [0_u8; 4]).unwrap();
        fs::write(root.join("._Otro.epub"), [0_u8; 4]).unwrap();
        let books = scan_txt_library(&root).unwrap();
        let titles: Vec<_> = books.iter().map(|book| book.title.as_str()).collect();
        assert_eq!(titles, ["Libro"]);
    }

    #[test]
    fn opening_another_book_releases_the_active_session_before_loading() {
        let root = temp_dir("release-session-books");
        let state = temp_dir("release-session-state");
        let first = root.join("First.txt");
        let second = root.join("Second.txt");
        fs::write(&first, "first book body ".repeat(100)).unwrap();
        fs::write(&second, "second book body ".repeat(100)).unwrap();
        let mut reader = ReaderUiState::with_roots(
            root.to_string_lossy().into_owned(),
            state.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        let first_path = first.to_string_lossy();
        let second_path = second.to_string_lossy();
        let first_book = reader
            .books
            .iter()
            .find(|book| book.path == first_path.as_ref())
            .unwrap()
            .clone();
        let second_book = reader
            .books
            .iter()
            .find(|book| book.path == second_path.as_ref())
            .unwrap()
            .clone();
        reader.request_open_book(first_book, None);
        while reader.tick() != ReaderTickOutcome::FirstPageReady {}
        assert!(reader.session.is_some());
        reader.request_open_book(second_book, None);
        assert!(reader.session.is_none());
        assert_eq!(
            reader.loading_stage(),
            Some(ReaderLoadingStage::OpeningFile)
        );
    }

    #[test]
    fn repeated_degraded_persistence_events_are_suppressed_until_status_changes() {
        let mut reader = ReaderUiState::default();
        reader.finish_persistence("anchor-cache", vec!["CACHE: failed".into()]);
        assert!(reader.take_persistence_event().is_some());
        reader.finish_persistence("anchor-cache", vec!["CACHE: failed".into()]);
        assert!(reader.take_persistence_event().is_none());
        reader.finish_persistence("anchor-cache", Vec::new());
        assert_eq!(
            reader.take_persistence_event().as_deref(),
            Some("status=saved scope=anchor-cache")
        );
    }
}
