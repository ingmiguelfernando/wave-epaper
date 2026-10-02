//! Card shown while asleep when no SD picture can be used. Its note says why,
//! so the picture can be fixed on a computer.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, UiTextStyle},
    },
    orientation::OrientedFrameBuffer,
};

pub struct SleepCard<'a> {
    /// Why no SD picture is shown.
    pub note: &'a str,
    pub battery_percent: Option<u8>,
    pub wake_hint: &'a str,
}

pub fn render_sleep_card(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    card: &SleepCard<'_>,
) -> Result<(), Infallible> {
    let body = preferences.body_style();
    let detail = preferences.detail_style();
    Rectangle::new(Point::new(16, 16), Size::new(448, 768))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    centered(display, "Wave", 330, preferences.large_style())?;
    centered(display, "Sleeping", 384, body)?;
    let mut baseline = 470;
    for line in detail.wrap(card.note, 400) {
        centered(display, &line, baseline, detail)?;
        baseline += i32::from(detail.line_height());
    }
    centered(display, card.wake_hint, 690, body)?;
    if let Some(percent) = card.battery_percent {
        centered(display, &format!("Battery {percent}%"), 736, detail)?;
    }
    Ok(())
}

fn centered(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    baseline: i32,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let left = (480 - style.text_width(text)) / 2;
    Text::new(text, Point::new(left, baseline), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::SleepCard;
    use crate::{
        app::{display::DisplayPreferences, render_sleep_card},
        framebuffer::FrameBuffer,
    };

    #[test]
    fn card_is_drawn_in_portrait_with_its_border() {
        let mut frame = FrameBuffer::new_white();
        let card = SleepCard {
            note: "SLEEP.BMP is 1024 × 768; it must be 480 × 800 or 800 × 480",
            battery_percent: Some(64),
            wake_hint: "Press any key to wake",
        };
        render_sleep_card(&mut frame, DisplayPreferences::default(), &card).unwrap();
        // The left border (logical x 16) maps to native (y, 479 - 16).
        assert_eq!(frame.is_black(Point::new(400, 463)), Some(true));
        assert_eq!(frame.is_black(Point::new(400, 475)), Some(false));
    }
}
