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
        typography::{Text, TextBounds, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            header::draw_header,
        },
    },
    bible_nav::{BibleNav, BibleNavView},
    orientation::OrientedFrameBuffer,
};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;

/// Paper-colored text for the inverted chip, row and chapter cell.
fn inverse_text(preferences: DisplayPreferences) -> UiTextStyle {
    preferences.text_style(UiTextRole::Body, BinaryColor::Off)
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
    let heading = preferences.heading_style();
    let large = preferences.large_style();
    let testament = if nav.selected_book().is_some_and(|book| book.number >= 40) {
        "New Testament"
    } else {
        "Old Testament"
    };
    let section = nav.section();
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
    let first = selected.saturating_sub(2);
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
        Text::new(&body.fit(&line, RIGHT - LEFT), Point::new(LEFT, 700), body).draw(display)?;
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
    Text::new(
        &detail.fit(&format!("{} chapters", book.chapters), RIGHT - LEFT),
        Point::new(LEFT, 200),
        detail,
    )
    .draw(display)?;

    const PER_ROW: usize = 8;
    const CELL: i32 = 50;
    const PITCH: i32 = 54;
    let total = usize::from(book.chapters);
    let current = usize::from(nav.chapter_cursor().max(1));
    let first = current.saturating_sub(1) / (PER_ROW * 6) * (PER_ROW * 6);
    for index in 0..(PER_ROW * 6) {
        let number = first + index + 1;
        if number > total {
            break;
        }
        let column = (index % PER_ROW) as i32;
        let row = (index / PER_ROW) as i32;
        let left = 16 + column * PITCH;
        let top = 240 + row * PITCH;
        let caption = number.to_string();
        if number == current {
            Rectangle::new(Point::new(left, top), Size::new(CELL as u32, CELL as u32))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
            let style = inverse_text(preferences);
            Text::new(&caption, Point::new(left + 8, top + 36), style).draw(display)?;
        } else {
            Rectangle::new(Point::new(left, top), Size::new(CELL as u32, CELL as u32))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            Text::new(
                &caption,
                Point::new(left + 8, top + 36),
                preferences.body_style(),
            )
            .draw(display)?;
        }
    }
    draw_bottom_bar(display, preferences, &CHAPTERS_HINTS)
}

fn draw_tab_strip(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    current: usize,
) -> Result<(), Infallible> {
    let detail = preferences.detail_style();
    let inverse = inverse_text(preferences);
    let mut x = LEFT;
    for (index, section) in crate::bible_nav::SECTIONS.iter().enumerate() {
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
        inverse_text(preferences)
    } else {
        preferences.heading_style()
    };
    let chapters_style = if selected {
        inverse_text(preferences)
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
        bible::parse_books,
        bible_nav::BibleNav,
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    fn sample_nav() -> BibleNav {
        BibleNav::new(parse_books(&crate::bible_nav::sample_books_txt()).unwrap())
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
}
