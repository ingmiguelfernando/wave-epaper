//! Bible book and chapter pickers. Drawing and state only: routes and the
//! reading view come with the main line.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::{DisplayPreferences, UiFontSize},
        reader_typography::reader_body_style,
        state::AppState,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            header::draw_header,
            option_list::draw_option_list,
        },
    },
    bible::{self, BibleBook, Testament},
    bible_nav::{BibleNav, BibleNavView, Section, SECTIONS},
    bible_reader,
    orientation::OrientedFrameBuffer,
};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;

/// Paper-colored text for the inverted tab, row and chapter cell.
fn inverse_text(preferences: DisplayPreferences, role: UiTextRole) -> UiTextStyle {
    preferences.text_style(role, BinaryColor::Off)
}

/// Row band height of the book list, as in the mockup's 58 px rows.
const ROW_HEIGHT: i32 = 58;
const ROW_LEFT: i32 = 16;
const ROW_RIGHT: i32 = 464;
const BOOKS_TOP: i32 = 286;
const BOOK_ROWS_SHOWN: usize = 7;

/// Spanish bottom bar of the book picker: ▲▼ libro, ● capítulos, BOOT sección.
pub const BOOKS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "libro"),
    (KeyCap::Select, "capítulos"),
    (KeyCap::Boot, "sección ›"),
];

/// Spanish bottom bar of the chapter grid: ▲▼ capítulo, ● leer, BOOT +10.
pub const CHAPTERS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "capítulo"),
    (KeyCap::Select, "leer"),
    (KeyCap::Boot, "+10"),
];

/// Spanish testament line of the book picker.
fn testament_label(testament: Testament) -> &'static str {
    match testament {
        Testament::New => "Nuevo Testamento",
        Testament::Old => "Antiguo Testamento",
    }
}

/// Draw the picker view the navigation state currently points at.
pub fn render_bible_nav(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    translation: &str,
    nav: &BibleNav,
) -> Result<(), Infallible> {
    match nav.view() {
        BibleNavView::Books => render_books(display, preferences, translation, nav),
        BibleNavView::Chapters => render_chapters(display, preferences, translation, nav),
    }
}

fn render_books(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    translation: &str,
    nav: &BibleNav,
) -> Result<(), Infallible> {
    draw_header(display, preferences, "IR A · LIBRO", translation)?;
    let detail = preferences.detail_style();
    let large = preferences.large_style();
    let section = nav.section();
    let testament = Testament::from_book_number(section.first).unwrap_or(Testament::Old);
    Text::new(
        &detail.fit(
            &format!(
                "{} · sección {} de 8",
                testament_label(testament),
                nav.section_cursor() + 1
            ),
            RIGHT - LEFT,
        ),
        Point::new(LEFT, 132),
        detail,
    )
    .draw(display)?;
    Text::new(
        &large.fit(section.name, RIGHT - LEFT),
        Point::new(LEFT, 176),
        large,
    )
    .draw(display)?;
    draw_tab_strip(display, preferences, nav.books(), nav.section_cursor())?;

    let books = nav.section_books();
    let selected = nav.book_cursor();
    let first = selected
        .saturating_sub(2)
        .min(books.len().saturating_sub(BOOK_ROWS_SHOWN));
    for (visible, index) in (first..books.len().min(first + BOOK_ROWS_SHOWN)).enumerate() {
        let book = books[index];
        let top = BOOKS_TOP + visible as i32 * ROW_HEIGHT;
        draw_book_row(
            display,
            preferences,
            top,
            &book.name,
            book.chapters,
            index == selected,
        )?;
    }
    if let Some((cursor, name)) = nav.next_section() {
        let body = preferences.body_style();
        let names = nav.next_section_books(cursor);
        let line = format!("Siguiente (BOOT): {name} · {}", names.join(", "));
        Text::new(&body.fit(&line, RIGHT - LEFT), Point::new(LEFT, 730), body).draw(display)?;
    }
    draw_bottom_bar(display, preferences, &BOOKS_HINTS)
}

