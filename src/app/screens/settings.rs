//! Settings groups: one page with a value per group, plus the two sub-lists
//! (Clock & alarms, System) that stay category routes.

use core::convert::Infallible;

use embedded_graphics::{pixelcolor::BinaryColor, prelude::Point};

use crate::app::typography::{Text, UiTextRole};
use crate::orientation::OrientedFrameBuffer;

/// Left inset of the group labels, matching the list rows' title inset.
const LIST_TEXT_LEFT: i32 = 34;

use super::category::item_count_text;
use crate::{
    alarm::AlarmSnapshot,
    app::{
        display::DisplayPreferences,
        menu::{category_entries, CATEGORY_PAGE_SIZE},
        router::ScreenRoute,
        state::AppState,
        widgets::{
            bottom_bar::{draw_bottom_bar, CHANGE_HINTS, CHOOSE_HINTS, OPEN_HINTS},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    build_info::FIRMWARE_VERSION,
    network::WifiConnectionState,
    sleep_screen::{SleepMode, SleepSource},
    weather_config::WeatherConfig,
};

/// Value of the Settings row that opens `route`, straight from state.
#[must_use]
pub fn group_value(state: &AppState, route: ScreenRoute) -> String {
    match route {
        ScreenRoute::Display => display_value(state.display),
        ScreenRoute::SleepScreen => sleep_screen_value(state),
        ScreenRoute::WeatherSettings => weather_value(state.weather_config.as_ref()),
        ScreenRoute::AiSettings => state.ai.as_ref().map_or_else(
            || "Not set up".into(),
            |config| config.settings_value().into(),
        ),
        ScreenRoute::Network => {
            network_value(&state.network.wifi_state, state.network.ssid.as_deref())
        }
        ScreenRoute::SettingsClockAlarms => clock_alarms_value(&state.alarms),
        ScreenRoute::Power => {
            power_value(state.board.power.and_then(|power| power.battery_percent))
        }
        ScreenRoute::SettingsSystem => version_text(),
        _ => String::new(),
    }
}

/// `v0.9.7`, on the status bar and the System row as in the mockup.
fn version_text() -> String {
    format!("v{FIRMWARE_VERSION}")
}

/// `Inter · Standard`, the mockup's font row.
#[must_use]
pub fn display_value(preferences: DisplayPreferences) -> String {
    format!(
        "{} · {}",
        preferences.font_family.label(),
        preferences.font_size.label()
    )
}

/// The sleep mode, with the starred photo count when it plays photos.
#[must_use]
pub fn sleep_screen_value(state: &AppState) -> String {
    match state.sleep_screen.mode {
        SleepMode::Photo => match state.sleep_screen.source {
            SleepSource::Starred => match state.photos.starred.len() {
                0 => "Photo · none starred".into(),
                starred => format!("Photo · {starred} starred"),
            },
            SleepSource::Folder => "Photo · folder".into(),
        },
        SleepMode::Clock => {
            format!("Clock · {} min", state.sleep_screen.clock_refresh.minutes())
        }
        SleepMode::Weather => "Weather".into(),
        SleepMode::ClockWeather => "Clock + weather".into(),
        SleepMode::Verse => "Verse".into(),
    }
}

/// `On · 2 h`, `Manual` when updates wait for a press, `Off`, or
/// `Not set up` without a weather file.
#[must_use]
pub fn weather_value(config: Option<&WeatherConfig>) -> String {
    match config {
        None => "Not set up".into(),
        Some(config) if !config.enabled => "Off".into(),
        Some(config) if config.refresh_minutes == 0 => "Manual".into(),
        Some(config) => format!("On · {}", hours_text(config.refresh_minutes)),
    }
}

/// The Wi-Fi state in the mockup's words; the SSID when connected.
#[must_use]
pub fn network_value(wifi_state: &WifiConnectionState, ssid: Option<&str>) -> String {
    match wifi_state {
        WifiConnectionState::Disabled => "Off".into(),
        WifiConnectionState::ConfigurationMissing => "Not set up".into(),
        WifiConnectionState::Connecting => "Connecting".into(),
        WifiConnectionState::Failed => "Failed".into(),
        WifiConnectionState::Connected => ssid.unwrap_or("Connected").to_owned(),
    }
}

/// `2 alarms` or `No alarms`, counting enabled alarms only.
#[must_use]
pub fn clock_alarms_value(alarms: &AlarmSnapshot) -> String {
    let count = alarms.alarms.iter().filter(|alarm| alarm.enabled).count();
    if count == 0 {
        "No alarms".into()
    } else {
        format!("{count} alarm{}", if count == 1 { "" } else { "s" })
    }
}

/// Battery percentage, or `--` while the gauge has no reading.
#[must_use]
pub fn power_value(battery_percent: Option<u8>) -> String {
    match battery_percent {
        Some(percent) => format!("{percent}%"),
        None => "--".into(),
    }
}

/// Minutes as `30 min`, `2 h`, `1.5 h`.
#[must_use]
pub(super) fn hours_text(minutes: u64) -> String {
    if minutes < 60 {
        format!("{minutes} min")
    } else if minutes % 60 == 0 {
        format!("{} h", minutes / 60)
    } else {
        format!("{:.1} h", minutes as f64 / 60.0)
    }
}

/// The mockup's `.set` rows: title and description left, value right.
pub fn render_settings(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let selected = state.category_selection(ScreenRoute::Settings);
    let version = version_text();
    draw_status_bar(display, state.display, ScreenRoute::Settings.label())?;
    draw_status_text(display, state.display, &version, STATUS_BAR_RIGHT)?;
    for (offset, entry) in category_entries(ScreenRoute::Settings).iter().enumerate() {
        let value = group_value(state, entry.route);
        let row = ListRow {
            title: entry.label,
            subtitle: entry.subtitle,
            value: &value,
            selected: offset == selected,
        };
        let top = STATUS_BAR_HEIGHT + offset as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }
    draw_bottom_bar(display, state.display, &OPEN_HINTS)
}

/// The Clock & alarms and System sub-lists render like the other categories.
pub fn render_settings_sublist(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let route = state.active_route();
    let entries = category_entries(route);
    let selected = state.category_selection(route);
    let status = item_count_text(entries.len());
    draw_status_bar(display, state.display, route.label())?;
    draw_status_text(display, state.display, &status, STATUS_BAR_RIGHT)?;
    for (offset, entry) in entries.iter().take(CATEGORY_PAGE_SIZE).enumerate() {
        let row = ListRow {
            title: entry.label,
            subtitle: entry.subtitle,
            value: entry.badge,
            selected: offset == selected,
        };
        let top = STATUS_BAR_HEIGHT + offset as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }
    draw_bottom_bar(display, state.display, &OPEN_HINTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alarm::{AlarmDefinition, AlarmScheduleKind};
    use crate::app::display::{UiFontFamily, UiFontSize};
    use crate::app::typography::UiTextRole;
    use embedded_graphics::pixelcolor::BinaryColor;

    fn alarm(hour: u8, enabled: bool) -> AlarmDefinition {
        AlarmDefinition {
            name: format!("Alarm {hour:02}"),
            hour,
            minute: 0,
            enabled,
            schedule: AlarmScheduleKind::Recurring { weekdays: 0x7f },
        }
    }

    #[test]
    fn display_value_joins_family_and_size() {
        let value = display_value(DisplayPreferences {
            font_family: UiFontFamily::Inter,
            font_size: UiFontSize::Standard,
        });
        assert_eq!(value, "Inter · Standard");
    }

    #[test]
    fn weather_value_covers_setup_off_manual_and_intervals() {
        assert_eq!(weather_value(None), "Not set up");
        let mut config = WeatherConfig::parse(crate::weather_config::SAMPLE_CONFIG).unwrap();
        config.enabled = false;
        assert_eq!(weather_value(Some(&config)), "Off");
        config.enabled = true;
        config.refresh_minutes = 120;
        assert_eq!(weather_value(Some(&config)), "On · 2 h");
        config.refresh_minutes = 90;
        assert_eq!(weather_value(Some(&config)), "On · 1.5 h");
        config.refresh_minutes = 0;
        assert_eq!(weather_value(Some(&config)), "Manual");
    }

    #[test]
    fn network_value_reads_like_the_mockup() {
        let value = |wifi_state| network_value(&wifi_state, Some("FOLLOUP"));
        assert_eq!(value(WifiConnectionState::Connected), "FOLLOUP");
        assert_eq!(value(WifiConnectionState::Disabled), "Off");
        let missing = WifiConnectionState::ConfigurationMissing;
        assert_eq!(value(missing), "Not set up");
        assert_eq!(value(WifiConnectionState::Connecting), "Connecting");
        assert_eq!(value(WifiConnectionState::Failed), "Failed");
        let connected = WifiConnectionState::Connected;
        assert_eq!(network_value(&connected, None), "Connected");
    }

    #[test]
    fn clock_alarms_value_counts_enabled_alarms_only() {
        let mut snapshot = AlarmSnapshot::default();
        assert_eq!(clock_alarms_value(&snapshot), "No alarms");
        snapshot.alarms.push(alarm(7, true));
        assert_eq!(clock_alarms_value(&snapshot), "1 alarm");
        snapshot.alarms.push(alarm(8, false));
        assert_eq!(clock_alarms_value(&snapshot), "1 alarm");
        snapshot.alarms.push(alarm(9, true));
        assert_eq!(clock_alarms_value(&snapshot), "2 alarms");
    }

    #[test]
    fn power_value_shows_dash_when_the_gauge_is_silent() {
        assert_eq!(power_value(None), "--");
        assert_eq!(power_value(Some(78)), "78%");
    }

    #[test]
    fn sleep_screen_value_counts_starred_photos() {
        let mut state = AppState::default();
        assert_eq!(sleep_screen_value(&state), "Photo · none starred");
        state.photos.starred.toggle("IMG_0407.jpg");
        state.photos.starred.toggle("IMG_0409.jpg");
        assert_eq!(sleep_screen_value(&state), "Photo · 2 starred");
        state.sleep_screen.source = SleepSource::Folder;
        assert_eq!(sleep_screen_value(&state), "Photo · folder");
        state.sleep_screen.mode = SleepMode::Clock;
        assert_eq!(sleep_screen_value(&state), "Clock · 1 min");
    }

    #[test]
    fn every_settings_row_has_a_value() {
        let state = AppState::default();
        let display = group_value(&state, ScreenRoute::Display);
        assert_eq!(display, "Inter · Standard");
        let system = group_value(&state, ScreenRoute::SettingsSystem);
        assert_eq!(system, format!("v{FIRMWARE_VERSION}"));
        for entry in category_entries(ScreenRoute::Settings) {
            let value = group_value(&state, entry.route);
            assert!(!value.is_empty(), "{}", entry.label);
        }
    }

    #[test]
    fn every_row_fits_beside_its_value_at_every_font() {
        let mut state = AppState::default();
        state.network.wifi_state = WifiConnectionState::Connected;
        state.network.ssid = Some("Home-5G".into());
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let style = |role| preferences.text_style(role, BinaryColor::On);
                for entry in category_entries(ScreenRoute::Settings) {
                    let value = group_value(&state, entry.route);
                    // As `draw_list_row`: text from x 34, the value ending at
                    // x 462, 12 px between them.
                    let room = 462 - style(UiTextRole::Body).text_width(&value) - 12 - 34;
                    let label = format!("{} {font_family:?} {font_size:?}", entry.label);
                    assert!(
                        style(UiTextRole::Heading).text_width(entry.label) <= room,
                        "{label}"
                    );
                    assert!(
                        style(UiTextRole::Detail).text_width(entry.subtitle) <= room,
                        "{label}"
                    );
                }
            }
        }
    }
}

/// Settings › AI as the mockup draws it: three groups of rows, the selected
/// row inverted, and an open list over the row it belongs to.
pub fn render_ai_settings(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    draw_status_bar(display, preferences, "Settings \u{203a} AI")?;
    let config = state.ai.clone().unwrap_or_default();
    let cursor = state.ai_settings_ui;
    let rows: [(&str, String); 8] = [
        ("Provider", provider_or_empty(&config.transcription_url)),
        ("Model", config.transcription_model.clone()),
        ("Language", config.language.label().into()),
        ("Provider", provider_or_empty(&config.summary_url)),
        ("Model", config.summary_model.clone()),
        ("Style", config.style.label().into()),
        ("Process", config.process.label().into()),
        ("API keys", "Not set".into()),
    ];
    let groups = [
        (0, "Transcription \u{00b7} /audio/transcriptions"),
        (3, "Summary \u{00b7} /chat/completions"),
        (6, "General"),
    ];
    if let (Some(highlighted), Some(row)) = (cursor.picker, cursor.list_row()) {
        // An open list replaces the rows, as the Weather list does.
        let (options, current) = config.options(row);
        let heading = preferences.heading_style();
        let title = rows[cursor.selected].0;
        Text::new(title, Point::new(LIST_TEXT_LEFT, 160), heading).draw(display)?;
        draw_option_list(display, preferences, 184, &options, current, highlighted)?;
        return draw_bottom_bar(display, preferences, &CHOOSE_HINTS);
    }
    let mut top = STATUS_BAR_HEIGHT + 16;
    for (index, (title, value)) in rows.iter().enumerate() {
        if let Some((_, label)) = groups.iter().find(|(first, _)| *first == index) {
            // Each group's label takes a short band above its first row.
            let style = preferences.text_style(UiTextRole::Detail, BinaryColor::On);
            let baseline = top + style.cap_height();
            Text::new(label, Point::new(LIST_TEXT_LEFT, baseline), style).draw(display)?;
            top += LIST_ROW_HEIGHT / 3;
        }
        draw_list_row(
            display,
            preferences,
            top,
            ListRow {
                title,
                subtitle: "",
                value,
                selected: index == cursor.selected,
            },
        )?;
        top += LIST_ROW_HEIGHT;
    }
    draw_bottom_bar(display, preferences, &CHANGE_HINTS)
}

/// The provider name of a URL, or `Not set` when the URL is empty.
fn provider_or_empty(url: &str) -> String {
    if url.is_empty() {
        "Not set".into()
    } else {
        crate::ai_config::provider_name(url)
    }
}
