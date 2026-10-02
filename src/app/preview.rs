//! Host-only screen previews for reviewing layouts without the board.
//!
//! `WAVE_PREVIEW_DIR=/tmp/wave cargo +stable test --target <host> --lib preview`
//! writes one PNG per screen. Without the variable the screens are still
//! rendered, so a layout that panics fails the test.

use std::{fs, path::Path};

use embedded_graphics::prelude::Point;

use super::{
    display::UiFontSize, reader_typography::reader_body_style, render_current_screen,
    typography::Text, AppState, ScreenRoute,
};
use crate::{
    board_services::BoardSnapshot,
    buttons::ButtonEvent,
    framebuffer::FrameBuffer,
    network::WifiConnectionState,
    orientation::{DisplayOrientation, OrientedFrameBuffer},
    power::PowerSnapshot,
    reader::{
        BookFont, BookFontSize, BookFormat, ReaderLocation, ReaderTickOutcome, ReaderUiState,
        ReadingTheme,
    },
    regional::TemperatureUnit,
    rtc::RtcDateTime,
    weather::{CurrentConditions, WeatherFetchState},
};


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
const SPANISH_SAMPLE: [&str; 2] = [
    "¿Dónde está el niño? ¡Ahí, señor!",
    "«Cien años» — “sí”, ‘no’… 18°C · Ñandú",
];

#[test]
fn render_screen_previews() {
    let output = std::env::var_os("WAVE_PREVIEW_DIR");
    if let Some(directory) = output.as_deref() {
        fs::create_dir_all(directory).unwrap();
    }
    let mut images = vec![("reader-fonts", render_reader_font_sheet())];
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

    let routes = [
        ("library", ScreenRoute::Reader),

    let mut page = sample_state();
    open_sample_book(&mut page);
    states.push(("reader-page", page));
    states
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
    state.router.navigate_to(ScreenRoute::ReaderPage);ai", ScreenRoute::Ai),
        ("games", ScreenRoute::Games),
        ("tools", ScreenRoute::Tools),
        ("settings", ScreenRoute::Settings),
        ("photos", ScreenRoute::Photos),
        ("weather", ScreenRoute::Weather),
        ("voice-notes", ScreenRoute::VoiceNotes),
    ];
    for (name, route) in routes {
        let mut state = sample_state();
        state.router.navigate_to(route);
        states.push((name, state));
    }
    states
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
    state.regional.temperature_unit = TemperatureUnit::Celsius;
    state.weather.state = WeatherFetchState::Ready;
    state.weather.current = Some(CurrentConditions {
        observed_at: "2026-10-02T13:30".into(),
        weather_code: 2,
        temperature_tenths_f: 644,
        apparent_temperature_tenths_f: 640,
        humidity_percent: 61,
        wind_speed_tenths_mph: 86,
    });
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
