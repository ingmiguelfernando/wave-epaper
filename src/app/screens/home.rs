//! Wave Home: status bar, date and weather strip, reading card and app rows.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
};

use crate::{
    app::{
        menu::{home_entries, MenuEntry},
        router::ScreenRoute,
        state::AppState,
        typography::{Text, UiTextRole},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            icons::{self, Icon, ICON_SIZE},
            list_row::draw_selection_marker,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    civil_date,
    network::WifiConnectionState,
    orientation::OrientedFrameBuffer,
    reader::{BookFormat, ReaderLocation},
    regional::TemperatureUnit,
    rtc::RtcDateTime,
    weather::{short_degrees_label, whole_degrees, CurrentConditions, WeatherFetchState},
};

const MARGIN: i32 = 16;
const CONTENT_WIDTH: i32 = 480 - 2 * MARGIN;
const STRIP_TOP: i32 = STATUS_BAR_HEIGHT;
const STRIP_HEIGHT: i32 = 40;
const CARD_TOP: i32 = STRIP_TOP + STRIP_HEIGHT + 4;
const CARD_HEIGHT: i32 = 98;
const ROWS_TOP: i32 = CARD_TOP + CARD_HEIGHT + 10;
const ROW_HEIGHT: i32 = 54;
const ROW_PITCH: i32 = ROW_HEIGHT + 5;
const BATTERY_WIDTH: i32 = 26;
const BATTERY_HEIGHT: i32 = 14;

const HOME_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "open"),
    (KeyCap::Power, "display menu"),
];

pub fn render_home(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_home_status_bar(display, state)?;
    draw_date_weather_strip(display, state)?;
    draw_reading_card(display, state)?;
    for (index, entry) in home_entries().iter().enumerate() {
        let top = ROWS_TOP + index as i32 * ROW_PITCH;
        draw_home_row(display, state, top, entry, state.home_selected == index)?;
    }
    draw_bottom_bar(display, state.display, &HOME_HINTS)
}

fn draw_home_status_bar(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    draw_status_bar(display, preferences, "WAVE")?;
    let mut right = STATUS_BAR_RIGHT;
    if matches!(state.network.wifi_state, WifiConnectionState::Connected) {
        right -= icons::WIFI.width();
        let top = (STATUS_BAR_HEIGHT - icons::WIFI.height()) / 2;
        icons::WIFI.draw(display, Point::new(right, top), BinaryColor::Off)?;
        right -= 10;
    }
    if let Some(percent) = state.board.power.and_then(|power| power.battery_percent) {
        right = draw_status_text(display, preferences, &format!("{percent}%"), right)? - 4;
        right -= BATTERY_WIDTH;
        let top = (STATUS_BAR_HEIGHT - BATTERY_HEIGHT) / 2;
        draw_battery(display, Point::new(right, top), percent)?;
        right -= 12;
    }
    let time = state.board.time_label(state.regional);
    draw_status_text(display, preferences, &time, right)?;
    Ok(())
}

/// Battery outline with a fill proportional to `percent`, in paper color.
fn draw_battery(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    percent: u8,
) -> Result<(), Infallible> {
    let paper = PrimitiveStyle::with_fill(BinaryColor::Off);
    let outline = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::Off)
        .stroke_width(2)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let body = Rectangle::new(top_left, Size::new(23, BATTERY_HEIGHT as u32));
    body.into_styled(outline).draw(display)?;
    let tip = Rectangle::new(top_left + Point::new(23, 4), Size::new(3, 6));
    tip.into_styled(paper).draw(display)?;
    let level = 17 * u32::from(percent.min(100)) / 100;
    if level > 0 {
        let fill = Rectangle::new(top_left + Point::new(3, 3), Size::new(level, 8));
        fill.into_styled(paper).draw(display)?;
    }
    Ok(())
}

fn draw_date_weather_strip(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let style = state.display.body_style();
    let baseline = STRIP_TOP + (STRIP_HEIGHT + style.cap_height()) / 2;
    let date = home_date_label(state);
    Text::new(&date, Point::new(MARGIN, baseline), style).draw(display)?;

    let unit = state.regional.temperature_unit;
    let (label, icon) = match home_conditions(state) {
        Some(current) => {
            let temperature = temperature_label(current.temperature_tenths_f, unit);
            let label = format!("{temperature} · {}", current.condition_label());
            let icon = icons::weather_icon_at(current.weather_code, current.is_day);
            (label, Some(icon))
        }
        // Settings › Weather turned it off or keeps it off Home.
        None if state.weather_config.is_some() && !state.weather_on_home() => return Ok(()),
        None => (weather_status_label(state.weather.state).to_string(), None),
    };
    let left = MARGIN + CONTENT_WIDTH - style.text_width(&label);
    Text::new(&label, Point::new(left, baseline), style).draw(display)?;
    if let Some(icon) = icon {
        let top = STRIP_TOP + (STRIP_HEIGHT - ICON_SIZE) / 2;
        let origin = Point::new(left - ICON_SIZE - 6, top);
        icon.draw(display, origin, BinaryColor::On)?;
    }
    Ok(())
}

