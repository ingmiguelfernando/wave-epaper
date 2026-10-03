//! Weather: now, the next hours and the next days, and a second page with
//! twelve hours. BOOT switches between °C and °F.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            big_digits::{big_text_width, draw_big_text},
            footer::draw_footer,
            header::draw_header,
            icons::{weather_icon_at, ICON_SIZE},
        },
    },
    orientation::OrientedFrameBuffer,
    regional::TemperatureUnit,
    weather::{condition_label, short_degrees_label, CurrentConditions, WeatherFetchState},
    weather_config::{refresh_label, WeatherConfig},
};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;
const PARAGRAPH_WIDTH: i32 = 436;
const NOW_TOP: i32 = 84;
/// Left edge of the text beside the big icon.
const NOW_TEXT: i32 = 162;
const TEMPERATURE_HEIGHT: i32 = 80;
const CHIPS_TOP: i32 = 242;
const CHIP_HEIGHT: i32 = 58;
const CHIP_GAP: i32 = 6;
const HOURS_TOP: i32 = 336;
const HOURS_HEIGHT: i32 = 112;
const DAYS_TOP: i32 = 482;
const DAY_HEIGHT: i32 = 52;
const HOUR_ROWS_TOP: i32 = 106;
const HOUR_ROW_HEIGHT: i32 = 42;
const FOOTER: &str = "DOWN MORE  SELECT REFRESH  BOOT \u{b0}C/\u{b0}F";
const DETAILS_FOOTER: &str = "UP BACK  SELECT REFRESH  BOOT \u{b0}C/\u{b0}F";

/// A title and what to do, shown instead of a forecast.
type Missing = (&'static str, &'static str);

const NOT_SET_UP: Missing = (
    "Weather is not set up",
    "Save WEATHER.TXT with your location in /RUSTMIX on the SD card. The SD card \
     guide has an example.",
);
const TURNED_OFF: Missing = (
    "Weather is off",
    "Turn it on in Settings \u{203a} Weather. While it is off, Wave makes no \
     weather requests.",
);

pub fn render_weather(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(display, state.display, "WEATHER", &place(state))?;
    let (config, current) = match shown_forecast(state) {
        Ok(found) => found,
        Err(missing) => return draw_missing(display, state, missing),
    };
    let unit = state.regional.temperature_unit;
    draw_now(display, state, current, unit)?;
    draw_chips(display, state, current, unit)?;
    draw_hours(display, state, unit)?;
    draw_days(display, state, unit)?;
    let detail = state.display.detail_style();
    let updated = detail.fit(&updated_line(state, config), RIGHT - LEFT);
    let baseline = DAYS_TOP + 4 * DAY_HEIGHT + 26;
    Text::new(&updated, Point::new(LEFT + 4, baseline), detail).draw(display)?;
    draw_footer(display, state.display, FOOTER)
}

/// The second page: the next twelve hours and where the forecast comes from.
pub fn render_weather_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_header(display, state.display, "WEATHER", &place(state))?;
    let config = match shown_forecast(state) {
        Ok((config, _)) => config,
        Err(missing) => return draw_missing(display, state, missing),
    };
    let unit = state.regional.temperature_unit;
    let detail = state.display.detail_style();
    let label = Point::new(LEFT + 2, HOUR_ROWS_TOP - 10);
    Text::new("NEXT 12 HOURS", label, detail).draw(display)?;
    for (index, hour) in state.weather.hourly.iter().take(12).enumerate() {
        let row = ForecastRow {
            name: hour.hour_label(),
            weather_code: hour.weather_code,
            is_day: hour.is_day,
            temperatures: short_degrees_label(hour.temperature_tenths_f, unit),
            rain: hour.precipitation_probability_percent,
        };
        let top = HOUR_ROWS_TOP + index as i32 * HOUR_ROW_HEIGHT;
        draw_forecast_row(display, state, top, HOUR_ROW_HEIGHT, &row)?;
    }
    let mut about = vec![
        updated_line(state, config),
        format!(
            "{} \u{b7} {:.2}, {:.2} \u{b7} {}",
            config.location, config.latitude, config.longitude, config.timezone
        ),
    ];
    if let Some(error) = state.weather.error.as_deref() {
        about.push(format!("Last error: {error}"));
    }
    let mut baseline = HOUR_ROWS_TOP + 12 * HOUR_ROW_HEIGHT + 30;
    for text in about {
        let text = detail.fit(&text, RIGHT - LEFT);
        Text::new(&text, Point::new(LEFT + 4, baseline), detail).draw(display)?;
        baseline += i32::from(detail.line_height()) + 6;
    }
    draw_footer(display, state.display, DETAILS_FOOTER)
}

