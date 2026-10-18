//! Named game sprites, drawn from shapes: cars seen from above, a cone, the
//! impact star, speed streaks and the move hint. The mockup's Race symbols
//! (`race-car`, `race-traffic`, `race-cone`, `race-burst`) scaled by 1.2,
//! small enough to stay inside one road cell, so the cells a step dirties
//! cover what changed. `games::canvas::GameSprite` names them; one canvas
//! command per sprite keeps a frame far below the command limit.

use core::convert::Infallible;

use embedded_graphics::{
    geometry::Angle,
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{
        Arc, CornerRadii, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
        StrokeAlignment, Triangle,
    },
};

use crate::{games::canvas::GameSprite, orientation::OrientedFrameBuffer};

/// Draw `sprite` with its top-left at `origin`. `ink` is the body color and
/// every detail takes the opposite, so a sprite works on a black card too.
/// No pixel is drawn outside the sprite's size.
pub fn draw_sprite(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    sprite: GameSprite,
    ink: BinaryColor,
) -> Result<(), Infallible> {
    let paper = if ink == BinaryColor::On {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    match sprite {
        GameSprite::Traffic => traffic_car(display, origin, ink, paper),
        GameSprite::PlayerCar => player_car(display, origin, ink, paper),
        GameSprite::Cone => cone(display, origin, ink, paper),
        GameSprite::Burst => burst(display, origin, ink, paper),
        GameSprite::MoveHint => move_hint(display, origin, ink),
        GameSprite::Streaks => streaks(display, origin, ink),
        GameSprite::CrashCar => crash_car(display, origin, ink, paper),
        GameSprite::Speaker => speaker(display, origin, paper),
    }
}

/// Wheels, the rounded body outline and its lines: `race-traffic`.
fn traffic_car(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    wheels(display, origin, ink)?;
    car_body(origin)
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(paper)
                .stroke_color(ink)
                .stroke_width(3)
                .stroke_alignment(StrokeAlignment::Inside)
                .build(),
        )
        .draw(display)?;
    // The windshield curve and the door line.
    Arc::with_center(
        origin + Point::new(22, 26),
        47,
        Angle::from_degrees(-29.0),
        Angle::from_degrees(58.0),
    )
    .into_styled(PrimitiveStyle::with_stroke(ink, 3))
    .draw(display)?;
    Line::new(origin + Point::new(18, 16), origin + Point::new(18, 36))
        .into_styled(PrimitiveStyle::with_stroke(ink, 3))
        .draw(display)?;
    Ok(())
}

/// The body outline: the back corners rounded, the nose rounder.
fn car_body(origin: Point) -> RoundedRectangle {
    let (back, nose) = (Size::new(10, 10), Size::new(17, 21));
    RoundedRectangle::new(
        Rectangle::new(origin + Point::new(2, 5), Size::new(68, 43)),
        CornerRadii {
            top_left: back,
            top_right: nose,
            bottom_right: nose,
            bottom_left: back,
        },
    )
}

/// The player's car: solid, with paper windows: `race-car`.
fn player_car(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    wheels(display, origin, ink)?;
    car_body(origin)
        .into_styled(PrimitiveStyle::with_fill(ink))
        .draw(display)?;
    windows(display, origin, paper)
}

/// The four wheels at the corners, drawn first so the body covers their
/// inner halves.
fn wheels(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
) -> Result<(), Infallible> {
    for (x, y) in [(11, 0), (48, 0), (11, 43), (48, 43)] {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(origin + Point::new(x, y), Size::new(14, 10)),
            Size::new(2, 2),
        )
        .into_styled(PrimitiveStyle::with_fill(ink))
        .draw(display)?;
    }
    Ok(())
}

/// The windshield, the cabin window and the hood stripe.
fn windows(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    RoundedRectangle::with_equal_corners(
        Rectangle::new(origin + Point::new(42, 14), Size::new(12, 24)),
        Size::new(5, 5),
    )
    .into_styled(PrimitiveStyle::with_fill(paper))
    .draw(display)?;
    RoundedRectangle::with_equal_corners(
        Rectangle::new(origin + Point::new(12, 15), Size::new(8, 22)),
        Size::new(2, 2),
    )
    .into_styled(PrimitiveStyle::with_fill(paper))
    .draw(display)?;
    Rectangle::new(origin + Point::new(24, 23), Size::new(14, 6))
        .into_styled(PrimitiveStyle::with_fill(paper))
        .draw(display)?;
    Ok(())
}

/// One cone: a solid triangle with a paper band: `race-cone`.
fn cone(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    Triangle::new(
        origin + Point::new(17, 1),
        origin + Point::new(3, 41),
        origin + Point::new(31, 41),
    )
    .into_styled(PrimitiveStyle::with_fill(ink))
    .draw(display)?;
    // The band widens toward the base.
    Triangle::new(
        origin + Point::new(12, 17),
        origin + Point::new(22, 17),
        origin + Point::new(23, 24),
    )
    .into_styled(PrimitiveStyle::with_fill(paper))
    .draw(display)?;
    Triangle::new(
        origin + Point::new(12, 17),
        origin + Point::new(23, 24),
        origin + Point::new(11, 24),
    )
    .into_styled(PrimitiveStyle::with_fill(paper))
    .draw(display)?;
    Ok(())
}

