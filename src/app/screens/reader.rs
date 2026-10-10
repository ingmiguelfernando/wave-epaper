//! Reader landing, library, bookmarks, loading, TXT / EPUB page, TOC and options screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle, RoundedRectangle},
};

use crate::{
    app::{
        display::{DisplayPreferences, UiFontSize},
        reader_typography::reader_body_style,
        state::AppState,
        typography::{Text, TextBounds, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{
                draw_bottom_bar, hints_right_edge, KeyCap, BACK_HINTS, BOTTOM_BAR_HEIGHT,
                CHANGE_HINTS, CHOOSE_HINTS, OPEN_HINTS,
            },
            header::draw_header,
            option_list::draw_option_list,
            reading_header::{draw_reading_header, READING_HEADER_HEIGHT},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    reader::{
        BookFontSize, BookFormat, ParagraphAlignment, ReaderChapterPageLabel, ReaderLibraryTab,
        ReaderLoadingStage, ReaderOption, ReaderSession, ReadingPreference, ReadingTheme,
        CHAPTER_TITLE_LINES, READER_BODY_INSET,
    },
    reading_stats::DayStats,
};

const READER_PAGE_HINTS: [(KeyCap, &str); 2] = [(KeyCap::UpDown, "page"), (KeyCap::Select, "menu")];

const RESUME_HINTS: [(KeyCap, &str); 2] =
    [(KeyCap::Select, "resume"), (KeyCap::Boot, "hold: back")];

const LIBRARY_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "book"),
    (KeyCap::Select, "read"),
    (KeyCap::Boot, "tab"),
];

const LOADING_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "hold: cancel")];

const MENU_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "activate"),
    (KeyCap::Boot, "hold: back"),
];

pub fn render_continue_reading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "CONTINUE READING",
        "PERSISTENT BOOK RESUME",
    )?;
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    if let Some(session) = state.reader.session.as_ref() {
        Text::new(
            &truncate(&session.book.title, 38),
            Point::new(24, 190),
            heading,
        )
        .draw(display)?;
        Text::new(
            &format!("{} is open.", session.display_page_label()),
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new("SELECT resumes the open page.", Point::new(24, 284), body).draw(display)?;
    } else if let Some(resume) = state.reader.resume.as_ref() {
        Text::new(&truncate(&resume.title, 38), Point::new(24, 190), heading).draw(display)?;
        Text::new(
            &format!("{} is ready to restore.", resume.display_page_label()),
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new(
            "SELECT loads the saved position.",
            Point::new(24, 284),
            body,
        )
        .draw(display)?;
    } else {
        Text::new("No saved book", Point::new(24, 190), heading).draw(display)?;
        Text::new(
            "Open Library and choose a TXT book.",
            Point::new(24, 240),
            body,
        )
        .draw(display)?;
        Text::new(
            "The last-read page is stored on the SD card.",
            Point::new(24, 284),
            body,
        )
        .draw(display)?;
    }
    draw_bottom_bar(display, state.display, &RESUME_HINTS)
}

/// Top of the tab chips, under the status bar.
const LIBRARY_TABS_TOP: i32 = STATUS_BAR_HEIGHT + 14;
/// Top of the first row, under the chips.
const LIBRARY_ROWS_TOP: i32 = LIBRARY_TABS_TOP + 54;
/// A book row holds the title line and the place line.
const LIBRARY_BOOK_ROW_HEIGHT: i32 = 72;

pub fn render_library(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let reader = &state.reader;
    let body = state.display.body_style();
    draw_status_bar(display, state.display, "Library")?;
    let count = library_count(
        reader.library_tab,
        reader.books.len(),
        reader.bookmarks.len(),
    );
    draw_status_text(display, state.display, &count, STATUS_BAR_RIGHT)?;
    draw_tabs(display, state, reader.library_tab)?;

    let visible = reader.visible_entries();
    if visible.is_empty() {
        let message = reader
            .library_error
            .as_deref()
            .unwrap_or(match reader.library_tab {
                ReaderLibraryTab::Recent => "No recent books yet.",
                ReaderLibraryTab::Bookmarks => "No saved bookmarks yet.",
                _ => "Copy TXT or EPUB books into /RUSTMIX/BOOKS.",
            });
        Text::new(
            &truncate(message, 54),
            Point::new(26, LIBRARY_ROWS_TOP + 40),
            body,
        )
        .draw(display)?;
    }
    let (step, rows) = (LIBRARY_BOOK_ROW_HEIGHT + 8, 7);
    // Scroll so the selected row stays on screen.
    let first = reader.library_selected.saturating_sub(rows - 1);
    for (index, entry) in visible.iter().enumerate().skip(first).take(rows) {
        let selected = reader.library_selected == index;
        let top = LIBRARY_ROWS_TOP + (index - first) as i32 * step;
        draw_book_row(display, state, top, selected, entry)?;
    }
    draw_bottom_bar(display, state.display, &LIBRARY_HINTS)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LibraryEntryColumns {
    badge: String,
    suffix: String,
}

/// The status bar's count: the books on the card, or the saved bookmarks on
/// the BOOKMARKS tab.
fn library_count(tab: ReaderLibraryTab, books: usize, bookmarks: usize) -> String {
    let (count, one, many) = if tab == ReaderLibraryTab::Bookmarks {
        (bookmarks, "bookmark", "bookmarks")
    } else {
        (books, "book", "books")
    };
    format!("{count} {}", if count == 1 { one } else { many })
}

fn library_entry_columns(
    reader: &crate::reader::ReaderUiState,
    entry: &crate::reader::ReaderLibraryEntry,
) -> LibraryEntryColumns {
    if reader.library_tab == ReaderLibraryTab::Bookmarks {
        let page = entry
            .location
            .as_ref()
            .map_or(1, |bookmark| reader.bookmark_display_page(bookmark));
        if let Some(chapter) = entry
            .location
            .as_ref()
            .and_then(|bookmark| reader.bookmark_display_chapter_page(bookmark))
        {
            LibraryEntryColumns {
                badge: format!("CH {}", chapter.chapter_number),
                suffix: format!("P {}", chapter.page_text()),
            }
        } else {
            LibraryEntryColumns {
                badge: "PAGE".into(),
                suffix: page.to_string(),
            }
        }
    } else {
        LibraryEntryColumns {
            badge: entry.book.format.badge().into(),
            suffix: "OPEN".into(),
        }
    }
}

fn bookmark_entry_columns(
    reader: &crate::reader::ReaderUiState,
    bookmark: &crate::reader::ReaderLocation,
) -> LibraryEntryColumns {
    if let Some(chapter) = reader.bookmark_display_chapter_page(bookmark) {
        LibraryEntryColumns {
            badge: format!("CH {}", chapter.chapter_number),
            suffix: format!("P {}", chapter.page_text()),
        }
    } else {
        LibraryEntryColumns {
            badge: "PAGE".into(),
            suffix: reader.bookmark_display_page(bookmark).to_string(),
        }
    }
}

pub fn render_bookmarks(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "BOOKMARKS",
        "PERSISTENT READER MARKS",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "MARKS.TXT",
            middle: &format!("{} saved", state.reader.bookmarks.len()),
            right: "SD FILE",
        },
    )?;
    let body = state.display.body_style();
    if state.reader.bookmarks.is_empty() {
        Text::new(
            "No saved bookmarks",
            Point::new(24, 210),
            state.display.heading_style(),
        )
        .draw(display)?;
        Text::new(
            "Open a Reader page, choose Reader Options,",
            Point::new(24, 264),
            body,
        )
        .draw(display)?;
        Text::new(
            "then select Add / Remove Bookmark.",
            Point::new(24, 306),
            body,
        )
        .draw(display)?;
    } else {
        for (index, bookmark) in state.reader.bookmarks.iter().take(8).enumerate() {
            let top = 164 + index as i32 * 64;
            let columns = bookmark_entry_columns(&state.reader, bookmark);
            draw_row(
                display,
                state,
                top,
                state.reader.bookmarks_selected == index,
                &truncate(&bookmark.title, 23),
                columns.badge.as_str(),
                columns.suffix.as_str(),
            )?;
        }
    }
    draw_bottom_bar(display, state.display, &OPEN_HINTS)
}