/// The configuration and current conditions, or why there are none.
fn shown_forecast(state: &AppState) -> Result<(&WeatherConfig, &CurrentConditions), Missing> {
    let config = state.weather_config.as_ref().ok_or(NOT_SET_UP)?;
    if !config.enabled {
        return Err(TURNED_OFF);
    }
    let waiting = match state.weather.state {
        WeatherFetchState::Fetching => "Getting the forecast...",
        WeatherFetchState::Retrying => "The update failed. Trying again in a moment.",
        WeatherFetchState::Failed => "The update failed. Press SELECT to try again.",
        _ if config.refresh_minutes == 0 => "Updates are manual. Press SELECT to update now.",
        _ => "Waiting for Wi-Fi. Press SELECT to update now.",
    };
    let missing = ("No forecast yet", waiting);
    let current = state.weather.current.as_ref().ok_or(missing)?;
    Ok((config, current))
}

/// Not set up, off, or no forecast yet: a title and what to do.
fn draw_missing(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    (title, text): Missing,
) -> Result<(), Infallible> {
    let preferences = state.display;
    let mut baseline = draw_paragraph(display, preferences.heading_style(), title, 150)?;
    baseline = draw_paragraph(display, preferences.body_style(), text, baseline + 8)?;
    if !state.weather_enabled() {
        return draw_footer(display, preferences, "HOLD BOOT BACK");
    }
    if let Some(error) = state.weather.error.as_deref() {
        draw_paragraph(display, preferences.detail_style(), error, baseline + 8)?;
    }
    draw_footer(display, preferences, "SELECT REFRESH  HOLD BOOT BACK")
}

/// Wrap `text` across the screen from `baseline`; returns the baseline of
/// the line after it.
pub(super) fn draw_paragraph(
    display: &mut OrientedFrameBuffer<'_>,
    style: UiTextStyle,
    text: &str,
    mut baseline: i32,
) -> Result<i32, Infallible> {
    for line in style.wrap(text, PARAGRAPH_WIDTH) {
        let line = style.fit(&line, PARAGRAPH_WIDTH);
        Text::new(&line, Point::new(22, baseline), style).draw(display)?;
        baseline += i32::from(style.line_height()) + 4;
    }
    Ok(baseline)
}

/// The configured place in capitals, for the header.
fn place(state: &AppState) -> String {
    let text = match &state.weather_config {
        Some(config) => config.location.to_uppercase(),
        None => "NOT SET UP".into(),
    };
    state.display.header_subtitle_style().fit(&text, 444)
}

