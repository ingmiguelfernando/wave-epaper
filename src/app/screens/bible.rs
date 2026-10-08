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
        display::DisplayPreferences,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            header::draw_header,
        },
    },
    bible::Testament,
    bible_nav::{BibleNav, BibleNavView, SECTIONS},
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

pub const BOOKS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "book"),
    (KeyCap::Select, "chapter"),
    (KeyCap::Boot, "section >"),
];

pub const CHAPTERS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "chapter"),
    (KeyCap::Select, "read"),
    (KeyCap::Boot, "+10"),
];

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
    draw_header(display, preferences, "GO TO · BOOK", translation)?;
    let detail = preferences.detail_style();
    let large = preferences.large_style();
    let section = nav.section();
    let testament = match Testament::from_book_number(section.first) {
        Some(Testament::New) => "New Testament",
        _ => "Old Testament",
    };
    Text::new(
        &detail.fit(
            &format!("{testament} · section {} of 8", nav.section_cursor() + 1),
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
    draw_tab_strip(display, preferences, nav.section_cursor())?;

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
        let line = format!("Next section (BOOT): {name} · {}", names.join(", "));
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
    draw_header(display, preferences, "GO TO · CHAPTER", translation)?;
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
        format!("Chapters {}–{last} of {total}", first + 1)
    } else {
        format!("{total} chapters")
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

fn draw_tab_strip(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    current: usize,
) -> Result<(), Infallible> {
    let detail = preferences.detail_style();
    let inverse = inverse_text(preferences, UiTextRole::Detail);
    let mut x = LEFT;
    for (index, section) in SECTIONS.iter().enumerate() {
        let width = detail.text_width(section.tab) + 16;
        if index == current {
            Rectangle::new(Point::new(x, 216), Size::new(width as u32, 34))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            Text::new(section.tab, Point::new(x + 8, 240), inverse).draw(display)?;
        } else {
            Rectangle::new(Point::new(x, 216), Size::new(width as u32, 34))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            Text::new(section.tab, Point::new(x + 8, 240), detail).draw(display)?;
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
    let caption = format!("{chapters} ch.");
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
