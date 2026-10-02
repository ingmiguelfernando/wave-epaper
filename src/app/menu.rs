//! Data-driven Home and category menu definitions for the main product shell.
//!
//! Category rows contain applications only. Hierarchical navigation uses the
//! dedicated GPIO0 Boot-button long press instead of synthetic Back rows.

use super::router::ScreenRoute;

pub const HOME_ENTRY_COUNT: usize = 9;
pub const CATEGORY_COUNT: usize = 5;
pub const CATEGORY_PAGE_SIZE: usize = 9;

/// Badge for entries whose screen is planned but not built yet.
pub const SOON_BADGE: &str = "SOON";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuEntry {
    pub label: &'static str,
    pub subtitle: &'static str,
    pub badge: &'static str,
    pub route: ScreenRoute,
}

const HOME_ENTRIES: [MenuEntry; HOME_ENTRY_COUNT] = [
    MenuEntry {
        label: "Photos",
        subtitle: "SD photo gallery and sleep screen set",
        badge: SOON_BADGE,
        route: ScreenRoute::Photos,
    },
    MenuEntry {
        label: "Library",
        subtitle: "Books, progress and bookmarks",
        badge: "",
        route: ScreenRoute::Reader,
    },
    MenuEntry {
        label: "Bible",
        subtitle: "Offline Bible reader",
        badge: SOON_BADGE,
        route: ScreenRoute::Bible,
    },
    MenuEntry {
        label: "Reading Stats",
        subtitle: "Reading time, streaks and finished books",
        badge: SOON_BADGE,
        route: ScreenRoute::ReadingStats,
    },
    MenuEntry {
        label: "Weather",
        subtitle: "Open-Meteo conditions and forecast",
        badge: "",
        route: ScreenRoute::Weather,
    },
    MenuEntry {
        label: "Games",
        subtitle: "E-paper friendly games",
        badge: "",
        route: ScreenRoute::Games,
    },
    MenuEntry {
        label: "AI",
        subtitle: "XiaoZhi and voice notes",
        badge: "",
        route: ScreenRoute::Ai,
    },
    MenuEntry {
        label: "Tools",
        subtitle: "Files, dictionary, converter and calendar",
        badge: "",
        route: ScreenRoute::Tools,
    },
    MenuEntry {
        label: "Settings",
        subtitle: "Display, network and device",
        badge: "",
        route: ScreenRoute::Settings,
    },
];

const READER_ENTRIES: [MenuEntry; 3] = [
    MenuEntry {
        label: "Continue Reading",
        subtitle: "Resume the last book",
        badge: "",
        route: ScreenRoute::ContinueReading,
    },
    MenuEntry {
        label: "Books",
        subtitle: "TXT and EPUB books on the SD card",
        badge: "",
        route: ScreenRoute::Library,
    },
    MenuEntry {
        label: "Bookmarks",
        subtitle: "Saved reading positions",
        badge: "",
        route: ScreenRoute::Bookmarks,
    },
];

const AI_ENTRIES: [MenuEntry; 2] = [
    MenuEntry {
        label: "XiaoZhi",
        subtitle: "Voice chat with xiaozhi.me",
        badge: SOON_BADGE,
        route: ScreenRoute::XiaoZhi,
    },
    MenuEntry {
        label: "Voice Notes",
        subtitle: "Record voice notes to the SD card",
        badge: "",
        route: ScreenRoute::VoiceNotes,
    },
];

const GAMES_ENTRIES: [MenuEntry; 1] = [MenuEntry {
    label: "SD Games",
    subtitle: "Games loaded from the SD card",
    badge: "",
    route: ScreenRoute::LuaApps,
}];

const TOOLS_ENTRIES: [MenuEntry; 4] = [
    MenuEntry {
        label: "File Browser",
        subtitle: "Browse the SD card",
        badge: "",
        route: ScreenRoute::Files,
    },
    MenuEntry {
        label: "Dictionary",
        subtitle: "Offline word lookup",
        badge: "",
        route: ScreenRoute::Dictionary,
    },
    MenuEntry {
        label: "Unit Converter",
        subtitle: "Offline unit conversions",
        badge: "",
        route: ScreenRoute::UnitConverter,
    },
    MenuEntry {
        label: "Calendar",
        subtitle: "Month view, agenda and events",
        badge: "",
        route: ScreenRoute::Calendar,
    },
];

