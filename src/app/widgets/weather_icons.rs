//! Weather icons drawn from the mockup's 24-unit SVG symbols at any size, so
//! large icons stay smooth where scaled pixel art turns into steps.
//!
//! Strokes are a round pen of the mockup's 1.7-unit width moved along each
//! line and arc. Cloud bodies are filled with paper first, so a cloud hides
//! the sun or moon behind it, as `fill: var(--icobg)` does in the mockup.

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle},
};

/// One weather symbol of the mockup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherIcon {
    Sun,
    Moon,
    PartlyCloudy,
    PartlyCloudyNight,
    Cloud,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Thunder,
}

impl WeatherIcon {
    /// The icon for one WMO weather code; clear and partly cloudy nights show
    /// the moon.
    #[must_use]
    pub const fn for_code(code: u16, is_day: bool) -> Self {
        match code {
            0 if !is_day => Self::Moon,
            0 => Self::Sun,
            1 | 2 if !is_day => Self::PartlyCloudyNight,
            1 | 2 => Self::PartlyCloudy,
            45 | 48 => Self::Fog,
            51..=57 => Self::Drizzle,
            61..=67 | 80..=82 => Self::Rain,
            71..=77 | 85 | 86 => Self::Snow,
            95..=99 => Self::Thunder,
            _ => Self::Cloud,
        }
    }
}

/// Mockup stroke width in grid units.
const STROKE: f32 = 1.7;

/// Draw `icon` in a `size`-pixel square at `top_left`: strokes in `ink`,
/// cloud bodies in `paper`.
pub fn draw_weather_icon<D>(
    display: &mut D,
    icon: WeatherIcon,
    top_left: Point,
    size: u32,
    ink: BinaryColor,
    paper: BinaryColor,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let scale = size as f32 / 24.0;
    let pen = Pen {
        origin: (top_left.x as f32, top_left.y as f32),
        scale,
        width: ((STROKE * scale).round() as u32).max(1),
        ink,
        paper,
    };
    let canvas = &mut Canvas { display, pen };
    match icon {
        WeatherIcon::Sun => {
            canvas.circle((12.0, 12.0), 4.3)?;
            for (from, to) in [
                ((12.0, 2.6), (12.0, 5.1)),
                ((12.0, 18.9), (12.0, 21.4)),
                ((2.6, 12.0), (5.1, 12.0)),
                ((18.9, 12.0), (21.4, 12.0)),
                ((5.4, 5.4), (7.2, 7.2)),
                ((16.8, 16.8), (18.6, 18.6)),
                ((5.4, 18.6), (7.2, 16.8)),
                ((16.8, 7.2), (18.6, 5.4)),
            ] {
                canvas.line(from, to)?;
            }
        }
        WeatherIcon::Moon => {
            canvas.crescent(MOON)?;
        }
        WeatherIcon::PartlyCloudy => {
            canvas.circle((8.2, 8.2), 3.3)?;
            for (from, to) in [
                ((8.2, 1.8), (8.2, 3.4)),
                ((1.8, 8.2), (3.4, 8.2)),
                ((3.7, 3.7), (4.8, 4.8)),
                ((12.7, 3.7), (11.6, 4.8)),
                ((3.7, 12.7), (4.8, 11.6)),
            ] {
                canvas.line(from, to)?;
            }
            canvas.cloud((4.2, 3.6), 0.82)?;
        }
        WeatherIcon::PartlyCloudyNight => {
            canvas.crescent(SMALL_MOON)?;
            canvas.cloud((4.2, 3.6), 0.82)?;
        }
        WeatherIcon::Cloud => canvas.cloud((0.0, -1.0), 1.0)?,
        WeatherIcon::Fog => {
            for (from, to) in [
                ((3.0, 8.5), (21.0, 8.5)),
                ((5.0, 12.5), (20.0, 12.5)),
                ((3.0, 16.5), (20.0, 16.5)),
                ((7.0, 20.5), (18.0, 20.5)),
            ] {
                canvas.line(from, to)?;
            }
        }
        WeatherIcon::Drizzle => {
            canvas.cloud(LOW_CLOUD.0, LOW_CLOUD.1)?;
            for at in [
                (8.0, 18.2),
                (12.0, 19.6),
                (16.0, 18.2),
                (10.0, 21.8),
                (14.0, 21.8),
            ] {
                canvas.dot(at)?;
            }
        }
        WeatherIcon::Rain => {
            canvas.cloud(LOW_CLOUD.0, LOW_CLOUD.1)?;
            for x in [8.6, 12.6, 16.6] {
                canvas.line((x, 16.8), (x - 1.4, 20.4))?;
            }
        }
        WeatherIcon::Snow => {
            canvas.cloud(LOW_CLOUD.0, LOW_CLOUD.1)?;
            for x in [8.0, 15.5] {
                canvas.line((x, 16.8), (x, 20.8))?;
                canvas.line((x - 2.0, 18.8), (x + 2.0, 18.8))?;
            }
        }
        WeatherIcon::Thunder => {
            canvas.cloud(LOW_CLOUD.0, LOW_CLOUD.1)?;
            canvas.line((12.8, 15.4), (10.2, 19.2))?;
            canvas.line((10.2, 19.2), (13.4, 19.2))?;
            canvas.line((13.4, 19.2), (11.2, 23.0))?;
        }
    }
    Ok(())
}

