//! Reading header: a title on the left, the clock and battery on the right,
//! and a rule under them. No black bar, so the page keeps its room.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{display::DisplayPreferences, typography::Text},
    orientation::OrientedFrameBuffer,
};

/// Height of the header including its rule.
pub const READING_HEADER_HEIGHT: i32 = 56;

/// Draw the header for a page. `title` is drawn in capitals on the left and
/// `status` (clock and battery) right-aligned, both at the baseline.
pub fn draw_reading_header(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    width: i32,
    title: &str,
    status: &str,
) -> Result<(), Infallible> {
    let title_style = preferences.heading_style();
    let status_style = preferences.detail_style();
    let baseline = READING_HEADER_HEIGHT - 18;
    let status_width = status_style.text_width(status);
    let title_room = width - 24 - status_width - 16;
    let title = title_style.fit(&title.to_uppercase(), title_room);
    Text::new(&title, Point::new(24, baseline), title_style).draw(display)?;
    Text::new(
        status,
        Point::new(width - 24 - status_width, baseline),
        status_style,
    )
    .draw(display)?;
    Rectangle::new(
        Point::new(14, READING_HEADER_HEIGHT),
        Size::new((width - 28) as u32, 2),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_baseline_sits_above_the_rule() {
        let baseline = READING_HEADER_HEIGHT - 18;
        let rule_top = READING_HEADER_HEIGHT;
        assert!(baseline < rule_top, "text must end above the rule");
        assert!(baseline > 0, "text must start inside the header");
    }
}
