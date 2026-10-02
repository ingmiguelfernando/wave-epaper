//! Bottom key-hint bar: black key caps followed by short action labels.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle, Triangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, UiTextRole, UiTextStyle},
    },
    orientation::OrientedFrameBuffer,
};

/// Top edge of the key-hint bar; screen content should stay above it.
pub const KEY_HINTS_TOP: i32 = 752;
const CAP_HEIGHT: i32 = 22;

/// Physical control drawn as a key cap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCap {
    /// The rocker: up and down.
    UpDown,
    /// Pressing the rocker.
    Select,
    Boot,
    Power,
}

pub fn draw_key_hints(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    hints: &[(KeyCap, &str)],
) -> Result<(), Infallible> {
    Rectangle::new(Point::new(0, KEY_HINTS_TOP), Size::new(480, 3))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let label_style = preferences.body_style();
    let cap_style = preferences.text_style(UiTextRole::Detail, BinaryColor::Off);
    let center = (KEY_HINTS_TOP + 3 + 800) / 2;
    let mut x = 14;
    for &(cap, label) in hints {
        x = draw_key_cap(display, cap, Point::new(x, center), cap_style)? + 6;
        let origin = Point::new(x, center + label_style.cap_height() / 2);
        Text::new(label, origin, label_style).draw(display)?;
        x += label_style.text_width(label) + 16;
    }
    Ok(())
}

/// Draw one key cap whose left edge and vertical center are at `anchor`.
/// Returns the cap's right edge.
fn draw_key_cap(
    display: &mut OrientedFrameBuffer<'_>,
    cap: KeyCap,
    anchor: Point,
    text_style: UiTextStyle,
) -> Result<i32, Infallible> {
    let text = match cap {
        KeyCap::Boot => "BOOT",
        KeyCap::Power => "PWR",
        KeyCap::UpDown | KeyCap::Select => "",
    };
    let width = match cap {
        KeyCap::UpDown => 34,
        KeyCap::Select => 24,
        KeyCap::Boot | KeyCap::Power => text_style.text_width(text) + 12,
    };
    let top_left = Point::new(anchor.x, anchor.y - CAP_HEIGHT / 2);
    let body = Rectangle::new(top_left, Size::new(width as u32, CAP_HEIGHT as u32));
    RoundedRectangle::with_equal_corners(body, Size::new(5, 5))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;

    let paper = PrimitiveStyle::with_fill(BinaryColor::Off);
    let (x, y) = (anchor.x, anchor.y);
    match cap {
        KeyCap::UpDown => {
            let up = Triangle::new(
                Point::new(x + 4, y + 4),
                Point::new(x + 14, y + 4),
                Point::new(x + 9, y - 5),
            );
            up.into_styled(paper).draw(display)?;
            let down = Triangle::new(
                Point::new(x + 20, y - 4),
                Point::new(x + 30, y - 4),
                Point::new(x + 25, y + 5),
            );
            down.into_styled(paper).draw(display)?;
        }
        KeyCap::Select => {
            Circle::with_center(Point::new(x + width / 2, y), 10)
                .into_styled(paper)
                .draw(display)?;
        }
        KeyCap::Boot | KeyCap::Power => {
            let origin = Point::new(x + 6, y + text_style.cap_height() / 2);
            Text::new(text, origin, text_style).draw(display)?;
        }
    }
    Ok(x + width)
}
