//! Host-only screen previews for reviewing layouts without the board.
//!
//! `WAVE_PREVIEW_DIR=/tmp/wave cargo +stable test --target <host> --lib preview`
//! writes one PNG per screen. Without the variable the screens are still
//! rendered, so a layout that panics fails the test.

use std::{fs, path::Path};

use embedded_graphics::prelude::Point;

use super::{
    display::UiFontSize, reader_typography::reader_body_style, render_current_screen,
    render_sleep_card, typography::Text, AppState, ScreenRoute, SleepCard,
};
use super::{
    display::{DisplayPreferences, UiFontFamily},
    render_sleep_clock, render_sleep_weather,
    screens::reading_stats::{render_reading_stats, CurrentBook},
    SleepClock, SleepWeather, SleepWeatherDay, SleepWeatherLine,
};
use crate::bible_nav::BibleNav;
use crate::{
    board_services::BoardSnapshot,
    buttons::ButtonEvent,
    framebuffer::FrameBuffer,
    games::{
        canvas::NativeGameCanvas,
        records::GameRecords,
        refresh_policy::GameRefreshPlan,
        sudoku::SudokuGame,
        sudoku_puzzles::{generate, SudokuDifficulty},
    },
    lua_runtime::{
        event_bridge::LuaEventBridge,
        manifest::{LuaAppEntry, LuaAppKind, LuaAppManifest},
        LuaAppSession,
    },
    network::WifiConnectionState,
    orientation::{DisplayOrientation, OrientedFrameBuffer},
    photos::{test_photos::jpeg_from_grey, ui::PhotosUiState, worker::prepare},
    power::PowerSnapshot,
    reader::{
        BookFont, BookFontSize, BookFormat, ReaderLocation, ReaderTickOutcome, ReaderUiState,
        ReadingTheme,
    },
    rtc::RtcDateTime,
    weather::{parse_open_meteo_response, WeatherSnapshot, SAMPLE_RESPONSE},
    weather_config::{WeatherConfig, SAMPLE_CONFIG},
};

const SPANISH_SAMPLE: [&str; 2] = [
    "¿Dónde está el niño? ¡Ahí, señor!",
    "«Cien años» — “sí”, ‘no’… 18°C · Ñandú",
];

/// Opening of Don Quijote (1605, public domain) for the Reader page preview.
const QUIJOTE: &str = "En un lugar de la Mancha, de cuyo nombre no quiero acordarme, no ha \
    mucho tiempo que vivía un hidalgo de los de lanza en astillero, adarga antigua, rocín \
    flaco y galgo corredor. Una olla de algo más vaca que carnero, salpicón las más noches, \
    duelos y quebrantos los sábados, lantejas los viernes, algún palomino de añadidura los \
    domingos, consumían las tres partes de su hacienda.\n\
    El resto della concluían sayo de velarte, calzas de velludo para las fiestas, con sus \
    pantuflos de lo mesmo, y los días de entresemana se honraba con su vellorí de lo más \
    fino. Tenía en su casa una ama que pasaba de los cuarenta, y una sobrina que no llegaba \
    a los veinte, y un mozo de campo y plaza, que así ensillaba el rocín como tomaba la \
    podadera. Frisaba la edad de nuestro hidalgo con los cincuenta años; era de complexión \
    recia, seco de carnes, enjuto de rostro, gran madrugador y amigo de la caza.\n";