type Spot = (f32, f32);

/// The cloud raised and shrunk above rain, snow and lightning.
const LOW_CLOUD: (Spot, f32) = ((1.2, -2.6), 0.9);

/// A crescent from the mockup's two arcs: the outer edge around `outer`'s
/// centre from `tip_a` to `tip_b`, and the inner edge around `inner`'s.
#[derive(Clone, Copy)]
struct Crescent {
    outer: Spot,
    inner: Spot,
    radius: f32,
    tip_a: Spot,
    tip_b: Spot,
}

/// `#w-moon`: both arcs have radius 8.2 between the tips (9.4, 4.4) and
/// (19.6, 14.6); the centres solve the SVG arc flags.
const MOON: Crescent = Crescent {
    outer: (11.74, 12.26),
    inner: (17.26, 6.74),
    radius: 8.2,
    tip_a: (9.4, 4.4),
    tip_b: (19.6, 14.6),
};

/// `#w-pmoon`: the same shape with radius 5 between (5.6, 3.6) and
/// (11.4, 9.4).
const SMALL_MOON: Crescent = Crescent {
    outer: (6.48, 8.52),
    inner: (10.52, 4.48),
    radius: 5.0,
    tip_a: (5.6, 3.6),
    tip_b: (11.4, 9.4),
};

/// `#cloud`: a flat base from (7.8, 19) to (16.8, 19) and three lobes. The
/// centres solve the arcs of the mockup's path.
const CLOUD_LOBES: [(Spot, f32, Spot, Spot); 3] = [
    // Left lobe, from the base round the left side to the top lobe.
    ((7.63, 13.40), 5.6, (7.8, 19.0), (9.1, 8.0)),
    // Top lobe, over the top to the right lobe.
    ((14.09, 11.68), 6.2, (9.1, 8.0), (20.2, 10.6)),
    // Right lobe, round the right side back to the base.
    ((18.5, 14.8), 4.53, (20.2, 10.6), (16.8, 19.0)),
];

/// Grid-to-pixel mapping and the round pen.
#[derive(Clone, Copy)]
struct Pen {
    origin: (f32, f32),
    scale: f32,
    width: u32,
    ink: BinaryColor,
    paper: BinaryColor,
}

impl Pen {
    fn pixel(self, at: Spot) -> (f32, f32) {
        (
            self.origin.0 + at.0 * self.scale,
            self.origin.1 + at.1 * self.scale,
        )
    }
}

struct Canvas<'a, D> {
    display: &'a mut D,
    pen: Pen,
}

