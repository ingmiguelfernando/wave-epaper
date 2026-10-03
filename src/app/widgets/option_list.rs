//! Option list for setting pickers: one row per choice, the highlighted row
//! inverted and the choice in use marked. Long lists scroll with the cursor.

use core::convert::Infallible;

use crate::{
    app::{
        display::DisplayPreferences,
        widgets::list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
    },
    orientation::OrientedFrameBuffer,
};

/// First pixel row of the footer rule drawn by `draw_footer`.
const FOOTER_TOP: i32 = 746;

/// Choice labels and the index of the value in use, defaulting to the first row.
#[must_use]
pub fn option_labels<T: Copy + PartialEq>(
    all: &[T],
    current: T,
    label: fn(T) -> &'static str,
) -> (Vec<&'static str>, usize) {
    (
        all.iter().copied().map(label).collect(),
        all.iter().position(|&value| value == current).unwrap_or(0),
    )
}

pub fn draw_option_list(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    top: i32,
    options: &[&str],
    current: usize,
    highlighted: usize,
) -> Result<(), Infallible> {
    let visible = ((FOOTER_TOP - top) / LIST_ROW_HEIGHT).max(1) as usize;
    let first = highlighted.saturating_sub(visible - 1);
    for (index, option) in options.iter().enumerate().skip(first).take(visible) {
        let row = ListRow {
            title: option,
            subtitle: "",
            value: if index == current { "IN USE" } else { "" },
            selected: index == highlighted,
        };
        let row_top = top + (index - first) as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, preferences, row_top, row)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::{draw_option_list, option_labels};
    use crate::{
        app::display::DisplayPreferences,
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
    };

    #[test]
    fn labels_preserve_order_and_find_the_current_value() {
        assert_eq!(
            option_labels(&[true, false], false, |value| if value {
                "On"
            } else {
                "Off"
            }),
            (vec!["On", "Off"], 1)
        );
    }

    #[test]
    fn missing_current_value_and_empty_choices_default_to_zero() {
        assert_eq!(option_labels(&[false], true, |_| "Off"), (vec!["Off"], 0));
        assert_eq!(option_labels::<bool>(&[], true, |_| "On"), (vec![], 0));
    }

    #[test]
    fn long_lists_scroll_to_keep_the_highlighted_row_visible() {
        let options = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
        let preferences = DisplayPreferences::default();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
        draw_option_list(&mut display, preferences, 184, &options, 0, 9).unwrap();
        drop(display);
        // Seven rows fit above the footer; the highlighted last row is the
        // seventh and fills its band (logical y 616..688) with black.
        assert_eq!(frame.is_black(Point::new(650, 479 - 240)), Some(true));
        assert_eq!(frame.is_black(Point::new(200, 479 - 240)), Some(false));
    }
}
