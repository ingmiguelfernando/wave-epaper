//! Large seven-segment numerals for the sleep screens. The bitmap fonts stop
//! at about 30 px, so the clock and temperature use graphics primitives.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
};

/// Segment stroke: about 1/8 of the glyph height.
fn stroke(height: i32) -> i32 {
    (height / 8).max(1).min(digit_width(height))
}

/// Box width of a digit or the minus sign.
fn digit_width(height: i32) -> i32 {
    (height / 2).max(1)
}

/// Gap between neighbouring glyphs.
fn glyph_gap(height: i32) -> i32 {
    (height / 10).max(2)
}

/// Box width of one glyph.
fn glyph_width(character: char, height: i32) -> i32 {
    match character {
        '0'..='9' | '-' => digit_width(height),
        ':' | '\u{b0}' => 2 * stroke(height),
        _ => digit_width(height) / 2,
    }
}

/// Total drawn width of `text` at `height`, gaps included.
#[must_use]
pub fn big_text_width(text: &str, height: i32) -> i32 {
    if height <= 0 {
        return 0;
    }
    let mut width: i32 = 0;
    for (index, character) in text.chars().enumerate() {
        if index > 0 {
            width = width.saturating_add(glyph_gap(height));
        }
        width = width.saturating_add(glyph_width(character, height));
    }
    width
}

/// Draw `text` centered on `center_x`, spanning `top` to `top + height`.
/// Returns the drawn width. Supports `0-9`, `:`, `°` and `-`.
pub fn draw_big_text<D>(
    display: &mut D,
    text: &str,
    center_x: i32,
    top: i32,
    height: i32,
) -> Result<i32, D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let total = big_text_width(text, height);
    if total == 0 {
        return Ok(0);
    }
    let gap = glyph_gap(height);
    let mut left = center_x - total / 2;
    for character in text.chars() {
        let width = glyph_width(character, height);
        match character {
            '0'..='9' => draw_digit(display, character, left, top, height)?,
            ':' => draw_colon(display, left, top, height)?,
            '\u{b0}' => draw_degree(display, left, top, height)?,
            '-' => draw_segment(display, Segment::G, left, top, height)?,
            _ => {}
        }
        left += width + gap;
    }
    Ok(total)
}