/// Big icon and temperature, the condition, and today's high and low.
fn draw_now(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    current: &CurrentConditions,
    unit: TemperatureUnit,
) -> Result<(), Infallible> {
    let icon = weather_icon_at(current.weather_code, current.is_day);
    icon.draw_scaled(display, Point::new(LEFT, NOW_TOP), 5, BinaryColor::On)?;
    let temperature = short_degrees_label(current.temperature_tenths_f, unit);
    let width = big_text_width(&temperature, TEMPERATURE_HEIGHT);
    let top = NOW_TOP + 6;
    draw_big_text(
        display,
        &temperature,
        NOW_TEXT + width / 2,
        top,
        TEMPERATURE_HEIGHT,
    )?;
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let mut baseline = top + TEMPERATURE_HEIGHT + 12 + heading.cap_height();
    let condition = heading.fit(current.condition_label(), RIGHT - NOW_TEXT);
    Text::new(&condition, Point::new(NOW_TEXT, baseline), heading).draw(display)?;
    if let Some(today) = state.weather.forecast.first() {
        baseline += i32::from(body.line_height()) + 4;
        let range = format!(
            "H {} \u{b7} L {}",
            short_degrees_label(today.high_tenths_f, unit),
            short_degrees_label(today.low_tenths_f, unit)
        );
        Text::new(&range, Point::new(NOW_TEXT, baseline), body).draw(display)?;
    }
    Ok(())
}

/// Feels like, humidity, wind and today's chance of rain in four boxes.
fn draw_chips(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    current: &CurrentConditions,
    unit: TemperatureUnit,
) -> Result<(), Infallible> {
    let today = state.weather.forecast.first();
    let rain = today.and_then(|today| today.precipitation_probability_percent);
    let feels = short_degrees_label(current.apparent_temperature_tenths_f, unit);
    let humidity = format!("{}%", current.humidity_percent);
    let chips = [
        ("Feels", feels),
        ("Humidity", humidity),
        ("Wind", current.wind_short_label(unit)),
        ("Rain", percent_label(rain)),
    ];
    let detail = state.display.detail_style();
    let heading = state.display.heading_style();
    let width = (RIGHT - LEFT - 3 * CHIP_GAP) / 4;
    let size = Size::new(width as u32, CHIP_HEIGHT as u32);
    let label_y = CHIPS_TOP + 8 + detail.cap_height();
    let value_y = CHIPS_TOP + CHIP_HEIGHT - 12;
    for (index, (label, value)) in chips.iter().enumerate() {
        let left = LEFT + index as i32 * (width + CHIP_GAP);
        outline(display, Point::new(left, CHIPS_TOP), size)?;
        centered(display, detail, label, left, width, label_y)?;
        centered(display, heading, value, left, width, value_y)?;
    }
    Ok(())
}

/// Every second hour from the next one, in six columns.
fn draw_hours(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    unit: TemperatureUnit,
) -> Result<(), Infallible> {
    let detail = state.display.detail_style();
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let label = Point::new(LEFT + 2, HOURS_TOP - 10);
    Text::new("NEXT HOURS", label, detail).draw(display)?;
    let size = Size::new((RIGHT - LEFT) as u32, HOURS_HEIGHT as u32);
    outline(display, Point::new(LEFT, HOURS_TOP), size)?;
    let bottom = HOURS_TOP + HOURS_HEIGHT;
    let width = (RIGHT - LEFT) / 6;
    let time_y = HOURS_TOP + 8 + body.cap_height();
    let degrees_y = bottom - 12;
    let hours = state.weather.hourly.iter().skip(1).step_by(2).take(6);
    for (index, hour) in hours.enumerate() {
        let left = LEFT + index as i32 * width;
        if index > 0 {
            Line::new(Point::new(left, HOURS_TOP), Point::new(left, bottom))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
        }
        centered(display, body, hour.hour_label(), left, width, time_y)?;
        let icon = weather_icon_at(hour.weather_code, hour.is_day);
        let icon_top_left = Point::new(left + (width - 2 * ICON_SIZE) / 2, HOURS_TOP + 26);
        icon.draw_scaled(display, icon_top_left, 2, BinaryColor::On)?;
        let degrees = short_degrees_label(hour.temperature_tenths_f, unit);
        centered(display, heading, &degrees, left, width, degrees_y)?;
    }
    Ok(())
}

