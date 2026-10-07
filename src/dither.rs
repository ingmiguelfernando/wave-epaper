//! Grey levels to black and white for the 1-bit panel.

/// Perceived brightness of an sRGB colour, 0 black to 255 white.
#[must_use]
pub fn luma(red: u8, green: u8, blue: u8) -> u8 {
    ((u32::from(red) * 77 + u32::from(green) * 150 + u32::from(blue) * 29) >> 8) as u8
}

/// Atkinson dithering of a `width` × `height` grey picture, row by row in
/// alternating directions. `grey(x, y)` reads a level and `black(x, y)` is
/// called for every pixel that turns black. Only six eighths of each error
/// spread, so light and dark areas stay clean instead of filling with dots.
pub fn atkinson(
    width: usize,
    height: usize,
    mut grey: impl FnMut(usize, usize) -> u8,
    mut black: impl FnMut(usize, usize),
) {
    // Errors in eighths for this row and the next two, offset by two so the
    // neighbours of the first and last pixel stay inside.
    let mut rows = [0; 3].map(|_| vec![0_i32; width + 4]);
    for y in 0..height {
        let reverse = y % 2 == 1;
        for step in 0..width {
            let x = if reverse { width - 1 - step } else { step };
            let at = x + 2;
            let value = i32::from(grey(x, y)) + rows[0][at] / 8;
            let is_black = value < 128;
            let error = if is_black { value } else { value - 255 };
            let (ahead, far, behind) = if reverse {
                (at - 1, at - 2, at + 1)
            } else {
                (at + 1, at + 2, at - 1)
            };
            rows[0][ahead] += error;
            rows[0][far] += error;
            rows[1][behind] += error;
            rows[1][at] += error;
            rows[1][ahead] += error;
            rows[2][at] += error;
            if is_black {
                black(x, y);
            }
        }
        rows.rotate_left(1);
        rows[2].fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{atkinson, luma};

    fn black_count(width: usize, height: usize, level: u8) -> usize {
        let mut black = 0;
        atkinson(width, height, |_, _| level, |_, _| black += 1);
        black
    }

    #[test]
    fn mid_grey_turns_about_half_black_and_extremes_stay_put() {
        let black = black_count(100, 100, 128);
        assert!((4_000..=6_000).contains(&black), "{black}");
        assert_eq!(black_count(10, 10, 255), 0);
        assert_eq!(black_count(10, 10, 0), 100);
    }

    #[test]
    fn darker_greys_get_more_black_and_near_white_stays_clean() {
        let counts = [32, 96, 160, 224].map(|level| black_count(100, 100, level));
        assert!(
            counts.windows(2).all(|pair| pair[0] > pair[1]),
            "{counts:?}"
        );
        // Floyd–Steinberg would scatter about 4% dots here.
        assert!(black_count(100, 100, 245) < 200);
    }

    #[test]
    fn luma_weights_green_most() {
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(0, 0, 0), 0);
        assert!(luma(0, 255, 0) > luma(255, 0, 0));
        assert!(luma(255, 0, 0) > luma(0, 0, 255));
    }
}
