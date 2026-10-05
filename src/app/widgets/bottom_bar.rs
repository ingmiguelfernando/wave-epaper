//! Bottom bar shared by every screen: black key caps followed by short action
//! labels, as in the mockup's footer.

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

/// Top edge of the bottom bar; screen content should stay above it.
pub const BOTTOM_BAR_TOP: i32 = 752;
const CAP_HEIGHT: i32 = 22;
const LEFT: i32 = 14;
const RIGHT: i32 = 480 - 14;
const CAP_GAP: i32 = 6;
const HINT_GAP: i32 = 16;

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

/// Screens where holding BOOT is the only action.
pub const BACK_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "hold: back")];

pub const OPEN_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "open"),
    (KeyCap::Boot, "hold: back"),
];

pub const RUN_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "run"),
    (KeyCap::Boot, "hold: back"),
];

/// Settings rows that open a picker or switch a value.
pub const CHANGE_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "change"),
    (KeyCap::Boot, "hold: back"),
];

/// Option pickers.
pub const CHOOSE_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "choose"),
    (KeyCap::Boot, "hold: cancel"),
];

/// On-screen keyboards.
pub const KEYBOARD_HINTS: [(KeyCap, &str); 4] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Boot, "H/V"),
    (KeyCap::Select, "key"),
    (KeyCap::Boot, "hold: back"),
];

/// Draw the bar with Body labels, or Detail labels when Body would not fit.
pub fn draw_bottom_bar(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    hints: &[(KeyCap, &str)],
) -> Result<(), Infallible> {
    Rectangle::new(Point::new(0, BOTTOM_BAR_TOP), Size::new(480, 3))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let cap_style = preferences.text_style(UiTextRole::Detail, BinaryColor::Off);
    let body = preferences.body_style();
    let label_style = if hints_width(hints, body, cap_style) <= RIGHT - LEFT {
        body
    } else {
        preferences.detail_style()
    };
    debug_assert!(
        hints_width(hints, label_style, cap_style) <= RIGHT - LEFT,
        "bottom bar hints are too wide: {hints:?}"
    );
    let center = (BOTTOM_BAR_TOP + 3 + 800) / 2;
    let mut x = LEFT;
    for &(cap, label) in hints {
        x = draw_key_cap(display, cap, Point::new(x, center), cap_style)? + CAP_GAP;
        let origin = Point::new(x, center + label_style.cap_height() / 2);
        Text::new(label, origin, label_style).draw(display)?;
        x += label_style.text_width(label) + HINT_GAP;
    }
    Ok(())
}

/// Width from the first cap to the end of the last label.
fn hints_width(hints: &[(KeyCap, &str)], label_style: UiTextStyle, cap_style: UiTextStyle) -> i32 {
    let items: i32 = hints
        .iter()
        .map(|&(cap, label)| cap_width(cap, cap_style) + CAP_GAP + label_style.text_width(label))
        .sum();
    items + HINT_GAP * (hints.len() as i32 - 1).max(0)
}

const fn cap_text(cap: KeyCap) -> &'static str {
    match cap {
        KeyCap::Boot => "BOOT",
        KeyCap::Power => "PWR",
        KeyCap::UpDown | KeyCap::Select => "",
    }
}

fn cap_width(cap: KeyCap, text_style: UiTextStyle) -> i32 {
    match cap {
        KeyCap::UpDown => 34,
        KeyCap::Select => 24,
        KeyCap::Boot | KeyCap::Power => text_style.text_width(cap_text(cap)) + 12,
    }
}

/// Draw one key cap whose left edge and vertical center are at `anchor`.
/// Returns the cap's right edge.
fn draw_key_cap(
    display: &mut OrientedFrameBuffer<'_>,
    cap: KeyCap,
    anchor: Point,
    text_style: UiTextStyle,
) -> Result<i32, Infallible> {
    let text = cap_text(cap);
    let width = cap_width(cap, text_style);
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

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::*;
    use crate::{
        app::display::{UiFontFamily, UiFontSize},
        framebuffer::FrameBuffer,
        games::dirty_regions::GAME_BOTTOM_BAR_RECT,
        orientation::DisplayOrientation,
    };

    /// Calendar's month view, the widest set in use.
    const WIDEST: [(KeyCap, &str); 4] = [
        (KeyCap::UpDown, "move"),
        (KeyCap::Select, "mode"),
        (KeyCap::Boot, "agenda"),
        (KeyCap::Boot, "hold: back"),
    ];

    fn render(preferences: DisplayPreferences, hints: &[(KeyCap, &str)]) -> FrameBuffer {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        draw_bottom_bar(&mut display, preferences, hints).unwrap();
        drop(display);
        frame
    }

    #[test]
    fn the_widest_hints_stay_inside_the_margin_at_every_font() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let frame = render(preferences, &WIDEST);
                // Logical (x, y) is native (y, 479 - x) in portrait.
                let ink = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(true);
                assert!(ink(240, BOTTOM_BAR_TOP));
                let overflow = (RIGHT..480).any(|x| (BOTTOM_BAR_TOP + 3..800).any(|y| ink(x, y)));
                assert!(!overflow, "{} {}", font_family.marker(), font_size.marker());
            }
        }
    }

    #[test]
    fn games_refresh_the_band_the_bar_draws() {
        assert_eq!(GAME_BOTTOM_BAR_RECT.y, BOTTOM_BAR_TOP);
        assert_eq!(GAME_BOTTOM_BAR_RECT.bottom(), 800);
        assert_eq!(GAME_BOTTOM_BAR_RECT.width, 480);
    }
}