#[test]
fn render_screen_previews() {
    let output = std::env::var_os("WAVE_PREVIEW_DIR");
    if let Some(directory) = output.as_deref() {
        fs::create_dir_all(directory).unwrap();
    }
    let mut images = vec![
        ("reader-fonts", render_reader_font_sheet()),
        ("sleep-card", render_sample_sleep_card()),
        (
            "sleep-clock",
            render_sample_sleep_clock(sample_state().display, false),
        ),
        (
            "sleep-clock-weather",
            render_sample_sleep_clock(sample_state().display, true),
        ),
        (
            "sleep-weather",
            render_sample_sleep_weather(sample_state().display),
        ),
        (
            "reading-stats",
            render_sample_reading_stats(sample_state().display, true),
        ),
        (
            "reading-stats-empty",
            render_sample_reading_stats(sample_state().display, false),
        ),
        (
            "bible-books",
            render_sample_bible(false, sample_state().display),
        ),
        (
            "bible-chapters",
            render_sample_bible(true, sample_state().display),
        ),
    ];
    for (name, state) in preview_states() {
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        images.push((name, frame));
    }
    if let Some(directory) = output.as_deref() {
        for (name, frame) in images {
            let png = encode_png(&frame, DisplayOrientation::Portrait);
            fs::write(Path::new(directory).join(format!("{name}.png")), png).unwrap();
        }
    }
    for family in UiFontFamily::ALL {
        for size in UiFontSize::ALL {
            let preferences = DisplayPreferences {
                font_family: family,
                font_size: size,
            };
            for (name, frame) in [
                ("sleep-clock", render_sample_sleep_clock(preferences, false)),
                (
                    "sleep-clock-weather",
                    render_sample_sleep_clock(preferences, true),
                ),
                ("sleep-weather", render_sample_sleep_weather(preferences)),
                (
                    "reading-stats",
                    render_sample_reading_stats(preferences, true),
                ),
                (
                    "reading-stats-empty",
                    render_sample_reading_stats(preferences, false),
                ),
            ] {
                if let Some(directory) = output.as_deref() {
                    let name = format!("{name}-{}-{}.png", family.marker(), size.marker());
                    fs::write(
                        Path::new(directory).join(name),
                        encode_png(&frame, DisplayOrientation::Portrait),
                    )
                    .unwrap();
                }
            }
        }
    }
}

/// Spanish sample text in every Reader family at the two smallest sizes.
fn render_reader_font_sheet() -> FrameBuffer {
    let mut frame = FrameBuffer::new_white();
    let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
    let mut baseline = 30;
    for family in [
        BookFont::Literata,
        BookFont::Serif,
        BookFont::AtkinsonHyperlegible,
        BookFont::Inter,
    ] {
        for size in [BookFontSize::Small, BookFontSize::Medium] {
            let style = reader_body_style(family, size, ReadingTheme::Classic);
            for line in SPANISH_SAMPLE {
                Text::new(line, Point::new(12, baseline), style)
                    .draw(&mut display)
                    .unwrap();
                baseline += i32::from(style.line_height());
            }
            baseline += 10;
        }
    }
    drop(display);
    frame
}

/// The card shown asleep when the SD picture cannot be used.
fn render_sample_sleep_card() -> FrameBuffer {
    let mut frame = FrameBuffer::new_white();
    let card = SleepCard {
        note: "SLEEP.BMP is 1024×768; it must be 480×800 or 800×480",
        battery_percent: Some(78),
        wake_hint: "Press any key to wake",
    };
    render_sleep_card(&mut frame, sample_state().display, &card).unwrap();
    frame
}

fn render_sample_sleep_clock(preferences: DisplayPreferences, with_weather: bool) -> FrameBuffer {
    let mut frame = FrameBuffer::new_white();
    let clock = SleepClock {
        time: "13:42",
        date: "Friday, October 2",
        weather: with_weather.then_some(SleepWeatherLine {
            weather_code: 2,
            summary: "18° · Partly cloudy",
            details: "H 21° · L 11° · Rain 10%",
        }),
        battery_percent: Some(78),
        wake_hint: "Press any key to wake",
    };
    render_sleep_clock(&mut frame, preferences, &clock).unwrap();
    frame
}

fn render_sample_sleep_weather(preferences: DisplayPreferences) -> FrameBuffer {
    let mut frame = FrameBuffer::new_white();
    let weather = SleepWeather {
        place: "Madrid",
        updated: "Fri, Oct 2 · updated 13:30",
        weather_code: 2,
        temperature: "18°",
        condition: "Partly cloudy",
        details: "H 21° · L 11° · Wind 12 km/h · Rain 10%",
        days: &[
            SleepWeatherDay {
                name: "Sat",
                weather_code: 0,
                range: "23° / 12°",
                rain: "0%",
            },
            SleepWeatherDay {
                name: "Sun",
                weather_code: 61,
                range: "17° / 10°",
                rain: "80%",
            },
            SleepWeatherDay {
                name: "Mon",
                weather_code: 95,
                range: "15° / 9°",
                rain: "60%",
            },
        ],
        battery_percent: Some(78),
        wake_hint: "Press any key to wake",
    };
    render_sleep_weather(&mut frame, preferences, &weather).unwrap();
    frame
}

