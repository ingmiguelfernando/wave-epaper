//! SD-backed voice notes list, details, title editor and recording screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        screens::{
            ai::split_stamp,
            games::draw_info_box,
            home::{draw_battery, BATTERY_HEIGHT, BATTERY_WIDTH},
        },
        state::AppState,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{
                draw_bottom_bar, KeyCap, BACK_HINTS, KEYBOARD_HINTS, OPEN_HINTS, RUN_HINTS,
            },
            chips::{draw_chip_row, Chip},
            display_numerals::{draw_numerals, numerals_width, Numerals},
            header::draw_header,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    regional::grouped,
    voice_note_metadata::format_storage_bytes,
    voice_notes::{
        ai_action_label, bytes_per_second, format_duration, VoiceNotesMode, DETAIL_AI,
        DETAIL_DELETE, DETAIL_EXPORT, DETAIL_PLAY, DETAIL_RETURN, DETAIL_TITLE,
        VOICE_TITLE_EDITOR_KEY_ROWS,
    },
};

/// The recording screen's layout, from the mockup's `Voice Notes: grabando`.
const TITLE_BASELINE: i32 = 76;
const TIMER_BASELINE: i32 = 236;
const METER_TOP: i32 = 262;
const METER_HEIGHT: i32 = 130;
const METER_LEFT: i32 = 58;
const BAR_WIDTH: i32 = 10;
const BAR_GAP: i32 = 5;
const CHIPS_TOP: i32 = 412;
const INFO_BOX_TOP: i32 = 470;

const DELETE_HINTS: [(KeyCap, &str); 2] = [(KeyCap::UpDown, "move"), (KeyCap::Select, "confirm")];

/// The recording screen's two keys: ● stops, BOOT pauses or resumes.
fn recording_hints(goes_to_ai: bool, paused: bool) -> [(KeyCap, &'static str); 2] {
    [
        (
            KeyCap::Select,
            if goes_to_ai {
                "stop & process"
            } else {
                "stop & save"
            },
        ),
        (KeyCap::Boot, if paused { "resume" } else { "pause" }),
    ]
}

const PLAYING_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "volume"),
    (KeyCap::Select, "stop"),
    (KeyCap::Boot, "hold: back"),
];

pub fn render_voice_notes(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let voice = &state.voice_notes;
    let note_count = format!("{} NOTES", voice.notes.len());
    let free = format_storage_bytes(voice.available_storage_bytes);
    draw_header(
        display,
        state.display,
        "VOICE NOTES",
        "SD PCM WAV RECORDINGS",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: voice.mode.label(),
            middle: &note_count,
            right: &free,
        },
    )?;
    let visible = voice.visible_note_range();
    let recordings = if voice.notes.is_empty() {
        "Recordings".into()
    } else {
        format!(
            "Recordings {}-{} of {}",
            visible.start + 1,
            visible.end,
            voice.notes.len()
        )
    };
    Text::new(&recordings, Point::new(22, 158), heading).draw(display)?;
    draw_action(display, 198, "Record new note", voice.selected == 0, body)?;
    let gain = format!(
        "Microphone gain: {} ({})",
        voice.mic_gain.label(),
        voice.mic_gain.db_label()
    );
    draw_action(display, 254, &gain, voice.selected == 1, body)?;
    for (visible_index, note_index) in visible.enumerate() {
        let note = &voice.notes[note_index];
        let row = note_index + 2;
        let duration = format_duration(note.duration_seconds);
        // A note with an AI record shows its state between title and duration.
        let online = state.network.wifi_state == crate::network::WifiConnectionState::Connected;
        let label = match note
            .ai_state
            .and_then(|ai| crate::voice_note_record::state_label(ai, online))
        {
            Some(state_text) => format!("{}   {}   {}", note.title, state_text, duration),
            None => format!("{}   {}", note.title, duration),
        };
        draw_action(
            display,
            308 + visible_index as i32 * 54,
            &label,
            voice.selected == row,
            body,
        )?;
    }
    if let Some(error) = voice.error.as_deref() {
        Text::new(error, Point::new(22, 686), state.display.detail_style()).draw(display)?;
    }
    draw_bottom_bar(display, state.display, &OPEN_HINTS)?;
    Ok(())
}

