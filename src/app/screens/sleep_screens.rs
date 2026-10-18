//! Clock, weather and verse sleep screens: the layouts, and the live screen
//! of the sleep mode in use, built from state.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        reader_typography::reader_body_style,
        state::{AppState, VerseOfDay},
        typography::{Text, TextBounds, UiTextStyle},
        widgets::{
            display_numerals::{draw_numerals, numerals_width, Numerals},
            weather_icons::{draw_weather_icon, WeatherIcon},
        },
    },
    civil_date,
    orientation::OrientedFrameBuffer,
    reader::{BookFont, BookFontSize, ReadingTheme},
    rtc::RtcDateTime,
    sleep_screen::SleepMode,
    weather::{short_degrees_label, CurrentConditions},
};

use super::weather::percent_label;

/// A forecast older than this reads `stale` on the sleep screens.
const STALE_MINUTES: i64 = 6 * 60;
/// Baseline of the sleep clock's digits, which rise about 95 px above it.
const CLOCK_BASELINE: i32 = 312;

pub struct SleepClock<'a> {
    pub time: &'a str,
    pub date: &'a str,
    pub weather: Option<SleepWeatherLine<'a>>,
    pub battery_percent: Option<u8>,
    pub wake_hint: &'a str,
}

pub struct SleepWeatherLine<'a> {
    pub icon: WeatherIcon,
    pub summary: &'a str,
    pub details: &'a str,
}

pub struct SleepWeather<'a> {
    pub place: &'a str,
    pub updated: &'a str,
    pub icon: WeatherIcon,
    pub temperature: &'a str,
    pub condition: &'a str,
    pub details: &'a str,
    pub days: &'a [SleepWeatherDay<'a>],
    pub battery_percent: Option<u8>,
    pub wake_hint: &'a str,
}

pub struct SleepWeatherDay<'a> {
    pub name: &'a str,
    pub icon: WeatherIcon,
    pub range: &'a str,
    pub rain: &'a str,
}

/// The verse of the day sleep screen: the clock's date line and rule at the
/// top, then the verse in the reader's style at the largest size that fits
/// (from `XLarge` down, else cut with `…`) with its reference and
/// translation under it in Heading, centred as one block above the clock
/// layout's footer.
pub fn render_sleep_verse(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    reader_font: BookFont,
    reader_theme: ReadingTheme,
    verse: &VerseOfDay,
    local: Option<RtcDateTime>,
    battery_percent: Option<u8>,
    wake_hint: &str,
) -> Result<SleepLayout, Infallible> {
    draw_frame(display)?;
    let mut block_top = VERSE_BLOCK_TOP;
    if let Some(date) = local.map(long_date) {
        let date_height = text_height(&date, preferences.large_style(), 408, 2);
        text_block(
            display,
            &date,
            preferences.large_style(),
            TextBounds::new(36, 48, 444, 48 + date_height),
            2,
            true,
        )?;
        let rule_y = 48 + date_height + 28;
        Line::new(Point::new(70, rule_y), Point::new(410, rule_y))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
            .draw(display)?;
        block_top = rule_y + 32;
    }
    let reference = if verse.translation.is_empty() {
        verse.reference.clone()
    } else {
        format!("{} · {}", verse.reference, verse.translation)
    };
    let heading = preferences.heading_style();
    let reference_height = text_height(&reference, heading, 408, 1);
    let layout = verse_layout(
        &verse.text,
        reader_styles_largest_first(reader_font, reader_theme),
        block_top,
        reference_height,
    );
    let line_height = i32::from(layout.style.line_height());
    let bounds = TextBounds::new(36, block_top, 444, layout.reference_top);
    for (index, line) in layout.lines.iter().enumerate() {
        let left = 240 - layout.style.text_width(line) / 2;
        let baseline = layout.verse_top + layout.style.cap_height() + index as i32 * line_height;
        Text::new(line, Point::new(left, baseline), layout.style).draw_clipped(display, bounds)?;
    }
    text_block(
        display,
        &reference,
        heading,
        TextBounds::new(
            36,
            layout.reference_top,
            444,
            layout.reference_top + reference_height,
        ),
        1,
        true,
    )?;
    draw_footer(display, preferences, wake_hint, battery_percent)?;
    Ok(SleepLayout::Verse)
}

/// Where the verse block may sit: under the date's rule, above the footer.
const VERSE_BLOCK_TOP: i32 = 120;
const VERSE_BLOCK_BOTTOM: i32 = 716;
const VERSE_WIDTH: i32 = 408;
/// Between the verse's last line and its reference.
const REFERENCE_GAP: i32 = 28;

/// The verse's lines in the chosen size, and where they and the reference go.
struct VerseLayout {
    style: UiTextStyle,
    lines: Vec<String>,
    verse_top: i32,
    reference_top: i32,
}

/// The reader's style at each size, largest first.
fn reader_styles_largest_first(font: BookFont, theme: ReadingTheme) -> [UiTextStyle; 4] {
    let [small, medium, large, xlarge] = BookFontSize::ALL;
    [xlarge, large, medium, small].map(|size| reader_body_style(font, size, theme))
}