/// The impact star: an ink star with a paper star inside: `race-burst`.
fn burst(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    star(display, origin + Point::new(30, 30), 28, 11, ink)?;
    star(display, origin + Point::new(30, 30), 12, 5, paper)
}

/// A filled five-point star from a fan of triangles around its center.
fn star(
    display: &mut OrientedFrameBuffer<'_>,
    center: Point,
    outer: i32,
    inner: i32,
    color: BinaryColor,
) -> Result<(), Infallible> {
    let corner = |index: i32, radius: i32| -> Point {
        // Five points, starting at the top; `index` runs over ten tips.
        let angle = (index as f32 * 36.0 - 90.0).to_radians();
        center
            + Point::new(
                (angle.cos() * radius as f32) as i32,
                (angle.sin() * radius as f32) as i32,
            )
    };
    let fill = PrimitiveStyle::with_fill(color);
    for tip in (0..10).step_by(2) {
        let left = corner(tip - 1, inner);
        let right = corner(tip + 1, inner);
        Triangle::new(corner(tip, outer), left, right)
            .into_styled(fill)
            .draw(display)?;
        Triangle::new(center, left, right)
            .into_styled(fill)
            .draw(display)?;
    }
    Ok(())
}

/// The dashed rounded outline of the cell Up would move the car to.
fn move_hint(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
) -> Result<(), Infallible> {
    const DASH: f32 = 7.0;
    const GAP: f32 = 6.0;
    let points = rounded_rect_points(2, 2, 68, 49, (12, 12), (12, 12));
    let style = PrimitiveStyle::with_stroke(ink, 3);
    let mut walked = 0.0;
    for pair in 0..points.len() {
        let start = points[pair];
        let end = points[(pair + 1) % points.len()];
        let dx = (end.x - start.x) as f32;
        let dy = (end.y - start.y) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        if length < 0.5 {
            continue;
        }
        let mut at = 0.0;
        while at < length {
            let phase = (walked + at) % (DASH + GAP);
            let step = if phase < DASH {
                DASH - phase
            } else {
                DASH + GAP - phase
            };
            let to = (at + step).min(length);
            if phase < DASH {
                let lerp = |t: f32| origin + start + Point::new((dx * t) as i32, (dy * t) as i32);
                Line::new(lerp(at / length), lerp(to / length))
                    .into_styled(style)
                    .draw(display)?;
            }
            at = to;
        }
        walked += length;
    }
    Ok(())
}

/// Three speed streaks in the cell behind the player's car.
fn streaks(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
) -> Result<(), Infallible> {
    for (x, y, width) in [(0, 0, 38), (14, 15, 24), (0, 30, 38)] {
        Rectangle::new(origin + Point::new(x, y), Size::new(width, 4))
            .into_styled(PrimitiveStyle::with_fill(ink))
            .draw(display)?;
    }
    Ok(())
}

/// The mockup's `#spk`: a speaker with two sound waves, in `paper` for the
/// black status bar, inside its 24 × 20 size.
fn speaker(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    color: BinaryColor,
) -> Result<(), Infallible> {
    let fill = PrimitiveStyle::with_fill(color);
    Rectangle::new(origin + Point::new(0, 6), Size::new(5, 8))
        .into_styled(fill)
        .draw(display)?;
    Triangle::new(
        origin + Point::new(4, 6),
        origin + Point::new(12, 0),
        origin + Point::new(12, 19),
    )
    .into_styled(fill)
    .draw(display)?;
    Triangle::new(
        origin + Point::new(4, 6),
        origin + Point::new(4, 13),
        origin + Point::new(12, 19),
    )
    .into_styled(fill)
    .draw(display)?;
    for diameter in [12_u32, 20] {
        Arc::with_center(
            origin + Point::new(11, 10),
            diameter,
            Angle::from_degrees(-45.0),
            Angle::from_degrees(90.0),
        )
        .into_styled(PrimitiveStyle::with_stroke(color, 2))
        .draw(display)?;
    }
    Ok(())
}