pub fn render_voice_note_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let metadata = state.display.body_style();
    if state.voice_notes.title_editing {
        return render_voice_note_title_editor(display, state);
    }
    draw_header(display, state.display, "VOICE NOTE", "SAVED WAV DETAILS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: state.voice_notes.mode.label(),
            middle: "MONO",
            right: "16 KHZ",
        },
    )?;
    let Some(note) = state.voice_notes.selected_note() else {
        Text::new("No saved note selected.", Point::new(22, 232), detail).draw(display)?;
        draw_bottom_bar(display, state.display, &BACK_HINTS)?;
        return Ok(());
    };
    if state.voice_notes.delete_confirmation {
        return render_voice_note_delete_confirmation(display, state);
    }

    Text::new(
        &note.title,
        Point::new(22, 158),
        state.display.heading_style(),
    )
    .draw(display)?;
    line(display, 204, "File", &note.file_name, metadata)?;
    line(display, 240, "Recorded", &note.recorded_at, metadata)?;
    line(
        display,
        276,
        "Playback",
        &format!(
            "{} / {}",
            format_duration(state.voice_notes.playback_elapsed_seconds()),
            format_duration(note.duration_seconds)
        ),
        metadata,
    )?;
    let volume = format!("{}%", state.audio.volume_percent);
    line(display, 312, "Volume", &volume, metadata)?;
    let playing = state.voice_notes.is_playing_selected();
    let play = if playing {
        "Stop playback"
    } else {
        "Play note"
    };
    let actions = [
        (DETAIL_PLAY, play),
        (DETAIL_AI, ai_action_label(note.ai_state)),
        (DETAIL_TITLE, "Edit friendly title"),
        (DETAIL_EXPORT, "Export / download"),
        (DETAIL_DELETE, "Delete note"),
        (DETAIL_RETURN, "Return to Voice Notes"),
    ];
    for (index, label) in actions {
        let top = 350 + index as i32 * 50;
        let selected = state.voice_notes.detail_selected == index;
        draw_action(display, top, label, selected, body)?;
    }
    if state.voice_notes.export_file.as_deref() == Some(note.file_name.as_str()) {
        line(
            display,
            672,
            "LAN",
            state.wifi_transfer.url_label(),
            metadata,
        )?;
        line(
            display,
            700,
            "Code",
            state.wifi_transfer.code_label(),
            metadata,
        )?;
        line(
            display,
            728,
            "Path",
            &format!("VOICE/{}", note.file_name),
            metadata,
        )?;
    } else if let Some(error) = state.voice_notes.error.as_deref() {
        Text::new(error, Point::new(22, 690), detail).draw(display)?;
    }
    let hints: &[(KeyCap, &str)] = if playing { &PLAYING_HINTS } else { &RUN_HINTS };
    draw_bottom_bar(display, state.display, hints)?;
    Ok(())
}

fn render_voice_note_title_editor(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let voice = &state.voice_notes;
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let title = voice.title_edit_buffer.iter().collect::<String>();
    let file = voice
        .selected_note()
        .map_or("VOICE---.WAV", |note| note.file_name.as_str());
    draw_header(
        display,
        state.display,
        "VOICE NOTE TITLE",
        "EDIT FRIENDLY TITLE",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "TITLE",
            middle: file,
            right: voice.title_editor_navigation_mode_label(),
        },
    )?;
    Text::new("Friendly title", Point::new(22, 170), body).draw(display)?;
    Rectangle::new(Point::new(22, 184), Size::new(436, 54))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    let title_label = if title.is_empty() {
        "_"
    } else {
        title.as_str()
    };
    Text::new(title_label, Point::new(34, 218), body).draw(display)?;
    Text::new(
        "Internal WAV filename remains unchanged.",
        Point::new(22, 276),
        detail,
    )
    .draw(display)?;
    draw_voice_title_keyboard(display, state)?;
    draw_bottom_bar(display, state.display, &KEYBOARD_HINTS)?;
    Ok(())
}

