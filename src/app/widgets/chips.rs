//! Chip: a bordered label, filled when active, with an optional drawn dot.
//! The mockup's `.chip` (2 px border, capitals) and its `●` marks, which the
//! fonts do not have.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
};

use crate::{
    app::typography::{Text, UiTextStyle},
    orientation::OrientedFrameBuffer,
};

/// One chip: its label in capitals, filled when it is the active one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chip<'a> {
    pub label: &'a str,
    pub filled: bool,
    /// A drawn dot before the label, for `● RECORDING`.
    pub dot: bool,
}

impl<'a> Chip<'a> {
    #[must_use]
    pub const fn new(label: &'a str, filled: bool) -> Self {
        Self {
            label,
            filled,
            dot: false,
        }
    }

    /// The same chip with a drawn dot before its label.
    #[must_use]
    pub const fn dotted(label: &'a str, filled: bool) -> Self {
        Self {
            label,
            filled,
            dot: true,
        }
    }
}

const BORDER: i32 = 2;
const PADDING: i32 = 10;
const DOT_DIAMETER: i32 = 8;
const DOT_GAP: i32 = 6;

/// The chip's width: the border, the padding, the dot and the label.
#[must_use]
pub fn chip_width(style: UiTextStyle, chip: &Chip<'_>) -> i32 {
    let dot = if chip.dot { DOT_DIAMETER + DOT_GAP } else { 0 };
    2 * (BORDER + PADDING) + dot + style.text_width(chip.label)
}

/// The chip's height: the capitals plus the padding and the border.
#[must_use]
pub fn chip_height(style: UiTextStyle) -> i32 {
    style.cap_height() + 2 * (BORDER + 4)
}

/// Draw one chip at `top_left`; returns its width, so a row can space them.
/// The label is ExtraBold at the Body sizes (mockup `.chip` weight 800).
pub fn draw_chip(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    style: UiTextStyle,
    chip: &Chip<'_>,
) -> Result<i32, Infallible> {
    let width = chip_width(style, chip);
    let height = chip_height(style);
    let ink = BinaryColor::On;
    let paper = BinaryColor::Off;
    let (fill, stroke) = if chip.filled {
        (ink, ink)
    } else {
        (paper, ink)
    };
    Rectangle::new(top_left, Size::new(width as u32, height as u32))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(fill)
                .stroke_color(stroke)
                .stroke_width(BORDER as u32)
                .stroke_alignment(StrokeAlignment::Inside)
                .build(),
        )
        .draw(display)?;
    let text_ink = if chip.filled { paper } else { ink };
    let text_style = style.with_color(text_ink);
    let baseline = top_left.y + (height + text_style.cap_height()) / 2 - BORDER;
    let mut left = top_left.x + BORDER + PADDING;
    if chip.dot {
        let center = Point::new(left + DOT_DIAMETER / 2, top_left.y + height / 2);
        Circle::with_center(center, DOT_DIAMETER as u32)
            .into_styled(PrimitiveStyle::with_fill(text_ink))
            .draw(display)?;
        left += DOT_DIAMETER + DOT_GAP;
    }
    Text::new(chip.label, Point::new(left, baseline), text_style).draw(display)?;
    Ok(width)
}

/// A row's width with `gap` between its chips, to centre it.
#[must_use]
pub fn chip_row_width(style: UiTextStyle, chips: &[Chip<'_>], gap: i32) -> i32 {
    let widths: i32 = chips.iter().map(|chip| chip_width(style, chip)).sum();
    widths + gap * (chips.len() as i32 - 1).max(0)
}

/// A row of chips with `gap` between them, starting at `left`.
pub fn draw_chip_row(
    display: &mut OrientedFrameBuffer<'_>,
    left: i32,
    top: i32,
    gap: i32,
    style: UiTextStyle,
    chips: &[Chip<'_>],
) -> Result<(), Infallible> {
    let mut x = left;
    for chip in chips {
        let width = draw_chip(display, Point::new(x, top), style, chip)?;
        x += width + gap;
    }
    Ok(())
}

/// Gap between chips when the mockup puts a `▸` between them (flex `gap: 6`
/// around an 8 px triangle): 20 px of room, triangle centred in it.
pub const ARROW_GAP: i32 = 20;

/// A centred row of chips with `▸` separators, as the mockup's `.chips`.
/// Returns the left edge of the row.
pub fn draw_chip_row_centered(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    style: UiTextStyle,
    chips: &[Chip<'_>],
) -> Result<i32, Infallible> {
    use embedded_graphics::primitives::{PrimitiveStyle, Triangle};

    let arrows = chips.len().saturating_sub(1) as i32 * ARROW_GAP;
    let total = chip_row_width(style, chips, 0) + arrows;
    let mut left = (480 - total) / 2;
    let start = left;
    let middle = top + chip_height(style) / 2;
    for (index, chip) in chips.iter().enumerate() {
        let width = draw_chip(display, Point::new(left, top), style, chip)?;
        left += width;
        if index + 1 < chips.len() {
            Triangle::new(
                Point::new(left + (ARROW_GAP - 6) / 2, middle - 4),
                Point::new(left + (ARROW_GAP - 6) / 2, middle + 4),
                Point::new(left + (ARROW_GAP - 6) / 2 + 6, middle),
            )
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
            left += ARROW_GAP;
        }
    }
    Ok(start)
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::{chip_height, chip_width, draw_chip, Chip};
    use crate::{
        app::AppState,
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    /// A chip draws inside its own box: the border, the dot and the label.
    #[test]
    fn a_chip_draws_inside_its_box() {
        let style = AppState::default().display.detail_style();
        let chip = Chip::dotted("RECORDING", true);
        let width = chip_width(style, &chip);
        let height = chip_height(style);
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        draw_chip(&mut display, Point::new(100, 200), style, &chip).unwrap();
        drop(display);
        let mut outside = 0_u32;
        for y in 0..800 {
            for x in 0..480 {
                if frame.is_black(Point::new(y, 479 - x)) != Some(true) {
                    continue;
                }
                if !(100..100 + width).contains(&x) || !(200..200 + height).contains(&y) {
                    outside += 1;
                }
            }
        }
        assert_eq!(outside, 0, "the chip drew outside its box");
    }
}