/// Book shown on the Home card: the open session first, else the saved resume.
struct ReadingCard {
    title: String,
    percent: Option<u8>,
    position: String,
}

fn reading_card(state: &AppState) -> Option<ReadingCard> {
    if let Some(session) = state.reader.session.as_ref() {
        let location = session.current_location();
        let percent = percent_of(location.byte_offset, session.source_size_bytes());
        return Some(ReadingCard {
            title: session.book.title.clone(),
            percent: Some(percent),
            position: position_label(&location),
        });
    }
    let resume = state.reader.resume.as_ref()?;
    // EPUB offsets index the flattened text, not the file, so only TXT gets a percentage.
    let percent = match resume.format {
        BookFormat::Text => Some(percent_of(resume.byte_offset, resume.size_bytes)),
        BookFormat::Epub => None,
    };
    Some(ReadingCard {
        title: resume.title.clone(),
        percent,
        position: position_label(resume),
    })
}

/// "Page 12", or "Ch 3 · Page 2/9" for EPUB books.
fn position_label(location: &ReaderLocation) -> String {
    let Some(chapter) = location.epub_chapter.as_ref() else {
        return format!("Page {}", location.page_index + 1);
    };
    let number = chapter.chapter_number;
    format!("Ch {number} · Page {}", chapter.page_text())
}

fn percent_of(offset: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    (offset.saturating_mul(100) / total).min(100) as u8
}

fn draw_reading_card(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let card = Rectangle::new(
        Point::new(MARGIN, CARD_TOP),
        Size::new(CONTENT_WIDTH as u32, CARD_HEIGHT as u32),
    );
    card.into_styled(border_style()).draw(display)?;

    let label_style = state.display.detail_style();
    let title_style = state.display.heading_style();
    let body = state.display.body_style();
    let left = MARGIN + 16;
    let width = CONTENT_WIDTH - 32;
    let label_baseline = CARD_TOP + 12 + label_style.cap_height();
    let title_baseline = label_baseline + 12 + title_style.cap_height();
    let progress_top = title_baseline + 14;
    let progress_baseline = progress_top + (12 + body.cap_height()) / 2;

    let label_origin = Point::new(left, label_baseline);
    Text::new("CONTINUE READING", label_origin, label_style).draw(display)?;
    let Some(book) = reading_card(state) else {
        let title_origin = Point::new(left, title_baseline);
        Text::new("Nothing in progress", title_origin, title_style).draw(display)?;
        let hint_origin = Point::new(left, progress_baseline);
        Text::new("Open Library to pick a book", hint_origin, body).draw(display)?;
        return Ok(());
    };

    let title = title_style.fit(&book.title, width);
    Text::new(&title, Point::new(left, title_baseline), title_style).draw(display)?;
    let minutes = minutes_read_today(state).map(|minutes| format!("· {minutes} min today"));
    match book.percent {
        Some(percent) => {
            // `12% · 25 min today` right of the progress bar.
            let mut label = format!("{percent}%");
            if let Some(minutes) = minutes.as_deref() {
                label = format!("{label} {minutes}");
            }
            let label_left = left + width - body.text_width(&label);
            let bar_width = (label_left - 10 - left).max(0) as u32;
            let bar = Rectangle::new(Point::new(left, progress_top), Size::new(bar_width, 12));
            draw_progress_bar(display, bar, percent)?;
            let origin = Point::new(label_left, progress_baseline);
            Text::new(&label, origin, body).draw(display)?;
        }
        None => {
            let origin = Point::new(left, progress_baseline);
            Text::new(&book.position, origin, body).draw(display)?;
            if let Some(minutes) = minutes.as_deref() {
                let label_left = left + width - body.text_width(minutes);
                let origin = Point::new(label_left, progress_baseline);
                Text::new(minutes, origin, body).draw(display)?;
            }
        }
    }
    Ok(())
}

/// Whole minutes read today, `None` without a clock or without reading.
#[must_use]
pub fn minutes_read_today(state: &AppState) -> Option<u32> {
    let today = state.local_day()?;
    let seconds = state.reading_stats.day(today).seconds;
    if seconds == 0 {
        None
    } else {
        Some(seconds / 60)
    }
}