fn draw_voice_title_keyboard(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let voice = &state.voice_notes;
    for (row_index, row) in VOICE_TITLE_EDITOR_KEY_ROWS.iter().enumerate() {
        for (column_index, label) in row.iter().enumerate() {
            let index = row_index * 7 + column_index;
            let left = 22 + column_index as i32 * 62;
            let top = 314 + row_index as i32 * 54;
            let selected = voice.title_editor_selected_key_index() == index;
            Rectangle::new(Point::new(left, top), Size::new(58, 46))
                .into_styled(PrimitiveStyle::with_stroke(
                    BinaryColor::On,
                    if selected { 3 } else { 1 },
                ))
                .draw(display)?;
            Text::new(
                label,
                Point::new(left + if label.len() > 3 { 4 } else { 15 }, top + 29),
                state.display.detail_style(),
            )
            .draw(display)?;
        }
    }
    Ok(())
}

fn render_voice_note_delete_confirmation(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(note) = state.voice_notes.selected_note() else {
        return Ok(());
    };
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    Text::new(
        "DELETE VOICE NOTE?",
        Point::new(22, 198),
        state.display.heading_style(),
    )
    .draw(display)?;
    Text::new(&note.title, Point::new(22, 264), body).draw(display)?;
    Text::new(&note.file_name, Point::new(22, 310), detail).draw(display)?;
    Text::new(
        "This permanently removes the WAV file.",
        Point::new(22, 370),
        detail,
    )
    .draw(display)?;
    draw_action(
        display,
        448,
        "Cancel",
        state.voice_notes.delete_confirm_selected == 0,
        body,
    )?;
    draw_action(
        display,
        516,
        "Delete permanently",
        state.voice_notes.delete_confirm_selected == 1,
        body,
    )?;
    draw_bottom_bar(display, state.display, &DELETE_HINTS)?;
    Ok(())
}

const RESULT_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "scroll"),
    (KeyCap::Boot, "tab"),
    (KeyCap::Select, "actions"),
];

/// The active tab's text, cleaned for the device fonts and wrapped to width.
fn result_body_lines(
    state: &AppState,
    record: &crate::voice_note_record::NoteRecord,
    audio: &str,
    width: i32,
) -> Vec<String> {
    use crate::voice_note_result::{prepare_text, ResultTab};
    let text = match state.voice_notes_result.tab {
        ResultTab::Summary => record.summary.as_str(),
        ResultTab::Transcript => record.transcript.as_str(),
        ResultTab::Audio => audio,
    };
    let body = state.display.body_style();
    let mut lines = Vec::new();
    // A note not summarized yet says where it is, above what it already has.
    let status = match state.voice_notes_result.tab {
        ResultTab::Summary => crate::voice_note_result::status_text(record),
        _ => None,
    };
    let text = match status {
        Some(status) if text.trim().is_empty() => status,
        Some(status) => format!("{status}\n\n{text}"),
        None => text.to_string(),
    };
    for paragraph in prepare_text(&text).split('\n') {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(body.wrap(paragraph, width));
        }
    }
    lines
}