pub fn render_loading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "OPENING BOOK",
        "RESPONSIVE FIRST-PAGE-FIRST CACHE",
    )?;
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let loading = state.reader.loading.as_ref();
    let title = loading.map_or("Book", |value| value.book.title.as_str());
    let stage = loading.map_or(ReaderLoadingStage::OpeningFile, |value| value.stage);
    Text::new(&truncate(title, 36), Point::new(24, 176), heading).draw(display)?;
    Text::new(stage.label(), Point::new(24, 238), body).draw(display)?;
    draw_progress(display, stage.progress())?;
    let message = loading.map_or("Preparing reader...", |value| value.message.as_str());
    Text::new(&truncate(message, 52), Point::new(24, 356), body).draw(display)?;
    Text::new(
        "The current page opens before full indexing.",
        Point::new(24, 410),
        body,
    )
    .draw(display)?;
    draw_bottom_bar(display, state.display, &LOADING_HINTS)
}

pub fn render_page(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(session) = state.reader.session.as_ref() else {
        return render_continue_reading(display, state);
    };
    let size = display.orientation().logical_size();
    let width = size.width as i32;
    let height = size.height as i32;
    let landscape = width > height;
    let body = page_body_geometry(width, height);
    let body_style = reader_body_style(
        state.reader.preferences.book_font,
        state.reader.preferences.font_size,
        state.reader.preferences.theme,
    );

    let marked = state.reader.current_page_is_bookmarked();
    let status = format!(
        "{}  {}",
        state.board.time_label(state.regional),
        state.board.battery_label()
    );
    draw_reading_header(
        display,
        state.display,
        width,
        &truncate(&session.book.title, if landscape { 52 } else { 30 }),
        &status,
        marked,
    )?;

    if state.reader.preferences.theme == ReadingTheme::HighContrast {
        Rectangle::new(
            Point::new(body.frame.left, body.frame.top),
            Size::new(body.frame.width() as u32, body.frame.height() as u32),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    }

    // A chapter's first page opens with its title; pagination kept its room.
    let line_step = i32::from(body_style.line_height()) + 2;
    let chapter_first_page = session
        .current_epub_chapter_page_label()
        .is_some_and(|label| label.page_number == 1);
    let title_band = if chapter_first_page {
        CHAPTER_TITLE_LINES as i32 * line_step
    } else {
        0
    };
    if chapter_first_page {
        if let Some(chapter_title) = session.current_epub_chapter_label() {
            let large = state.display.large_style();
            let text_width = body.text.right - body.text.left;
            Text::new(
                &large.fit(chapter_title, text_width),
                Point::new(
                    body.text.left,
                    body.text.top + i32::from(large.line_height()) - 4,
                ),
                large,
            )
            .draw(display)?;
            // The rule sits close under the title, as in the mockup; the text
            // starts after the band pagination kept free.
            Rectangle::new(
                Point::new(
                    body.text.left,
                    body.text.top + i32::from(large.line_height()) + 8,
                ),
                Size::new(text_width as u32, 2),
            )
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
        }
    }

    if let Some(page) = session.current_cached_page() {
        let first_baseline = body.text.top + title_band + i32::from(body_style.line_height());
        for (index, line) in page
            .lines
            .iter()
            .take(session.layout.lines_per_page)
            .enumerate()
        {
            let baseline = first_baseline + index as i32 * line_step;
            if baseline >= body.text.bottom {
                break;
            }
            for (run, left) in reader_line_runs(
                line.text.as_str(),
                line.paragraph_end,
                session.layout.paragraph_alignment,
                body_style,
                body.text,
            ) {
                Text::new(run, Point::new(left, baseline), body_style)
                    .draw_clipped(display, body.text)?;
            }
        }
    } else {
        let baseline = body.text.top + i32::from(body_style.line_height());
        Text::new(
            "Preparing page...",
            Point::new(body.text.left, baseline),
            body_style,
        )
        .draw_clipped(display, body.text)?;
    }

    let footer_label = if state.reader.preferences.show_progress {
        // The label may carry the minutes left in the chapter; it drops them
        // when it would run into the bar's hints.
        let week = state.local_day().map_or_else(DayStats::default, |today| {
            state.reading_stats.total(today.saturating_sub(6), today)
        });
        // Body size keeps the percent glyph whole (KNOWN_ISSUES).
        let label_style = state.display.body_style();
        let full = reader_progress_label(session, week);
        if fits_beside_hints(&full, label_style, width, state.display) {
            full
        } else {
            reader_progress_label(session, DayStats::default())
        }
    } else {
        String::new()
    };
    if !footer_label.is_empty() {
        // Same baseline as the bar's words: centred on the bar, cap height down.
        let label_style = state.display.body_style();
        let label_width = label_style.text_width(&footer_label);
        let bar_center = (height - BOTTOM_BAR_HEIGHT + 3 + height) / 2;
        let baseline = bar_center + label_style.cap_height() / 2;
        Text::new(
            &footer_label,
            Point::new(width - 18 - label_width, baseline),
            label_style,
        )
        .draw(display)?;
    }
    draw_bottom_bar(display, state.display, &READER_PAGE_HINTS)?;
    Ok(())
}

/// Whether a right-aligned footer label ends before the bar's hints begin.
fn fits_beside_hints(
    label: &str,
    style: UiTextStyle,
    width: i32,
    preferences: DisplayPreferences,
) -> bool {
    width - 18 - style.text_width(label) >= hints_right_edge(&READER_PAGE_HINTS, preferences)
}

/// Footer label for a page: the chapter and percent for an EPUB, the page and
/// percent for a TXT book. The percent is the place in the whole book. The
/// minutes left count an EPUB chapter's remaining pages at the week's pace.
fn reader_progress_label(session: &ReaderSession, week: DayStats) -> String {
    progress_label(
        session.current_epub_chapter_page_label().as_ref(),
        session.current_absolute_page() + 1,
        session.place_percent(),
        week,
    )
}

/// The footer label from its parts: `Ch. 8 · 12% · 9 min left` for an EPUB
/// chapter, `p. 41 · 12%` for a page of a TXT book.
fn progress_label(
    chapter: Option<&ReaderChapterPageLabel>,
    page: usize,
    percent: u8,
    week: DayStats,
) -> String {
    let base = match chapter {
        Some(chapter) => format!("Ch. {} · {percent}%", chapter.chapter_number),
        None => format!("p. {page} · {percent}%"),
    };
    match pace_text(chapter, week) {
        Some(pace) => format!("{base} · {pace}"),
        None => base,
    }
}

/// Recorded pages in the week needed before the pace is worth showing.
const MIN_PACE_PAGES: u32 = 20;

/// Minutes left in an EPUB chapter at the week's own pace: the pages left in
/// the chapter times the week's seconds per page, rounded up. A remainder
/// under a minute reads `< 1 min left`. `None` for a TXT book or when the
/// week recorded fewer than `MIN_PACE_PAGES` pages.
fn pace_text(chapter: Option<&ReaderChapterPageLabel>, week: DayStats) -> Option<String> {
    let chapter = chapter?;
    if week.pages < MIN_PACE_PAGES {
        return None;
    }
    let pages_left = chapter.page_count.saturating_sub(chapter.page_number) as u64;
    // Minutes left = ceil(pages_left × week seconds / (week pages × 60)).
    let numerator = pages_left * u64::from(week.seconds);
    let denominator = u64::from(week.pages) * 60;
    if numerator < denominator {
        return Some("< 1 min left".to_string());
    }
    let minutes = (numerator + denominator - 1) / denominator;
    Some(format!("{minutes} min left"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReaderBodyGeometry {
    text: TextBounds,
    frame: ReaderFrameBounds,
}

/// The text box of a Reader page: right under the reading header's rule and
/// above the bottom bar, in either orientation.
const fn page_body_geometry(width: i32, height: i32) -> ReaderBodyGeometry {
    ReaderBodyGeometry::new(width, READING_HEADER_HEIGHT + 6, 0, height - 54)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReaderFrameBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl ReaderFrameBounds {
    #[must_use]
    const fn width(self) -> i32 {
        self.right - self.left
    }

    #[must_use]
    const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

impl ReaderBodyGeometry {
    /// Shared Reader body rectangle used by Classic and High Contrast. The
    /// stronger High Contrast frame stays outside this viewport, so switching
    /// themes never changes TXT pagination or cache fingerprints. Pagination
    /// wraps lines to the same width (`ReaderLayout::line_width`).
    #[must_use]
    const fn new(width: i32, status_top: i32, status_height: i32, footer_line: i32) -> Self {
        let text = TextBounds::new(
            READER_BODY_INSET,
            status_top + status_height + 18,
            width - READER_BODY_INSET,
            footer_line - 12,
        );
        let frame = ReaderFrameBounds {
            left: text.left - 8,
            top: text.top - 8,
            right: text.right + 8,
            bottom: text.bottom + 8,
        };
        Self { text, frame }
    }
}

pub fn render_options(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(display, state.display, "READER OPTIONS", "READER ACTIONS")?;
    for (index, option) in ReaderOption::ALL.iter().copied().enumerate() {
        let badge = match option {
            ReaderOption::Bookmark if state.reader.current_page_is_bookmarked() => "REMOVE",
            ReaderOption::Bookmark => "ADD",
            ReaderOption::Bookmarks => "LIST",
            ReaderOption::TableOfContents if state.reader.has_structured_toc() => "LIST",
            _ => option.badge(),
        };
        draw_row(
            display,
            state,
            146 + index as i32 * 66,
            state.reader.options_selected == index,
            option.label(),
            badge,
            "",
        )?;
    }
    draw_bottom_bar(display, state.display, &MENU_HINTS)
}

pub fn render_preferences(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "READING PREFERENCES",
        "SETTINGS-STYLE ROW EDITOR",
    )?;
    if let Some(highlighted) = state.reader.preferences_picker {
        let (options, current) = state.reader.preference_options();
        let heading = state.display.heading_style();
        Text::new(
            state.reader.selected_preference().label(),
            Point::new(22, 160),
            heading,
        )
        .draw(display)?;
        draw_option_list(display, state.display, 184, &options, current, highlighted)?;
        draw_bottom_bar(display, state.display, &CHOOSE_HINTS)?;
        return Ok(());
    }
    for (index, preference) in ReadingPreference::ALL.iter().copied().enumerate() {
        let badge = match preference {
            ReadingPreference::ReadingTheme => state.reader.preferences.theme.label(),
            ReadingPreference::Orientation => state.reader.preferences.orientation.label(),
            ReadingPreference::BookFontSize => state.reader.preferences.font_size.label(),
            ReadingPreference::BookFont => state.reader.preferences.book_font.label(),
            ReadingPreference::ParagraphAlignment => {
                state.reader.preferences.paragraph_alignment.label()
            }
            ReadingPreference::ShowProgress if state.reader.preferences.show_progress => "On",
            ReadingPreference::ShowProgress => "Off",
        };
        draw_row(
            display,
            state,
            156 + index as i32 * 78,
            state.reader.preferences_selected == index,
            preference.label(),
            badge,
            "",
        )?;
    }
    draw_bottom_bar(display, state.display, &CHANGE_HINTS)
}

pub fn render_toc(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(
        display,
        state.display,
        "TABLE OF CONTENTS",
        if state.reader.has_structured_toc() {
            "EPUB NAVIGATION"
        } else {
            "TXT FOUNDATION"
        },
    )?;
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let toc = state.reader.toc_entries();
    if toc.is_empty() {
        Text::new("No structured TOC", Point::new(24, 200), heading).draw(display)?;
        Text::new(
            "Ordinary TXT files do not provide a formal",
            Point::new(24, 258),
            body,
        )
        .draw(display)?;
        Text::new(
            "table of contents. EPUB books expose their",
            Point::new(24, 300),
            body,
        )
        .draw(display)?;
        Text::new(
            "navigation entries on this screen.",
            Point::new(24, 342),
            body,
        )
        .draw(display)?;
        return draw_bottom_bar(display, state.display, &BACK_HINTS);
    }

    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "EPUB TOC",
            middle: &format!("{} entries", toc.len()),
            right: "SELECT OPEN",
        },
    )?;
    let first = state.reader.toc_selected.saturating_sub(7);
    for (row, entry) in toc.iter().skip(first).take(8).enumerate() {
        let index = first + row;
        draw_row(
            display,
            state,
            166 + row as i32 * 64,
            state.reader.toc_selected == index,
            &truncate(&entry.label, 27),
            "CH",
            &(entry.spine_index + 1).to_string(),
        )?;
    }
    draw_bottom_bar(display, state.display, &OPEN_HINTS)
}

/// Runs of one Reader line with the x where each is drawn. A justified line
/// is drawn word by word so its spare width is shared pixel-exactly between
/// the gaps, the leftmost gaps taking the remainder.
fn reader_line_runs(
    line: &str,
    paragraph_end: bool,
    alignment: ParagraphAlignment,
    style: crate::app::typography::UiTextStyle,
    bounds: TextBounds,
) -> Vec<(&str, i32)> {
    let spare = (bounds.width() - style.text_width(line)).max(0);
    let left = match alignment {
        ParagraphAlignment::Left => bounds.left,
        ParagraphAlignment::Center => bounds.left + spare / 2,
        ParagraphAlignment::Right => bounds.left + spare,
        ParagraphAlignment::Justified if !paragraph_end => {
            return justified_runs(line, style, bounds);
        }
        ParagraphAlignment::Justified => bounds.left,
    };
    vec![(line, left)]
}

fn justified_runs(
    line: &str,
    style: crate::app::typography::UiTextStyle,
    bounds: TextBounds,
) -> Vec<(&str, i32)> {
    // Pagination joins words with single ASCII spaces; a no-break space stays
    // inside its word.
    let words: Vec<&str> = line.split(' ').filter(|word| !word.is_empty()).collect();
    if words.len() < 2 {
        return vec![(line, bounds.left)];
    }
    let gaps = words.len() as i32 - 1;
    let words_width: i32 = words.iter().map(|word| style.text_width(word)).sum();
    let minimum = gaps * style.text_width(" ");
    let spaces = (bounds.width() - words_width).max(minimum);
    let mut x = bounds.left;
    let mut runs = Vec::with_capacity(words.len());
    for (index, word) in words.into_iter().enumerate() {
        runs.push((word, x));
        let wider = i32::from((index as i32) < spaces % gaps);
        x += style.text_width(word) + spaces / gaps + wider;
    }
    runs
}

/// The tab chips in the order short BOOT walks them; the open tab is filled.
/// Labels drop to the detail size when the body size would not fit.
fn draw_tabs(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    active: ReaderLibraryTab,
) -> Result<(), Infallible> {
    const TABS: [ReaderLibraryTab; 4] = [
        ReaderLibraryTab::Recent,
        ReaderLibraryTab::Books,
        ReaderLibraryTab::Bookmarks,
        ReaderLibraryTab::Files,
    ];
    const PAD: i32 = 10;
    const GAP: i32 = 8;
    const HEIGHT: i32 = 36;
    let chips_width = |style: UiTextStyle| {
        TABS.iter()
            .map(|tab| style.text_width(tab.label()) + 2 * PAD)
            .sum::<i32>()
            + GAP * (TABS.len() as i32 - 1)
    };
    let body = state.display.body_style();
    let style = if chips_width(body) <= 448 {
        body
    } else {
        state.display.detail_style()
    };
    let baseline = LIBRARY_TABS_TOP + (HEIGHT + style.cap_height()) / 2;
    let mut left = 16;
    for tab in TABS {
        let width = style.text_width(tab.label()) + 2 * PAD;
        let chip = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(left, LIBRARY_TABS_TOP),
                Size::new(width as u32, HEIGHT as u32),
            ),
            Size::new(6, 6),
        );
        let ink = if tab == active {
            chip.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            BinaryColor::Off
        } else {
            chip.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            BinaryColor::On
        };
        let origin = Point::new(left + PAD, baseline);
        Text::new(tab.label(), origin, style.with_color(ink)).draw(display)?;
        left += width + GAP;
    }
    Ok(())
}

/// A book row: the title in the Reader's book face and the format chip, then
/// the place. A TXT book shows a bar and its percent; an EPUB shows its
/// chapter, since its saved offset counts the book's text, not the file; a
/// book never opened shows `new`. FILES shows the file size instead. The
/// author line waits for the EPUB parser.
/// A book row: the title with the author under it, the format chip over the
/// row's state on the right (a drawn check for a finished book, the percent,
/// `Ch. N` for an old EPUB place or `new`), and a bar under the author for a
/// book in progress. BOOKMARKS rows show their place in the state slot.
/// Row percents draw in Body size (KNOWN_ISSUES percent glyph).
fn draw_book_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    selected: bool,
    entry: &crate::reader::ReaderLibraryEntry,
) -> Result<(), Infallible> {
    let ink = if selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let book_size = if state.display.font_size == UiFontSize::Large {
        BookFontSize::Medium
    } else {
        BookFontSize::Small
    };
    let title_style = reader_body_style(
        state.reader.preferences.book_font,
        book_size,
        ReadingTheme::Classic,
    )
    .with_color(ink);
    let detail = state.display.detail_style().with_color(ink);
    // Percent glyphs need Body size (KNOWN_ISSUES).
    let body = state.display.body_style().with_color(ink);
    let frame = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(
        Point::new(20, top),
        Size::new(440, LIBRARY_BOOK_ROW_HEIGHT as u32),
    )
    .into_styled(frame)
    .draw(display)?;

    // The right column: the format chip over the row's state.
    let bookmarks = state.reader.library_tab == ReaderLibraryTab::Bookmarks;
    let chip = entry.book.format.badge();
    let chip_width = detail.text_width(chip) + 12;
    let state_text = row_state_text(state, entry, bookmarks);
    let state_width = state_text
        .as_ref()
        .map_or(0, |text| body.text_width(text).max(detail.text_width(text)));
    let column_left;
    let text_width;
    let column_width;
    (column_left, text_width, column_width) = row_column_layout(chip_width, state_width);
    let cap = detail.cap_height();
    RoundedRectangle::with_equal_corners(
        Rectangle::new(
            Point::new(column_left + (column_width - chip_width) / 2, top + 10),
            Size::new(chip_width as u32, (cap + 12) as u32),
        ),
        Size::new(4, 4),
    )
    .into_styled(PrimitiveStyle::with_stroke(ink, 1))
    .draw(display)?;
    Text::new(
        chip,
        Point::new(
            column_left + (column_width - chip_width) / 2 + 6,
            top + 10 + 6 + cap,
        ),
        detail,
    )
    .draw(display)?;

    let title_baseline = top + 30;
    let author_baseline = top + 52;
    Text::new(
        &title_style.fit(&entry.book.title, text_width),
        Point::new(34, title_baseline),
        title_style,
    )
    .draw(display)?;
    let author = row_author(&state.reader, entry);
    if !author.is_empty() {
        Text::new(
            &detail.fit(author, text_width),
            Point::new(34, author_baseline),
            detail,
        )
        .draw(display)?;
    }

    if state.reader.library_tab == ReaderLibraryTab::Files {
        let size = crate::storage::format_bytes(entry.book.size_bytes);
        Text::new(&size, Point::new(34, author_baseline), detail).draw(display)?;
        return Ok(());
    }

    // The state slot under the chip: the check replaces the percent when the
    // book is finished.
    let state_baseline = top + 52;
    if state.reading_stats.is_finished(&entry.book.path) {
        draw_check(
            display,
            ink,
            Point::new(column_left, state_baseline - cap - 4),
        )?;
    } else if let Some(text) = state_text {
        Text::new(&text, Point::new(column_left, state_baseline), body).draw(display)?;
    }
    // The bar under the author marks a book in progress.
    if !bookmarks && entry.location.is_some() {
        if let Some(percent) = row_percent(entry) {
            let bar_top = author_baseline + 8;
            let bar_width = text_width as u32;
            Rectangle::new(Point::new(34, bar_top), Size::new(bar_width, 8))
                .into_styled(PrimitiveStyle::with_stroke(ink, 1))
                .draw(display)?;
            Rectangle::new(
                Point::new(34, bar_top),
                Size::new(bar_width * u32::from(percent) / 100, 8),
            )
            .into_styled(PrimitiveStyle::with_fill(ink))
            .draw(display)?;
        }
    }
    Ok(())
}