fn render_chapters(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    translation: &str,
    nav: &BibleNav,
) -> Result<(), Infallible> {
    draw_header(display, preferences, "IR A · CAPÍTULO", translation)?;
    let Some(book) = nav.selected_book() else {
        return render_books(display, preferences, translation, nav);
    };
    let heading = preferences.heading_style();
    let detail = preferences.detail_style();
    Text::new(
        &heading.fit(&book.name, RIGHT - LEFT),
        Point::new(LEFT, 160),
        heading,
    )
    .draw(display)?;

    const PER_ROW: usize = 8;
    const PER_PAGE: usize = PER_ROW * 6;
    const CELL: i32 = 50;
    const PITCH: i32 = 54;
    const GRID_LEFT: i32 = (480 - (PER_ROW as i32 * PITCH - (PITCH - CELL))) / 2;
    let total = usize::from(book.chapters);
    let current = usize::from(nav.chapter_cursor().max(1));
    let first = (current - 1) / PER_PAGE * PER_PAGE;
    let caption = if total > PER_PAGE {
        let last = (first + PER_PAGE).min(total);
        format!("Capítulos {}–{last} de {total}", first + 1)
    } else {
        format!("{total} capítulos")
    };
    Text::new(
        &detail.fit(&caption, RIGHT - LEFT),
        Point::new(LEFT, 200),
        detail,
    )
    .draw(display)?;

    for index in 0..PER_PAGE {
        let number = first + index + 1;
        if number > total {
            break;
        }
        let column = (index % PER_ROW) as i32;
        let row = (index / PER_ROW) as i32;
        let left = GRID_LEFT + column * PITCH;
        let top = 240 + row * PITCH;
        let cell = Rectangle::new(Point::new(left, top), Size::new(CELL as u32, CELL as u32));
        let style = if number == current {
            cell.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            inverse_text(preferences, UiTextRole::Body)
        } else {
            cell.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            preferences.body_style()
        };
        let label = number.to_string();
        let origin = Point::new(
            left + (CELL - style.text_width(&label)) / 2,
            top + (CELL + style.cap_height()) / 2,
        );
        Text::new(&label, origin, style).draw(display)?;
    }
    draw_bottom_bar(display, preferences, &CHAPTERS_HINTS)
}

/// The thumb-index label of a section: its first book's short name in capitals.
/// A section with no book on the card keeps its English code, never blank.
fn thumb_label(books: &[BibleBook], section: &Section) -> String {
    books
        .iter()
        .find(|book| (section.first..=section.last).contains(&book.number))
        .map_or_else(
            || section.tab.to_string(),
            |book| book.short_name.to_uppercase(),
        )
}

fn draw_tab_strip(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    books: &[BibleBook],
    current: usize,
) -> Result<(), Infallible> {
    let detail = preferences.detail_style();
    let inverse = inverse_text(preferences, UiTextRole::Detail);
    let mut x = LEFT;
    for (index, section) in SECTIONS.iter().enumerate() {
        // The thumb index names each section by its first book, as a printed
        // Bible's edge does.
        let tab = thumb_label(books, section);
        let width = detail.text_width(&tab) + 16;
        if index == current {
            Rectangle::new(Point::new(x, 216), Size::new(width as u32, 34))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            Text::new(&tab, Point::new(x + 8, 240), inverse).draw(display)?;
        } else {
            Rectangle::new(Point::new(x, 216), Size::new(width as u32, 34))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            Text::new(&tab, Point::new(x + 8, 240), detail).draw(display)?;
        }
        x += width + 6;
    }
    Ok(())
}

