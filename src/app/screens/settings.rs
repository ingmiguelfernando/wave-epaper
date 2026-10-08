//! Settings groups: one page with a value per group, plus the two sub-lists
//! (Clock & alarms, System) that stay category routes.

use core::convert::Infallible;

use crate::orientation::OrientedFrameBuffer;

use super::category::item_count_text;
use crate::{
    alarm::AlarmSnapshot,
    app::{
        display::DisplayPreferences,
        menu::{category_entries, CATEGORY_PAGE_SIZE},
        router::ScreenRoute,
        state::AppState,
        widgets::{
            bottom_bar::{draw_bottom_bar, OPEN_HINTS},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    build_info::FIRMWARE_VERSION,
    network::WifiConnectionState,
    sleep_screen::SleepSource,
    weather_config::WeatherConfig,
};

/// One settings group: what it covers on the one-page Settings.
struct SettingsGroup {
    title: &'static str,
    subtitle: &'static str,
}

const GROUPS: [SettingsGroup; 7] = [
    SettingsGroup {
        title: "Display",
        subtitle: "Font, size, ghost cleanup",
    },
    SettingsGroup {
        title: "Sleep screen",
        subtitle: "Photo, clock, weather",
    },
    SettingsGroup {
        title: "Weather",
        subtitle: "Service, interval, location",
    },
    SettingsGroup {
        title: "Wi-Fi & transfer",
        subtitle: "Network, file portal",
    },
    SettingsGroup {
        title: "Clock & alarms",
        subtitle: "Time, date, alarms",
    },
    SettingsGroup {
        title: "Power",
        subtitle: "Auto-sleep, battery log",
    },
    SettingsGroup {
        title: "System",
        subtitle: "Version, SD, diagnostics",
    },
];

/// Value of the group at `index`, straight from state. The index follows
/// [`GROUPS`]; tests pin the order.
#[must_use]
pub fn group_value(state: &AppState, index: usize) -> String {
    match index {
        0 => display_value(state.display),
        1 => sleep_screen_value(state),
        2 => weather_value(state.weather_config.as_ref()),
        3 => network_value(&state.network.wifi_state, state.network.ssid.as_deref()),
        4 => clock_alarms_value(&state.alarms),
        5 => power_value(state.board.power.and_then(|power| power.battery_percent)),
        _ => format!("v{FIRMWARE_VERSION}"),
    }
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

/// Sleep screen mode, with the starred photo count when it plays photos.
/// One function, so the main line's sleep-mode work changes only this.
#[must_use]
pub fn sleep_screen_value(state: &AppState) -> String {
    match state.sleep_screen.source {
        SleepSource::Starred => {
            let starred = state.photos.starred.len();
            if starred == 0 {
                "Photo · none starred".into()
            } else {
                format!("Photo · {starred} starred")
            }
        }
        SleepSource::Folder => "Folder".into(),
    }
}

/// `On · 2 h`, `Manual` (no config file) or `Off`.
#[must_use]
pub fn weather_value(config: Option<&WeatherConfig>) -> String {
    match config {
        None => "Manual".into(),
        Some(config) if !config.enabled => "Off".into(),
        Some(config) => {
            if config.refresh_minutes == 0 {
                "On · manual".into()
            } else {
                format!("On · {}", hours_text(config.refresh_minutes))
            }
        }
    }
}

/// The Wi-Fi state as the status bar words it; the SSID when connected.
#[must_use]
pub fn network_value(wifi_state: &WifiConnectionState, ssid: Option<&str>) -> String {
    match wifi_state {
        WifiConnectionState::Connected => ssid
            .map(str::to_owned)
            .unwrap_or_else(|| "Connected".into()),
        other => other.label().to_owned(),
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
fn hours_text(minutes: u64) -> String {
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
    draw_status_bar(display, state.display, ScreenRoute::Settings.label())?;
    draw_status_text(display, state.display, FIRMWARE_VERSION, STATUS_BAR_RIGHT)?;
    for (offset, group) in GROUPS.iter().enumerate() {
        let value = group_value(state, offset);
        let row = ListRow {
            title: group.title,
            subtitle: group.subtitle,
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
    use crate::network::{NetworkSnapshot, NtpSyncState};

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
    fn weather_value_covers_manual_off_and_intervals() {
        assert_eq!(weather_value(None), "Manual");
        let mut config = WeatherConfig::parse(crate::weather_config::SAMPLE_CONFIG).unwrap();
        config.enabled = false;
        assert_eq!(weather_value(Some(&config)), "Off");
        config.enabled = true;
        config.refresh_minutes = 120;
        assert_eq!(weather_value(Some(&config)), "On · 2 h");
        config.refresh_minutes = 90;
        assert_eq!(weather_value(Some(&config)), "On · 1.5 h");
        config.refresh_minutes = 0;
        assert_eq!(weather_value(Some(&config)), "On · manual");
    }

    #[test]
    fn network_value_shows_the_ssid_when_connected() {
        assert_eq!(
            network_value(&WifiConnectionState::Connected, Some("FOLLOUP")),
            "FOLLOUP"
        );
        assert_eq!(
            network_value(&WifiConnectionState::Connected, None),
            "Connected"
        );
        assert_eq!(network_value(&WifiConnectionState::Disabled, None), "OFF");
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
        assert_eq!(sleep_screen_value(&state), "Folder");
    }

    #[test]
    fn group_values_follow_the_group_order() {
        let state = AppState::default();
        assert_eq!(group_value(&state, 0), "Inter · Standard");
        assert_eq!(group_value(&state, 6), format!("v{FIRMWARE_VERSION}"));
    }

    #[test]
    fn network_snapshot_defaults_do_not_panic_in_values() {
        let snapshot = NetworkSnapshot::default();
        assert_eq!(
            network_value(&snapshot.wifi_state, snapshot.ssid.as_deref()),
            "NO CONFIG"
        );
        let _ = NtpSyncState::default();
    }

    #[test]
    fn every_value_and_subtitle_fit_at_every_font_family_and_size() {
        use crate::app::typography::UiTextRole;
        use embedded_graphics::pixelcolor::BinaryColor;

        let state = AppState::default();
        let mut longest_value = String::new();
        for index in 0..7 {
            let value = group_value(&state, index);
            if value.chars().count() > longest_value.chars().count() {
                longest_value = value;
            }
        }
        // The widest plausible SSID keeps the Wi-Fi row honest too.
        longest_value.clone_from(&"VeryLongNetworkName-5G".to_string());

        for family in UiFontFamily::ALL {
            for size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family: family,
                    font_size: size,
                };
                let heading = preferences.text_style(UiTextRole::Heading, BinaryColor::On);
                let detail = preferences.text_style(UiTextRole::Detail, BinaryColor::On);
                let body = preferences.text_style(UiTextRole::Body, BinaryColor::On);
                // Row text width: title and subtitle share the left column;
                // the value sits right of it with a 12 px gutter.
                let available = 480 - 18 - 34 - 12;
                for group in [
                    ("Display", "Font, size, ghost cleanup"),
                    ("Sleep screen", "Photo, clock, weather"),
                    ("Weather", "Service, interval, location"),
                    ("Wi-Fi & transfer", "Network, file portal"),
                    ("Clock & alarms", "Time, date, alarms"),
                    ("Power", "Auto-sleep, battery log"),
                    ("System", "Version, SD, diagnostics"),
                ] {
                    let title = heading.fit(group.0, available);
                    assert_eq!(title, group.0, "title {family:?} {size:?}");
                    let subtitle = detail.fit(group.1, available);
                    assert_eq!(
                        subtitle, group.1,
                        "subtitle {family:?}
{size:?}"
                    );
                }
                let value = body.fit(&longest_value, available);
                assert_eq!(value, longest_value, "value {family:?} {size:?}");
            }
        }
    }
}
