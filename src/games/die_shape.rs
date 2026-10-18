//! The shape of one die, in integers: the outline of a rounded square turned
//! by an angle, and where its pips sit. Pure geometry for the Dice screens,
//! so the host can test it and the device draws it without float
//! trigonometry. Proportions come from the mockup's `die-1` to `die-6`
//! symbols, a 100-unit square with a 6-unit stroke, a 17-unit corner and
//! 9.5-unit pips.

/// Pip centres on a 100-unit die, from the mockup's `die-1` to `die-6`.
pub const PIPS: [&[(i32, i32)]; 7] = [
    &[],
    &[(50, 50)],
    &[(28, 28), (72, 72)],
    &[(28, 28), (50, 50), (72, 72)],
    &[(28, 28), (72, 28), (28, 72), (72, 72)],
    &[(28, 28), (72, 28), (50, 50), (28, 72), (72, 72)],
    &[(28, 26), (72, 26), (28, 50), (72, 50), (28, 74), (72, 74)],
];

/// Corner radius of the rounded square, in hundredths of its size.
pub const CORNER_PERCENT: i32 = 17;
/// Stroke of a settled die, in hundredths of its size.
pub const STROKE_PERCENT: i32 = 6;
/// Stroke of the dashed rest outline, in hundredths of its size.
pub const REST_STROKE_PERCENT: i32 = 4;
/// Dash and gap of the rest outline, in hundredths of the size.
pub const DASH_PERCENT: (i32, i32) = (10, 8);
/// Pip diameter, in hundredths of the size.
pub const PIP_PERCENT: i32 = 19;
/// Points on each rounded corner.
const CORNER_STEPS: i32 = 6;
/// Fixed-point scale of `sin_cos`.
pub const UNIT: i32 = 1024;

/// `(sin, cos)` of `degrees`, scaled by `UNIT`, from Bhaskara's rational
/// approximation (error under 0.2%), with no float maths.
#[must_use]
pub fn sin_cos(degrees: i32) -> (i32, i32) {
    (sin_scaled(degrees), sin_scaled(degrees + 90))
}

fn sin_scaled(degrees: i32) -> i32 {
    let turn = degrees.rem_euclid(360);
    let (half, sign) = if turn < 180 {
        (turn, 1)
    } else {
        (turn - 180, -1)
    };
    let product = half * (180 - half);
    sign * (4 * product * UNIT / (40_500 - product))
}

/// Rotate `(dx, dy)` about the origin by `degrees`.
#[must_use]
pub fn rotate(dx: i32, dy: i32, degrees: i32) -> (i32, i32) {
    let (sin, cos) = sin_cos(degrees);
    ((dx * cos - dy * sin) / UNIT, (dx * sin + dy * cos) / UNIT)
}

/// The closed outline of the rounded square of `size` centred on
/// `(cx, cy)` and turned by `degrees`, as points in order. The last point
/// joins the first.
#[must_use]
pub fn outline(cx: i32, cy: i32, size: i32, degrees: i32) -> Vec<(i32, i32)> {
    let radius = size * CORNER_PERCENT / 100;
    let inner = size / 2 - radius;
    let mut points = Vec::with_capacity((4 * (CORNER_STEPS + 1)) as usize);
    // Corner centres clockwise from the top-right, each sweeping 90 degrees
    // from the direction it starts at.
    for (corner, (sx, sy)) in [(1, -1), (1, 1), (-1, 1), (-1, -1)].into_iter().enumerate() {
        let start = corner as i32 * 90 - 90;
        for step in 0..=CORNER_STEPS {
            let angle = start + 90 * step / CORNER_STEPS;
            let (sin, cos) = sin_cos(angle);
            let dx = sx * inner + radius * cos / UNIT;
            let dy = sy * inner + radius * sin / UNIT;
            let (rx, ry) = rotate(dx, dy, degrees);
            points.push((cx + rx, cy + ry));
        }
    }
    points
}

/// Where the pips of `face` (1 to 6) sit for a die of `size` centred on
/// `(cx, cy)` and turned by `degrees`. Empty for any other face.
#[must_use]
pub fn pip_centres(face: u8, cx: i32, cy: i32, size: i32, degrees: i32) -> Vec<(i32, i32)> {
    let Some(pips) = PIPS.get(usize::from(face)) else {
        return Vec::new();
    };
    pips.iter()
        .map(|&(px, py)| {
            let (rx, ry) = rotate((px - 50) * size / 100, (py - 50) * size / 100, degrees);
            (cx + rx, cy + ry)
        })
        .collect()
}