fn draw_book_row(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    top: i32,
    name: &str,
    chapters: u16,
    selected: bool,
) -> Result<(), Infallible> {
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 2)
    };
    Rectangle::new(
        Point::new(ROW_LEFT, top),
        Size::new((ROW_RIGHT - ROW_LEFT) as u32, ROW_HEIGHT as u32),
    )
    .into_styled(style)
    .draw(display)?;
    let name_style = if selected {
        inverse_text(preferences, UiTextRole::Heading)
    } else {
        preferences.heading_style()
    };
    let chapters_style = if selected {
        inverse_text(preferences, UiTextRole::Body)
    } else {
        preferences.body_style()
    };
    Text::new(name, Point::new(ROW_LEFT + 20, top + 40), name_style).draw(display)?;
    let caption = format!("{chapters} cap.");
    let width = chapters_style.text_width(&caption);
    Text::new(
        &caption,
        Point::new(ROW_RIGHT - width - 20, top + 40),
        chapters_style,
    )
    .draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::display::{DisplayPreferences, UiFontFamily, UiFontSize},
        bible::parse_index,
        bible_nav::BibleNav,
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    fn sample_nav() -> BibleNav {
        BibleNav::new(parse_index(&crate::bible_nav::sample_books_txt()).unwrap())
    }

    fn render(preferences: DisplayPreferences, nav: &BibleNav) -> FrameBuffer {
        let mut frame = FrameBuffer::new_white();
        {
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            render_bible_nav(&mut display, preferences, "RVR1960", nav).unwrap();
        }
        frame
    }

    #[test]
    fn books_and_chapters_render_at_every_typography_size() {
        for family in [UiFontFamily::Inter, UiFontFamily::AtkinsonHyperlegible] {
            for size in [UiFontSize::Compact, UiFontSize::Standard, UiFontSize::Large] {
                let preferences = DisplayPreferences {
                    font_family: family,
                    font_size: size,
                };
                let books = render(preferences, &sample_nav());
                let mut chapters_nav = sample_nav();
                chapters_nav.next_section_cyclic();
                chapters_nav.next_section_cyclic();
                chapters_nav.move_book(1);
                chapters_nav.open_chapters();
                let chapters = render(preferences, &chapters_nav);
                assert!(ink(&books) > 1000);
                assert!(ink(&chapters) > 1000);
            }
        }
    }

    fn ink(frame: &FrameBuffer) -> u32 {
        frame
            .as_bytes()
            .iter()
            .map(|byte| u32::from(byte.count_zeros()))
            .sum()
    }

    #[test]
    fn next_section_line_clears_a_full_book_list() {
        let mut nav = sample_nav();
        // History has twelve books, so all seven rows are drawn.
        nav.next_section_cyclic();
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::AtkinsonHyperlegible,
            font_size: UiFontSize::Large,
        };
        let frame = render(preferences, &nav);
        let rows_bottom = BOOKS_TOP + BOOK_ROWS_SHOWN as i32 * ROW_HEIGHT;
        for y in rows_bottom + 2..rows_bottom + 20 {
            for x in LEFT..RIGHT {
                // Logical (x, y) is native (y, 479 - x) in portrait.
                assert_eq!(
                    frame.is_black(Point::new(y, 479 - x)),
                    Some(false),
                    "({x}, {y})"
                );
            }
        }
    }
}

/// Book picker for the Home › Bible route and its book list. The route, not
/// the picker's own view, decides what is drawn.
pub fn render_bible_books_screen(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    match state.bible.translation() {
        Some(translation) => render_books(display, state.display, translation, state.bible.nav()),
        None => render_bible_missing(display, state),
    }
}

/// Chapter grid of the selected book.
pub fn render_bible_chapters_screen(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    match state.bible.translation() {
        Some(translation) => {
            render_chapters(display, state.display, translation, state.bible.nav())
        }
        None => render_bible_missing(display, state),
    }
}

/// Spanish bottom bar of the reading view.
pub const READING_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "página"),
    (KeyCap::Select, "menú"),
    (KeyCap::Boot, "capítulos"),
];

/// Pages of one chapter at the Reader's body style, or 1 when it cannot load.
/// The turn path and the drawing both use this, so they agree on the count.
#[must_use]
pub fn chapter_pages(state: &AppState, position: bible_reader::Position) -> usize {
    let Some((lines, per_page)) = chapter_lines(state, position) else {
        return 1;
    };
    bible_reader::paginate(lines.len(), per_page)
}

/// The page of the chapter at `position` where verse `verse` begins; the
/// first page when the chapter or the verse is not found.
#[must_use]
pub fn verse_page(state: &AppState, position: bible_reader::Position, verse: u16) -> usize {
    let Some((lines, per_page)) = chapter_lines(state, position) else {
        return 0;
    };
    let starts_verse = |line: &bible_reader::Line| {
        let span = line.verse_label().and_then(bible::verse_span);
        matches!(span, Some((first, last)) if first <= verse && verse <= last)
    };
    lines
        .iter()
        .position(starts_verse)
        .map_or(0, |index| index / per_page)
}