/// The next four days, one line each.
fn draw_days(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    unit: TemperatureUnit,
) -> Result<(), Infallible> {
    let label = Point::new(LEFT + 2, DAYS_TOP - 10);
    Text::new("NEXT DAYS", label, state.display.detail_style()).draw(display)?;
    let days = state.weather.forecast.iter().skip(1).take(4);
    for (index, day) in days.enumerate() {
        let row = ForecastRow {
            name: day.weekday_label(),
            weather_code: day.weather_code,
            is_day: true,
            temperatures: format!(
                "{} / {}",
                short_degrees_label(day.high_tenths_f, unit),
                short_degrees_label(day.low_tenths_f, unit)
            ),
            rain: day.precipitation_probability_percent,
        };
        let top = DAYS_TOP + index as i32 * DAY_HEIGHT;
        draw_forecast_row(display, state, top, DAY_HEIGHT, &row)?;
    }
    Ok(())
}

/// One forecast line: name, icon, condition, temperatures and the chance of
/// rain.
struct ForecastRow<'a> {
    name: &'a str,
    weather_code: u16,
    is_day: bool,
    temperatures: String,
    rain: Option<u8>,
}

fn draw_forecast_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    height: i32,
    row: &ForecastRow<'_>,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let middle = top + height / 2;
    let heading_y = middle + heading.cap_height() / 2;
    let body_y = middle + body.cap_height() / 2;
    Text::new(row.name, Point::new(LEFT + 4, heading_y), heading).draw(display)?;
    let icon = weather_icon_at(row.weather_code, row.is_day);
    let icon_top_left = Point::new(LEFT + 74, middle - ICON_SIZE / 2);
    icon.draw(display, icon_top_left, BinaryColor::On)?;
    let rain = percent_label(row.rain);
    let rain_left = RIGHT - 4 - body.text_width(&rain);
    Text::new(&rain, Point::new(rain_left, body_y), body).draw(display)?;
    let temperatures_left = RIGHT - 58 - heading.text_width(&row.temperatures);
    let temperatures = Point::new(temperatures_left, heading_y);
    Text::new(&row.temperatures, temperatures, heading).draw(display)?;
    let condition_left = LEFT + 112;
    let condition_width = temperatures_left - 12 - condition_left;
    let condition = body.fit(condition_label(row.weather_code), condition_width);
    Text::new(&condition, Point::new(condition_left, body_y), body).draw(display)?;
    let rule = Size::new((RIGHT - LEFT) as u32, 2);
    Rectangle::new(Point::new(LEFT, top + height - 2), rule)
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    Ok(())
}

/// `Updated 13:30 · next 15:30 · Open-Meteo`, or what is happening instead
/// of the next update.
fn updated_line(state: &AppState, config: &WeatherConfig) -> String {
    let weather = &state.weather;
    let observed = weather
        .current
        .as_ref()
        .and_then(|current| current.observed_at.get(11..16))
        .unwrap_or("--:--");
    let next = match weather.state {
        WeatherFetchState::Fetching => "updating now".to_string(),
        WeatherFetchState::Retrying => "retrying".to_string(),
        WeatherFetchState::Stale | WeatherFetchState::Failed => "update failed".to_string(),
        _ => next_update(observed, config.refresh_minutes),
    };
    let provider = provider_name(&weather.provider);
    format!("Updated {observed} \u{b7} {next} \u{b7} {provider}")
}

/// `next 15:30` two hours after `13:30`; `manual updates` without a schedule.
fn next_update(observed: &str, interval_minutes: u64) -> String {
    if interval_minutes == 0 {
        return "manual updates".into();
    }
    match clock_minutes(observed) {
        Some(clock) => {
            let next = clock + interval_minutes;
            format!("next {:02}:{:02}", next / 60 % 24, next % 60)
        }
        None => format!("every {}", refresh_label(interval_minutes)),
    }
}