impl<D> Canvas<'_, D>
where
    D: DrawTarget<Color = BinaryColor>,
{
    /// One round pen mark centred at a pixel position.
    fn stamp(&mut self, at: (f32, f32)) -> Result<(), D::Error> {
        let centre = Point::new(at.0.round() as i32, at.1.round() as i32);
        Circle::with_center(centre, self.pen.width)
            .into_styled(PrimitiveStyle::with_fill(self.pen.ink))
            .draw(self.display)
    }

    fn dot(&mut self, at: Spot) -> Result<(), D::Error> {
        self.stamp(self.pen.pixel(at))
    }

    /// A straight stroke between two grid points, with round ends.
    fn line(&mut self, from: Spot, to: Spot) -> Result<(), D::Error> {
        let (a, b) = (self.pen.pixel(from), self.pen.pixel(to));
        let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let steps = (length * 2.0).ceil().max(1.0) as u32;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            self.stamp((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))?;
        }
        Ok(())
    }

    /// A stroke along the circle round `centre` from `from` to `to`, turning
    /// clockwise on screen when `clockwise`.
    fn arc(
        &mut self,
        centre: Spot,
        radius: f32,
        from: Spot,
        to: Spot,
        clockwise: bool,
    ) -> Result<(), D::Error> {
        let start = (from.1 - centre.1).atan2(from.0 - centre.0);
        let end = (to.1 - centre.1).atan2(to.0 - centre.0);
        let tau = core::f32::consts::TAU;
        let mut sweep = (end - start).rem_euclid(tau);
        if !clockwise {
            sweep -= tau;
        }
        self.sweep(centre, radius, start, sweep)
    }

    fn sweep(&mut self, centre: Spot, radius: f32, start: f32, sweep: f32) -> Result<(), D::Error> {
        let (cx, cy) = self.pen.pixel(centre);
        let r = radius * self.pen.scale;
        let steps = (sweep.abs() * r * 2.0).ceil().max(1.0) as u32;
        for step in 0..=steps {
            let angle = start + sweep * step as f32 / steps as f32;
            self.stamp((cx + r * angle.cos(), cy + r * angle.sin()))?;
        }
        Ok(())
    }

    fn circle(&mut self, centre: Spot, radius: f32) -> Result<(), D::Error> {
        self.sweep(centre, radius, 0.0, core::f32::consts::TAU)
    }

    fn crescent(&mut self, moon: Crescent) -> Result<(), D::Error> {
        self.arc(moon.outer, moon.radius, moon.tip_a, moon.tip_b, false)?;
        self.arc(moon.inner, moon.radius, moon.tip_b, moon.tip_a, true)
    }

    /// The mockup's cloud moved by `shift` and scaled by `factor`: its body in
    /// paper, then its outline in ink.
    fn cloud(&mut self, shift: Spot, factor: f32) -> Result<(), D::Error> {
        let place = |at: Spot| (shift.0 + at.0 * factor, shift.1 + at.1 * factor);
        for (centre, radius, _, _) in CLOUD_LOBES {
            let (cx, cy) = self.pen.pixel(place(centre));
            let diameter = (2.0 * radius * factor * self.pen.scale).round() as u32;
            Circle::with_center(Point::new(cx.round() as i32, cy.round() as i32), diameter)
                .into_styled(PrimitiveStyle::with_fill(self.pen.paper))
                .draw(self.display)?;
        }
        let (left, top) = self.pen.pixel(place((7.63, 13.4)));
        let (right, bottom) = self.pen.pixel(place((18.5, 19.0)));
        Rectangle::new(
            Point::new(left.round() as i32, top.round() as i32),
            Size::new(
                (right - left).round().max(0.0) as u32,
                (bottom - top).round().max(0.0) as u32,
            ),
        )
        .into_styled(PrimitiveStyle::with_fill(self.pen.paper))
        .draw(self.display)?;

        self.line(place((16.8, 19.0)), place((7.8, 19.0)))?;
        for (centre, radius, from, to) in CLOUD_LOBES {
            self.arc(place(centre), radius * factor, place(from), place(to), true)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    const ALL: [WeatherIcon; 10] = [
        WeatherIcon::Sun,
        WeatherIcon::Moon,
        WeatherIcon::PartlyCloudy,
        WeatherIcon::PartlyCloudyNight,
        WeatherIcon::Cloud,
        WeatherIcon::Fog,
        WeatherIcon::Drizzle,
        WeatherIcon::Rain,
        WeatherIcon::Snow,
        WeatherIcon::Thunder,
    ];

    /// Black pixels of `icon` drawn at (40, 40) in a `size` square, in logical
    /// portrait coordinates.
    fn ink(icon: WeatherIcon, size: u32) -> Vec<(i32, i32)> {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        draw_weather_icon(
            &mut display,
            icon,
            Point::new(40, 40),
            size,
            BinaryColor::On,
            BinaryColor::Off,
        )
        .unwrap();
        drop(display);
        let mut black = Vec::new();
        for y in 0..800 {
            for x in 0..480 {
                // Portrait logical (x, y) is native (y, 479 - x).
                if frame.is_black(Point::new(y, 479 - x)) == Some(true) {
                    black.push((x, y));
                }
            }
        }
        black
    }

    #[test]
    fn every_icon_draws_inside_its_square_at_small_and_large_sizes() {
        for icon in ALL {
            for size in [24, 52, 130] {
                let black = ink(icon, size);
                assert!(!black.is_empty(), "{icon:?} at {size} draws nothing");
                let margin = (STROKE * size as f32 / 24.0).ceil() as i32;
                let (low, high) = (40 - margin, 40 + size as i32 + margin);
                assert!(
                    black
                        .iter()
                        .all(|&(x, y)| (low..high).contains(&x) && (low..high).contains(&y)),
                    "{icon:?} at {size} leaves its square"
                );
            }
        }
    }

    #[test]
    fn the_sun_is_a_ring_with_rays_and_an_empty_middle() {
        let black = ink(WeatherIcon::Sun, 120);
        let centre = (40 + 60, 40 + 60);
        assert!(!black.contains(&centre), "the middle of the sun is paper");
        // The ring passes 4.3 units right of the centre, the ray tip at 21.4.
        assert!(black.contains(&(40 + 60 + 21, 40 + 60)), "ring");
        assert!(black.contains(&(40 + 107, 40 + 60)), "right ray");
    }

    #[test]
    fn the_cloud_body_hides_what_is_behind_it() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        Rectangle::new(Point::new(40, 40), Size::new(120, 120))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut display)
            .unwrap();
        draw_weather_icon(
            &mut display,
            WeatherIcon::Cloud,
            Point::new(40, 40),
            120,
            BinaryColor::On,
            BinaryColor::Off,
        )
        .unwrap();
        drop(display);
        // Portrait logical (x, y) is native (y, 479 - x); one unit is 5 px.
        let black = |x: i32, y: i32| frame.is_black(Point::new(40 + y * 5, 479 - (40 + x * 5)));
        assert_eq!(black(13, 14), Some(false), "inside the cloud is paper");
        assert_eq!(black(2, 2), Some(true), "outside keeps what was there");
    }

    #[test]
    fn codes_map_as_the_bitmap_icons_did() {
        assert_eq!(WeatherIcon::for_code(0, true), WeatherIcon::Sun);
        assert_eq!(WeatherIcon::for_code(0, false), WeatherIcon::Moon);
        assert_eq!(WeatherIcon::for_code(2, false), WeatherIcon::PartlyCloudyNight);
        assert_eq!(WeatherIcon::for_code(3, true), WeatherIcon::Cloud);
        assert_eq!(WeatherIcon::for_code(48, true), WeatherIcon::Fog);
        assert_eq!(WeatherIcon::for_code(55, true), WeatherIcon::Drizzle);
        assert_eq!(WeatherIcon::for_code(81, true), WeatherIcon::Rain);
        assert_eq!(WeatherIcon::for_code(86, true), WeatherIcon::Snow);
        assert_eq!(WeatherIcon::for_code(96, false), WeatherIcon::Thunder);
    }
}