/// Column split of a row: where the state column starts, how much room the
/// title keeps, and the state column's width, given the chip's and the state's
/// widths.
fn row_column_layout(chip_width: i32, state_width: i32) -> (i32, i32, i32) {
    let column_width = chip_width.max(state_width) + 4;
    let column_left = 20 + 440 - 12 - column_width;
    let text_width = column_left - 12 - 34;
    (column_left, text_width, column_width)
}

/// The row's state text: its place on BOOKMARKS, else the percent or the
/// chapter of an old EPUB place, `new` when the book was never opened. `None`
/// when the row draws a check instead.
fn row_state_text(
    state: &AppState,
    entry: &crate::reader::ReaderLibraryEntry,
    bookmarks: bool,
) -> Option<String> {
    if state.reading_stats.is_finished(&entry.book.path) {
        return None;
    }
    if bookmarks {
        let columns = library_entry_columns(&state.reader, entry);
        return Some(format!("{} {}", columns.badge, columns.suffix));
    }
    let Some(location) = entry.location.as_ref() else {
        return Some("new".to_string());
    };
    if let Some(percent) = row_percent(entry) {
        return Some(format!("{percent}%"));
    }
    let label = location.epub_chapter.as_ref().map_or_else(
        || "opened".to_string(),
        |chapter| format!("Ch. {}", chapter.chapter_number),
    );
    Some(label)
}