/// Smallest padding that keeps a die of `size` turned by any angle, with
/// its stroke, inside `size + 2 * padding`: the corner of the square
/// reaches 0.71 of the size from the centre.
#[must_use]
pub fn tilt_padding(size: i32, degrees: i32) -> i32 {
    let (sin, cos) = sin_cos(degrees);
    let reach = (sin.abs() + cos.abs()) * size / UNIT;
    (reach - size) / 2 + size * STROKE_PERCENT / 100 + 2
}

#[cfg(test)]
mod tests {
    use super::{outline, pip_centres, rotate, sin_cos, tilt_padding, PIPS, UNIT};

    #[test]
    fn the_pips_are_the_mockups() {
        assert_eq!(PIPS[1], &[(50, 50)]);
        assert_eq!(PIPS[2], &[(28, 28), (72, 72)]);
        assert_eq!(PIPS[3], &[(28, 28), (50, 50), (72, 72)]);
        assert_eq!(PIPS[4], &[(28, 28), (72, 28), (28, 72), (72, 72)]);
        assert_eq!(PIPS[5].len(), 5);
        assert_eq!(PIPS[6].len(), 6);
    }

    #[test]
    fn sine_and_cosine_are_close_at_the_angles_the_screens_use() {
        for degrees in [0, 14, 24, 30, 31, 45, 60, 90, 135, 180, 270, -24, -30] {
            let (sin, cos) = sin_cos(degrees);
            let radians = f64::from(degrees).to_radians();
            let want_sin = (radians.sin() * f64::from(UNIT)).round() as i32;
            let want_cos = (radians.cos() * f64::from(UNIT)).round() as i32;
            assert!(
                (sin - want_sin).abs() <= 4,
                "sin {degrees}: {sin} vs {want_sin}"
            );
            assert!(
                (cos - want_cos).abs() <= 4,
                "cos {degrees}: {cos} vs {want_cos}"
            );
        }
    }

    #[test]
    fn a_square_die_has_the_mockups_edges_and_corners() {
        let points = outline(100, 100, 100, 0);
        let left = points.iter().map(|p| p.0).min().unwrap();
        let right = points.iter().map(|p| p.0).max().unwrap();
        let top = points.iter().map(|p| p.1).min().unwrap();
        let bottom = points.iter().map(|p| p.1).max().unwrap();
        assert_eq!((left, right, top, bottom), (50, 150, 50, 150));
        // Rounded: the exact corner of the square is not on the outline.
        assert!(!points.contains(&(50, 50)));
        assert!(points.contains(&(50, 67)) || points.contains(&(50, 66)));
    }

    #[test]
    fn a_turned_die_stays_inside_its_padding() {
        for degrees in [-45, -31, -24, 0, 14, 24, 31, 45] {
            let size = 150;
            let pad = tilt_padding(size, degrees);
            for (x, y) in outline(200, 200, size, degrees) {
                assert!(
                    x >= 200 - size / 2 - pad && x <= 200 + size / 2 + pad,
                    "{degrees}: x {x}"
                );
                assert!(
                    y >= 200 - size / 2 - pad && y <= 200 + size / 2 + pad,
                    "{degrees}: y {y}"
                );
            }
        }
    }

    #[test]
    fn rotating_a_quarter_turn_swaps_the_axes() {
        let (x, y) = rotate(10, 0, 90);
        assert!(x.abs() <= 1 && (y - 10).abs() <= 1, "{x},{y}");
    }

    #[test]
    fn pips_turn_with_the_die_and_a_blank_face_has_none() {
        assert!(pip_centres(0, 0, 0, 100, 0).is_empty());
        assert!(pip_centres(7, 0, 0, 100, 0).is_empty());
        // One pip sits at the centre however the die is turned.
        assert_eq!(pip_centres(1, 70, 80, 150, 31), vec![(70, 80)]);
        // Face 2 on a 100 die: pips at (-22,-22) and (22,22) from the centre.
        assert_eq!(pip_centres(2, 0, 0, 100, 0), vec![(-22, -22), (22, 22)]);
        let turned = pip_centres(2, 0, 0, 100, 90);
        assert!(
            (turned[0].0 - 22).abs() <= 1 && (turned[0].1 + 22).abs() <= 1,
            "{turned:?}"
        );
    }
}
