//! ES8311 playback controls and readable board-profile details.

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
            bottom_bar::{draw_bottom_bar, BACK_HINTS, RUN_HINTS},
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    audio::{
        AUDIO_AMP_ENABLE_GPIO, AUDIO_BCLK_GPIO, AUDIO_DIN_GPIO, AUDIO_DOUT_GPIO, AUDIO_MCLK_GPIO,
        AUDIO_SAMPLE_RATE_HZ, AUDIO_WS_GPIO,
    },
    orientation::OrientedFrameBuffer,
};

const I2S_MODE_LABEL: &str = "TX + RX / S16 STEREO";

fn rx_input_label() -> String {
    format!("DIN GPIO{AUDIO_DIN_GPIO} · voice notes")
}

pub fn render_audio(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let audio = &state.audio;
    let volume = format!("{}%", audio.volume_percent);
    let amp = if audio.amplifier_enabled { "ON" } else { "OFF" };
    let mute = if audio.muted { "Muted" } else { "Active" };

    draw_header(display, state.display, "AUDIO", "PLAYBACK AND ALARMS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: audio.playback_state.label(),
            middle: &volume,
            right: amp,
        },
    )?;

    Text::new("Playback controls", Point::new(22, 158), heading).draw(display)?;
    line(display, 202, "Status", mute, body)?;
    line(display, 236, "Volume", &volume, body)?;
    line(display, 270, "Amplifier", amp, body)?;

    let labels = [
        "Play test chime",
        "Stop playback",
        "Increase volume",
        "Decrease volume",
        if audio.muted { "Unmute" } else { "Mute" },
        "Audio details",
    ];
    for (index, label) in labels.into_iter().enumerate() {
        draw_action(
            display,
            318 + index as i32 * 58,
            label,
            state.audio_action_selected == index,
            body,
        )?;
    }
    draw_bottom_bar(display, state.display, &RUN_HINTS)?;
    Ok(())
}

pub fn render_audio_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let audio = &state.audio;
    let address = audio.codec_address_label();
    let volume = format!("{}%", audio.volume_percent);
    let amp = if audio.amplifier_enabled { "ON" } else { "OFF" };
    let mute = if audio.muted { "MUTED" } else { "ACTIVE" };

    draw_header(
        display,
        state.display,
        "AUDIO DETAILS",
        "ES8311 BOARD PROFILE",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: audio.playback_state.label(),
            middle: &volume,
            right: amp,
        },
    )?;

    Text::new("Codec", Point::new(22, 160), heading).draw(display)?;
    line(display, 204, "Device", "ES8311 BSP-REF58", body)?;
    line(display, 238, "Address", &address, body)?;
    line(display, 272, "I2S mode", I2S_MODE_LABEL, body)?;
    line(
        display,
        306,
        "Sample rate",
        &format!("{AUDIO_SAMPLE_RATE_HZ} Hz"),
        body,
    )?;

    Text::new("Routing", Point::new(22, 370), heading).draw(display)?;
    line(
        display,
        414,
        "TX pins",
        &format!("M{AUDIO_MCLK_GPIO} B{AUDIO_BCLK_GPIO} W{AUDIO_WS_GPIO} D{AUDIO_DOUT_GPIO}"),
        body,
    )?;
    line(display, 448, "RX input", &rx_input_label(), body)?;
    line(
        display,
        482,
        "Amplifier",
        &format!("GPIO{AUDIO_AMP_ENABLE_GPIO} {amp}"),
        body,
    )?;
    line(display, 516, "Mute", mute, body)?;
    line(display, 550, "Volume", &volume, body)?;

    Text::new("Last error", Point::new(22, 614), heading).draw(display)?;
    Text::new(
        audio.error.as_deref().unwrap_or("none"),
        Point::new(22, 652),
        detail,
    )
    .draw(display)?;
    draw_bottom_bar(display, state.display, &BACK_HINTS)?;
    Ok(())
}

fn line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(22, y), style).draw(display)?;
    Text::new(value, Point::new(170, y), style).draw(display)?;
    Ok(())
}

fn draw_action(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 48))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 31),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 31), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_audio, render_audio_details, rx_input_label, I2S_MODE_LABEL};
    use crate::{
        app::{
            display::{DisplayPreferences, UiFontFamily, UiFontSize},
            AppState,
        },
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
    };

    #[test]
    fn audio_details_describe_bidirectional_i2s_and_voice_notes_input() {
        assert_eq!(I2S_MODE_LABEL, "TX + RX / S16 STEREO");
        assert_eq!(rx_input_label(), "DIN GPIO21 · voice notes");
    }

    #[test]
    fn audio_details_corrected_rows_fit_every_ui_font_and_size() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let mut state = AppState::default();
                state.display = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let body = state.display.body_style();
                for (label, value) in [
                    ("I2S mode", I2S_MODE_LABEL.to_string()),
                    ("RX input", rx_input_label()),
                ] {
                    assert!(22 + body.text_width(label) < 170);
                    assert!(170 + body.text_width(&value) <= 458);
                }
                assert!(body.line_height() < 34);
                let mut frame = FrameBuffer::new_white();
                let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
                render_audio_details(&mut display, &state).unwrap();
            }
        }
    }

    #[test]
    fn audio_overview_and_details_render_when_codec_is_unavailable() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        let state = AppState::default();
        render_audio(&mut display, &state).unwrap();
        render_audio_details(&mut display, &state).unwrap();
    }
}