const SETTINGS_ENTRIES: [MenuEntry; 8] = [
    MenuEntry {
        label: "Alarms",
        subtitle: "Alarm schedules, snooze and dismiss",
        badge: "",
        route: ScreenRoute::Alarms,
    },
    MenuEntry {
        label: "Audio",
        subtitle: "Speaker test and alarm chime",
        badge: "",
        route: ScreenRoute::Audio,
    },
    MenuEntry {
        label: "Clock",
        subtitle: "Time, date and battery",
        badge: "",
        route: ScreenRoute::Clock,
    },
    MenuEntry {
        label: "Display",
        subtitle: "Interface font and size",
        badge: "",
        route: ScreenRoute::Display,
    },
    MenuEntry {
        label: "Device Info",
        subtitle: "Firmware, board and memory",
        badge: "",
        route: ScreenRoute::DeviceInfo,
    },
    MenuEntry {
        label: "Environment",
        subtitle: "Temperature and humidity",
        badge: "",
        route: ScreenRoute::Environment,
    },
    MenuEntry {
        label: "Motion",
        subtitle: "Accelerometer and gyroscope",
        badge: "",
        route: ScreenRoute::Motion,
    },
    MenuEntry {
        label: "Network",
        subtitle: "Wi-Fi, time sync and file transfer",
        badge: "",
        route: ScreenRoute::Network,
    },
];

#[must_use]
pub const fn home_entries() -> &'static [MenuEntry] {
    &HOME_ENTRIES
}

#[must_use]
pub const fn category_entries(route: ScreenRoute) -> &'static [MenuEntry] {
    match route {
        ScreenRoute::Reader => &READER_ENTRIES,
        ScreenRoute::Ai => &AI_ENTRIES,
        ScreenRoute::Games => &GAMES_ENTRIES,
        ScreenRoute::Tools => &TOOLS_ENTRIES,
        ScreenRoute::Settings => &SETTINGS_ENTRIES,
        _ => &[],
    }
}

#[must_use]
pub const fn category_index(route: ScreenRoute) -> Option<usize> {
    match route {
        ScreenRoute::Reader => Some(0),
        ScreenRoute::Ai => Some(1),
        ScreenRoute::Games => Some(2),
        ScreenRoute::Tools => Some(3),
        ScreenRoute::Settings => Some(4),
        _ => None,
    }
}

/// Position of `route` on Home, for tests and shortcuts.
#[must_use]
pub fn home_index(route: ScreenRoute) -> Option<usize> {
    home_entries().iter().position(|entry| entry.route == route)
}

#[cfg(test)]
mod tests {
    use super::{
        category_entries, home_entries, home_index, MenuEntry, HOME_ENTRY_COUNT, SOON_BADGE,
    };
    use crate::app::router::ScreenRoute;

    const CATEGORIES: [ScreenRoute; 5] = [
        ScreenRoute::Reader,
        ScreenRoute::Ai,
        ScreenRoute::Games,
        ScreenRoute::Tools,
        ScreenRoute::Settings,
    ];

    #[test]
    fn home_lists_photos_first_and_every_category() {
        assert_eq!(home_entries().len(), HOME_ENTRY_COUNT);
        assert_eq!(home_index(ScreenRoute::Photos), Some(0));
        for route in CATEGORIES {
            assert!(home_index(route).is_some(), "{route:?} missing from Home");
        }
    }

    #[test]
    fn categories_have_entries_without_synthetic_back_rows() {
        assert_eq!(category_entries(ScreenRoute::Reader).len(), 3);
        assert_eq!(category_entries(ScreenRoute::Ai).len(), 2);
        assert_eq!(category_entries(ScreenRoute::Games).len(), 1);
        assert_eq!(category_entries(ScreenRoute::Tools).len(), 4);
        assert_eq!(category_entries(ScreenRoute::Settings).len(), 8);
        for route in CATEGORIES {
            assert!(category_entries(route)
                .iter()
                .all(|entry| entry.route != ScreenRoute::Home));
        }
    }

    #[test]
    fn only_unbuilt_screens_carry_the_soon_badge() {
        let mut all: Vec<MenuEntry> = home_entries().to_vec();
        for route in CATEGORIES {
            all.extend_from_slice(category_entries(route));
        }
        for entry in all {
            assert_eq!(
                entry.badge == SOON_BADGE,
                entry.route.is_placeholder(),
                "{}",
                entry.label
            );
        }
    }

    #[test]
    fn tools_keep_existing_offline_apps() {
        let tools = category_entries(ScreenRoute::Tools);
        for route in [
            ScreenRoute::Files,
            ScreenRoute::Dictionary,
            ScreenRoute::UnitConverter,
            ScreenRoute::Calendar,
        ] {
            assert!(tools.iter().any(|entry| entry.route == route));
        }
    }
}