/// A month of sample reading around 2026-10-03, or an empty history.
fn render_sample_reading_stats(preferences: DisplayPreferences, with_data: bool) -> FrameBuffer {
    const TODAY: u32 = 20_729; // 2026-10-03, a Saturday
    let mut stats = crate::reading_stats::ReadingStats::default();
    if with_data {
        for day in 0..84 {
            let minutes = 6 + (day * 7) % 64;
            stats.record(
                TODAY - 83 + day,
                minutes * 60,
                minutes / 2,
                Some("/sdcard/RUSTMIX/BOOKS/Don Quijote.txt"),
            );
        }
        stats.mark_finished("/sdcard/RUSTMIX/BOOKS/Other.txt");
    }
    let current = with_data.then(|| CurrentBook {
        title: "Don Quijote de la Mancha",
        path: "/sdcard/RUSTMIX/BOOKS/Don Quijote.txt",
        percent: Some(12),
    });
    let mut frame = FrameBuffer::new_white();
    let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
    render_reading_stats(&mut display, preferences, &stats, TODAY, current.as_ref()).unwrap();
    drop(display);
    frame
}

fn preview_states() -> Vec<(&'static str, AppState)> {
    let mut states = vec![
        ("home", sample_state()),
        ("home-first-boot", AppState::default()),
    ];

    let mut selected = sample_state();
    selected.home_selected = 4;
    states.push(("home-weather-selected", selected));

    let mut large = sample_state();
    large.display.font_size = UiFontSize::Large;
    states.push(("home-large-font", large));

    // Settings previews: the one-page groups, both sub-lists, and the page
    // at the Large size.
    let mut settings = sample_state();
    settings.router.navigate_to(ScreenRoute::Settings);
    settings.photos.starred.toggle("IMG_0407.jpg");
    settings.photos.starred.toggle("IMG_0409.jpg");
    states.push(("settings", settings));

    let mut settings_system = sample_state();
    settings_system
        .router
        .navigate_to(ScreenRoute::SettingsSystem);
    states.push(("settings-system", settings_system));

    let mut settings_clock = sample_state();
    settings_clock
        .router
        .navigate_to(ScreenRoute::SettingsClockAlarms);
    states.push(("settings-clock-alarms", settings_clock));

    let mut settings_large = sample_state();
    settings_large.display.font_size = UiFontSize::Large;
    settings_large.router.navigate_to(ScreenRoute::Settings);
    settings_large.photos.starred.toggle("IMG_0407.jpg");
    settings_large.photos.starred.toggle("IMG_0409.jpg");
    states.push(("settings-large-font", settings_large));

    let routes = [
        ("library", ScreenRoute::Reader),
        ("ai", ScreenRoute::Ai),
        ("games", ScreenRoute::Games),
        ("tools", ScreenRoute::Tools),
        ("weather", ScreenRoute::Weather),
        ("weather-details", ScreenRoute::WeatherDetails),
        ("weather-settings", ScreenRoute::WeatherSettings),
        ("voice-notes", ScreenRoute::VoiceNotes),
    ];
    for (name, route) in routes {
        let mut state = sample_state();
        state.router.navigate_to(route);
        if route == ScreenRoute::Games {
            sample_games_catalog(&mut state);
            // The hub's Sudoku card shows a running game and its best time.
            state.lua_runtime.sudoku_save = Some(sample_sudoku_save());
            state.lua_runtime.records.tetris_zen = 18_950;
            state.lua_runtime.records.sudoku_medium = 761;
        }
        if route == ScreenRoute::Ai {
            state.voice_notes.notes = sample_voice_notes();
        }
        states.push((name, state));
    }

    // The AI state labels, one note per state, online and offline.
    let mut ai_states = sample_state();
    ai_states.router.navigate_to(ScreenRoute::Ai);
    ai_states.voice_notes.notes = sample_ai_state_notes();
    states.push(("ai-states", ai_states.clone()));
    ai_states.network.wifi_state = WifiConnectionState::Failed;
    // Offline, the newest note (shown first on the hub) waits for a connection.
    ai_states.voice_notes.notes[3].ai_state = Some(crate::voice_note_record::NoteState::Queued);
    states.push(("ai-states-offline", ai_states));

    let mut list_states = sample_state();
    list_states.router.navigate_to(ScreenRoute::VoiceNotes);
    list_states.voice_notes.notes = sample_ai_state_notes();
    states.push(("voice-notes-states", list_states));

    let mut page = sample_state();
    open_sample_book(&mut page);
    states.push(("reader-page", page));

    let mut power = sample_state();
    power.router.navigate_to(ScreenRoute::Power);
    record_sample_battery_day(&mut power);
    states.push(("power", power.clone()));
    power.power_ui.picker = Some(2);
    states.push(("power-picker", power));

    let mut display_picker = sample_state();
    display_picker.router.navigate_to(ScreenRoute::Display);
    display_picker.display_picker = Some(1);
    states.push(("display-picker", display_picker));

    let mut reader_preferences_picker = sample_state();
    open_sample_book(&mut reader_preferences_picker);
    reader_preferences_picker.reader.begin_preferences_edit();
    reader_preferences_picker.reader.open_preference_picker();
    reader_preferences_picker
        .router
        .navigate_to(ScreenRoute::ReaderPreferences);
    states.push(("reader-preferences-picker", reader_preferences_picker));

    let mut gallery = sample_state();
    open_sample_gallery(&mut gallery);
    states.push(("photos", gallery.clone()));
    gallery.apply(ButtonEvent::Select);
    states.push(("photo-viewer", gallery.clone()));
    gallery.apply(ButtonEvent::Select);
    states.push(("photo-actions", gallery));

    let mut sleep_screen = sample_state();
    sleep_screen.router.navigate_to(ScreenRoute::SleepScreen);
    states.push(("sleep-screen", sleep_screen));

    let mut weather_picker = sample_state();
    weather_picker
        .router
        .navigate_to(ScreenRoute::WeatherSettings);
    weather_picker.apply(ButtonEvent::Down);
    weather_picker.apply(ButtonEvent::Select);
    states.push(("weather-settings-picker", weather_picker));

    let mut weather_off = sample_state();
    weather_off.router.navigate_to(ScreenRoute::Weather);
    weather_off.weather_config.as_mut().unwrap().enabled = false;
    states.push(("weather-off", weather_off));

    let mut weather_large = sample_state();
    weather_large.display.font_size = UiFontSize::Large;
    weather_large.router.navigate_to(ScreenRoute::Weather);
    states.push(("weather-large-font", weather_large));

    let mut audio_details = sample_state();
    audio_details.router.navigate_to(ScreenRoute::AudioDetails);
    states.push(("audio-details", audio_details));

    let mut tetris = sample_state();
    tetris.lua_runtime.session = Some(tetris_sample_session());
    tetris.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("tetris", tetris));

    let mut tetris_start = sample_state();
    tetris_start.lua_runtime.session = Some(tetris_start_session());
    tetris_start.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("tetris-start", tetris_start));

    let mut tetris_classic = sample_state();
    tetris_classic.lua_runtime.session = Some(tetris_classic_session());
    tetris_classic.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("tetris-classic", tetris_classic));

    let mut sudoku_start = sample_state();
    sudoku_start.lua_runtime.session = Some(sudoku_start_session());
    sudoku_start.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("sudoku-start", sudoku_start));

    let mut sudoku_row = sample_state();
    sudoku_row.lua_runtime.session = Some(sudoku_sample_session(0));
    sudoku_row.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("sudoku-row", sudoku_row));

    let mut sudoku_number = sample_state();
    sudoku_number.lua_runtime.session = Some(sudoku_sample_session(2));
    sudoku_number.router.navigate_to(ScreenRoute::LuaGame);
    states.push(("sudoku-number", sudoku_number));

    states.push((
        "bible-reading",
        bible_preview_state(ScreenRoute::BibleReading, 1),
    ));
    states.push(("bible-reading-large", bible_preview_state_large()));
    states.push(("bible-menu", bible_preview_state(ScreenRoute::BibleMenu, 1)));
    states.push(("bible-missing", AppState::default()));

    let mut ai = sample_state();
    ai.ai = Some(sample_ai_config());
    ai.router.navigate_to(ScreenRoute::AiSettings);
    states.push(("settings-ai", ai));

    let mut ai_picker = sample_state();
    ai_picker.ai = Some(sample_ai_config());
    ai_picker.ai_settings_ui.selected = 2;
    ai_picker.ai_settings_ui.picker = Some(1);
    ai_picker.router.navigate_to(ScreenRoute::AiSettings);
    states.push(("settings-ai-picker", ai_picker));

    let mut ai_hub = sample_state();
    ai_hub.ai = Some(sample_ai_config());
    ai_hub.router.navigate_to(ScreenRoute::Ai);
    states.push(("ai-providers", ai_hub));
    states
}