/// Minutes after midnight of `HH:MM`.
fn clock_minutes(time: &str) -> Option<u64> {
    let (hours, minutes) = time.split_once(':')?;
    Some(hours.parse::<u64>().ok()? * 60 + minutes.parse::<u64>().ok()?)
}

fn provider_name(provider: &str) -> &str {
    if provider == "open-meteo" {
        "Open-Meteo"
    } else {
        provider
    }
}

/// `40%`, or `--` when the provider has no value.
fn percent_label(percent: Option<u8>) -> String {
    percent.map_or_else(|| "--".into(), |percent| format!("{percent}%"))
}

/// Draw `text` centered in the column `left..left + width`.
fn centered(
    display: &mut OrientedFrameBuffer<'_>,
    style: UiTextStyle,
    text: &str,
    left: i32,
    width: i32,
    baseline: i32,
) -> Result<(), Infallible> {
    let text = style.fit(text, width - 8);
    let x = left + (width - style.text_width(&text)) / 2;
    Text::new(&text, Point::new(x, baseline), style).draw(display)?;
    Ok(())
}

/// A 2 px frame, as around the chips and the hours.
fn outline(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    size: Size,
) -> Result<(), Infallible> {
    Rectangle::new(top_left, size)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)
}

#[cfg(test)]
mod tests {
    use super::{
        next_update, render_weather, render_weather_details, shown_forecast, updated_line,
    };
    use crate::{
        app::{
            display::{DisplayPreferences, UiFontFamily, UiFontSize},
            AppState,
        },
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
        weather::{parse_open_meteo_response, WeatherFetchState, SAMPLE_RESPONSE},
        weather_config::{WeatherConfig, SAMPLE_CONFIG},
    };

    fn set_up() -> AppState {
        let mut state = AppState::default();
        state.set_weather_config(Some(WeatherConfig::parse(SAMPLE_CONFIG).unwrap()));
        state
    }

    fn forecast() -> AppState {
        let mut state = set_up();
        let data = parse_open_meteo_response(SAMPLE_RESPONSE).unwrap();
        state.weather.record_success(data);
        state
    }

    #[test]
    fn every_state_renders_in_every_typography() {
        let mut off = forecast();
        off.weather_config.as_mut().unwrap().enabled = false;
        let mut failed = set_up();
        failed.weather.record_failure("Wi-Fi is not connected");
        let states = [AppState::default(), off, set_up(), failed, forecast()];
        for mut state in states {
            for family in UiFontFamily::ALL {
                for size in UiFontSize::ALL {
                    state.display = DisplayPreferences {
                        font_family: family,
                        font_size: size,
                    };
                    let mut frame = FrameBuffer::new_white();
                    let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
                    render_weather(&mut display, &state).unwrap();
                    render_weather_details(&mut display, &state).unwrap();
                }
            }
        }
    }

    #[test]
    fn missing_forecast_says_what_to_do() {
        let state = AppState::default();
        let (title, _) = shown_forecast(&state).unwrap_err();
        assert_eq!(title, "Weather is not set up");
        let mut manual = set_up();
        manual.weather_config.as_mut().unwrap().refresh_minutes = 0;
        let (title, text) = shown_forecast(&manual).unwrap_err();
        assert_eq!(title, "No forecast yet");
        assert!(text.starts_with("Updates are manual"));
    }

    #[test]
    fn updated_line_names_the_next_update() {
        let mut state = forecast();
        let config = state.weather_config.clone().unwrap();
        assert_eq!(
            updated_line(&state, &config),
            "Updated 13:30 \u{b7} next 15:30 \u{b7} Open-Meteo"
        );
        state.weather.state = WeatherFetchState::Stale;
        assert!(updated_line(&state, &config).contains("update failed"));
        assert_eq!(next_update("23:30", 60), "next 00:30");
        assert_eq!(next_update("13:30", 0), "manual updates");
        assert_eq!(next_update("--:--", 30), "every 30 minutes");
    }
}
