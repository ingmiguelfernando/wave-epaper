//! Screen rendering boundary for the product shell.

use core::convert::Infallible;

use crate::orientation::OrientedFrameBuffer;

use super::{router::ScreenRoute, state::AppState};

pub mod ai;
pub mod alarms;
pub mod audio;
pub mod bible;
pub mod calendar;
pub mod category;
pub mod clock;
pub mod device_info;
pub mod dictionary;
pub mod display;
pub mod environment;
pub mod files;
pub mod games;
pub mod home;
pub mod lua_game;
pub mod motion;
pub mod network;
pub mod photos;
pub mod placeholder;
pub mod power;
pub mod power_key;
pub mod reader;
pub mod reading_stats;
pub mod settings;
pub mod sleep_card;
pub mod sleep_screens;
pub mod sleep_settings;
pub mod unit_converter;
pub mod voice_notes;
pub mod weather;
pub mod weather_settings;

/// Draw the active screen selected by the router.
pub fn render_active_screen(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    match state.active_route() {
        ScreenRoute::Home => home::render_home(display, state),
        ScreenRoute::Ai => ai::render_ai_hub(display, state),
        ScreenRoute::Settings => settings::render_settings(display, state),
        ScreenRoute::SettingsClockAlarms | ScreenRoute::SettingsSystem => {
            settings::render_settings_sublist(display, state)
        }
        route if route.is_category() => category::render_category(display, state),
        route if route.is_placeholder() => placeholder::render_placeholder(display, state),
        ScreenRoute::ContinueReading => reader::render_continue_reading(display, state),
        ScreenRoute::Library => reader::render_library(display, state),
        ScreenRoute::Bookmarks | ScreenRoute::ReaderBookmarks => {
            reader::render_bookmarks(display, state)
        }
        ScreenRoute::ReaderLoading => reader::render_loading(display, state),
        ScreenRoute::ReaderPage => reader::render_page(display, state),
        ScreenRoute::ReaderOptions => reader::render_options(display, state),
        ScreenRoute::ReaderPreferences => reader::render_preferences(display, state),
        ScreenRoute::ReaderToc => reader::render_toc(display, state),
        ScreenRoute::Calendar => calendar::render_calendar(display, state),
        ScreenRoute::CalendarAgenda => calendar::render_calendar_agenda(display, state),
        ScreenRoute::CalendarEventDetails => {
            calendar::render_calendar_event_details(display, state)
        }
        ScreenRoute::CalendarEventEditor => calendar::render_calendar_event_editor(display, state),
        ScreenRoute::CalendarDeleteConfirmation => {
            calendar::render_calendar_delete_confirmation(display, state)
        }
        ScreenRoute::VoiceNotes => voice_notes::render_voice_notes(display, state),
        ScreenRoute::VoiceNoteDetails => voice_notes::render_voice_note_details(display, state),
        ScreenRoute::VoiceNoteRecording => voice_notes::render_voice_note_recording(display, state),
        ScreenRoute::Games => games::render_games_hub(display, state),
        ScreenRoute::LuaApps => lua_game::render_lua_apps(display, state),
        ScreenRoute::LuaGame => lua_game::render_lua_game(display, state),
        ScreenRoute::LuaGameError => lua_game::render_lua_error(display, state),
        ScreenRoute::Dictionary => dictionary::render_dictionary(display, state),
        ScreenRoute::UnitConverter => unit_converter::render_unit_converter(display, state),
        ScreenRoute::Clock => clock::render_clock(display, state),
        ScreenRoute::ClockDetails => clock::render_clock_details(display, state),
        ScreenRoute::Environment => environment::render_environment(display, state),
        ScreenRoute::EnvironmentDetails => environment::render_environment_details(display, state),
        ScreenRoute::Motion => motion::render_motion(display, state),
        ScreenRoute::MotionEvents => motion::render_motion_events(display, state),
        ScreenRoute::MotionDetails => motion::render_motion_details(display, state),
        ScreenRoute::Network => network::render_network(display, state),
        ScreenRoute::NetworkDetails => network::render_network_details(display, state),
        ScreenRoute::WifiTransfer => network::render_wifi_transfer(display, state),
        ScreenRoute::Weather => weather::render_weather(display, state),
        ScreenRoute::WeatherDetails => weather::render_weather_details(display, state),
        ScreenRoute::WeatherSettings => weather_settings::render_weather_settings(display, state),
        ScreenRoute::Alarms => alarms::render_alarms(display, state),
        ScreenRoute::Audio => audio::render_audio(display, state),
        ScreenRoute::AudioDetails => audio::render_audio_details(display, state),
        ScreenRoute::Files => files::render_files(display, state),
        ScreenRoute::Display => display::render_display(display, state),
        ScreenRoute::Power => power::render_power(display, state),
        ScreenRoute::SleepScreen => sleep_settings::render_sleep_settings(display, state),
        ScreenRoute::ReadingStats => {
            render_reading_stats_from_state(display, state);
            Ok(())
        }
        ScreenRoute::Photos => photos::render_photos(display, state),
        ScreenRoute::PhotoViewer => photos::render_photo_viewer(display, state),
        ScreenRoute::PowerKeyMenu => power_key::render_power_key_menu(display, state),
        ScreenRoute::DeviceInfo => device_info::render_device_info(display, state),
        ScreenRoute::DeviceInfoBoard => device_info::render_device_info_board(display, state),
        ScreenRoute::DeviceInfoRuntime => device_info::render_device_info_runtime(display, state),
        ScreenRoute::Bible | ScreenRoute::BibleBooks => {
            bible::render_bible_books_screen(display, state)
        }
        ScreenRoute::BibleChapters => bible::render_bible_chapters_screen(display, state),
        ScreenRoute::BibleReading => bible::render_bible_reading(display, state),
        ScreenRoute::BibleMenu => bible::render_bible_menu(display, state),
        ScreenRoute::BibleMissing => bible::render_bible_missing(display, state),
        ScreenRoute::Reader | ScreenRoute::Tools | ScreenRoute::GamesTbd | ScreenRoute::XiaoZhi => {
            unreachable!("category and placeholder routes handled above")
        }
    }
}

/// Reading Stats needs the RTC's civil day; without a set clock there is
/// nothing to show but the header.
fn render_reading_stats_from_state(display: &mut OrientedFrameBuffer<'_>, state: &AppState) {
    let current = state
        .reader
        .session
        .as_ref()
        .map(|session| reading_stats::CurrentBook {
            title: &session.book.title,
            path: &session.book.path,
            percent: None,
        });
    let _ = match state.local_day() {
        Some(today) => reading_stats::render_reading_stats(
            display,
            state.display,
            &state.reading_stats,
            today,
            current.as_ref(),
        ),
        None => reading_stats::render_without_clock(display, state.display),
    };
}