/// The chapter's laid-out lines and the lines per page, at the Reader's body
/// style; `None` when the chapter cannot load.
fn chapter_lines(
    state: &AppState,
    position: bible_reader::Position,
) -> Option<(Vec<bible_reader::Line>, usize)> {
    let translation = state.bible.translation()?;
    let book = state.bible.books().get(position.book)?;
    let loaded = bible::load_chapter(state.bible.root(), translation, book, position.chapter);
    let items = loaded.ok()?;
    let body = reader_body_style(
        state.reader.preferences.book_font,
        state.reader.preferences.font_size,
        state.reader.preferences.theme,
    );
    let width = RIGHT - LEFT;
    let first = verse_first_width(body, width);
    let lines = bible_reader::layout(&items, width, first, |text| body.text_width(text));
    Some((lines, reading_lines_per_page(body)))
}

/// Reading view of the open place: the chapter's verses in the Reader's body
/// style, one page at a time.
pub fn render_bible_reading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(position) = state.bible.position() else {
        return render_bible_books_screen(display, state);
    };
    let (Some(translation), Some(book)) = (
        state.bible.translation(),
        state.bible.books().get(position.book),
    ) else {
        return render_bible_missing(display, state);
    };
    let preferences = state.display;
    let body = reader_body_style(
        state.reader.preferences.book_font,
        state.reader.preferences.font_size,
        state.reader.preferences.theme,
    );
    let large = preferences.large_style();
    let width = RIGHT - LEFT;
    let items = match bible::load_chapter(state.bible.root(), translation, book, position.chapter) {
        Ok(items) => items,
        Err(_) => return render_bible_missing(display, state),
    };
    let first = verse_first_width(body, width);
    let lines = bible_reader::layout(&items, width, first, |text| body.text_width(text));
    let per_page = reading_lines_per_page(body);
    let pages = bible_reader::paginate(lines.len(), per_page);
    // The sentinel from a backward turn means "last page"; clamp it here.
    let page = position.page.min(pages - 1);
    let chapter_title = format!("{} {}", book.short_name, position.chapter);

    draw_header(display, preferences, "BIBLIA", translation)?;
    let title = format!("{} {}", book.name, position.chapter);
    Text::new(&large.fit(&title, width), Point::new(LEFT, 150), large).draw(display)?;
    Rectangle::new(Point::new(LEFT, 166), Size::new(width as u32, 2))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;

    let small = preferences.detail_style();
    // Section headings in bold capitals, at the compact size like the mockup.
    let heading = DisplayPreferences {
        font_size: UiFontSize::Compact,
        ..preferences
    }
    .heading_style();
    let mut top = READING_TOP;
    for line in lines.iter().skip(page * per_page).take(per_page) {
        top = draw_reading_line(display, body, heading, small, top, line, width)?;
    }

    let indicator = format!("{chapter_title} · {}/{pages}", page + 1);
    let detail = preferences.detail_style();
    let shown = detail.fit(&indicator, width);
    // Right-aligned above the bottom bar, as the mockup places it.
    let left = RIGHT - detail.text_width(&shown);
    Text::new(&shown, Point::new(left, READING_INDICATOR_TOP), detail).draw(display)?;
    draw_bottom_bar(display, preferences, &READING_HINTS)
}

/// Width left for a verse's first line: the full width less the widest
/// indent and a three-digit verse number, so the drawn line never overflows.
fn verse_first_width(body: UiTextStyle, width: i32) -> i32 {
    let number = body.text_width("999") + 6;
    width - 24 - number
}

/// Vertical room for verse lines: from under the title rule down to the
/// indicator line above the bottom bar.
const READING_TOP: i32 = 186;
const READING_INDICATOR_TOP: i32 = 716;

/// Whole verse lines that fit between `READING_TOP` and the indicator, at the
/// body style's pitch. Never zero, so a large face still shows a line.
fn reading_lines_per_page(body: UiTextStyle) -> usize {
    let pitch = i32::from(body.line_height()) + 8;
    let room = (READING_INDICATOR_TOP - READING_TOP).max(pitch);
    (room / pitch).max(1) as usize
}

