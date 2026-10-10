//! Reading header: a title on the left, the clock and battery on the right,
//! and a rule under them. No black bar, so the page keeps its room.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle, Triangle},
};

use crate::{
    app::{display::DisplayPreferences, typography::Text},
    orientation::OrientedFrameBuffer,
};

/// Height of the header including its rule.
pub const READING_HEADER_HEIGHT: i32 = 56;
/// Width of the bookmark ribbon at the right end of the header.
const RIBBON_WIDTH: i32 = 14;
/// Room the ribbon takes from the status: its width and a gap.
const RIBBON_ROOM: i32 = RIBBON_WIDTH + 10;

/// Draw the header for a page. `title` is drawn in capitals on the left and
/// `status` (clock and battery) right-aligned, both at the baseline. A
/// `marked` page hangs a bookmark ribbon from the top edge at the right, and
/// the status moves left to make room for it.
pub fn draw_reading_header(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    width: i32,
    title: &str,
    status: &str,
    marked: bool,
) -> Result<(), Infallible> {
    let title_style = preferences.heading_style();
    // Body size keeps the battery's percent glyph whole (KNOWN_ISSUES).
    let status_style = preferences.body_style();
    let baseline = READING_HEADER_HEIGHT - 18;
    let status_right = width - 24 - if marked { RIBBON_ROOM } else { 0 };
    let status_width = status_style.text_width(status);
    let title_room = status_right - status_width - 16 - 24;
    let title = title_style.fit(&title.to_uppercase(), title_room);
    Text::new(&title, Point::new(24, baseline), title_style).draw(display)?;
    Text::new(
        status,
        Point::new(status_right - status_width, baseline),
        status_style,
    )
    .draw(display)?;
    if marked {
        let left = width - 12 - RIBBON_WIDTH;
        Rectangle::new(Point::new(left, 0), Size::new(RIBBON_WIDTH as u32, 30))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
        // A notch cut into the tail makes it read as a ribbon.
        Triangle::new(
            Point::new(left, 30),
            Point::new(left + RIBBON_WIDTH, 30),
            Point::new(left + RIBBON_WIDTH / 2, 23),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(display)?;
    }
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

    #[test]
    fn the_bookmark_ribbon_leaves_the_status_clear() {
        use crate::{framebuffer::FrameBuffer, orientation::DisplayOrientation};

        for marked in [false, true] {
            let mut frame = FrameBuffer::new_white();
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            let preferences = DisplayPreferences::default();
            draw_reading_header(
                &mut display,
                preferences,
                480,
                "Quijote",
                "18:42  BAT 78%",
                marked,
            )
            .unwrap();
            drop(display);
            // Portrait logical (x, y) is native (y, 479 - x).
            let ink = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(true);
            let ribbon_left = 480 - 12 - RIBBON_WIDTH;
            assert_eq!(
                ink(ribbon_left + 2, 4),
                marked,
                "ribbon drawn only when marked"
            );
            // With the ribbon, the gap between the status and it stays white.
            let gap = (480 - 24 - RIBBON_ROOM + 1)..ribbon_left;
            let touched = gap
                .clone()
                .any(|x| (0..READING_HEADER_HEIGHT).any(|y| ink(x, y)));
            assert!(!(marked && touched), "nothing in the gap {gap:?}");
        }
    }
}
