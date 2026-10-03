//! Grey levels to black and white for the 1-bit panel.

/// Perceived brightness of an sRGB colour, 0 black to 255 white.
#[must_use]
pub fn luma(red: u8, green: u8, blue: u8) -> u8 {
    ((u32::from(red) * 77 + u32::from(green) * 150 + u32::from(blue) * 29) >> 8) as u8
}

/// Floyd–Steinberg dithering of a `width` × `height` grey picture. `grey(x, y)`
/// reads a level and `black(x, y)` is called for every pixel that turns black.
pub fn floyd_steinberg(
    width: usize,
    height: usize,
    mut grey: impl FnMut(usize, usize) -> u8,
    mut black: impl FnMut(usize, usize),
) {
    // Diffused errors in sixteenths, offset by one so x - 1 never underflows.
    let mut current = vec![0_i32; width + 2];
    let mut next = vec![0_i32; width + 2];
    for y in 0..height {
        for x in 0..width {
            let value = i32::from(grey(x, y)) + current[x + 1] / 16;
            let is_black = value < 128;
            let error = if is_black { value } else { value - 255 };
            current[x + 2] += error * 7;
            next[x] += error * 3;
            next[x + 1] += error * 5;
            next[x + 2] += error;
            if is_black {
                black(x, y);
            }
        }
        core::mem::swap(&mut current, &mut next);
        next.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{floyd_steinberg, luma};

    #[test]
    fn mid_grey_turns_about_half_black_and_extremes_stay_put() {
        let mut black = 0;
        floyd_steinberg(100, 100, |_, _| 128, |_, _| black += 1);
        assert!((4_500..=5_500).contains(&black), "{black}");
        let mut white_black = 0;
        floyd_steinberg(10, 10, |_, _| 255, |_, _| white_black += 1);
        assert_eq!(white_black, 0);
        let mut ink = 0;
        floyd_steinberg(10, 10, |_, _| 0, |_, _| ink += 1);
        assert_eq!(ink, 100);
    }

    #[test]
    fn luma_weights_green_most() {
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(0, 0, 0), 0);
        assert!(luma(0, 255, 0) > luma(255, 0, 0));
        assert!(luma(255, 0, 0) > luma(0, 0, 255));
    }
}