/// The largest style whose lines fit above the reference, else the smallest
/// cut with `…`; the verse and reference centred as one block from `top`.
fn verse_layout(
    text: &str,
    styles: [UiTextStyle; 4],
    top: i32,
    reference_height: i32,
) -> VerseLayout {
    let room = VERSE_BLOCK_BOTTOM - top - REFERENCE_GAP - reference_height;
    let fitted = styles.iter().find_map(|style| {
        let lines = style.wrap(text, VERSE_WIDTH);
        let height = lines.len() as i32 * i32::from(style.line_height());
        (height <= room).then_some((*style, lines))
    });
    let (style, lines) = fitted.unwrap_or_else(|| {
        let smallest = styles[3];
        (smallest, cut_verse_lines(text, smallest, VERSE_WIDTH, room))
    });
    let height = lines.len() as i32 * i32::from(style.line_height());
    let verse_top = top + (room - height) / 2;
    VerseLayout {
        style,
        lines,
        verse_top,
        reference_top: verse_top + height + REFERENCE_GAP,
    }
}

/// Wrap the verse to as many `width` lines as `band` allows, cutting the
/// overflow with a trailing `…` (the spec's cut for this screen).
fn cut_verse_lines(text: &str, style: UiTextStyle, width: i32, band: i32) -> Vec<String> {
    let line_height = i32::from(style.line_height());
    if line_height <= 0 {
        return Vec::new();
    }
    let limit = (band / line_height).max(1) as usize;
    let wrapped = style.wrap(text, width);
    if wrapped.len() <= limit {
        return wrapped;
    }
    let mut lines: Vec<String> = wrapped[..limit - 1].to_vec();
    let mut last = wrapped[limit - 1].clone();
    let budget = width - style.text_width("…");
    while !last.is_empty() && style.text_width(&last) > budget {
        last = match last.rsplit_once(' ') {
            Some((head, _)) => head.to_string(),
            None => String::new(),
        };
    }
    lines.push(format!("{last}…"));
    lines
}

pub fn render_sleep_clock(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    clock: &SleepClock<'_>,
) -> Result<(), Infallible> {
    draw_frame(display)?;
    draw_numerals(
        display,
        clock.time,
        Numerals::Clock,
        (36, 444),
        CLOCK_BASELINE,
        BinaryColor::On,
    )?;
    let date_height = text_height(clock.date, preferences.large_style(), 408, 2);
    text_block(
        display,
        clock.date,
        preferences.large_style(),
        TextBounds::new(36, 340, 444, 340 + date_height),
        2,
        true,
    )?;
    let rule_y = 340 + date_height + 40;
    Line::new(Point::new(70, rule_y), Point::new(410, rule_y))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    if let Some(weather) = &clock.weather {
        let (icon, summary) = centered_group(
            weather.summary,
            preferences.large_style(),
            52,
            12,
            rule_y + 28,
        );
        draw_weather_icon(
            display,
            weather.icon,
            Point::new(icon.left, icon.top),
            52,
            BinaryColor::On,
            BinaryColor::Off,
        )?;
        text_block(
            display,
            weather.summary,
            preferences.large_style(),
            summary,
            2,
            true,
        )?;
        let details_top = icon.bottom.max(summary.bottom) + 6;
        text_block(
            display,
            weather.details,
            preferences.body_style(),
            TextBounds::new(36, details_top, 444, details_top + 64),
            2,
            true,
        )?;
    }
    draw_footer(display, preferences, clock.wake_hint, clock.battery_percent)
}

pub fn render_sleep_weather(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    weather: &SleepWeather<'_>,
) -> Result<(), Infallible> {
    draw_frame(display)?;
    let layout = weather_layout(preferences, weather);
    text_block(
        display,
        weather.place,
        preferences.detail_style(),
        TextBounds::new(46, 48, 434, 70),
        1,
        false,
    )?;
    text_block(
        display,
        weather.updated,
        preferences.body_style(),
        TextBounds::new(46, 70, 434, layout.hero_top - 22),
        2,
        false,
    )?;
    draw_weather_icon(
        display,
        weather.icon,
        Point::new(layout.icon.left, layout.icon.top),
        130,
        BinaryColor::On,
        BinaryColor::Off,
    )?;
    draw_numerals(
        display,
        weather.temperature,
        Numerals::Temperature,
        (layout.condition.left, layout.condition.right),
        layout.temperature_top + Numerals::Temperature.digit_height(),
        BinaryColor::On,
    )?;
    text_block(
        display,
        weather.condition,
        preferences.heading_style(),
        layout.condition,
        2,
        true,
    )?;
    text_block(
        display,
        weather.details,
        preferences.body_style(),
        layout.details,
        3,
        true,
    )?;
    Line::new(
        Point::new(42, layout.rule_y),
        Point::new(438, layout.rule_y),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
    .draw(display)?;
    for (index, day) in weather.days.iter().take(3).enumerate() {
        let left = 36 + index as i32 * 136;
        text_block(
            display,
            day.name,
            preferences.heading_style(),
            TextBounds::new(left + 4, layout.rule_y + 20, left + 132, layout.icon_y - 8),
            1,
            true,
        )?;
        draw_weather_icon(
            display,
            day.icon,
            Point::new(left + 42, layout.icon_y),
            52,
            BinaryColor::On,
            BinaryColor::Off,
        )?;
        text_block(
            display,
            day.range,
            preferences.body_style(),
            TextBounds::new(left + 4, layout.range_y, left + 132, layout.rain_y - 6),
            1,
            true,
        )?;
        text_block(
            display,
            day.rain,
            preferences.detail_style(),
            TextBounds::new(left + 4, layout.rain_y, left + 132, layout.rain_y + 34),
            1,
            true,
        )?;
    }
    draw_footer(
        display,
        preferences,
        weather.wake_hint,
        weather.battery_percent,
    )
}

/// Which layout a live sleep screen drew.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepLayout {
    Clock,
    Weather,
    Verse,
}

impl SleepLayout {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Weather => "weather",
            Self::Verse => "verse",
        }
    }
}

