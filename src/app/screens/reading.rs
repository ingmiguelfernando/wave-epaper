//! Settings › Reading: the book font, size, alignment, margins, hyphenation,
//! progress and reading stats, in one option list per row.

use core::convert::Infallible;

use embedded_graphics::prelude::Point;

use crate::{
    app::{
        state::AppState,
        typography::Text,
        widgets::{
            bottom_bar::{draw_bottom_bar, CHANGE_HINTS, CHOOSE_HINTS},
            list_row::{draw_list_row, first_visible, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
    reader::ReadingSetting,
};

pub fn render_settings_reading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let prefs = state.reader.preferences;
    draw_status_bar(display, state.display, "Settings › Reading")?;
    draw_status_text(display, state.display, "PREFS.TXT", STATUS_BAR_RIGHT)?;
    let top = STATUS_BAR_HEIGHT;
    if let Some(highlighted) = state.reader_settings_picker {
        let Some(&setting) = ReadingSetting::ALL.get(state.reader_settings_selected) else {
            return Ok(());
        };
        let (options, current) = prefs.options(setting);
        Text::new(
            setting.label(),
            Point::new(22, top + 40),
            state.display.heading_style(),
        )
        .draw(display)?;
        draw_option_list(
            display,
            state.display,
            top + 64,
            &options,
            current,
            highlighted,
        )?;
        draw_bottom_bar(display, state.display, &CHOOSE_HINTS)?;
        return Ok(());
    }
    let first = first_visible(state.reader_settings_selected, top);
    for (index, setting) in ReadingSetting::ALL.iter().copied().enumerate().skip(first) {
        let (options, current) = prefs.options(setting);
        let value = options.get(current).copied().unwrap_or("");
        let row = ListRow {
            title: setting.label(),
            subtitle: "",
            value,
            selected: state.reader_settings_selected == index,
        };
        draw_list_row(
            display,
            state.display,
            top + (index - first) as i32 * LIST_ROW_HEIGHT,
            row,
        )?;
    }
    draw_bottom_bar(display, state.display, &CHANGE_HINTS)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::widgets::bottom_bar::BOTTOM_BAR_TOP;

    /// The rows end above the bottom bar; when a list outgrows the space the
    /// window scrolls, which keeps every UI size inside the same bounds.
    #[test]
    fn every_row_fits_above_the_bar() {
        let top = STATUS_BAR_HEIGHT;
        let last_bottom = top + ReadingSetting::ALL.len() as i32 * LIST_ROW_HEIGHT;
        assert!(
            last_bottom <= BOTTOM_BAR_TOP,
            "row at {last_bottom} passes the bar"
        );
    }
}