/// Draw one laid-out line and return the top of the next one. A verse number
/// is drawn small and raised in front of its first line.
fn draw_reading_line(
    display: &mut OrientedFrameBuffer<'_>,
    body: UiTextStyle,
    heading: UiTextStyle,
    small: UiTextStyle,
    top: i32,
    line: &bible_reader::Line,
    width: i32,
) -> Result<i32, Infallible> {
    let pitch = i32::from(body.line_height()) + 8;
    let baseline = top + body.cap_height();
    match line {
        bible_reader::Line::Heading(text) => {
            let text = heading.fit(&text.to_uppercase(), width);
            Text::new(&text, Point::new(LEFT, baseline), heading).draw(display)?;
            Ok(top + pitch)
        }
        bible_reader::Line::Verse {
            number,
            paragraph,
            text,
        } => {
            let indent = if *paragraph { 24 } else { 0 };
            let mut x = LEFT + indent;
            if let Some(number) = number {
                // Small and raised: the Detail size, sitting above the baseline.
                let number = number.as_str();
                Text::new(number, Point::new(x, baseline - 8), small).draw(display)?;
                x += small.text_width(number) + 6;
            }
            Text::new(&body.fit(text, RIGHT - x), Point::new(x, baseline), body).draw(display)?;
            Ok(top + pitch)
        }
    }
}

/// Options of the reading menu. The verse of the day shows only when today's
/// verse resolves on this card; the translation switch comes later.
#[must_use]
pub fn bible_menu_items(state: &AppState) -> Vec<&'static str> {
    let mut items = vec!["Ir a libro", "Ir a capítulo"];
    let verse_today = state
        .local_day()
        .and_then(|day| state.bible.verse_of_day(day));
    if verse_today.is_some() {
        items.push("Versículo del día");
    }
    items
}

/// Menu of the reading view: one option list, the option at `highlighted`.
pub fn render_bible_menu(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    let items = bible_menu_items(state);
    let heading = preferences.heading_style();
    draw_header(display, preferences, "BIBLIA", "Menú")?;
    Text::new("Menú", Point::new(LEFT, 160), heading).draw(display)?;
    draw_option_list(
        display,
        preferences,
        184,
        &items,
        usize::MAX,
        state.bible_menu_selected,
    )?;
    draw_bottom_bar(display, preferences, &MENU_HINTS)
}

/// Spanish bottom bar of the reading menu.
pub const MENU_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "mover"),
    (KeyCap::Select, "elegir"),
    (KeyCap::Boot, "mantener: volver"),
];

/// The Spanish message for a card without Bible text.
pub fn render_bible_missing(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    draw_header(display, preferences, "BIBLIA", "SIN TEXTO")?;
    let body = preferences.body_style();
    let lines = [
        "No hay Biblia en la tarjeta.",
        "Copie la carpeta de la Biblia en /RUSTMIX/BIBLE.",
        "Vea SD_CARD_SETUP para preparar el texto.",
    ];
    for (index, line) in lines.iter().enumerate() {
        Text::new(
            &body.fit(line, RIGHT - LEFT),
            Point::new(LEFT, 200 + index as i32 * 40),
            body,
        )
        .draw(display)?;
    }
    draw_bottom_bar(display, preferences, &MISSING_HINTS)
}

/// Only BOOT is live here: it returns to the previous screen.
pub const MISSING_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "mantener: volver")];

#[cfg(test)]
mod thumb_index_tests {
    use super::{thumb_label, SECTIONS};
    use crate::bible::{parse_index, short_name};

    /// The thumb index names each section by its first book, in capitals.
    #[test]
    fn the_thumb_index_names_each_section_by_its_first_book() {
        let books = parse_index(&crate::bible_nav::sample_books_txt()).unwrap();
        let labels: Vec<String> = SECTIONS
            .iter()
            .map(|section| thumb_label(&books, section))
            .collect();
        assert_eq!(labels[0], short_name("Génesis").to_uppercase());
        assert!(labels.iter().all(|label| !label.is_empty()));
        assert!(
            labels.iter().all(|label| label == &label.to_uppercase()),
            "every tab is in capitals: {labels:?}"
        );
    }

    /// A section with no book on the card keeps its English code, never blank.
    #[test]
    fn an_empty_section_keeps_its_code() {
        let label = thumb_label(&[], &SECTIONS[0]);
        assert_eq!(label, SECTIONS[0].tab);
    }
}