/// The row's percent: the saved place's, else a TXT book's bytes read.
fn row_percent(entry: &crate::reader::ReaderLibraryEntry) -> Option<u8> {
    let location = entry.location.as_ref()?;
    if let Some(percent) = location.place_percent {
        return Some(percent);
    }
    if entry.book.format == BookFormat::Text {
        let size = entry.book.size_bytes.max(1);
        return Some((location.byte_offset.saturating_mul(100) / size).min(100) as u8);
    }
    None
}

/// A row's author: RECENT and BOOKMARKS rows carry only the saved place, so
/// the author comes from the scanned book with the same path.
fn row_author<'a>(
    reader: &'a crate::reader::ReaderUiState,
    entry: &'a crate::reader::ReaderLibraryEntry,
) -> &'a str {
    if !entry.book.author.is_empty() {
        return &entry.book.author;
    }
    reader
        .books
        .iter()
        .find(|book| book.path == entry.book.path)
        .map_or("", |book| book.author.as_str())
}

/// A check mark, since the fonts have no `✓` glyph.
fn draw_check(
    display: &mut OrientedFrameBuffer<'_>,
    ink: BinaryColor,
    top_left: Point,
) -> Result<(), Infallible> {
    Line::new(top_left + Point::new(0, 6), top_left + Point::new(5, 11))
        .into_styled(PrimitiveStyle::with_stroke(ink, 2))
        .draw(display)?;
    Line::new(top_left + Point::new(5, 11), top_left + Point::new(15, 1))
        .into_styled(PrimitiveStyle::with_stroke(ink, 2))
        .draw(display)?;
    Ok(())
}
fn draw_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    selected: bool,
    label: &str,
    badge: &str,
    suffix: &str,
) -> Result<(), Infallible> {
    let body = if selected {
        state.display.text_style(UiTextRole::Body, BinaryColor::Off)
    } else {
        state.display.body_style()
    };
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(20, top), Size::new(440, 50))
        .into_styled(style)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(32, top + 32),
        body,
    )
    .draw(display)?;
    Text::new(label, Point::new(58, top + 32), body).draw(display)?;
    Text::new(badge, Point::new(338, top + 32), body).draw(display)?;
    Text::new(suffix, Point::new(402, top + 32), body).draw(display)?;
    Ok(())
}