/// A configured AI setup: Groq for transcription, OpenRouter for summaries.
fn sample_ai_config() -> crate::ai_config::AiConfig {
    crate::ai_config::AiConfig {
        transcription_url: "https://api.groq.com/openai/v1".into(),
        transcription_model: "whisper-large-v3-turbo".into(),
        language: crate::ai_config::AiLanguage::Auto,
        summary_url: "https://openrouter.ai/api/v1".into(),
        summary_model: "llama-3.3-70b-instruct".into(),
        style: crate::ai_config::SummaryStyle::BulletsTodos,
        process: crate::ai_config::AiProcess::Online,
    }
}

/// A Bible state over a temp card holding Genesis 1, opened on `route`.
fn bible_preview_state(route: ScreenRoute, chapter: u16) -> AppState {
    let root = std::env::temp_dir().join(format!("wave-preview-bible-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let folder = root.join("RVR1960");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("index.tsv"), "GEN\tGénesis\t2\tGEN.txt\n").unwrap();
    let mut text = String::new();
    for number in 1..=2 {
        text.push_str(&format!("C\t{number}\nH\tLa creación\n"));
        for verse in 1..=12 {
            text.push_str(&format!("V\t{verse}\t1\tEn el principio creó Dios los cielos y la tierra, y la tierra estaba desordenada y vacía.\n"));
        }
    }
    std::fs::write(folder.join("GEN.txt"), text).unwrap();
    let mut state = sample_state();
    state.bible = crate::bible_state::BibleUiState::with_root(&root);
    state.bible.open_at(crate::bible_reader::Position {
        book: 0,
        chapter,
        page: 0,
    });
    state.router.navigate_to(route);
    state
}

/// The same reading view at the Large body size, so the fit is seen at its limit.
fn bible_preview_state_large() -> AppState {
    let mut state = bible_preview_state(ScreenRoute::BibleReading, 1);
    state.reader.preferences.font_size = crate::reader::BookFontSize::Large;
    state
}

/// The Bible picker previews: the books list and Psalms' chapter grid.
fn render_sample_bible(with_chapters: bool, preferences: DisplayPreferences) -> FrameBuffer {
    let books = crate::bible::parse_index(&crate::bible_nav::sample_books_txt()).unwrap();
    let mut nav = BibleNav::new(books);
    if with_chapters {
        nav.next_section_cyclic();
        nav.next_section_cyclic();
        nav.move_book(1);
        nav.open_chapters();
    }
    let mut frame = FrameBuffer::new_white();
    let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
    crate::app::screens::bible::render_bible_nav(&mut display, preferences, "RVR1960", &nav)
        .unwrap();
    drop(display);
    frame
}

/// A Sudoku game one third in, as the hub's Continue card reads it.
fn sample_sudoku_save() -> crate::games::sudoku_save::SudokuSave {
    let puzzle =
        "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    let grid = |text: &str| {
        let mut out = [0_u8; 81];
        for (index, byte) in text.bytes().enumerate() {
            out[index] = byte - b'0';
        }
        out
    };
    crate::games::sudoku_save::SudokuSave {
        difficulty: crate::games::sudoku_puzzles::SudokuDifficulty::Medium,
        puzzle: grid(puzzle),
        board: grid(puzzle),
        seconds: 761,
    }
}

/// Notes oldest first, as the catalog scans them, so the AI hub's recent
/// list reads newest first.
/// Four notes, one in each AI state, so the labels show side by side.
fn sample_ai_state_notes() -> Vec<crate::voice_notes::VoiceNoteEntry> {
    use crate::voice_note_record::NoteState;
    let mut notes = sample_voice_notes();
    notes[0].ai_state = Some(NoteState::Queued);
    notes[1].ai_state = Some(NoteState::Summarizing);
    notes[2].ai_state = Some(NoteState::Done);
    notes.push(crate::voice_notes::VoiceNoteEntry {
        file_name: "NOTE_004.WAV".into(),
        title: "Voice note 004".into(),
        recorded_at: "2026-10-03  18:20:01".into(),
        wav_bytes: 3_400_000,
        pcm_bytes: 1_900_000,
        duration_seconds: 95,
        ai_state: Some(NoteState::Failed),
    });
    notes
}

fn sample_voice_notes() -> Vec<crate::voice_notes::VoiceNoteEntry> {
    let note =
        |name: &str, title: &str, stamp: &str, seconds: u32| crate::voice_notes::VoiceNoteEntry {
            file_name: name.into(),
            title: title.into(),
            recorded_at: stamp.into(),
            wav_bytes: 3_400_000,
            pcm_bytes: 1_900_000,
            duration_seconds: seconds,
            ai_state: None,
        };
    vec![
        note(
            "NOTE_001.WAV",
            "Idea: reading club",
            "2026-09-30  09:15:12",
            180,
        ),
        note("NOTE_002.WAV", "Call with Ana", "2026-10-01  07:30:45", 420),
        note(
            "NOTE_003.WAV",
            "Team meeting: Q4 budget",
            "2026-10-02  12:04:33",
            1_080,
        ),
    ]
}

/// Four SD games so the hub preview looks like a card in use.
fn sample_games_catalog(state: &mut AppState) {
    let manifest = |id: &str, name: &str| crate::lua_runtime::manifest::LuaAppManifest {
        id: id.into(),
        name: name.into(),
        kind: crate::lua_runtime::manifest::LuaAppKind::Game,
        entry: "MAIN.LUA".into(),
        version: "1.0".into(),
        input: vec![],
    };
    for (id, name) in [
        ("hgrid", "Hello Grid"),
        ("mines", "Minesweeper"),
        ("sudoku", "Sudoku"),
        ("tetris", "Tetris"),
    ] {
        state
            .lua_runtime
            .catalog
            .entries
            .push(crate::lua_runtime::manifest::LuaAppEntry {
                directory_name: id.to_ascii_uppercase(),
                directory: std::path::PathBuf::from("/sdcard/RUSTMIX/APPS"),
                manifest: manifest(id, name),
            });
    }
    state.lua_runtime.catalog.warning = None;
}

/// A session for any `tetris.init` line, loaded through the device bridge.
fn tetris_session(source: &str) -> (NativeGameCanvas, LuaEventBridge) {
    let mut canvas = NativeGameCanvas::default();
    let event_bridge = LuaEventBridge::load(source, &mut canvas).unwrap();
    (canvas, event_bridge)
}

/// The Tetris start list as the device opens it: `tetris.init()`, no moves.
fn tetris_start_session() -> LuaAppSession {
    let source = "tetris.init()";
    let (canvas, event_bridge) = tetris_session(source);
    tetris_app_session(source, canvas, event_bridge)
}

/// A Classic game a few drops in: slow gravity, the same moves as Zen.
fn tetris_classic_session() -> LuaAppSession {
    let source = "tetris.init('classic', 1803)";
    let (mut canvas, mut event_bridge) = tetris_session(source);
    play_sample_drops(&mut event_bridge, &mut canvas);
    tetris_app_session(source, canvas, event_bridge)
}

/// A Zen Tetris game a few drops in, drawn through the native SD game canvas.
fn tetris_sample_session() -> LuaAppSession {
    let source = "tetris.init('zen', 1803)";
    let (mut canvas, mut event_bridge) = tetris_session(source);
    play_sample_drops(&mut event_bridge, &mut canvas);
    tetris_app_session(source, canvas, event_bridge)
}

/// Four drops with a sideways move each, the same keys the Zen preview uses.
fn play_sample_drops(event_bridge: &mut LuaEventBridge, canvas: &mut NativeGameCanvas) {
    for _ in 0..4 {
        event_bridge
            .apply_button(ButtonEvent::Up, 1_000, canvas)
            .unwrap();
        event_bridge
            .apply_button(ButtonEvent::Up, 1_000, canvas)
            .unwrap();
        event_bridge.apply_boot_short_press(4_000, canvas).unwrap();
        event_bridge
            .apply_button(ButtonEvent::Down, 2_000, canvas)
            .unwrap();
        event_bridge
            .apply_button(ButtonEvent::Down, 2_000, canvas)
            .unwrap();
        event_bridge.apply_boot_short_press(4_000, canvas).unwrap();
    }
}

/// Wrap a loaded Tetris bridge in the session the screens draw from.
fn tetris_app_session(
    source: &str,
    canvas: NativeGameCanvas,
    event_bridge: LuaEventBridge,
) -> LuaAppSession {
    LuaAppSession {
        entry: LuaAppEntry {
            directory_name: "TETRIS".into(),
            directory: std::path::PathBuf::from("/sdcard/RUSTMIX/APPS/TETRIS"),
            manifest: LuaAppManifest {
                id: "tetris".into(),
                name: "Tetris".into(),
                kind: LuaAppKind::Game,
                entry: "MAIN.LUA".into(),
                version: "1.0".into(),
                input: vec![],
            },
        },
        source_bytes: source.len(),
        canvas,
        refresh_plan: GameRefreshPlan::PartialFullscreen { regions: vec![] },
        event_bridge,
    }
}

/// The start list as the card opens it, with a saved game to continue.
fn sudoku_start_session() -> LuaAppSession {
    let mut canvas = NativeGameCanvas::default();
    let mut event_bridge = LuaEventBridge::load("sudoku.init()", &mut canvas).unwrap();
    if let LuaEventBridge::Sudoku(game) = &mut event_bridge {
        game.prepare(Some(sample_sudoku_save()), GameRecords::default());
        game.render_initial(&mut canvas).unwrap();
    }
    sudoku_session(canvas, event_bridge)
}

/// A Medium game seven minutes in, `selects` presses into the three-step
/// entry.
fn sudoku_sample_session(selects: usize) -> LuaAppSession {
    let generated = generate(SudokuDifficulty::Medium, 1803);
    let mut board = generated.puzzle;
    // Every fifth open cell already holds the player's answer.
    let open: Vec<usize> = (0..81).filter(|&index| board[index] == 0).collect();
    for &index in open.iter().step_by(5) {
        board[index] = generated.solution[index];
    }
    let givens = generated.puzzle.map(|cell| cell != 0);
    let difficulty = Some(SudokuDifficulty::Medium);
    let game = SudokuGame::new(board, givens, generated.puzzle, difficulty, 461);
    let mut canvas = NativeGameCanvas::default();
    game.render_initial(&mut canvas).unwrap();
    let mut event_bridge = LuaEventBridge::Sudoku(game);
    let selects = std::iter::repeat(ButtonEvent::Select).take(selects);
    for event in std::iter::once(ButtonEvent::Down).chain(selects) {
        event_bridge
            .apply_button(event, 2_000, &mut canvas)
            .unwrap();
    }
    sudoku_session(canvas, event_bridge)
}

fn sudoku_session(canvas: NativeGameCanvas, event_bridge: LuaEventBridge) -> LuaAppSession {
    LuaAppSession {
        entry: LuaAppEntry {
            directory_name: "SUDOKU".into(),
            directory: std::path::PathBuf::from("/sdcard/RUSTMIX/APPS/SUDOKU"),
            manifest: LuaAppManifest {
                id: "sudoku".into(),
                name: "Sudoku".into(),
                kind: LuaAppKind::Game,
                entry: "MAIN.LUA".into(),
                version: "1.0".into(),
                input: vec![],
            },
        },
        source_bytes: "sudoku.init()".len(),
        canvas,
        refresh_plan: GameRefreshPlan::PartialFullscreen { regions: vec![] },
        event_bridge,
    }
}

/// Seven photos: four prepared (two starred), one waiting, one unreadable.
fn open_sample_gallery(state: &mut AppState) {
    let root = std::env::temp_dir().join(format!("wave-preview-photos-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let photos = root.join("PHOTOS");
    fs::create_dir_all(&photos).unwrap();
    for variant in 0..6 {
        let jpeg = jpeg_from_grey(&scene(300, 400, variant), 300, 400);
        let name = format!("IMG_04{:02}.jpg", 12 - variant);
        fs::write(photos.join(name), jpeg).unwrap();
    }
    fs::write(photos.join("IMG_0399.jpg"), b"not a photo").unwrap();
    state.photos = PhotosUiState::with_roots(&photos, root.join("CACHE"));
    state.photos.refresh();
    for job in state.photos.cache_jobs().into_iter().take(5) {
        let result = prepare(&photos, state.photos.cache_directory(), &job);
        state.photos.on_job_result(&result);
    }
    state.photos.starred.toggle("IMG_0407.jpg");
    state.photos.starred.toggle("IMG_0409.jpg");
    state.photos.selected = 1;
    state.router.navigate_to(ScreenRoute::Photos);
}

/// A grey landscape: sky gradient, sun and a hill line that varies.
fn scene(width: usize, height: usize, variant: usize) -> Vec<u8> {
    let (sun_x, sun_y) = (width * (1 + variant % 4) / 5, height / 4);
    let radius = width / 9;
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let phase = x as f32 / width as f32 * (3.0 + variant as f32);
            let ridge = (height as f32 * (0.5 + phase.sin() / 10.0)) as usize;
            let sun = x.abs_diff(sun_x).pow(2) + y.abs_diff(sun_y).pow(2) < radius.pow(2);
            let level = if y > ridge {
                40 + (y - ridge) * 60 / height
            } else if sun {
                250
            } else {
                110 + y * 120 / height
            };
            pixels.push(level as u8);
        }
    }
    pixels
}

/// A day of use with an afternoon charge, sampled every 15 minutes.
fn record_sample_battery_day(state: &mut AppState) {
    let now = state.board.rtc.unwrap().epoch_minutes();
    for step in (0..96_u32).rev() {
        let percent = match step {
            32.. => 95 - (96 - step) * 25 / 64,
            24..=31 => 70 + (32 - step) * 30 / 8,
            _ => 78 + step * 22 / 24,
        };
        state.battery_log.record(now - step * 15, percent as u8);
    }
}

/// Open a Spanish TXT sample on its first page, showing word wrapping,
/// hyphenation and justification.
fn open_sample_book(state: &mut AppState) {
    let root = std::env::temp_dir().join(format!("wave-preview-{}", std::process::id()));
    let books = root.join("BOOKS");
    let reader_state = root.join("READER");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&books).unwrap();
    fs::create_dir_all(&reader_state).unwrap();
    fs::write(books.join("Quijote.txt"), QUIJOTE.repeat(3)).unwrap();
    let mut reader = ReaderUiState::with_roots(
        books.to_string_lossy().into_owned(),
        reader_state.to_string_lossy().into_owned(),
    );
    reader.refresh_library();
    reader.library_selected = 1;
    assert!(reader.apply_library_button(ButtonEvent::Select));
    assert_eq!(reader.tick(), ReaderTickOutcome::FirstPageReady);
    state.reader = reader;
    state.router.navigate_to(ScreenRoute::ReaderPage);
}