fn draw_progress_bar(
    display: &mut OrientedFrameBuffer<'_>,
    area: Rectangle,
    percent: u8,
) -> Result<(), Infallible> {
    area.into_styled(border_style()).draw(display)?;
    let inner = area.size.width.saturating_sub(4);
    let filled = inner * u32::from(percent.min(100)) / 100;
    if filled > 0 {
        let height = area.size.height.saturating_sub(4);
        let fill = Rectangle::new(area.top_left + Point::new(2, 2), Size::new(filled, height));
        let black = PrimitiveStyle::with_fill(BinaryColor::On);
        fill.into_styled(black).draw(display)?;
    }
    Ok(())
}

fn draw_home_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    entry: &MenuEntry,
    selected: bool,
) -> Result<(), Infallible> {
    let area = Rectangle::new(
        Point::new(MARGIN, top),
        Size::new(CONTENT_WIDTH as u32, ROW_HEIGHT as u32),
    );
    let ink = if selected {
        let fill = PrimitiveStyle::with_fill(BinaryColor::On);
        area.into_styled(fill).draw(display)?;
        draw_selection_marker(display, MARGIN + 10, top + ROW_HEIGHT / 2)?;
        BinaryColor::Off
    } else {
        area.into_styled(border_style()).draw(display)?;
        BinaryColor::On
    };

    let icon_left = MARGIN + 36;
    let icon_origin = Point::new(icon_left, top + (ROW_HEIGHT - ICON_SIZE) / 2);
    home_icon(entry.route, state).draw(display, icon_origin, ink)?;

    let title_style = state.display.text_style(UiTextRole::Heading, ink);
    let title_left = icon_left + ICON_SIZE + 12;
    let baseline = top + (ROW_HEIGHT + title_style.cap_height()) / 2;
    Text::new(entry.label, Point::new(title_left, baseline), title_style).draw(display)?;

    let meta = home_meta(entry, state);
    if !meta.is_empty() {
        let meta_style = state.display.text_style(UiTextRole::Body, ink);
        let left = MARGIN + CONTENT_WIDTH - 14 - meta_style.text_width(&meta);
        let baseline = top + (ROW_HEIGHT + meta_style.cap_height()) / 2;
        Text::new(&meta, Point::new(left, baseline), meta_style).draw(display)?;
    }
    Ok(())
}

fn border_style() -> PrimitiveStyle<BinaryColor> {
    PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(2)
        .stroke_alignment(StrokeAlignment::Inside)
        .build()
}

fn home_icon(route: ScreenRoute, state: &AppState) -> &'static Icon {
    match route {
        ScreenRoute::Photos => &icons::PHOTO,
        ScreenRoute::Reader => &icons::BOOK,
        ScreenRoute::Bible => &icons::BIBLE,
        ScreenRoute::ReadingStats => &icons::STATS,
        ScreenRoute::Weather => current_weather_icon(state),
        ScreenRoute::Games => &icons::GAMES,
        ScreenRoute::Ai => &icons::AI,
        ScreenRoute::Tools => &icons::TOOLS,
        _ => &icons::SETTINGS,
    }
}

fn current_weather_icon(state: &AppState) -> &'static Icon {
    match home_conditions(state) {
        Some(current) => icons::weather_icon_at(current.weather_code, current.is_day),
        None => &icons::PARTLY_CLOUDY,
    }
}

/// Current conditions, unless Settings › Weather keeps weather off Home.
fn home_conditions(state: &AppState) -> Option<&CurrentConditions> {
    if state.weather_on_home() {
        state.weather.current.as_ref()
    } else {
        None
    }
}

/// Short status at the right end of a Home row.
fn home_meta(entry: &MenuEntry, state: &AppState) -> String {
    let books = state.reader.books.len();
    match entry.route {
        ScreenRoute::Reader if books > 0 => books.to_string(),
        ScreenRoute::ReadingStats => reading_stats_meta(state),
        ScreenRoute::Weather => match home_conditions(state) {
            Some(current) => {
                let unit = state.regional.temperature_unit;
                short_degrees_label(current.temperature_tenths_f, unit)
            }
            None if state.weather_config.is_some() && !state.weather_enabled() => "Off".into(),
            None => String::new(),
        },
        _ => entry.badge.to_string(),
    }
}

/// `5-day streak` when the RTC gives a day and more than today reads.
fn reading_stats_meta(state: &AppState) -> String {
    state.local_day().map_or_else(String::new, |today| {
        let streak = state.reading_stats.streak(today);
        if streak > 1 {
            format!("{streak}-day streak")
        } else {
            String::new()
        }
    })
}

fn weather_status_label(state: WeatherFetchState) -> &'static str {
    match state {
        WeatherFetchState::Disabled => "Weather off",
        WeatherFetchState::ConfigurationMissing => "Weather not set up",
        WeatherFetchState::Failed => "Weather unavailable",
        _ => "Weather pending",
    }
}

fn temperature_label(tenths_f: i16, unit: TemperatureUnit) -> String {
    format!("{}{}", whole_degrees(tenths_f, unit), unit.suffix())
}