/// The player's car turned 14 degrees, for the crash picture. The tilt is
/// baked in: the crash screen draws the same picture every time.
fn crash_car(
    display: &mut OrientedFrameBuffer<'_>,
    origin: Point,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), Infallible> {
    // Rotation of -14 degrees, about the car's own center, placed at the
    // sprite's center: the turn grows the car past its unrotated box.
    const SIN: f32 = -0.241_921_9;
    const COS: f32 = 0.970_295_7;
    let tilt = |x: i32, y: i32| -> Point {
        let dx = (x - 36) as f32;
        let dy = (y - 26) as f32;
        origin
            + Point::new(42, 35)
            + Point::new((dx * COS - dy * SIN) as i32, (dx * SIN + dy * COS) as i32)
    };
    let quad = |display: &mut OrientedFrameBuffer<'_>,
                corners: [(i32, i32); 4],
                color: BinaryColor|
     -> Result<(), Infallible> {
        let [a, b, c, d] = corners.map(|(x, y)| tilt(x, y));
        let fill = PrimitiveStyle::with_fill(color);
        Triangle::new(a, b, c).into_styled(fill).draw(display)?;
        Triangle::new(a, c, d).into_styled(fill).draw(display)?;
        Ok(())
    };
    // Wheels first, so the body covers their inner halves.
    for (x, y) in [(11, 0), (48, 0), (11, 43), (48, 43)] {
        quad(
            display,
            [(x, y), (x + 14, y), (x + 14, y + 10), (x, y + 10)],
            ink,
        )?;
    }
    // The body as a fan of triangles over its rounded outline.
    let outline = rounded_rect_points(2, 5, 68, 43, (10, 10), (17, 21));
    let fill = PrimitiveStyle::with_fill(ink);
    let center = origin + Point::new(42, 35);
    for index in 0..outline.len() {
        let start = outline[index];
        let end = outline[(index + 1) % outline.len()];
        Triangle::new(center, tilt(start.x, start.y), tilt(end.x, end.y))
            .into_styled(fill)
            .draw(display)?;
    }
    // The paper windows and hood stripe.
    quad(display, [(42, 14), (54, 14), (54, 38), (42, 38)], paper)?;
    quad(display, [(12, 15), (20, 15), (20, 37), (12, 37)], paper)?;
    quad(display, [(24, 23), (38, 23), (38, 29), (24, 29)], paper)?;
    Ok(())
}

/// Points around a rounded rectangle's outline, clockwise from the top edge.
/// The left corners take the `back` radii, the right ones the `front` radii.
fn rounded_rect_points(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    back: (i32, i32),
    front: (i32, i32),
) -> Vec<Point> {
    let x1 = x + width - 1;
    let y1 = y + height - 1;
    let fit = |(rx, ry): (i32, i32)| (rx.min(width / 2), ry.min(height / 2));
    let (back, front) = (fit(back), fit(front));
    let mut points = Vec::new();
    for (cx, cy, (rx, ry), start) in [
        (x1 - front.0, y + front.1, front, -90),
        (x1 - front.0, y1 - front.1, front, 0),
        (x + back.0, y1 - back.1, back, 90),
        (x + back.0, y + back.1, back, 180),
    ] {
        for step in 0..=6 {
            let angle = (start + step * 15) as f32 * core::f32::consts::PI / 180.0;
            points.push(Point::new(
                cx + (rx as f32 * angle.cos()) as i32,
                cy + (ry as f32 * angle.sin()) as i32,
            ));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use embedded_graphics::pixelcolor::BinaryColor;
    use embedded_graphics::prelude::{Drawable, Point, Primitive, Size};
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

    use super::draw_sprite;
    use crate::{
        framebuffer::FrameBuffer,
        games::canvas::GameSprite,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    /// The speaker is paper-colored: on a black bar it leaves paper pixels
    /// inside its own size and none outside.
    #[test]
    fn the_speaker_draws_paper_inside_its_size_on_a_black_bar() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        Rectangle::new(Point::new(0, 0), Size::new(480, 300))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut display)
            .unwrap();
        let origin = Point::new(100, 200);
        draw_sprite(&mut display, origin, GameSprite::Speaker, BinaryColor::On).unwrap();
        drop(display);
        let (width, height) = GameSprite::Speaker.size();
        let (mut inside, mut outside) = (0_u32, 0_u32);
        for y in 0..300 {
            for x in 0..480 {
                if frame.is_black(Point::new(y, 479 - x)) == Some(true) {
                    continue;
                }
                if x >= origin.x && x < origin.x + width && y >= origin.y && y < origin.y + height {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
        assert_eq!(outside, 0, "paper outside the speaker's size");
        assert!(inside > 60, "the speaker drew {inside} paper pixels");
    }

    /// Every sprite drawn alone at a cell's place leaves no ink outside its
    /// own size, so the cells a step dirties cover what changed.
    #[test]
    fn every_sprite_stays_inside_its_own_size() {
        for sprite in [
            GameSprite::Traffic,
            GameSprite::PlayerCar,
            GameSprite::Cone,
            GameSprite::Burst,
            GameSprite::MoveHint,
            GameSprite::Streaks,
            GameSprite::CrashCar,
        ] {
            let mut frame = FrameBuffer::new_white();
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            let origin = Point::new(100, 200);
            draw_sprite(&mut display, origin, sprite, BinaryColor::On).unwrap();
            drop(display);
            let (width, height) = sprite.size();
            let mut outside = 0_u32;
            let mut inside = 0_u32;
            for y in 0..800 {
                for x in 0..480 {
                    if frame.is_black(Point::new(y, 479 - x)) != Some(true) {
                        continue;
                    }
                    if x >= origin.x
                        && x < origin.x + width
                        && y >= origin.y
                        && y < origin.y + height
                    {
                        inside += 1;
                    } else {
                        outside += 1;
                    }
                }
            }
            assert_eq!(outside, 0, "{sprite:?} drew ink outside its size");
            assert!(inside > 0, "{sprite:?} drew nothing");
        }
    }
}