/// Plausible data so previews resemble a device in use.
fn sample_state() -> AppState {
    let mut state = AppState::default();
    state.board = BoardSnapshot {
        rtc: Some(RtcDateTime {
            year: 2026,
            month: 10,
            day: 2,
            weekday: 5,
            hour: 13,
            minute: 42,
            second: 0,
        }),
        power: Some(PowerSnapshot {
            battery_percent: Some(78),
            ..PowerSnapshot::default()
        }),
        ..BoardSnapshot::default()
    };
    state.network.wifi_state = WifiConnectionState::Connected;
    let config = WeatherConfig::parse(SAMPLE_CONFIG).unwrap();
    state.update_weather_snapshot(WeatherSnapshot::provisioned(&config));
    state.set_weather_config(Some(config));
    let forecast = parse_open_meteo_response(SAMPLE_RESPONSE).unwrap();
    state.weather.record_success(forecast);
    state.reader.resume = Some(ReaderLocation {
        path: "/sdcard/RUSTMIX/BOOKS/Cien años de soledad.txt".into(),
        title: "Cien años de soledad".into(),
        format: BookFormat::Text,
        size_bytes: 2_000_000,
        modified_seconds: 0,
        page_index: 41,
        byte_offset: 240_000,
        epub_chapter: None,
    });
    // Reading history: 25 min today closing a 5-day streak.
    let today = crate::civil_date::days_from_civil(2026, 10, 2) as u32;
    for offset in 1..=4 {
        state
            .reading_stats
            .record(today - offset, 30 * 60, 40, None);
    }
    state.reading_stats.record(today, 25 * 60, 22, None);
    state
}

