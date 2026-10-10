//! Display numerals: the sleep clock and the large temperatures in Inter
//! ExtraBold. Digits sit in cells of equal width, as the mockup's tabular
//! figures, so the clock does not shift when a digit changes.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
};

use crate::app::{
    display_assets::{DISPLAY_CLOCK, DISPLAY_TEMPERATURE},
    typography::{Text, TextBounds, UiTextStyle},
};

/// Which strike draws the text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Numerals {
    Clock,
    Temperature,
}

impl Numerals {
    fn style(self, color: BinaryColor) -> UiTextStyle {
        match self {
            Self::Clock => UiTextStyle::new(&DISPLAY_CLOCK, color),
            Self::Temperature => UiTextStyle::new(&DISPLAY_TEMPERATURE, color),
        }
    }

    /// Height of a digit above the baseline.
    #[must_use]
    pub fn digit_height(self) -> i32 {
        self.style(BinaryColor::On).cap_height()
    }

    /// Space between glyphs: the mockup's tight letter spacing.
    const fn tracking(self) -> i32 {
        match self {
            Self::Clock => -4,
            Self::Temperature => -3,
        }
    }
}

/// Drawn width of `text`, digits in equal cells.
#[must_use]
pub fn numerals_width(text: &str, numerals: Numerals) -> i32 {
    let style = numerals.style(BinaryColor::On);
    let cell = digit_cell(style);
    let count = text.chars().count() as i32;
    let advances: i32 = text
        .chars()
        .map(|character| advance(display_char(character), style, cell))
        .sum();
    advances + numerals.tracking() * (count - 1).max(0)
}

/// Draw `text` with its baseline at `baseline`, centred between `left` and
/// `right`. Text wider than that starts at `left` and is cut at `right`, so
/// the sign of a long negative value stays visible.
pub fn draw_numerals<D>(
    display: &mut D,
    text: &str,
    numerals: Numerals,
    (left, right): (i32, i32),
    baseline: i32,
    color: BinaryColor,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = numerals.style(color);
    let cell = digit_cell(style);
    let width = numerals_width(text, numerals);
    let mut x = if width <= right - left {
        left + (right - left - width) / 2
    } else {
        left
    };
    let bounds = TextBounds::new(left, baseline - style.cap_height() - 8, right, baseline + 8);
    let mut buffer = [0_u8; 4];
    for character in text.chars().map(display_char) {
        let room = advance(character, style, cell);
        let inset = (room - style.char_advance(character)) / 2;
        let glyph = character.encode_utf8(&mut buffer);
        Text::new(glyph, Point::new(x + inset, baseline), style).draw_clipped(display, bounds)?;
        x += room + numerals.tracking();
    }
    Ok(())
}

/// Width of a digit cell: the widest digit.
fn digit_cell(style: UiTextStyle) -> i32 {
    ('0'..='9')
        .map(|digit| style.char_advance(digit))
        .max()
        .unwrap_or(0)
}

fn advance(character: char, style: UiTextStyle, cell: i32) -> i32 {
    if character.is_ascii_digit() {
        cell
    } else {
        style.char_advance(character)
    }
}

/// A hyphen reads as the minus sign.
const fn display_char(character: char) -> char {
    if character == '-' {
        '\u{2212}'
    } else {
        character
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    #[test]
    fn digits_take_equal_cells_so_the_clock_keeps_its_width() {
        assert_eq!(
            numerals_width("11:11", Numerals::Clock),
            numerals_width("08:48", Numerals::Clock)
        );
        assert!(numerals_width("18:42", Numerals::Clock) <= 408, "fits the frame");
    }

    #[test]
    fn digits_match_the_mockup_sizes() {
        // The mockup's 128 px clock and 96 px temperature in Inter ExtraBold.
        assert!((88..=96).contains(&Numerals::Clock.digit_height()));
        assert!((66..=74).contains(&Numerals::Temperature.digit_height()));
    }

    #[test]
    fn a_hyphen_is_drawn_as_the_minus_sign_and_kept_when_cut() {
        assert_eq!(
            numerals_width("-12\u{b0}", Numerals::Temperature),
            numerals_width("\u{2212}12\u{b0}", Numerals::Temperature)
        );
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        let baseline = 300;
        draw_numerals(
            &mut display,
            "-12345\u{b0}",
            Numerals::Temperature,
            (100, 300),
            baseline,
            BinaryColor::On,
        )
        .unwrap();
        drop(display);
        // Portrait logical (x, y) is native (y, 479 - x).
        let black = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(true);
        let height = Numerals::Temperature.digit_height();
        let minus = (100..140).any(|x| (baseline - height..baseline).any(|y| black(x, y)));
        assert!(minus, "the minus sign starts the cut text");
        let outside = (300..480).any(|x| (baseline - height - 8..baseline + 8).any(|y| black(x, y)));
        assert!(!outside, "nothing past the right edge");
    }
}