fn draw_progress(display: &mut OrientedFrameBuffer<'_>, percent: u8) -> Result<(), Infallible> {
    Rectangle::new(Point::new(24, 282), Size::new(432, 38))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    let width = 4 * percent as u32;
    Rectangle::new(Point::new(30, 288), Size::new(width.min(420), 26))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    Ok(())
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    output.push_str("...");
    output
}

#[cfg(test)]
mod tests {
    use super::{
        bookmark_entry_columns, library_count, library_entry_columns, page_body_geometry,
        reader_body_style, reader_line_runs, render_bookmarks, render_continue_reading,
        render_library, render_loading, render_options, render_preferences, render_toc,
        ReaderBodyGeometry,
    };
    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
        reader::{
            BookFormat, ParagraphAlignment, PendingReaderOpen, ReaderBook, ReaderChapterPageLabel,
            ReaderLibraryEntry, ReaderLibraryTab, ReaderLoadingStage, ReaderLocation, ReadingTheme,
        },
    };

    /// Every Reader size, book face and orientation: the page's lines start
    /// under the header's rule, all of them fit the text box, and the box ends
    /// above the bottom bar.
    #[test]
    fn every_page_size_fits_between_the_header_and_the_bar() {
        use crate::{
            app::widgets::{bottom_bar::BOTTOM_BAR_HEIGHT, reading_header::READING_HEADER_HEIGHT},
            reader::{BookFont, BookFontSize, ReaderOrientation, ReaderPreferences},
        };
        for orientation in ReaderOrientation::ALL {
            let width = orientation.screen_width();
            let height = 1280 - width;
            let body = page_body_geometry(width, height);
            assert!(
                body.text.top > READING_HEADER_HEIGHT,
                "{width}: under the rule"
            );
            assert!(
                body.frame.bottom < height - BOTTOM_BAR_HEIGHT,
                "{width}: above the bar"
            );
            for font_size in BookFontSize::ALL {
                for book_font in BookFont::ALL {
                    let preferences = ReaderPreferences {
                        orientation,
                        font_size,
                        book_font,
                        ..ReaderPreferences::default()
                    };
                    let lines = preferences.layout().lines_per_page as i32;
                    let style = reader_body_style(book_font, font_size, ReadingTheme::Classic);
                    let line_height = i32::from(style.line_height());
                    let last = body.text.top + line_height + (lines - 1) * (line_height + 2);
                    assert!(
                        last < body.text.bottom,
                        "{} {} {}: line {lines} at {last} passes {}",
                        orientation.marker(),
                        font_size.label(),
                        book_font.label(),
                        body.text.bottom
                    );
                }
            }
        }
    }

    /// A chapter's first page keeps room for its title: it holds fewer lines
    /// than a later page, the title and its rule fit the band, and the lines
    /// still end above the bar, at every Reader size in both orientations.
    #[test]
    fn a_chapter_first_page_holds_its_title_and_fewer_lines() {
        use crate::{
            app::{
                display::{DisplayPreferences, UiFontFamily, UiFontSize},
                widgets::{bottom_bar::BOTTOM_BAR_HEIGHT, reading_header::READING_HEADER_HEIGHT},
            },
            reader::{BookFont, BookFontSize, ReaderOrientation, ReaderPreferences},
        };
        for orientation in ReaderOrientation::ALL {
            let width = orientation.screen_width();
            let height = 1280 - width;
            let body = super::page_body_geometry(width, height);
            assert!(
                body.text.top > READING_HEADER_HEIGHT,
                "{width}: under the rule"
            );
            assert!(
                body.frame.bottom < height - BOTTOM_BAR_HEIGHT,
                "{width}: above the bar"
            );
            for font_size in BookFontSize::ALL {
                for book_font in BookFont::ALL {
                    let preferences = ReaderPreferences {
                        orientation,
                        font_size,
                        book_font,
                        ..ReaderPreferences::default()
                    };
                    let lines = preferences.layout().lines_per_page;
                    let style = reader_body_style(book_font, font_size, ReadingTheme::Classic);
                    let line_height = i32::from(style.line_height());
                    let line_step = line_height + 2;
                    let first_lines = lines.saturating_sub(super::CHAPTER_TITLE_LINES);
                    let where_ = format!(
                        "{} {} {}",
                        orientation.marker(),
                        font_size.label(),
                        book_font.label()
                    );
                    assert!(first_lines > 0, "{where_}: the first page keeps text");
                    assert!(
                        first_lines < lines,
                        "{where_}: fewer lines than a later page"
                    );
                    let band = super::CHAPTER_TITLE_LINES as i32 * line_step;
                    let last =
                        body.text.top + band + line_height + (first_lines as i32 - 1) * line_step;
                    assert!(
                        last < body.text.bottom,
                        "{where_}: line at {last} passes {}",
                        body.text.bottom
                    );
                    // The title and its rule fit the band at every UI font size.
                    for family in UiFontFamily::ALL {
                        for ui_size in UiFontSize::ALL {
                            let display = DisplayPreferences {
                                font_family: family,
                                font_size: ui_size,
                            };
                            let title_height = i32::from(display.large_style().line_height()) + 10;
                            assert!(
                                title_height <= band,
                                "{where_} {:?} {:?}: title of {title_height} passes the {band} band",
                                family,
                                ui_size
                            );
                        }
                    }
                }
            }
        }
    }

    /// The pace joins the label at twenty recorded pages: `9 min left` at the
    /// week's own speed, `< 1 min left` under a minute, and nothing before.
    #[test]
    fn the_page_label_keeps_its_pace_only_with_twenty_pages() {
        use crate::reading_stats::DayStats;
        let chapter = ReaderChapterPageLabel {
            chapter_number: 8,
            page_number: 3,
            page_count: 12,
        };
        // Twenty pages at a minute each: nine pages left, nine minutes.
        let week = DayStats {
            seconds: 1_200,
            pages: 20,
        };
        assert_eq!(
            super::progress_label(Some(&chapter), 1, 12, week),
            "Ch. 8 · 12% · 9 min left"
        );
        let quiet = DayStats {
            seconds: 1_140,
            pages: 19,
        };
        assert_eq!(
            super::progress_label(Some(&chapter), 1, 12, quiet),
            "Ch. 8 · 12%"
        );
        // A TXT page carries no pace, and a short remainder reads its words.
        assert_eq!(super::progress_label(None, 41, 12, week), "p. 41 · 12%");
        let last_page = ReaderChapterPageLabel {
            chapter_number: 8,
            page_number: 11,
            page_count: 12,
        };
        let half_minute = DayStats {
            seconds: 30,
            pages: 20,
        };
        assert_eq!(
            super::progress_label(Some(&last_page), 1, 12, half_minute),
            "Ch. 8 · 12% · < 1 min left"
        );
    }

    /// At the Large size the drawn label ends before the bar's hints: the
    /// minutes are dropped when the full label would run into them.
    #[test]
    fn the_label_fits_beside_the_hints_at_large() {
        use crate::app::display::{DisplayPreferences, UiFontFamily, UiFontSize};
        for family in UiFontFamily::ALL {
            let preferences = DisplayPreferences {
                font_family: family,
                font_size: UiFontSize::Large,
            };
            let style = preferences.body_style();
            let full = "Ch. 8 · 12% · 9 min left";
            let short = "Ch. 8 · 12%";
            assert!(
                super::fits_beside_hints(short, style, 480, preferences),
                "{family:?}: the label without minutes must fit"
            );
            let chosen = if super::fits_beside_hints(full, style, 480, preferences) {
                full
            } else {
                short
            };
            assert!(
                480 - 18 - style.text_width(chosen)
                    >= crate::app::widgets::bottom_bar::hints_right_edge(
                        &super::READER_PAGE_HINTS,
                        preferences,
                    ),
                "{family:?}: {chosen:?} runs into the hints"
            );
        }
    }

    /// The chapter title's rule draws on a chapter's first page and not on a
    /// later one: its row is solid across the text width only there.
    #[test]
    fn the_title_rule_draws_only_on_a_chapters_first_page() {
        use crate::app::widgets::reading_header::READING_HEADER_HEIGHT;
        use embedded_graphics::prelude::Point;
        let root = std::env::temp_dir().join(format!("wave-rule-{}", std::process::id()));
        let books = root.join("BOOKS");
        let state_root = root.join("READER");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&books).unwrap();
        std::fs::create_dir_all(&state_root).unwrap();
        let bodies = ["uno ".repeat(500), "dos ".repeat(500)];
        std::fs::write(books.join("Libro.epub"), crate::epub::sample_epub(&bodies)).unwrap();
        let mut reader = crate::reader::ReaderUiState::with_roots(
            books.to_string_lossy().into_owned(),
            state_root.to_string_lossy().into_owned(),
        );
        reader.refresh_library();
        reader.library_selected = 0;
        assert!(reader.apply_library_button(ButtonEvent::Select));
        for _ in 0..8 {
            if reader.tick() == crate::reader::ReaderTickOutcome::FirstPageReady {
                break;
            }
        }
        let mut state = AppState::default();
        state.reader = reader;
        state.router.navigate_to(ScreenRoute::ReaderPage);
        let rule_row = READING_HEADER_HEIGHT
            + 6
            + 18
            + i32::from(state.display.large_style().line_height())
            + 8;
        let rule_runs_across = |state: &AppState| {
            let mut frame = FrameBuffer::new_white();
            render_current_screen(&mut frame, state).unwrap();
            // Portrait logical (x, y) is native (y, 479 - x).
            (24 + 2..480 - 24 - 2)
                .all(|x| frame.is_black(Point::new(rule_row, 479 - x)) == Some(true))
        };
        assert!(
            rule_runs_across(&state),
            "the first page of the chapter draws the title's rule"
        );
        // The precondition: a later page exists inside the same chapter.
        let label = state
            .reader
            .session
            .as_ref()
            .unwrap()
            .current_epub_chapter_page_label()
            .unwrap();
        assert!(label.page_count > 1, "the chapter has a later page");
        state.reader.next_page();
        let later = state
            .reader
            .session
            .as_ref()
            .unwrap()
            .current_epub_chapter_page_label()
            .unwrap();
        assert_eq!(later.page_number, 2, "the turn stays inside the chapter");
        assert!(!rule_runs_across(&state), "a later page draws no rule");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_recent_row_takes_its_author_from_the_scanned_book() {
        let mut reader = crate::reader::ReaderUiState::default();
        reader.books = vec![ReaderBook {
            path: "/books/Libro.epub".into(),
            title: "Libro".into(),
            author: "Autor".into(),
            format: BookFormat::Epub,
            size_bytes: 10,
            modified_seconds: 1,
        }];
        // A RECENT row carries the saved place only.
        let entry = ReaderLibraryEntry {
            book: ReaderBook {
                path: "/books/Libro.epub".into(),
                title: "Libro".into(),
                author: String::new(),
                format: BookFormat::Epub,
                size_bytes: 10,
                modified_seconds: 1,
            },
            location: None,
        };
        assert_eq!(super::row_author(&reader, &entry), "Autor");
    }

    /// Every row fits at the Large size: the state column holds its widest
    /// text and the title keeps room beside it, as the row draws them.
    #[test]
    fn every_row_fits_at_large() {
        use crate::app::display::{DisplayPreferences, UiFontFamily, UiFontSize};
        for family in UiFontFamily::ALL {
            let preferences = DisplayPreferences {
                font_family: family,
                font_size: UiFontSize::Large,
            };
            let detail = preferences.detail_style();
            let body = preferences.body_style();
            let chip_width = detail.text_width("EPUB") + 12;
            let state_width = body.text_width("100%").max(detail.text_width("Ch. 00"));
            let (column_left, text_width, _column_width) =
                super::row_column_layout(chip_width, state_width);
            assert!(
                text_width >= 180,
                "{family:?}: the title keeps only {text_width} px"
            );
            assert!(column_left > 34, "{family:?}: the columns overlap");
        }
    }

    #[test]
    fn preference_picker_wraps_applies_and_draws() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::ReaderPreferences);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.reader.preferences_picker, Some(0));
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        state.apply(ButtonEvent::Up);
        assert_eq!(state.reader.preferences_picker, Some(1));
        state.apply(ButtonEvent::Select);
        assert_eq!(state.reader.preferences.theme, ReadingTheme::HighContrast);
        assert_eq!(state.reader.preferences_picker, None);
        assert_eq!(state.active_route(), ScreenRoute::ReaderPreferences);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn back_closes_the_preference_picker_before_leaving_the_editor() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::ReaderPreferences);
        let initial_theme = state.reader.preferences.theme;
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Up);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::ReaderPreferences);
        assert_eq!(state.reader.preferences_picker, None);
        assert_eq!(state.reader.preferences.theme, initial_theme);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::ReaderOptions);
    }

    #[test]
    fn high_contrast_frame_stays_outside_shared_text_viewport() {
        let body = ReaderBodyGeometry::new(480, 80, 42, 746);
        assert!(body.frame.left < body.text.left);
        assert!(body.frame.top < body.text.top);
        assert!(body.frame.right > body.text.right);
        assert!(body.frame.bottom > body.text.bottom);
        assert_eq!(body.text.left, 24);
        assert_eq!(body.text.right, 456);
    }

    #[test]
    fn library_bookmarks_tab_uses_saved_status_and_page_columns() {
        let bookmark = ReaderLocation {
            path: "POIROT~1.TXT".into(),
            title: "POIROT~1".into(),
            format: BookFormat::Text,
            size_bytes: 123,
            modified_seconds: 456,
            byte_offset: 789,
            page_index: 11,
            epub_chapter: None,
            place_percent: None,
        };
        let mut reader = crate::reader::ReaderUiState::default();
        reader.library_tab = ReaderLibraryTab::Bookmarks;
        let entry = ReaderLibraryEntry {
            book: bookmark.as_book(),
            location: Some(bookmark),
        };
        assert_eq!(
            library_count(ReaderLibraryTab::Bookmarks, 24, 9),
            "9 bookmarks"
        );
        assert_eq!(library_count(ReaderLibraryTab::Recent, 24, 9), "24 books");
        assert_eq!(library_count(ReaderLibraryTab::Books, 1, 0), "1 book");
        assert_eq!(
            library_entry_columns(&reader, &entry),
            super::LibraryEntryColumns {
                badge: "PAGE".into(),
                suffix: "12".into(),
            }
        );
    }
    #[test]
    fn epub_bookmark_columns_show_chapter_and_chapter_page_total() {
        let bookmark = ReaderLocation {
            path: "NOVEL.EPU".into(),
            title: "Novel".into(),
            format: BookFormat::Epub,
            size_bytes: 123,
            modified_seconds: 456,
            byte_offset: 789,
            page_index: 11,
            place_percent: None,
            epub_chapter: Some(ReaderChapterPageLabel {
                chapter_number: 4,
                page_number: 3,
                page_count: 12,
            }),
        };
        let reader = crate::reader::ReaderUiState::default();
        assert_eq!(
            bookmark_entry_columns(&reader, &bookmark),
            super::LibraryEntryColumns {
                badge: "CH 4".into(),
                suffix: "P 3/12".into(),
            }
        );
    }

    #[test]
    fn library_books_and_files_tabs_keep_format_and_open_columns() {
        let entry = ReaderLibraryEntry {
            book: ReaderBook {
                path: "POIROT~1.TXT".into(),
                title: "POIROT~1".into(),
                author: String::new(),
                format: BookFormat::Text,
                size_bytes: 123,
                modified_seconds: 456,
            },
            location: None,
        };
        for tab in [ReaderLibraryTab::Books, ReaderLibraryTab::Files] {
            let mut reader = crate::reader::ReaderUiState::default();
            reader.library_tab = tab;
            assert_eq!(
                library_entry_columns(&reader, &entry),
                super::LibraryEntryColumns {
                    badge: "TXT".into(),
                    suffix: "OPEN".into(),
                }
            );
        }
    }

    #[test]
    fn reader_screens_render_without_sd_card() {
        let mut state = AppState::default();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_continue_reading(&mut display, &state).unwrap();
        render_library(&mut display, &state).unwrap();
        render_bookmarks(&mut display, &state).unwrap();
        render_options(&mut display, &state).unwrap();
        render_preferences(&mut display, &state).unwrap();
        render_toc(&mut display, &state).unwrap();
        state.reader.loading = Some(PendingReaderOpen {
            book: ReaderBook {
                path: "a.txt".into(),
                title: "A".into(),
                author: String::new(),
                format: BookFormat::Text,
                size_bytes: 1,
                modified_seconds: 0,
            },
            stage: ReaderLoadingStage::OpeningFile,
            encoding: None,
            epub_document: None,
            resume: None,
            message: "Preparing".into(),
        });
        render_loading(&mut display, &state).unwrap();
    }
    #[test]
    fn paragraph_alignment_moves_or_justifies_reader_lines_inside_bounds() {
        use ParagraphAlignment::{Center, Justified, Left, Right};

        let style = AppState::default().display.body_style();
        let bounds = crate::app::typography::TextBounds::new(20, 0, 220, 100);
        let line = "short line";
        let left = reader_line_runs(line, true, Left, style, bounds)[0].1;
        let center = reader_line_runs(line, true, Center, style, bounds)[0].1;
        let right = reader_line_runs(line, true, Right, style, bounds)[0].1;
        assert!(left < center);
        assert!(center < right);
        assert_eq!(
            reader_line_runs("one two", true, Justified, style, bounds),
            [("one two", 20)]
        );

        let runs = reader_line_runs("one two three", false, Justified, style, bounds);
        let words: Vec<_> = runs.iter().map(|(word, _)| *word).collect();
        assert_eq!(words, ["one", "two", "three"]);
        assert_eq!(runs[0].1, bounds.left);
        let (last, x) = runs[2];
        assert_eq!(x + style.text_width(last), bounds.right);
        let first_gap = runs[1].1 - style.text_width("one") - runs[0].1;
        let second_gap = runs[2].1 - style.text_width("two") - runs[1].1;
        assert!((first_gap - second_gap).abs() <= 1);
    }

    #[test]
    fn reader_line_width_matches_the_rendered_text_viewport() {
        use crate::reader::{ReaderOrientation, ReaderPreferences};

        let mut preferences = ReaderPreferences::default();
        for orientation in [ReaderOrientation::Portrait, ReaderOrientation::Landscape] {
            preferences.orientation = orientation;
            let body = ReaderBodyGeometry::new(orientation.screen_width(), 80, 42, 700);
            assert_eq!(body.text.width(), preferences.layout().line_width());
        }
    }
}