fn home_date_label(state: &AppState) -> String {
    state.board.rtc.map_or_else(
        || "Date unavailable".into(),
        |rtc| compact_local_date(state.regional.localize_rtc(rtc)),
    )
}

fn compact_local_date(local: RtcDateTime) -> String {
    let weekday = civil_date::WEEKDAY_SHORT
        .get(usize::from(local.weekday))
        .copied()
        .unwrap_or("---");
    let month = local
        .month
        .checked_sub(1)
        .and_then(|index| civil_date::MONTH_SHORT.get(usize::from(index)))
        .copied()
        .unwrap_or("---");
    format!("{weekday}, {month} {}", local.day)
}

#[cfg(test)]
mod tests {
    use super::{
        compact_local_date, home_meta, minutes_read_today, percent_of, reading_stats_meta,
        temperature_label,
    };
    use crate::{
        app::{menu::home_entries, router::ScreenRoute, AppState},
        regional::TemperatureUnit,
        rtc::RtcDateTime,
        weather::{parse_open_meteo_response, SAMPLE_RESPONSE},
        weather_config::{WeatherConfig, SAMPLE_CONFIG},
    };

    #[test]
    fn renders_compact_dashboard_date() {
        assert_eq!(
            compact_local_date(RtcDateTime {
                year: 2026,
                month: 6,
                day: 4,
                weekday: 4,
                hour: 8,
                minute: 13,
                second: 0,
            }),
            "Thu, Jun 4"
        );
    }

    #[test]
    fn strip_temperature_rounds_to_whole_degrees() {
        assert_eq!(temperature_label(644, TemperatureUnit::Celsius), "18°C");
        assert_eq!(temperature_label(784, TemperatureUnit::Fahrenheit), "78°F");
        assert_eq!(temperature_label(140, TemperatureUnit::Celsius), "-10°C");
    }

    #[test]
    fn weather_row_follows_settings() {
        let mut state = AppState::default();
        state.set_weather_config(Some(WeatherConfig::parse(SAMPLE_CONFIG).unwrap()));
        let data = parse_open_meteo_response(SAMPLE_RESPONSE).unwrap();
        state.weather.record_success(data);
        let entry = home_entries()
            .iter()
            .find(|entry| entry.route == ScreenRoute::Weather)
            .unwrap();
        assert_eq!(home_meta(entry, &state), "18°");
        state.weather_config.as_mut().unwrap().show_on_home = false;
        assert_eq!(home_meta(entry, &state), "");
        state.weather_config.as_mut().unwrap().enabled = false;
        assert_eq!(home_meta(entry, &state), "Off");
    }

    #[test]
    fn reading_percent_is_bounded() {
        assert_eq!(percent_of(240_000, 2_000_000), 12);
        assert_eq!(percent_of(5, 0), 0);
        assert_eq!(percent_of(9, 4), 100);
    }

    fn day_at(year: u16, month: u8, day: u8) -> u32 {
        crate::civil_date::days_from_civil(i64::from(year), month, day) as u32
    }

    #[test]
    fn reading_stats_row_shows_the_streak_only_when_there_is_one() {
        let mut state = AppState::default();
        let today = day_at(2026, 10, 8);
        state.board.rtc = Some(RtcDateTime {
            year: 2026,
            month: 10,
            day: 8,
            ..RtcDateTime::default()
        });
        assert_eq!(reading_stats_meta(&state), "");
        // Four past days above five minutes plus today: a 5-day streak.
        for offset in 1..=4 {
            state.reading_stats.record(today - offset, 600, 0, None);
        }
        state.reading_stats.record(today, 600, 0, None);
        assert_eq!(reading_stats_meta(&state), "5-day streak");
        // Only today read: no streak label.
        let mut fresh = AppState::default();
        fresh.board.rtc = state.board.rtc;
        fresh.reading_stats.record(today, 600, 0, None);
        assert_eq!(reading_stats_meta(&fresh), "");
        let entry = home_entries()
            .iter()
            .find(|entry| entry.route == ScreenRoute::ReadingStats)
            .unwrap();
        assert_eq!(home_meta(entry, &state), "5-day streak");
    }

    #[test]
    fn continue_card_minutes_need_a_clock_and_reading_time() {
        let mut state = AppState::default();
        assert_eq!(minutes_read_today(&state), None);
        state.board.rtc = Some(RtcDateTime {
            year: 2026,
            month: 10,
            day: 8,
            ..RtcDateTime::default()
        });
        assert_eq!(minutes_read_today(&state), None);
        let today = day_at(2026, 10, 8);
        state.reading_stats.record(today, 25 * 60, 0, None);
        assert_eq!(minutes_read_today(&state), Some(25));
        // Fractions of a minute stay hidden.
        state.reading_stats.record(today, 59, 0, None);
        assert_eq!(minutes_read_today(&state), Some(25));
    }
}
