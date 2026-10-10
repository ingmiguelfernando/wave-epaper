//! Settings › Reading: the book font, size, alignment, margins, hyphenation,
//! progress and reading stats, in one option list per row.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, CHANGE_HINTS, CHOOSE_HINTS},
            header::draw_header,
            option_list::draw_option_list,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    reader::ReadingSetting,
};

/// Top of the first row and the stride of the seven rows above the bar.
const ROWS_TOP: i32 = 150;
const ROW_STRIDE: i32 = 64;
const ROW_HEIGHT: i32 = 52;

pub fn render_settings_reading(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let prefs = state.reader.preferences;
    let body = state.display.body_style();
    draw_header(display, state.display, "READING", "BOOK PREFERENCES")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: prefs.book_font.label(),
            middle: prefs.font_size.label(),
            right: "PREFS.TXT",
        },
    )?;
    if let Some(highlighted) = state.reader_settings_picker {
        let Some(&setting) = ReadingSetting::ALL.get(state.reader_settings_selected) else {
            return Ok(());
        };
        let (options, current) = prefs.options(setting);
        Text::new(
            setting.label(),
            Point::new(22, 120),
            state.display.heading_style(),
        )
        .draw(display)?;
        draw_option_list(display, state.display, 150, &options, current, highlighted)?;
        draw_bottom_bar(display, state.display, &CHOOSE_HINTS)?;
        return Ok(());
    }
    for (index, setting) in ReadingSetting::ALL.iter().copied().enumerate() {
        let (options, current) = prefs.options(setting);
        let value = options.get(current).copied().unwrap_or("");
        draw_setting_row(
            display,
            ROWS_TOP + index as i32 * ROW_STRIDE,
            setting.label(),
            value,
            state.reader_settings_selected == index,
            body,
        )?;
    }
    draw_bottom_bar(display, state.display, &CHANGE_HINTS)?;
    Ok(())
}

/// One Settings row: its label on the left, its value on the right.
fn draw_setting_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    value: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let ink = if selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let border = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, ROW_HEIGHT as u32))
        .into_styled(border)
        .draw(display)?;
    let text = style.with_color(ink);
    Text::new(
        &text.fit(label, 240),
        Point::new(38, top + ROW_HEIGHT / 2 + text.cap_height() / 2),
        text,
    )
    .draw(display)?;
    let value_width = text.text_width(value);
    Text::new(
        value,
        Point::new(
            442 - value_width,
            top + ROW_HEIGHT / 2 + text.cap_height() / 2,
        ),
        text,
    )
    .draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::widgets::bottom_bar::BOTTOM_BAR_TOP;

    /// The seven rows share the screen and end above the bottom bar.
    #[test]
    fn every_row_fits_above_the_bar() {
        let last_bottom =
            ROWS_TOP + (ReadingSetting::ALL.len() as i32 - 1) * ROW_STRIDE + ROW_HEIGHT;
        assert!(
            last_bottom < BOTTOM_BAR_TOP,
            "row at {last_bottom} passes the bar"
        );
    }
}