/// The seven segments of one digit, drawn with slightly beveled ends.
#[derive(Clone, Copy)]
enum Segment {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

fn draw_digit<D>(
    display: &mut D,
    character: char,
    left: i32,
    top: i32,
    height: i32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    use Segment::{A, B, C, D as DSegment, E, F, G};
    const LIT: [&[Segment]; 10] = [
        &[A, B, C, DSegment, E, F],
        &[B, C],
        &[A, B, G, E, DSegment],
        &[A, B, G, C, DSegment],
        &[F, G, B, C],
        &[A, F, G, C, DSegment],
        &[A, F, G, E, C, DSegment],
        &[A, B, C],
        &[A, B, C, DSegment, E, F, G],
        &[A, B, C, DSegment, F, G],
    ];
    let index = character as usize - '0' as usize;
    for segment in LIT[index] {
        draw_segment(display, *segment, left, top, height)?;
    }
    Ok(())
}

fn draw_segment<D>(
    display: &mut D,
    segment: Segment,
    left: i32,
    top: i32,
    height: i32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stroke = stroke(height);
    let width = digit_width(height);
    let middle = (height - stroke) / 2;
    let rectangle = match segment {
        Segment::A => Rectangle::new(
            Point::new(left + stroke / 2, top),
            Size::new((width - stroke).max(1) as u32, stroke as u32),
        ),
        Segment::G => Rectangle::new(
            Point::new(left + stroke / 2, top + middle),
            Size::new((width - stroke).max(1) as u32, stroke as u32),
        ),
        Segment::D => Rectangle::new(
            Point::new(left + stroke / 2, top + height - stroke),
            Size::new((width - stroke).max(1) as u32, stroke as u32),
        ),
        Segment::F => Rectangle::new(
            Point::new(left, top + stroke / 2),
            Size::new(stroke as u32, (height / 2 - stroke / 2) as u32),
        ),
        Segment::B => Rectangle::new(
            Point::new(left + width - stroke, top + stroke / 2),
            Size::new(stroke as u32, (height / 2 - stroke / 2) as u32),
        ),
        Segment::E => Rectangle::new(
            Point::new(left, top + height / 2),
            Size::new(stroke as u32, (height / 2 - stroke / 2) as u32),
        ),
        Segment::C => Rectangle::new(
            Point::new(left + width - stroke, top + height / 2),
            Size::new(stroke as u32, (height / 2 - stroke / 2) as u32),
        ),
    };
    RoundedRectangle::with_equal_corners(
        rectangle,
        Size::new((stroke / 2) as u32, (stroke / 2) as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(display)?;
    Ok(())
}

/// Two stacked dots, centered in the glyph box.
fn draw_colon<D>(display: &mut D, left: i32, top: i32, height: i32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stroke = stroke(height);
    let center = left + stroke;
    for dot_top in [top + height / 4, top + height * 3 / 4] {
        Rectangle::new(
            Point::new(center - stroke / 2, dot_top - stroke / 2),
            Size::new(stroke as u32, stroke as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    }
    Ok(())
}

/// Hollow square near the cap line, as a superscript degree sign.
fn draw_degree<D>(display: &mut D, left: i32, top: i32, height: i32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stroke = stroke(height);
    let side = 2 * stroke;
    let bar = (stroke / 2).max(1) as u32;
    for (offset, size) in [
        (Point::new(left, top), Size::new(side as u32, bar)),
        (
            Point::new(left, top + side - bar as i32),
            Size::new(side as u32, bar),
        ),
        (Point::new(left, top), Size::new(bar, side as u32)),
        (
            Point::new(left + side - bar as i32, top),
            Size::new(bar, side as u32),
        ),
    ] {
        Rectangle::new(offset, size)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::{big_text_width, draw_big_text};
    use crate::{
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    #[test]
    fn big_text_width_sums_glyph_boxes_and_gaps() {
        assert_eq!(big_text_width("", 150), 0);
        assert_eq!(big_text_width("8", 150), 75);
        assert_eq!(big_text_width("13:42", 150), 396);
        assert_eq!(big_text_width("18\u{b0}", 110), 158);
        for digit in '0'..='9' {
            assert_eq!(big_text_width(&digit.to_string(), 150), 75);
        }
        assert_eq!(big_text_width(":", 150), 36);
        assert_eq!(big_text_width("°", 110), 26);
        assert_eq!(big_text_width("-", 110), 55);
        assert_eq!(big_text_width("-123°", 110), 290);
    }

    #[test]
    fn nonpositive_heights_and_empty_text_draw_nothing() {
        let mut frame = FrameBuffer::new_white();
        for height in [-150, -1, 0] {
            assert_eq!(big_text_width("18°", height), 0);
            assert_eq!(
                draw_big_text(&mut frame, "18°", 240, 180, height).unwrap(),
                0
            );
        }
        assert_eq!(draw_big_text(&mut frame, "", 240, 180, 150).unwrap(), 0);
        assert_eq!(frame, FrameBuffer::new_white());
    }

    #[test]
    fn every_digit_lights_only_its_expected_segments() {
        let expected = [
            [true, true, true, true, true, true, false],
            [false, true, true, false, false, false, false],
            [true, true, false, true, true, false, true],
            [true, true, true, true, false, false, true],
            [false, true, true, false, false, true, true],
            [true, false, true, true, false, true, true],
            [true, false, true, true, true, true, true],
            [true, true, true, false, false, false, false],
            [true, true, true, true, true, true, true],
            [true, true, true, true, false, true, true],
        ];
        let points = [
            (137, 29),
            (165, 60),
            (165, 130),
            (137, 161),
            (109, 130),
            (109, 60),
            (137, 95),
        ];
        for (digit, lit) in expected.iter().enumerate() {
            let mut frame = FrameBuffer::new_white();
            assert_eq!(
                draw_big_text(&mut frame, &digit.to_string(), 137, 20, 150).unwrap(),
                75
            );
            for ((x, y), expected) in points.iter().zip(lit) {
                assert_eq!(
                    frame.is_black(Point::new(*x, *y)),
                    Some(*expected),
                    "digit {digit} at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn punctuation_draws_colon_minus_and_hollow_degree() {
        let mut frame = FrameBuffer::new_white();
        draw_big_text(&mut frame, ":", 100, 20, 110).unwrap();
        assert_eq!(frame.is_black(Point::new(100, 47)), Some(true));
        assert_eq!(frame.is_black(Point::new(100, 102)), Some(true));
        assert_eq!(frame.is_black(Point::new(100, 75)), Some(false));
        frame.clear_white();
        draw_big_text(&mut frame, "°", 100, 20, 110).unwrap();
        assert_eq!(frame.is_black(Point::new(100, 22)), Some(true));
        assert_eq!(frame.is_black(Point::new(100, 33)), Some(false));
        frame.clear_white();
        draw_big_text(&mut frame, "-", 100, 20, 110).unwrap();
        assert_eq!(frame.is_black(Point::new(100, 75)), Some(true));
        assert_eq!(frame.is_black(Point::new(100, 25)), Some(false));
    }

    #[test]
    fn digits_draw_segments_with_gaps_between_them() {
        let mut frame = FrameBuffer::new_white();
        {
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            draw_big_text(&mut display, "8", 240, 180, 150).unwrap();
        }
        // Top segment (logical y 180..198) maps to native (y, 479 - x).
        assert_eq!(frame.is_black(Point::new(189, 479 - 240)), Some(true));
        // Middle segment sits at logical y 246..264.
        assert_eq!(frame.is_black(Point::new(255, 479 - 240)), Some(true));
        // The gap between the two segments stays paper.
        assert_eq!(frame.is_black(Point::new(222, 479 - 240)), Some(false));
    }
}