/// Encode the logical (rotated) screen as a 1-bit grayscale PNG.
fn encode_png(frame: &FrameBuffer, orientation: DisplayOrientation) -> Vec<u8> {
    let size = orientation.logical_size();
    let (width, height) = (size.width as usize, size.height as usize);
    let row_bytes = width.div_ceil(8);
    let mut raw = Vec::with_capacity((row_bytes + 1) * height);
    for y in 0..height {
        raw.push(0); // PNG filter type: none
        let mut row = vec![0xFF_u8; row_bytes];
        for x in 0..width {
            let logical = Point::new(x as i32, y as i32);
            let native = orientation.map_logical_to_native(logical);
            if native.and_then(|point| frame.is_black(point)) == Some(true) {
                row[x / 8] &= !(0x80 >> (x % 8));
            }
        }
        raw.extend_from_slice(&row);
    }

    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    // Bit depth 1, grayscale, default compression, filter and interlace.
    header.extend_from_slice(&[1, 0, 0, 0, 0]);

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    write_chunk(&mut png, b"IHDR", &header);
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
    write_chunk(&mut png, b"IDAT", &compressed);
    write_chunk(&mut png, b"IEND", &[]);
    png
}

fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn crc32_matches_reference_value() {
    assert_eq!(crc32(b"IEND"), 0xAE42_6082);
}
