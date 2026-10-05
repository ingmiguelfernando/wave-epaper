//! Settings › Weather: the service switch, how often it updates, units and
//! whether Home shows the weather.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::Text,
        widgets::{
            bottom_bar::{draw_bottom_bar, BACK_HINTS, CHANGE_HINTS, CHOOSE_HINTS},
            header::draw_header,
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    weather_config::{refresh_label, WeatherConfig, WeatherSetting},
};

use super::weather::draw_paragraph;

const ROWS_TOP: i32 = 184;
const NOT_SET_UP: &str = "Save WEATHER.TXT with your location in /RUSTMIX on the SD card. \
    The SD card guide has an example. This screen then turns the service on or off and \
    sets how often it updates.";

pub fn render_weather_settings(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    draw_header(display, preferences, "WEATHER", "FORECAST AND UPDATES")?;
    let Some(config) = state.weather_config.as_ref() else {
        let heading = preferences.heading_style();
        let baseline = draw_paragraph(display, heading, "Weather is not set up", 150)?;
        draw_paragraph(display, preferences.body_style(), NOT_SET_UP, baseline + 8)?;
        return draw_bottom_bar(display, preferences, &BACK_HINTS);
    };
    let place = preferences.body_style().fit(&config.location, 170);
    draw_status_row(
        display,
        preferences,
        StatusRow {
            left: if config.enabled { "ON" } else { "OFF" },
            middle: &place,
            right: config.units.label(),
        },
    )?;
    let heading = preferences.heading_style();
    if let Some(highlighted) = state.weather_settings_ui.picker {
        let setting = state.weather_settings_ui.setting();
        let (options, current) = config.options(setting);
        Text::new(setting.label(), Point::new(22, 160), heading).draw(display)?;
        draw_option_list(display, preferences, 184, &options, current, highlighted)?;
        return draw_bottom_bar(display, preferences, &CHOOSE_HINTS);
    }

    Text::new("Forecast", Point::new(22, 160), heading).draw(display)?;
    for (index, setting) in WeatherSetting::ALL.into_iter().enumerate() {
        let value = config.value_label(setting);
        let row = ListRow {
            title: setting.label(),
            subtitle: subtitle(setting),
            value: &value,
            selected: state.weather_settings_ui.selected == index,
        };
        let top = ROWS_TOP + index as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, preferences, top, row)?;
    }

    let body = preferences.body_style();
    let location = format!("Location: {}, from WEATHER.TXT", config.location);
    let mut baseline = ROWS_TOP + WeatherSetting::ALL.len() as i32 * LIST_ROW_HEIGHT + 34;
    for line in [location.as_str(), "Provider: Open-Meteo, no key"] {
        let line = body.fit(line, 436);
        Text::new(&line, Point::new(22, baseline), body).draw(display)?;
        baseline += i32::from(body.line_height()) + 4;
    }
    draw_infobox(display, state, &battery_note(config), baseline + 6)?;
    draw_bottom_bar(display, preferences, &CHANGE_HINTS)
}

fn subtitle(setting: WeatherSetting) -> &'static str {
    match setting {
        WeatherSetting::Service => "Fetch the forecast on a schedule",
        WeatherSetting::Refresh => "Each update turns Wi-Fi on briefly",
        WeatherSetting::Units => "BOOT on Weather switches them too",
        WeatherSetting::ShowOnHome => "Weather strip and row on Home",
    }
}

/// What updates cost in battery at the chosen interval.
fn battery_note(config: &WeatherConfig) -> String {
    let schedule = match config.refresh_minutes {
        0 => "Manual: Wi-Fi turns on only when you press SELECT on Weather.".to_string(),
        minutes => {
            // About 0.15 mAh per update.
            let tenths = 2160 / minutes;
            format!(
                "Every {} is about {}.{} mAh a day.",
                refresh_label(minutes),
                tenths / 10,
                tenths % 10
            )
        }
    };
    format!(
        "Each update turns Wi-Fi on for about 4 s (0.1\u{2013}0.2 mAh). {schedule} When off, \
         Wave makes no weather requests and Home hides the weather."
    )
}

/// A framed note under the rows.
fn draw_infobox(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    text: &str,
    top: i32,
) -> Result<(), Infallible> {
    let body = state.display.body_style();
    let lines = body.wrap(text, 408);
    let line_height = i32::from(body.line_height()) + 2;
    let height = lines.len() as i32 * line_height + 18;
    Rectangle::new(Point::new(22, top), Size::new(436, height as u32))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    let mut baseline = top + 10 + body.cap_height();
    for line in lines {
        Text::new(&line, Point::new(36, baseline), body).draw(display)?;
        baseline += line_height;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::battery_note;
    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        weather_config::{WeatherConfig, SAMPLE_CONFIG},
    };

    #[test]
    fn rows_pickers_and_the_missing_file_render() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::WeatherSettings);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        state.set_weather_config(Some(WeatherConfig::parse(SAMPLE_CONFIG).unwrap()));
        for _ in 0..4 {
            render_current_screen(&mut frame, &state).unwrap();
            state.apply(ButtonEvent::Select);
            assert!(state.weather_settings_ui.picker.is_some());
            render_current_screen(&mut frame, &state).unwrap();
            state.back();
            state.apply(ButtonEvent::Down);
        }
        assert_eq!(state.weather_settings_ui.selected, 0);
        assert_eq!(state.active_route(), ScreenRoute::WeatherSettings);
    }

    #[test]
    fn battery_note_follows_the_interval() {
        let mut config = WeatherConfig::parse(SAMPLE_CONFIG).unwrap();
        assert!(battery_note(&config).contains("Every 2 hours is about 1.8 mAh a day."));
        config.refresh_minutes = 30;
        assert!(battery_note(&config).contains("about 7.2 mAh"));
        config.refresh_minutes = 0;
        assert!(battery_note(&config).contains("Manual"));
    }
}