pub fn render_voice_note_result(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    use crate::voice_note_result::ResultTab;
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    draw_header(display, state.display, "VOICE NOTE", "AI RESULT")?;
    let Some(note) = state.voice_notes.selected_note() else {
        Text::new("No saved note selected.", Point::new(22, 232), detail).draw(display)?;
        draw_bottom_bar(display, state.display, &BACK_HINTS)?;
        return Ok(());
    };
    let heading = state.display.heading_style();
    // The summarized title when the record has one, as the mockup shows.
    let record = state.voice_notes_result.record.as_ref();
    let title = record
        .map(|record| record.title.as_str())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(note.title.as_str());
    Text::new(&heading.fit(title, 436), Point::new(22, 158), heading).draw(display)?;
    // Stamp, language, then the providers: transcriber → summarizer.
    let (date, time) = crate::app::screens::ai::split_stamp(&note.recorded_at);
    let when = match (date, time) {
        (Some(date), Some(time)) => format!("{date} · {time}"),
        (Some(date), None) => date,
        _ => note.recorded_at.clone(),
    };
    let meta = match record {
        Some(record) => crate::voice_note_result::meta_line(&when, record),
        None => format!("{} · {}", when, format_duration(note.duration_seconds)),
    };
    let meta = crate::voice_note_result::prepare_text(&meta);
    // The provider line may need two rows; both show in full, as the mockup has them.
    for (row, line) in detail.wrap(&meta, 436).iter().take(2).enumerate() {
        Text::new(line, Point::new(22, 186 + row as i32 * 20), detail).draw(display)?;
    }

    // Tabs: the active one is filled, the others outlined.
    let tabs = [ResultTab::Summary, ResultTab::Transcript, ResultTab::Audio];
    let mut tab_left = 22;
    for tab in tabs {
        let active = tab == state.voice_notes_result.tab;
        let tab_style = if active {
            PrimitiveStyle::with_fill(BinaryColor::On)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 2)
        };
        let tab_width = body.text_width(tab.label()) + 20;
        Rectangle::new(Point::new(tab_left, 230), Size::new(tab_width as u32, 30))
            .into_styled(tab_style)
            .draw(display)?;
        let text_style = state.display.text_style(
            crate::app::typography::UiTextRole::Body,
            if active {
                BinaryColor::Off
            } else {
                BinaryColor::On
            },
        );
        Text::new(tab.label(), Point::new(tab_left + 10, 250), text_style).draw(display)?;
        tab_left += tab_width + 8;
    }

    // Body: wrapped lines from the scroll offset, between the tabs and the footer.
    let body_top = 280;
    let footer_top = 700;
    let line_step = i32::from(body.line_height()) + 4;
    let visible = ((footer_top - body_top) / line_step).max(1) as usize;
    match state.voice_notes_result.record.as_ref() {
        Some(record) => {
            let audio = crate::voice_note_result::audio_text(
                &note.file_name,
                note.duration_seconds,
                note.pcm_bytes,
            );
            let lines = result_body_lines(state, &record, &audio, 436);
            // Clamp to the last page here: input has no line count, so it scrolls freely.
            let last_start = lines.len().saturating_sub(visible);
            let scroll = state.voice_notes_result.scroll.min(last_start);
            for (row, line) in lines.iter().skip(scroll).take(visible).enumerate() {
                Text::new(
                    line,
                    Point::new(22, body_top + row as i32 * line_step + 16),
                    body,
                )
                .draw(display)?;
            }
        }
        None => {
            Text::new("No AI result yet.", Point::new(22, body_top + 16), body).draw(display)?;
        }
    }

    Text::new(
        "AI-generated · may contain mistakes · audio kept on SD",
        Point::new(22, footer_top + 16),
        detail,
    )
    .draw(display)?;
    draw_bottom_bar(display, state.display, &RESULT_HINTS)?;
    Ok(())
}

/// The recording screen after the mockup's `Voice Notes: grabando`: the time
/// large, a level meter and what the note costs on the card.
pub fn render_voice_note_recording(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let voice = &state.voice_notes;
    let recording = matches!(
        voice.mode,
        VoiceNotesMode::Recording | VoiceNotesMode::Paused
    );
    draw_recording_status_bar(display, state)?;
    draw_recording_title(display, state)?;
    if !recording {
        // Saved, Error and Idle keep their message in the info box.
        let message = match voice.mode {
            VoiceNotesMode::Saved => "Saved as recovery-safe WAV.".to_string(),
            VoiceNotesMode::Error => voice
                .error
                .clone()
                .unwrap_or_else(|| "Recording failed".into()),
            _ => "Preparing microphone capture...".to_string(),
        };
        draw_info_box(display, state.display, &message, 150)?;
        let hints = recording_hints(state.voice_note_goes_to_ai(), voice.recording_paused);
        draw_bottom_bar(display, state.display, &hints)?;
        return Ok(());
    }

    // The elapsed time, centred in the sleep clock's numerals.
    let timer = elapsed_label(voice.elapsed_seconds);
    let numerals = if numerals_width(&timer, Numerals::Clock) > 448 {
        Numerals::Temperature
    } else {
        Numerals::Clock
    };
    draw_numerals(
        display,
        &timer,
        numerals,
        (16, 464),
        TIMER_BASELINE,
        BinaryColor::On,
    )?;

    draw_level_meter(display, &voice.level_bars)?;

    // The chips: the active one filled.
    let detail = state.display.detail_style();
    let chips = [
        Chip::dotted("RECORDING", voice.mode == VoiceNotesMode::Recording),
        Chip::new("PAUSED", voice.mode == VoiceNotesMode::Paused),
    ];
    draw_chip_row(display, 16, CHIPS_TOP, 6, detail, &chips)?;

    let lines = recording_info_lines(voice);
    draw_info_box(display, state.display, &lines.join("\n"), INFO_BOX_TOP)?;
    let hints = recording_hints(state.voice_note_goes_to_ai(), voice.recording_paused);
    draw_bottom_bar(display, state.display, &hints)
}

