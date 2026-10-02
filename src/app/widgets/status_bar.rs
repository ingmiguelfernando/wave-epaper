//! Black status bar shared by the Wave screens: title left, status right.

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
    },
    orientation::OrientedFrameBuffer,
};

pub const STATUS_BAR_HEIGHT: i32 = 44;
/// Right edge for status text, matching the 16 px side margin.
pub const STATUS_BAR_RIGHT: i32 = 480 - 16;

pub fn draw_status_bar(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    title: &str,
) -> Result<(), Infallible> {
    Rectangle::new(Point::zero(), Size::new(480, STATUS_BAR_HEIGHT as u32))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let style = preferences.text_style(UiTextRole::Heading, BinaryColor::Off);
    Text::new(title, Point::new(16, baseline(style)), style).draw(display)?;
    Ok(())
}

/// Draw right-aligned status text ending at `right` and return its left edge.
pub fn draw_status_text(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    text: &str,
    right: i32,
) -> Result<i32, Infallible> {
    let style = preferences.text_style(UiTextRole::Body, BinaryColor::Off);
    let left = right - style.text_width(text);
    Text::new(text, Point::new(left, baseline(style)), style).draw(display)?;
    Ok(left)
}

fn baseline(style: UiTextStyle) -> i32 {
    (STATUS_BAR_HEIGHT + style.cap_height()) / 2
}
