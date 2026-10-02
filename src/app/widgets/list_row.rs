//! Full-width settings-style list row: title, description and a right value.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle, Triangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, UiTextRole},
    },
    orientation::OrientedFrameBuffer,
};

pub const LIST_ROW_HEIGHT: i32 = 72;
const TEXT_LEFT: i32 = 34;
const VALUE_RIGHT: i32 = 480 - 18;

pub struct ListRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub value: &'a str,
    pub selected: bool,
}

pub fn draw_list_row(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    top: i32,
    row: ListRow<'_>,
) -> Result<(), Infallible> {
    let black = PrimitiveStyle::with_fill(BinaryColor::On);
    let ink = if row.selected {
        Rectangle::new(Point::new(0, top), Size::new(480, LIST_ROW_HEIGHT as u32))
            .into_styled(black)
            .draw(display)?;
        draw_selection_marker(display, 10, top + LIST_ROW_HEIGHT / 2)?;
        BinaryColor::Off
    } else {
        Rectangle::new(Point::new(0, top + LIST_ROW_HEIGHT - 2), Size::new(480, 2))
            .into_styled(black)
            .draw(display)?;
        BinaryColor::On
    };

    let title_style = preferences.text_style(UiTextRole::Heading, ink);
    let detail_style = preferences.text_style(UiTextRole::Detail, ink);
    let value_style = preferences.text_style(UiTextRole::Body, ink);

    let mut text_right = VALUE_RIGHT;
    if !row.value.is_empty() {
        let left = VALUE_RIGHT - value_style.text_width(row.value);
        let baseline = top + (LIST_ROW_HEIGHT + value_style.cap_height()) / 2;
        Text::new(row.value, Point::new(left, baseline), value_style).draw(display)?;
        text_right = left - 12;
    }

    let block = title_style.cap_height() + 10 + detail_style.cap_height();
    let title_baseline = top + (LIST_ROW_HEIGHT - block) / 2 + title_style.cap_height();
    let title = title_style.fit(row.title, text_right - TEXT_LEFT);
    Text::new(&title, Point::new(TEXT_LEFT, title_baseline), title_style).draw(display)?;
    if !row.subtitle.is_empty() {
        let baseline = title_baseline + 10 + detail_style.cap_height();
        let subtitle = detail_style.fit(row.subtitle, text_right - TEXT_LEFT);
        Text::new(&subtitle, Point::new(TEXT_LEFT, baseline), detail_style).draw(display)?;
    }
    Ok(())
}

/// Draw the white `>` marker that leads an inverted (selected) row.
pub fn draw_selection_marker(
    display: &mut OrientedFrameBuffer<'_>,
    left: i32,
    center_y: i32,
) -> Result<(), Infallible> {
    Triangle::new(
        Point::new(left, center_y - 7),
        Point::new(left, center_y + 7),
        Point::new(left + 8, center_y),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
    .draw(display)?;
    Ok(())
}