/// The live sleep screen of the mode in use: the clock, with the weather
/// line when the mode has one, or the weather. Without a forecast the
/// weather mode shows the clock until the first update.
pub fn render_sleep_mode(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<SleepLayout, Infallible> {
    let mode = state.sleep_screen.mode;
    let local = state.board.rtc.map(|rtc| state.regional.localize_rtc(rtc));
    let battery_percent = state.board.power.and_then(|power| power.battery_percent);
    let wake_hint = state.power.wake_keys.wake_hint();
    // The verse screen: the date at the top, the verse centred in the
    // reader's style, its reference and translation under it. Without a
    // verse (no card, no list, or the day not loaded) the clock draws.
    if mode == SleepMode::Verse {
        if let Some(verse) = state
            .verse_of_day
            .as_ref()
            .filter(|verse| !verse.text.is_empty())
        {
            render_sleep_verse(
                display,
                state.display,
                state.reader.preferences.book_font,
                state.reader.preferences.theme,
                verse,
                local,
                battery_percent,
                wake_hint,
            )?;
            return Ok(SleepLayout::Verse);
        }
    }
    let forecast = state
        .weather
        .current
        .as_ref()
        .filter(|_| state.weather_enabled() && mode.shows_weather());
    if let Some(current) = forecast.filter(|_| mode == SleepMode::Weather) {
        render_live_weather(display, state, current, local)?;
        return Ok(SleepLayout::Weather);
    }
    let time = local.map_or_else(|| "--:--".into(), RtcDateTime::time_hm);
    let date = local.map_or_else(|| "Clock not set".into(), long_date);
    let line = forecast.map(|current| clock_weather_line(state, current, local));
    let weather = line
        .as_ref()
        .map(|(icon, summary, details)| SleepWeatherLine {
            icon: *icon,
            summary,
            details,
        });
    let clock = SleepClock {
        time: &time,
        date: &date,
        weather,
        battery_percent,
        wake_hint,
    };
    render_sleep_clock(display, state.display, &clock)?;
    Ok(SleepLayout::Clock)
}

fn render_live_weather(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    current: &CurrentConditions,
    local: Option<RtcDateTime>,
) -> Result<(), Infallible> {
    let unit = state.regional.temperature_unit;
    let place = state
        .weather_config
        .as_ref()
        .map_or("", |config| config.location.as_str());
    let observed = &current.observed_at;
    let date = observed.get(..10).and_then(short_date).unwrap_or_default();
    let time = observed.get(11..16).unwrap_or("--:--");
    let mut updated = format!("{date} · updated {time}");
    if is_stale(observed, local) {
        updated.push_str(" · stale");
    }
    let today = state.weather.forecast.first();
    let range = today.map_or_else(String::new, |today| {
        format!(
            "H {} · L {} · ",
            short_degrees_label(today.high_tenths_f, unit),
            short_degrees_label(today.low_tenths_f, unit)
        )
    });
    let rain = today.and_then(|day| day.precipitation_probability_percent);
    let rain = percent_label(rain);
    let wind = current.wind_short_label(unit);
    let details = format!("{range}Wind {wind} · Rain {rain}");
    let labels: Vec<(String, String)> = state
        .weather
        .forecast
        .iter()
        .skip(1)
        .take(3)
        .map(|day| {
            let high = short_degrees_label(day.high_tenths_f, unit);
            let low = short_degrees_label(day.low_tenths_f, unit);
            let rain = percent_label(day.precipitation_probability_percent);
            (format!("{high} / {low}"), rain)
        })
        .collect();
    let days: Vec<SleepWeatherDay<'_>> = state
        .weather
        .forecast
        .iter()
        .skip(1)
        .zip(&labels)
        .map(|(day, (range, rain))| SleepWeatherDay {
            name: day.weekday_label(),
            icon: WeatherIcon::for_code(day.weather_code, true),
            range,
            rain,
        })
        .collect();
    let temperature = short_degrees_label(current.temperature_tenths_f, unit);
    let weather = SleepWeather {
        place,
        updated: &updated,
        icon: WeatherIcon::for_code(current.weather_code, current.is_day),
        temperature: &temperature,
        condition: current.condition_label(),
        details: &details,
        days: &days,
        battery_percent: state.board.power.and_then(|power| power.battery_percent),
        wake_hint: state.power.wake_keys.wake_hint(),
    };
    render_sleep_weather(display, state.display, &weather)
}

/// The clock's weather line: `18° · Partly cloudy` over today's range and
/// rain, or when the forecast was last updated once it is stale.
fn clock_weather_line(
    state: &AppState,
    current: &CurrentConditions,
    local: Option<RtcDateTime>,
) -> (WeatherIcon, String, String) {
    let unit = state.regional.temperature_unit;
    let temperature = short_degrees_label(current.temperature_tenths_f, unit);
    let summary = format!("{temperature} · {}", current.condition_label());
    let details = match state.weather.forecast.first() {
        _ if is_stale(&current.observed_at, local) => {
            let time = current.observed_at.get(11..16).unwrap_or("--:--");
            format!("Updated {time} · stale")
        }
        Some(today) => format!(
            "H {} · L {} · Rain {}",
            short_degrees_label(today.high_tenths_f, unit),
            short_degrees_label(today.low_tenths_f, unit),
            percent_label(today.precipitation_probability_percent)
        ),
        None => String::new(),
    };
    let icon = WeatherIcon::for_code(current.weather_code, current.is_day);
    (icon, summary, details)
}

/// `Friday, October 2`.
#[must_use]
pub fn long_date(local: RtcDateTime) -> String {
    let weekday = civil_date::WEEKDAY_LONG.get(usize::from(local.weekday));
    let month = usize::from(local.month.saturating_sub(1));
    let month = civil_date::MONTH_LONG.get(month);
    match (weekday, month) {
        (Some(weekday), Some(month)) => format!("{weekday}, {month} {}", local.day),
        _ => "Date unavailable".into(),
    }
}

/// `Fri, Oct 2` for `2026-10-02`.
fn short_date(date: &str) -> Option<String> {
    let (year, month, day) = parse_date(date)?;
    let days = civil_date::days_from_civil(year, month, day);
    let weekday = civil_date::WEEKDAY_SHORT[usize::from(civil_date::weekday(days))];
    let month = civil_date::MONTH_SHORT.get(usize::from(month.checked_sub(1)?))?;
    Some(format!("{weekday}, {month} {day}"))
}

fn parse_date(date: &str) -> Option<(i64, u8, u8)> {
    let mut parts = date.splitn(3, '-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

/// A forecast observed at `observed` (`2026-10-02T13:30`, provider time) is
/// stale six hours after it, by the local clock; unknown without a clock.
fn is_stale(observed: &str, local: Option<RtcDateTime>) -> bool {
    let Some(local) = local else {
        return false;
    };
    let minutes = |year, month, day, hour: u8, minute: u8| {
        civil_date::days_from_civil(year, month, day) * 1440
            + i64::from(hour) * 60
            + i64::from(minute)
    };
    let Some((year, month, day)) = observed.get(..10).and_then(parse_date) else {
        return false;
    };
    let hour = observed.get(11..13).and_then(|text| text.parse().ok());
    let minute = observed.get(14..16).and_then(|text| text.parse().ok());
    let (Some(hour), Some(minute)) = (hour, minute) else {
        return false;
    };
    let then = minutes(year, month, day, hour, minute);
    let now = minutes(
        i64::from(local.year),
        local.month,
        local.day,
        local.hour,
        local.minute,
    );
    now - then > STALE_MINUTES
}

fn text_height(text: &str, style: UiTextStyle, width: i32, limit: usize) -> i32 {
    fitted_lines(text, style, width, limit).len() as i32 * i32::from(style.line_height())
}

fn text_width(text: &str, style: UiTextStyle, width: i32) -> i32 {
    fitted_lines(text, style, width, 2)
        .iter()
        .map(|line| style.text_width(line))
        .max()
        .unwrap_or(0)
}

fn centered_group(
    text: &str,
    style: UiTextStyle,
    icon_size: i32,
    gap: i32,
    top: i32,
) -> (TextBounds, TextBounds) {
    let width = text_width(text, style, 408 - icon_size - gap);
    let height = text_height(text, style, width, 2);
    let group_height = icon_size.max(height);
    let left = 240 - (icon_size + gap + width) / 2;
    let icon_top = top + (group_height - icon_size) / 2;
    let text_top = top + (group_height - height) / 2;
    (
        TextBounds::new(left, icon_top, left + icon_size, icon_top + icon_size),
        TextBounds::new(
            left + icon_size + gap,
            text_top,
            left + icon_size + gap + width,
            text_top + height,
        ),
    )
}

struct WeatherLayout {
    hero_top: i32,
    icon: TextBounds,
    temperature_top: i32,
    condition: TextBounds,
    details: TextBounds,
    rule_y: i32,
    icon_y: i32,
    range_y: i32,
    rain_y: i32,
}

fn weather_layout(preferences: DisplayPreferences, weather: &SleepWeather<'_>) -> WeatherLayout {
    let hero_top = 70 + text_height(weather.updated, preferences.body_style(), 388, 2) + 22;
    let temperature_height = Numerals::Temperature.digit_height();
    let numerals = numerals_width(weather.temperature, Numerals::Temperature).min(240);
    let width = numerals.max(text_width(
        weather.condition,
        preferences.heading_style(),
        240,
    ));
    let condition_height = text_height(weather.condition, preferences.heading_style(), width, 2);
    let column_height = temperature_height + 8 + condition_height;
    let hero_height = 130.max(column_height);
    let left = 240 - (130 + 8 + width) / 2;
    let icon_top = hero_top + (hero_height - 130) / 2;
    let temperature_top = hero_top + (hero_height - column_height) / 2;
    let condition_top = temperature_top + temperature_height + 8;
    let details_top = hero_top + hero_height + 12;
    let details_bottom =
        details_top + text_height(weather.details, preferences.body_style(), 408, 3);
    let rule_y = details_bottom + 34;
    let icon_y = rule_y + 20 + i32::from(preferences.heading_style().line_height()) + 8;
    let range_y = icon_y + 52 + 12;
    let rain_y = range_y + i32::from(preferences.body_style().line_height()) + 6;
    WeatherLayout {
        hero_top,
        icon: TextBounds::new(left, icon_top, left + 130, icon_top + 130),
        temperature_top,
        condition: TextBounds::new(
            left + 138,
            condition_top,
            left + 138 + width,
            condition_top + condition_height,
        ),
        details: TextBounds::new(36, details_top, 444, details_bottom),
        rule_y,
        icon_y,
        range_y,
        rain_y,
    }
}

fn draw_frame(display: &mut OrientedFrameBuffer<'_>) -> Result<(), Infallible> {
    Rectangle::new(Point::new(16, 16), Size::new(448, 768))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    Ok(())
}

fn fitted_lines(text: &str, style: UiTextStyle, width: i32, limit: usize) -> Vec<String> {
    let wrapped = style.wrap(text, width);
    let mut lines = Vec::new();
    for (index, line) in wrapped.iter().take(limit).enumerate() {
        let line = if index + 1 == limit && wrapped.len() > limit {
            format!("{line}...")
        } else {
            line.clone()
        };
        lines.push(style.fit(&line, width));
    }
    lines
}

fn text_block(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    style: UiTextStyle,
    bounds: TextBounds,
    limit: usize,
    centered: bool,
) -> Result<(), Infallible> {
    let line_height = i32::from(style.line_height());
    let limit = limit.min(((bounds.bottom - bounds.top) / line_height) as usize);
    for (index, line) in fitted_lines(text, style, bounds.width(), limit)
        .iter()
        .enumerate()
    {
        let left = if centered {
            bounds.left + (bounds.width() - style.text_width(line)) / 2
        } else {
            bounds.left
        };
        let baseline = bounds.top + style.cap_height() + index as i32 * line_height;
        Text::new(line, Point::new(left, baseline), style).draw_clipped(display, bounds)?;
    }
    Ok(())
}

fn footer_widths(style: UiTextStyle, percent: Option<u8>) -> (String, i32) {
    let battery = percent.map_or_else(String::new, |percent| format!("{percent}%"));
    let width = style.text_width(&battery);
    let hint_width = 408 - if battery.is_empty() { 0 } else { width + 16 };
    (battery, hint_width)
}

fn draw_footer(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    hint: &str,
    percent: Option<u8>,
) -> Result<(), Infallible> {
    let style = preferences.body_style();
    let (battery, hint_width) = footer_widths(style, percent);
    let hint = style.fit(hint, hint_width);
    let bounds = TextBounds::new(36, 746, 444, 774);
    Text::new(&hint, Point::new(36, 768), style).draw_clipped(display, bounds)?;
    Text::new(
        &battery,
        Point::new(444 - style.text_width(&battery), 768),
        style,
    )
    .draw_clipped(display, bounds)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{
            display::{UiFontFamily, UiFontSize},
            render_sleep_clock, render_sleep_mode, render_sleep_weather,
        },
        framebuffer::FrameBuffer,
    };

    fn clock(with_weather: bool) -> SleepClock<'static> {
        SleepClock {
            time: "13:42",
            date: "Friday, October 2",
            weather: with_weather.then_some(SleepWeatherLine {
                icon: WeatherIcon::PartlyCloudy,
                summary: "18° · Partly cloudy",
                details: "H 21° · L 11° · Rain 10%",
            }),
            battery_percent: Some(100),
            wake_hint: "Press any key to wake",
        }
    }

    const DAYS: [SleepWeatherDay<'static>; 4] = [
        SleepWeatherDay {
            name: "Sat",
            icon: WeatherIcon::Sun,
            range: "23° / 12°",
            rain: "0%",
        },
        SleepWeatherDay {
            name: "Sun",
            icon: WeatherIcon::Rain,
            range: "17° / 10°",
            rain: "80%",
        },
        SleepWeatherDay {
            name: "Mon",
            icon: WeatherIcon::Thunder,
            range: "15° / 9°",
            rain: "60%",
        },
        SleepWeatherDay {
            name: "IGNORED",
            icon: WeatherIcon::Snow,
            range: "-40° / -50°",
            rain: "100%",
        },
    ];

    fn weather() -> SleepWeather<'static> {
        SleepWeather {
            place: "Madrid",
            updated: "Fri, Oct 2 · updated 13:30",
            icon: WeatherIcon::PartlyCloudy,
            temperature: "18°",
            condition: "Partly cloudy",
            details: "H 21° · L 11° · Wind 12 km/h · Rain 10%",
            days: &DAYS[..3],
            battery_percent: Some(100),
            wake_hint: "Press any key to wake",
        }
    }

    fn black(frame: &FrameBuffer, x: i32, y: i32) -> bool {
        frame.is_black(Point::new(y, 479 - x)).unwrap()
    }

    /// Any ink inside `bounds`.
    fn inked(frame: &FrameBuffer, bounds: TextBounds) -> bool {
        (bounds.left..bounds.right).any(|x| (bounds.top..bounds.bottom).any(|y| black(frame, x, y)))
    }

    /// Ink along the clock's digits, and none just above them.
    fn check_clock_digits(frame: &FrameBuffer) {
        let top = CLOCK_BASELINE - Numerals::Clock.digit_height();
        assert!(
            inked(frame, TextBounds::new(36, top, 444, CLOCK_BASELINE)),
            "digits"
        );
        assert!(
            !inked(frame, TextBounds::new(36, top - 20, 444, top - 2)),
            "above the digits"
        );
    }

    fn check_frame(frame: &FrameBuffer) {
        for x in [15, 16, 17, 462, 463, 464] {
            assert!(black(frame, x, 400), "frame column {x}");
        }
        for y in [15, 16, 17, 782, 783, 784] {
            assert!(black(frame, 240, y), "frame row {y}");
        }
        assert!(!black(frame, 14, 400));
        assert!(!black(frame, 465, 400));
        assert!(!black(frame, 240, 14));
        assert!(!black(frame, 240, 785));
        assert!(!black(frame, 0, 0));
    }

    /// A saved verse mode draws the clock without a verse for the day (no
    /// card or no list), and the verse screen with one.
    #[test]
    fn the_verse_mode_falls_back_to_the_clock_without_a_verse() {
        use crate::app::state::{AppState, VerseOfDay};
        let mut state = AppState::default();
        state.sleep_screen.mode = SleepMode::Verse;
        let mut frame = FrameBuffer::new_white();
        assert_eq!(
            render_sleep_mode(&mut frame, &state).unwrap(),
            SleepLayout::Clock
        );
        // The day loaded and found no verse: the clock again.
        state.verse_of_day = Some(VerseOfDay {
            day: 1,
            text: String::new(),
            reference: "Sal 23:1".into(),
            translation: "RVR1960".into(),
        });
        assert_eq!(
            render_sleep_mode(&mut frame, &state).unwrap(),
            SleepLayout::Clock
        );
        state.verse_of_day.as_mut().unwrap().text = "El Señor es mi pastor; nada me falta.".into();
        assert_eq!(
            render_sleep_mode(&mut frame, &state).unwrap(),
            SleepLayout::Verse
        );
    }

    /// The long verse takes a smaller size or a cut and stays above its
    /// reference: the verse, an empty gap, the reference, then nothing down
    /// to the footer, the block centred in its room.
    #[test]
    fn the_long_verse_stays_above_its_reference() {
        use crate::app::state::{AppState, VerseOfDay};
        let mut state = AppState::default();
        state.sleep_screen.mode = SleepMode::Verse;
        let text = "Bienaventurados los que andan en la ley de Jehová. ".repeat(20);
        state.verse_of_day = Some(VerseOfDay {
            day: 1,
            text: text.clone(),
            reference: "Sal 119:1-6".into(),
            translation: "RVR1960".into(),
        });
        let mut frame = FrameBuffer::new_white();
        assert_eq!(
            render_sleep_mode(&mut frame, &state).unwrap(),
            SleepLayout::Verse
        );
        // No clock here, so no date: the block starts at the top of its room.
        let heading = state.display.heading_style();
        let reference_height = text_height("Sal 119:1-6 · RVR1960", heading, 408, 1);
        let reader = &state.reader.preferences;
        let styles = reader_styles_largest_first(reader.book_font, reader.theme);
        let layout = verse_layout(&text, styles, VERSE_BLOCK_TOP, reference_height);
        let verse_bottom = layout.reference_top - REFERENCE_GAP;
        let reference_bottom = layout.reference_top + reference_height;
        let verse = TextBounds::new(36, layout.verse_top, 444, verse_bottom);
        assert!(inked(&frame, verse), "verse");
        let gap = TextBounds::new(36, verse_bottom + 4, 444, layout.reference_top - 2);
        assert!(!inked(&frame, gap), "gap");
        let reference = TextBounds::new(36, layout.reference_top, 444, reference_bottom);
        assert!(inked(&frame, reference), "reference");
        let below = TextBounds::new(36, reference_bottom + 2, 444, 744);
        assert!(!inked(&frame, below), "nothing down to the footer");
        let above = layout.verse_top - VERSE_BLOCK_TOP;
        assert!((above - (VERSE_BLOCK_BOTTOM - reference_bottom)).abs() <= 1);
    }

    #[test]
    fn clock_pixels_cover_frame_digit_and_empty_weather_area() {
        let mut frame = FrameBuffer::from_native_bytes(vec![0; 48_000]).unwrap();
        render_sleep_clock(&mut frame, DisplayPreferences::default(), &clock(false)).unwrap();
        check_frame(&frame);
        check_clock_digits(&frame);
        let rule_y = 380
            + text_height(
                clock(false).date,
                DisplayPreferences::default().large_style(),
                408,
                2,
            );
        assert!(black(&frame, 240, rule_y));
        assert!(!black(&frame, 240, rule_y + 28));
    }

    #[test]
    fn clock_weather_pixels_cover_digit_and_scaled_icon() {
        let mut frame = FrameBuffer::new_white();
        render_sleep_clock(&mut frame, DisplayPreferences::default(), &clock(true)).unwrap();
        check_frame(&frame);
        check_clock_digits(&frame);
        let preferences = DisplayPreferences::default();
        let rule_y = 380 + text_height(clock(true).date, preferences.large_style(), 408, 2);
        let (icon, _) = centered_group(
            "18° · Partly cloudy",
            preferences.large_style(),
            52,
            12,
            rule_y + 28,
        );
        assert!(inked(&frame, icon), "the weather icon");
        assert!(!black(&frame, icon.left, icon.top));
    }

    #[test]
    fn weather_pixels_cover_frame_temperature_and_three_forecast_icons() {
        let mut frame = FrameBuffer::from_native_bytes(vec![0; 48_000]).unwrap();
        render_sleep_weather(&mut frame, DisplayPreferences::default(), &weather()).unwrap();
        check_frame(&frame);
        let layout = weather_layout(DisplayPreferences::default(), &weather());
        let top = layout.temperature_top;
        let digits = TextBounds::new(layout.condition.left, top, layout.condition.right, top + 70);
        assert!(inked(&frame, digits), "the temperature");
        assert!(inked(&frame, layout.icon), "the current icon");
        for index in 0..3 {
            let left = 36 + index * 136 + 42;
            let icon = TextBounds::new(left, layout.icon_y, left + 52, layout.icon_y + 52);
            assert!(inked(&frame, icon), "day {index} icon");
        }
        for y in layout.rule_y - 1..=layout.rule_y + 1 {
            assert!(black(&frame, 42, y));
            assert!(black(&frame, 438, y));
        }
    }

    #[test]
    fn sample_text_fits_every_size_and_family_without_truncation() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                for (text, style, width, lines, height) in [
                    ("Friday, October 2", preferences.large_style(), 408, 2, 88),
                    (
                        "18° · Partly cloudy",
                        preferences.large_style(),
                        344,
                        2,
                        100,
                    ),
                    (
                        "H 21° · L 11° · Rain 10%",
                        preferences.body_style(),
                        408,
                        2,
                        64,
                    ),
                    (
                        "Fri, Oct 2 · updated 13:30",
                        preferences.body_style(),
                        388,
                        2,
                        70,
                    ),
                    ("Partly cloudy", preferences.heading_style(), 240, 2, 58),
                    (
                        "H 21° · L 11° · Wind 12 km/h · Rain 10%",
                        preferences.body_style(),
                        408,
                        3,
                        69,
                    ),
                    ("23° / 12°", preferences.body_style(), 128, 1, 34),
                    ("Sat", preferences.heading_style(), 128, 1, 36),
                    ("100%", preferences.detail_style(), 128, 1, 34),
                ] {
                    let wrapped = style.wrap(text, width);
                    assert!(wrapped.len() <= lines);
                    assert!(wrapped.len() as i32 * i32::from(style.line_height()) <= height);
                    assert!(wrapped.iter().all(|line| style.text_width(line) <= width));
                    assert_eq!(fitted_lines(text, style, width, lines), wrapped);
                }
                let (battery, width) = footer_widths(preferences.body_style(), Some(100));
                assert!(preferences.body_style().text_width("Press any key to wake") <= width);
                assert_eq!(
                    width + preferences.body_style().text_width(&battery) + 16,
                    408
                );
                for with_weather in [false, true] {
                    let mut frame = FrameBuffer::new_white();
                    render_sleep_clock(&mut frame, preferences, &clock(with_weather)).unwrap();
                    check_frame(&frame);
                }
                let mut frame = FrameBuffer::new_white();
                render_sleep_weather(&mut frame, preferences, &weather()).unwrap();
                check_frame(&frame);
            }
        }
    }

    #[test]
    fn extreme_strings_stay_in_their_regions_for_every_typography() {
        let long = "A very long weather description with extreme conditions and an exceptionallylongunbrokenlocationname";
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let mut sample = weather();
                sample.place = long;
                sample.updated = long;
                sample.temperature = "-12345°";
                sample.condition = long;
                sample.details = long;
                sample.wake_hint = long;
                let days = [SleepWeatherDay {
                    name: long,
                    icon: WeatherIcon::Thunder,
                    range: "-12345° / -12345°",
                    rain: "Rain 100% with an exceptionallylongunbrokenlabel",
                }];
                sample.days = &days;
                let layout = weather_layout(preferences, &sample);
                let height = Numerals::Temperature.digit_height();
                let mut frame = FrameBuffer::new_white();
                render_sleep_weather(&mut frame, preferences, &sample).unwrap();
                let minus = TextBounds::new(
                    layout.condition.left,
                    layout.temperature_top,
                    layout.condition.left + 50,
                    layout.temperature_top + height,
                );
                assert!(
                    inked(&frame, minus),
                    "negative temperature must retain its minus sign"
                );
                for y in 19..782 {
                    for x in 19..462 {
                        if !black(&frame, x, y) {
                            continue;
                        }
                        assert!(
                            [
                                TextBounds::new(46, 48, 434, 70),
                                TextBounds::new(46, 70, 434, layout.hero_top - 22),
                                layout.icon,
                                TextBounds::new(
                                    layout.condition.left,
                                    layout.temperature_top - 2,
                                    layout.condition.right,
                                    layout.temperature_top + height + 2
                                ),
                                layout.condition,
                                layout.details,
                                TextBounds::new(42, layout.rule_y - 1, 439, layout.rule_y + 2),
                                TextBounds::new(36, layout.rule_y + 20, 444, layout.rain_y + 34),
                                TextBounds::new(36, 746, 444, 774),
                            ]
                            .iter()
                            .any(|b| x >= b.left && x < b.right && y >= b.top && y < b.bottom),
                            "escaped pixel {x},{y}"
                        );
                    }
                }
                let (battery, width) = footer_widths(preferences.body_style(), Some(100));
                assert!(
                    preferences
                        .body_style()
                        .text_width(&preferences.body_style().fit(long, width))
                        <= width
                );
                assert_eq!(battery, "100%");
            }
        }
    }

    #[test]
    fn groups_are_centered_and_rows_fit_every_typography() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                for summary in [
                    "18° · Partly cloudy",
                    "-123° · Freezing rain",
                    "",
                    "An exceptionallylongunbrokencondition followed by more words than fit",
                ] {
                    let rule_y =
                        380 + text_height("Friday, October 2", preferences.large_style(), 408, 2);
                    let (icon, text) =
                        centered_group(summary, preferences.large_style(), 52, 12, rule_y + 28);
                    assert!((icon.left + text.right - 480).abs() <= 1);
                    assert!((icon.top + icon.bottom - text.top - text.bottom).abs() <= 1);
                    assert_eq!(text.left - icon.right, 12);
                    assert!(icon.left >= 36 && text.right <= 444);
                    assert_eq!(icon.top.min(text.top), rule_y + 28);
                    assert!(icon.bottom.max(text.bottom) + 6 + 64 < 746);
                }
                for temperature in ["18°", "-40°", "-12345°", "", "-12345678901234567890°"] {
                    for condition in [
                        "Partly cloudy",
                        "Freezing rain and exceptionallylongunbrokenconditions",
                        "",
                    ] {
                        let mut sample = weather();
                        sample.temperature = temperature;
                        sample.condition = condition;
                        sample.updated =
                            "Updated at an unusually long time with a very long location label";
                        sample.details = "H -40° · L -50° · Wind 999 km/h · Rain 100% with unusually long extra details";
                        let layout = weather_layout(preferences, &sample);
                        let height = Numerals::Temperature.digit_height();
                        assert!((layout.icon.left + layout.condition.right - 480).abs() <= 1);
                        assert!(
                            (layout.icon.top + layout.icon.bottom
                                - layout.temperature_top
                                - layout.condition.bottom)
                                .abs()
                                <= 1
                        );
                        assert!(layout.icon.left >= 36 && layout.condition.right <= 444);
                        let numerals = numerals_width(temperature, Numerals::Temperature);
                        assert!(numerals.min(240) <= layout.condition.width());
                        assert_eq!(layout.condition.top, layout.temperature_top + height + 8);
                        assert!(layout.details.top >= layout.icon.bottom + 12);
                        assert_eq!(layout.rule_y - layout.details.bottom, 34);
                        assert!(layout.rain_y + 34 < 746);
                        assert!(i32::from(preferences.detail_style().line_height()) <= 34);
                    }
                }
            }
        }
    }

    #[test]
    fn clock_extremes_stay_below_time_and_above_footer() {
        let long = "An exceptionallylongunbrokenlocationname and a very long weather description with more words";
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let mut sample = clock(true);
                sample.date = long;
                sample.weather = Some(SleepWeatherLine {
                    icon: WeatherIcon::Thunder,
                    summary: long,
                    details: long,
                });
                let mut frame = FrameBuffer::new_white();
                render_sleep_clock(&mut frame, preferences, &sample).unwrap();
                let date_bottom = 340 + text_height(long, preferences.large_style(), 408, 2);
                let rule_y = date_bottom + 40;
                let (icon, summary) =
                    centered_group(long, preferences.large_style(), 52, 12, rule_y + 28);
                let details_top = icon.bottom.max(summary.bottom) + 6;
                for y in 331..746 {
                    for x in 19..462 {
                        if black(&frame, x, y) {
                            assert!(
                                [
                                    TextBounds::new(36, 340, 444, date_bottom),
                                    TextBounds::new(70, rule_y - 1, 411, rule_y + 2),
                                    icon,
                                    summary,
                                    TextBounds::new(36, details_top, 444, details_top + 64),
                                ]
                                .iter()
                                .any(|b| x >= b.left && x < b.right && y >= b.top && y < b.bottom),
                                "escaped clock pixel {x},{y}: icon {icon:?} summary {summary:?} \
                                 details from {details_top}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn forecast_ignores_extra_days_and_accepts_no_days_or_battery() {
        let mut three = FrameBuffer::new_white();
        let mut four = FrameBuffer::new_white();
        let mut sample = weather();
        render_sleep_weather(&mut three, DisplayPreferences::default(), &sample).unwrap();
        sample.days = &DAYS;
        render_sleep_weather(&mut four, DisplayPreferences::default(), &sample).unwrap();
        assert_eq!(three, four);
        sample.days = &[];
        sample.battery_percent = None;
        render_sleep_weather(&mut four, DisplayPreferences::default(), &sample).unwrap();
        assert!(!black(
            &four,
            102,
            weather_layout(DisplayPreferences::default(), &sample).icon_y + 4
        ));
        assert_eq!(
            footer_widths(DisplayPreferences::default().body_style(), None),
            (String::new(), 408)
        );
    }

    fn at(day: u8, hour: u8, minute: u8) -> RtcDateTime {
        RtcDateTime {
            year: 2026,
            month: 10,
            day,
            weekday: (day + 3) % 7,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn dates_read_in_words_and_forecasts_go_stale_after_six_hours() {
        assert_eq!(long_date(at(2, 13, 42)), "Friday, October 2");
        assert_eq!(short_date("2026-10-02").as_deref(), Some("Fri, Oct 2"));
        assert_eq!(short_date("2026-13-02"), None);
        assert_eq!(short_date("today"), None);
        let observed = "2026-10-02T13:30";
        assert!(!is_stale(observed, Some(at(2, 19, 30))));
        assert!(is_stale(observed, Some(at(2, 19, 31))));
        assert!(is_stale(observed, Some(at(3, 8, 0))));
        assert!(!is_stale(observed, None));
        assert!(!is_stale("not a time", Some(at(9, 0, 0))));
    }

    #[test]
    fn sleep_modes_pick_their_layout() {
        use crate::weather::{parse_open_meteo_response, SAMPLE_RESPONSE};
        use crate::weather_config::{WeatherConfig, SAMPLE_CONFIG};

        let mut state = AppState::default();
        state.board.rtc = Some(at(2, 13, 42));
        let mut frame = FrameBuffer::new_white();
        let mut layout =
            |state: &AppState| crate::app::render_sleep_mode(&mut frame, state).unwrap();
        state.sleep_screen.mode = SleepMode::Clock;
        assert_eq!(layout(&state), SleepLayout::Clock);
        // No forecast yet: the weather mode keeps the clock.
        state.sleep_screen.mode = SleepMode::Weather;
        assert_eq!(layout(&state), SleepLayout::Clock);
        state.set_weather_config(Some(WeatherConfig::parse(SAMPLE_CONFIG).unwrap()));
        state
            .weather
            .record_success(parse_open_meteo_response(SAMPLE_RESPONSE).unwrap());
        assert_eq!(layout(&state), SleepLayout::Weather);
        state.sleep_screen.mode = SleepMode::ClockWeather;
        assert_eq!(layout(&state), SleepLayout::Clock);
        let (_, summary, details) = clock_weather_line(
            &state,
            state.weather.current.as_ref().unwrap(),
            Some(at(3, 8, 0)),
        );
        assert!(summary.contains(" · "));
        assert_eq!(details, "Updated 13:30 · stale");
    }
}