/// `● REC · Voice Notes` with the battery at the right, `PAUSED · Voice
/// Notes` while paused, as the XiaoZhi call screen draws its status.
fn draw_recording_status_bar(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    draw_status_bar(display, preferences, "")?;
    let voice = &state.voice_notes;
    let recording = voice.mode == VoiceNotesMode::Recording;
    let mut left = 16;
    if recording {
        Circle::with_center(Point::new(22, STATUS_BAR_HEIGHT / 2), 12)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(display)?;
        left = 36;
    }
    let style = preferences.text_style(UiTextRole::Heading, BinaryColor::Off);
    let baseline = (STATUS_BAR_HEIGHT + style.cap_height()) / 2;
    let title = if recording {
        "REC · Voice Notes"
    } else {
        "PAUSED · Voice Notes"
    };
    Text::new(title, Point::new(left, baseline), style).draw(display)?;
    if let Some(percent) = state.board.power.and_then(|power| power.battery_percent) {
        let text = format!("{percent}%");
        let right = draw_status_text(display, preferences, &text, STATUS_BAR_RIGHT)?;
        let top = (STATUS_BAR_HEIGHT - BATTERY_HEIGHT) / 2;
        draw_battery(display, Point::new(right - 4 - BATTERY_WIDTH, top), percent)?;
    }
    Ok(())
}

/// `New note · Oct 2, 12:04` centred in Heading, and under it in Detail what
/// the note becomes: the AI's title, or the file it is saved as.
fn draw_recording_title(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let voice = &state.voice_notes;
    let heading = state.display.heading_style();
    let detail = state.display.detail_style();
    let (date, time) = voice
        .active_recorded_at
        .as_deref()
        .map(split_stamp)
        .unwrap_or((None, None));
    let title = match (date, time) {
        (Some(date), Some(time)) => format!("New note · {date}, {time}"),
        _ => "New note".to_string(),
    };
    let left = 240 - heading.text_width(&title) / 2;
    Text::new(&title, Point::new(left, TITLE_BASELINE), heading).draw(display)?;
    let note = if state.voice_note_goes_to_ai() {
        "The title is generated from the summary".to_string()
    } else {
        format!(
            "Saved as {}",
            voice.active_file.as_deref().unwrap_or("VOICE---.WAV")
        )
    };
    let left = 240 - detail.text_width(&note) / 2;
    Text::new(&note, Point::new(left, TITLE_BASELINE + 26), detail).draw(display)?;
    Ok(())
}

/// The level meter: 30 bars, one per second of audio, the newest at the
/// right, on a log scale so speech fills the band and silence is the
/// shortest bar. The band is centred between two rules.
fn draw_level_meter(display: &mut OrientedFrameBuffer<'_>, bars: &[u16]) -> Result<(), Infallible> {
    let ink = PrimitiveStyle::with_fill(BinaryColor::On);
    for y in [METER_TOP, METER_TOP + METER_HEIGHT - 2] {
        Rectangle::new(Point::new(16, y), Size::new(448, 2))
            .into_styled(ink)
            .draw(display)?;
    }
    let center = METER_TOP + METER_HEIGHT / 2;
    for index in 0..30_usize {
        let peak = bars.get(index).copied().unwrap_or(0);
        let height = meter_bar_height(peak);
        let x = METER_LEFT + index as i32 * (BAR_WIDTH + BAR_GAP);
        Rectangle::new(
            Point::new(x, center - height / 2),
            Size::new(BAR_WIDTH as u32, height as u32),
        )
        .into_styled(ink)
        .draw(display)?;
    }
    Ok(())
}

/// A bar's height: 10 px for silence, 110 px for a full-scale peak, in 6 dB
/// steps (`16 - peak.leading_zeros()`).
#[must_use]
pub fn meter_bar_height(peak: u16) -> i32 {
    let level = 16 - i32::from(peak.leading_zeros() as u8);
    10 + level * 100 / 16
}

/// `03:27`, or `1:03:27` past an hour.
#[must_use]
pub fn elapsed_label(seconds: u32) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            (seconds / 60) % 60,
            seconds % 60
        )
    } else {
        format_duration(seconds)
    }
}

/// What the note costs on the card: its size so far, the free space and the
/// WAV hours it still takes, and the radio state. Without the free space,
/// the first line only.
#[must_use]
pub fn recording_info_lines(voice: &crate::voice_notes::VoiceNotesUiState) -> Vec<String> {
    let written = u64::from(voice.pcm_bytes) + 44;
    let first = format!("WAV · 16 kHz mono · {} so far", size_one_decimal(written));
    let Some(free) = voice
        .available_storage_bytes
        .map(|available| available.saturating_sub(written))
    else {
        return vec![first];
    };
    let hours = free / u64::from(bytes_per_second()) / 3600;
    vec![
        first,
        format!(
            "SD: {} free (≈ {} h)",
            size_one_decimal(free),
            grouped(hours.min(u64::from(u32::MAX)) as u32)
        ),
        "Wi-Fi off while recording".into(),
    ]
}

/// `6.3 MB` or `14.2 GB`, one decimal in the binary units.
#[must_use]
pub fn size_one_decimal(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{}.{:01} GB", bytes / GB, (bytes % GB) * 10 / GB)
    } else if bytes >= MB {
        format!("{}.{:01} MB", bytes / MB, (bytes % MB) * 10 / MB)
    } else {
        format!("{} KB", bytes / KB)
    }
}
fn line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(22, y), style).draw(display)?;
    Text::new(value, Point::new(176, y), style).draw(display)?;
    Ok(())
}

fn draw_action(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Rectangle::new(Point::new(22, top), Size::new(436, 46))
        .into_styled(if selected {
            PrimitiveStyle::with_stroke(BinaryColor::On, 5)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 1)
        })
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 30),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 30), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_voice_note_details, render_voice_note_recording, render_voice_notes};
    use crate::{
        app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer,
        voice_notes::VoiceNoteEntry,
    };

    #[test]
    fn voice_note_screens_render_without_sd_card() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        let state = AppState::default();
        render_voice_notes(&mut display, &state).unwrap();
        render_voice_note_details(&mut display, &state).unwrap();
        render_voice_note_recording(&mut display, &state).unwrap();
    }

    #[test]
    fn friendly_title_grid_editor_renders_with_saved_note() {
        let mut state = AppState::default();
        state.voice_notes.notes.push(VoiceNoteEntry {
            file_name: "VOICE001.WAV".into(),
            title: "VOICE NOTE 001".into(),
            recorded_at: "2026-06-06  11:43:24".into(),
            wav_bytes: 44,
            pcm_bytes: 0,
            duration_seconds: 0,
            ai_state: None,
        });
        state.voice_notes.selected = 2;
        state.voice_notes.begin_title_edit();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_voice_note_details(&mut display, &state).unwrap();
    }

    /// The info box lines for 3:27 written and 14.2 GB free; without the
    /// free space, the first line only.
    #[test]
    fn the_info_box_counts_the_card_cost() {
        use super::recording_info_lines;
        use crate::voice_notes::VoiceNotesUiState;
        let mut voice = VoiceNotesUiState::default();
        // 3:27 of WAV at 32 000 bytes a second.
        voice.pcm_bytes = 207 * 32_000;
        voice.available_storage_bytes = Some(15_247_133_901 + 6_624_044);
        let lines = recording_info_lines(&voice);
        assert_eq!(lines[0], "WAV · 16 kHz mono · 6.3 MB so far");
        assert_eq!(lines[1], "SD: 14.2 GB free (≈ 132 h)");
        assert_eq!(lines[2], "Wi-Fi off while recording");
        voice.available_storage_bytes = None;
        let lines = recording_info_lines(&voice);
        assert_eq!(
            lines,
            ["WAV · 16 kHz mono · 6.3 MB so far"],
            "without the free space, the first line only"
        );
    }
}
