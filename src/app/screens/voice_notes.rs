//! SD-backed voice notes list, details, title editor and recording screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
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
            chips::{chip_row_width, draw_chip_row, Chip},
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
        DETAIL_DELETE, DETAIL_EXPORT, DETAIL_PLAY, DETAIL_RETURN, DETAIL_TITLE, METER_BARS,
        VOICE_TITLE_EDITOR_KEY_ROWS,
    },
};

/// The recording screen's layout, from the mockup's `Voice Notes: grabando`.
const TITLE_BASELINE: i32 = 76;
const TIMER_BASELINE: i32 = 236;
const METER_TOP: i32 = 262;
const METER_HEIGHT: i32 = 130;
const BAR_WIDTH: i32 = 10;
const BAR_GAP: i32 = 5;
const METER_LEFT: i32 = (480 - (METER_BARS as i32 * (BAR_WIDTH + BAR_GAP) - BAR_GAP)) / 2;
const CHIPS_TOP: i32 = 412;
/// User-visible words that must not name WAV: a note can be Ogg too.
const LIST_SUBTITLE: &str = "SD AUDIO RECORDINGS";
const RESULT_SUBTITLE: &str = "SAVED AUDIO";
const TITLE_EDITOR_NOTE: &str = "Internal filename remains unchanged.";
const DELETE_NOTE: &str = "This permanently removes the audio file.";
const SAVED_MESSAGE: &str = "Saved as recovery-safe audio.";
const INFO_BOX_TOP: i32 = 470;

/// The result screen's layout, from the mockup's `Voice Notes: resultado`.
const RESULT_TITLE_BASELINE: i32 = 86;
const RESULT_CHIPS_TOP: i32 = 210;
const BODY_TOP: i32 = 254;
const FOOTER_TOP: i32 = 704;
/// Extra space above a heading line.
const HEADING_GAP: i32 = 12;

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
    draw_header(display, state.display, "VOICE NOTES", LIST_SUBTITLE)?;
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
    draw_header(display, state.display, "VOICE NOTE", RESULT_SUBTITLE)?;
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
        .map_or("VOICE---", |note| note.file_name.as_str());
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
    Text::new(TITLE_EDITOR_NOTE, Point::new(22, 276), detail).draw(display)?;
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
    Text::new(DELETE_NOTE, Point::new(22, 370), detail).draw(display)?;
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
    (KeyCap::Select, "menu"),
];

/// One row of the result body: a wrapped line with its marker and indent.
#[derive(Clone, Debug, Eq, PartialEq)]
struct BodyRow {
    text: String,
    /// The text's x offset from the body's left; the marker sits before it.
    indent: i32,
    marker: BodyMarker,
    heading: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BodyMarker {
    None,
    Bullet,
    Todo,
}

/// The marker column: a 14 px marker and an 8 px gap before the text.
const MARKER_COLUMN: i32 = 22;

/// The active tab's text as draw rows: the summary keeps its markers and
/// headings, the other tabs stay plain text. The first row of a marker line
/// draws the marker; its wrapped lines start under the text.
fn result_body_rows(
    state: &AppState,
    record: &crate::voice_note_record::NoteRecord,
    audio: &str,
    width: i32,
) -> Vec<BodyRow> {
    use crate::voice_note_result::{parse_summary, prepare_text, ResultTab, SummaryLine};
    let body = result_body_style(state.display);
    let text = match state.voice_notes_result.tab {
        ResultTab::Summary => record.summary.as_str(),
        ResultTab::Transcript => record.transcript.as_str(),
        ResultTab::Audio => audio,
    };
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
    let plain_row = |text: String| BodyRow {
        text,
        indent: 0,
        marker: BodyMarker::None,
        heading: false,
    };
    if state.voice_notes_result.tab != ResultTab::Summary {
        let mut rows = Vec::new();
        for paragraph in prepare_text(&text).split('\n') {
            if paragraph.trim().is_empty() {
                rows.push(plain_row(String::new()));
            } else {
                rows.extend(body.wrap(paragraph, width).into_iter().map(plain_row));
            }
        }
        return rows;
    }
    let mut rows = Vec::new();
    for line in parse_summary(&text) {
        let (marker, indent, heading, content) = match &line {
            SummaryLine::Bullet(text) => (BodyMarker::Bullet, MARKER_COLUMN, false, text),
            SummaryLine::Todo(text) => (BodyMarker::Todo, MARKER_COLUMN, false, text),
            SummaryLine::Heading(text) => (BodyMarker::None, 0, true, text),
            SummaryLine::Text(text) => (BodyMarker::None, 0, false, text),
        };
        let content = prepare_text(content);
        let wrapped = if content.trim().is_empty() {
            vec![String::new()]
        } else {
            body.wrap(&content, width - indent)
        };
        for (index, line) in wrapped.into_iter().enumerate() {
            rows.push(BodyRow {
                text: line,
                indent,
                marker: if index == 0 { marker } else { BodyMarker::None },
                heading,
            });
        }
    }
    rows
}

/// Where each drawn row goes: its index and its text band's top. Heading
/// rows add their gap; rows past the footer are not drawn.
fn body_row_tops(
    rows: &[BodyRow],
    first: usize,
    count: usize,
    body_top: i32,
    line_step: i32,
    footer: i32,
) -> Vec<(usize, i32)> {
    let mut tops = Vec::new();
    let mut y = body_top;
    for (offset, row) in rows.iter().enumerate().skip(first).take(count) {
        if row.heading {
            y += HEADING_GAP;
        }
        if y + line_step > footer {
            break;
        }
        tops.push((offset, y));
        y += line_step;
    }
    tops
}

/// The first row of the last page, counted from the end so the heading gaps
/// cannot push the last row off it.
fn last_visible_start(rows: &[BodyRow], body_top: i32, line_step: i32, footer: i32) -> usize {
    let budget = footer - body_top;
    let mut used = 0;
    let mut start = rows.len();
    while start > 0 {
        let row = &rows[start - 1];
        let height = line_step + if row.heading { HEADING_GAP } else { 0 };
        if used + height > budget {
            break;
        }
        used += height;
        start -= 1;
    }
    start
}

/// The body style one UI size up from the user's, near the mockup's 18 px.
fn result_body_style(preferences: DisplayPreferences) -> UiTextStyle {
    let mut bigger = preferences;
    bigger.font_size = match preferences.font_size {
        crate::app::display::UiFontSize::Compact => crate::app::display::UiFontSize::Standard,
        crate::app::display::UiFontSize::Standard => crate::app::display::UiFontSize::Large,
        crate::app::display::UiFontSize::Large => crate::app::display::UiFontSize::Large,
    };
    bigger.body_style()
}

/// Cut `text` so `text…` fits `max_width`.
fn cut_with_ellipsis(style: UiTextStyle, text: &str, max_width: i32) -> String {
    let budget = max_width - style.text_width("\u{2026}");
    let mut width = 0;
    let mut out = String::new();
    for character in text.chars() {
        let advance = style.char_advance(character);
        if width + advance > budget {
            break;
        }
        width += advance;
        out.push(character);
    }
    out.push('\u{2026}');
    out
}

pub fn render_voice_note_result(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    use crate::voice_note_result::ResultTab;
    let preferences = state.display;
    let detail = preferences.detail_style();
    draw_status_bar(display, preferences, "Voice Note")?;
    let clock = state.board.time_label(state.regional);
    draw_status_text(display, preferences, &clock, STATUS_BAR_RIGHT)?;
    let Some(note) = state.voice_notes.selected_note() else {
        Text::new("No saved note selected.", Point::new(22, 232), detail).draw(display)?;
        draw_bottom_bar(display, preferences, &BACK_HINTS)?;
        return Ok(());
    };

    // The title in ExtraBold (mockup weight 800), up to two lines; the second
    // cut with `…`.
    let record = state.voice_notes_result.record.as_ref();
    let title = record
        .map(|record| record.title.as_str())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(note.title.as_str());
    let large = preferences.extrabold_style();
    let mut title_lines = large.wrap(title, 436);
    if title_lines.len() > 2 {
        title_lines.truncate(2);
        title_lines[1] = cut_with_ellipsis(large, &title_lines[1], 436);
    }
    let mut y = RESULT_TITLE_BASELINE;
    for line in &title_lines {
        Text::new(line, Point::new(22, y), large).draw(display)?;
        y += i32::from(large.line_height());
    }

    // Stamp, language, then the providers: transcriber → summarizer.
    let (date, time) = split_stamp(&note.recorded_at);
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
        Text::new(line, Point::new(22, y + 10 + row as i32 * 20), detail).draw(display)?;
    }

    // The tabs as chips, the active one filled, from the left margin.
    let tabs = [ResultTab::Summary, ResultTab::Transcript, ResultTab::Audio];
    let chips: Vec<Chip<'_>> = tabs
        .iter()
        .map(|tab| Chip::new(tab.label(), *tab == state.voice_notes_result.tab))
        .collect();
    let label = preferences.extrabold_body_style();
    draw_chip_row(display, 22, RESULT_CHIPS_TOP, 8, label, &chips)?;

    // The body, one UI size up, scrolled by wrapped row.
    let body = result_body_style(preferences);
    let rows = match state.voice_notes_result.record.as_ref() {
        Some(record) => {
            let audio = crate::voice_note_result::audio_text(
                &note.file_name,
                note.duration_seconds,
                note.pcm_bytes,
            );
            result_body_rows(state, record, &audio, 436)
        }
        None => vec![BodyRow {
            text: "No AI result yet.".into(),
            indent: 0,
            marker: BodyMarker::None,
            heading: false,
        }],
    };
    let line_step = i32::from(body.line_height()) + 4;
    // Clamp to the last page here: input has no line count, so it scrolls freely.
    let last_start = last_visible_start(&rows, BODY_TOP, line_step, FOOTER_TOP);
    let scroll = state.voice_notes_result.scroll.min(last_start);
    for (index, y) in body_row_tops(&rows, scroll, rows.len(), BODY_TOP, line_step, FOOTER_TOP) {
        let row = &rows[index];
        match row.marker {
            BodyMarker::Bullet => {
                Text::new("\u{2022}", Point::new(22, y + body.cap_height()), body).draw(display)?;
            }
            BodyMarker::Todo => {
                Rectangle::new(Point::new(22, y + 2), Size::new(14, 14))
                    .into_styled(
                        embedded_graphics::primitives::PrimitiveStyleBuilder::new()
                            .stroke_color(BinaryColor::On)
                            .stroke_width(2)
                            .build(),
                    )
                    .draw(display)?;
            }
            BodyMarker::None => {}
        }
        let style = if row.heading {
            preferences.heading_style()
        } else {
            body
        };
        Text::new(
            &row.text,
            Point::new(22 + row.indent, y + style.cap_height()),
            style,
        )
        .draw(display)?;
    }

    Text::new(
        "AI-generated · may contain mistakes · audio kept on SD",
        Point::new(22, FOOTER_TOP + 16),
        detail,
    )
    .draw(display)?;
    draw_bottom_bar(display, preferences, &RESULT_HINTS)?;
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
            VoiceNotesMode::Saved => SAVED_MESSAGE.to_string(),
            VoiceNotesMode::Error => voice
                .error
                .clone()
                .unwrap_or_else(|| "Recording failed".into()),
            _ => "Preparing microphone capture...".to_string(),
        };
        draw_info_box(display, state.display, &message, 150)?;
        // Only a hold of BOOT leaves: ● and ▲▼ have no recording to act on.
        draw_bottom_bar(display, state.display, &BACK_HINTS)?;
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
    let label = state.display.extrabold_body_style();
    let chips = [
        Chip::dotted("RECORDING", voice.mode == VoiceNotesMode::Recording),
        Chip::new("PAUSED", voice.mode == VoiceNotesMode::Paused),
    ];
    let left = (480 - chip_row_width(label, &chips, 6)) / 2;
    draw_chip_row(display, left, CHIPS_TOP, 6, label, &chips)?;

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
    let title = match voice.mode {
        VoiceNotesMode::Recording => "REC · Voice Notes",
        VoiceNotesMode::Paused => "PAUSED · Voice Notes",
        _ => "Voice Notes",
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
            voice.active_file.as_deref().unwrap_or("VOICE---")
        )
    };
    let left = 240 - detail.text_width(&note) / 2;
    Text::new(&note, Point::new(left, TITLE_BASELINE + 26), detail).draw(display)?;
    Ok(())
}

/// The level meter: one bar per second of audio, the newest at the right
/// and silence before the first, on a log scale so speech fills the band.
/// The bars are centred on the screen between two rules.
fn draw_level_meter(display: &mut OrientedFrameBuffer<'_>, bars: &[u16]) -> Result<(), Infallible> {
    let ink = PrimitiveStyle::with_fill(BinaryColor::On);
    for y in [METER_TOP, METER_TOP + METER_HEIGHT - 2] {
        Rectangle::new(Point::new(16, y), Size::new(448, 2))
            .into_styled(ink)
            .draw(display)?;
    }
    let center = METER_TOP + METER_HEIGHT / 2;
    let silent = METER_BARS.saturating_sub(bars.len());
    let peaks = core::iter::repeat(0)
        .take(silent)
        .chain(bars.iter().copied());
    for (index, peak) in peaks.take(METER_BARS).enumerate() {
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
/// audio hours it still takes, and the radio state. Without the free space,
/// the first line only.
#[must_use]
pub fn recording_info_lines(voice: &crate::voice_notes::VoiceNotesUiState) -> Vec<String> {
    let written = u64::from(voice.pcm_bytes) + 44;
    let first = format!(
        "Audio · 16 kHz mono · {} so far",
        size_one_decimal(written)
    );
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
            "SD: {} free (~{} h) · Wi-Fi off while recording",
            size_one_decimal(free),
            grouped(hours.min(u64::from(u32::MAX)) as u32)
        ),
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
    use embedded_graphics::prelude::Point;

    use super::{
        recording_info_lines, render_voice_note_details, render_voice_note_recording,
        render_voice_note_result, render_voice_notes, DELETE_NOTE, LIST_SUBTITLE, RESULT_SUBTITLE,
        SAVED_MESSAGE, TITLE_EDITOR_NOTE,
    };
    use crate::{
        app::AppState,
        framebuffer::FrameBuffer,
        orientation::OrientedFrameBuffer,
        voice_notes::{VoiceNoteEntry, VoiceNotesUiState},
    };

    #[test]
    fn screens_say_audio_not_wav() {
        assert_eq!(RESULT_SUBTITLE, "SAVED AUDIO");
        assert_eq!(LIST_SUBTITLE, "SD AUDIO RECORDINGS");
        assert_eq!(TITLE_EDITOR_NOTE, "Internal filename remains unchanged.");
        assert_eq!(DELETE_NOTE, "This permanently removes the audio file.");
        assert_eq!(SAVED_MESSAGE, "Saved as recovery-safe audio.");
        for text in [
            LIST_SUBTITLE,
            RESULT_SUBTITLE,
            TITLE_EDITOR_NOTE,
            DELETE_NOTE,
            SAVED_MESSAGE,
        ] {
            assert!(
                !text.contains("WAV") && !text.contains("wav"),
                "{text} names one kind"
            );
        }
        // The recording line names audio, not the container; the free-space
        // hours stay as they are (they follow the recorder's format).
        let mut voice = VoiceNotesUiState::default();
        voice.pcm_bytes = 32_000;
        let lines = recording_info_lines(&voice);
        assert!(lines[0].starts_with("Audio · 16 kHz mono · "));
        assert!(!lines[0].contains("WAV"));
    }

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
        // 3:27 of audio at 32 000 bytes a second.
        voice.pcm_bytes = 207 * 32_000;
        voice.available_storage_bytes = Some(15_247_133_901 + 6_624_044);
        let lines = recording_info_lines(&voice);
        assert_eq!(lines[0], "Audio · 16 kHz mono · 6.3 MB so far");
        assert_eq!(
            lines[1],
            "SD: 14.2 GB free (~132 h) · Wi-Fi off while recording"
        );
        voice.available_storage_bytes = None;
        let lines = recording_info_lines(&voice);
        assert_eq!(
            lines,
            ["Audio · 16 kHz mono · 6.3 MB so far"],
            "without the free space, the first line only"
        );
    }

    /// The meter stays between the ends of its rules, and a note's first
    /// second is the bar at the right, with silence before it.
    #[test]
    fn the_meter_fits_with_the_newest_bar_at_the_right() {
        use super::{METER_HEIGHT, METER_LEFT, METER_TOP};
        let mut state = AppState::default();
        state
            .voice_notes
            .begin_recording("VOICE001.WAV".into(), "2026-10-02  12:04:00".into());
        state
            .voice_notes
            .update_recording_progress(4_096, u16::MAX, 0);
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_voice_note_recording(&mut display, &state).unwrap();
        drop(display);
        let band = METER_TOP + 2..METER_TOP + METER_HEIGHT - 2;
        let outside = (0..480)
            .filter(|x| !(16..464).contains(x))
            .any(|x| band.clone().any(|y| ink(&frame, x, y)));
        assert!(!outside, "no bar runs past the rules");
        let center = METER_TOP + METER_HEIGHT / 2;
        let newest = METER_LEFT + 29 * 15 + 5;
        assert!(ink(&frame, newest, center - 50), "newest at the right");
        assert!(ink(&frame, METER_LEFT + 5, center), "silence is a stub");
        assert!(!ink(&frame, METER_LEFT + 5, center - 50));
    }

    /// The mockup's summary: three bullets, a heading and three to-dos.
    fn summary_state() -> AppState {
        use crate::{app::router::ScreenRoute, voice_note_record::NoteRecord};
        let mut state = AppState::default();
        state
            .voice_notes
            .notes
            .push(crate::voice_notes::VoiceNoteEntry {
                file_name: "VOICE012.WAV".into(),
                title: "Reunión de equipo: presupuesto Q4".into(),
                recorded_at: "2026-10-02  12:04:00".into(),
                wav_bytes: 4_000_000,
                pcm_bytes: 3_999_956,
                duration_seconds: 124,
                ai_state: None,
            });
        state.voice_notes.selected = 2;
        state.voice_notes_result.record = Some(NoteRecord {
            state: crate::voice_note_record::NoteState::Done,
            error: String::new(),
            title: "Reunión de equipo: presupuesto Q4".into(),
            language: "Spanish".into(),
            transcribed_by: "Groq whisper".into(),
            summarized_by: "OpenRouter llama".into(),
            summary: "• Se aprobó el presupuesto.\n\
                      • El lanzamiento pasa al 15 de noviembre.\n\
                      • Ana presentará el plan el viernes.\n\
                      Pendientes\n\
                      \u{2610} Carlos: enviar este resumen al equipo (hoy).\n\
                      \u{2610} Ana: plan de contratación (viernes).\n\
                      \u{2610} Revisar proveedores de impresión (próxima semana)."
                .into(),
            transcript: "Hablamos del presupuesto.".into(),
        });
        state.router.navigate_to(ScreenRoute::VoiceNoteResult);
        state
    }

    /// Ink at a logical screen point (the frame stores the screen rotated).
    fn ink(frame: &FrameBuffer, x: i32, y: i32) -> bool {
        frame.is_black(Point::new(y, 479 - x)) == Some(true)
    }

    /// The 2 px ring of a to-do box at a row's top: its top border and its
    /// paper interior. The stroke is centred on the rect's edge.
    fn has_todo_box(frame: &FrameBuffer, row_top: i32) -> bool {
        let border = (row_top + 1..row_top + 4).any(|y| (22..36).all(|x| ink(frame, x, y)));
        let interior = !ink(frame, 28, row_top + 9);
        border && interior
    }

    /// The summary frame draws one box per to-do: every to-do row has its
    /// ring and no other row has one.
    #[test]
    fn the_summary_frame_draws_one_box_per_todo() {
        use super::{body_row_tops, result_body_rows, BODY_TOP, FOOTER_TOP};
        let state = summary_state();
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_voice_note_result(&mut display, &state).unwrap();
        drop(display);
        let record = state.voice_notes_result.record.as_ref().unwrap();
        let rows = result_body_rows(&state, record, "", 436);
        let body = super::result_body_style(state.display);
        let line_step = i32::from(body.line_height()) + 4;
        let mut boxes = 0;
        for (index, y) in body_row_tops(&rows, 0, rows.len(), BODY_TOP, line_step, FOOTER_TOP) {
            let is_todo = rows[index].marker == super::BodyMarker::Todo;
            assert_eq!(
                has_todo_box(&frame, y),
                is_todo,
                "row {index} draws its box exactly when it is a to-do"
            );
            boxes += usize::from(is_todo);
        }
        assert_eq!(boxes, 3, "the mockup's three to-dos draw three boxes");
    }

    /// A long to-do's second wrapped line starts under its text, not under
    /// the box.
    #[test]
    fn a_long_todo_wraps_under_its_text() {
        use super::{body_row_tops, result_body_rows, BodyMarker, BODY_TOP, FOOTER_TOP};
        let mut state = summary_state();
        let record = state.voice_notes_result.record.as_mut().unwrap();
        record.summary =
            "☐ Carlos: enviar este resumen al equipo hoy mismo antes de que acabe el día.".into();
        let record = state.voice_notes_result.record.as_ref().unwrap();
        let rows = result_body_rows(&state, record, "", 436);
        assert!(rows.len() >= 2, "the to-do wraps");
        assert_eq!(rows[0].marker, BodyMarker::Todo);
        assert_eq!(
            rows[1].marker,
            BodyMarker::None,
            "the second line has no box"
        );
        assert_eq!(rows[0].indent, rows[1].indent, "it starts under the text");
        // And the frame: the second line's ink starts at the text column.
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_voice_note_result(&mut display, &state).unwrap();
        drop(display);
        let body = super::result_body_style(state.display);
        let line_step = i32::from(body.line_height()) + 4;
        let tops = body_row_tops(&rows, 0, rows.len(), BODY_TOP, line_step, FOOTER_TOP);
        let second_y = tops[1].1;
        let first_ink =
            (22..200).find(|x| (second_y..second_y + line_step).any(|y| ink(&frame, *x, y)));
        assert!(
            first_ink.is_some_and(|x| x >= 44),
            "the second line starts under the text, not under the box: {first_ink:?}"
        );
    }

    /// The last scroll position shows the last to-do.
    #[test]
    fn the_last_scroll_position_shows_the_last_todo() {
        use super::{
            body_row_tops, last_visible_start, result_body_rows, BodyMarker, BODY_TOP, FOOTER_TOP,
        };
        let mut state = summary_state();
        state.voice_notes_result.scroll = usize::MAX / 2;
        let record = state.voice_notes_result.record.as_ref().unwrap();
        let rows = result_body_rows(&state, record, "", 436);
        let todo_index = rows
            .iter()
            .rposition(|row| row.marker == BodyMarker::Todo)
            .expect("the summary has to-dos");
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_voice_note_result(&mut display, &state).unwrap();
        drop(display);
        let body = super::result_body_style(state.display);
        let line_step = i32::from(body.line_height()) + 4;
        let last_start = last_visible_start(&rows, BODY_TOP, line_step, FOOTER_TOP);
        let tops = body_row_tops(
            &rows,
            last_start,
            rows.len(),
            BODY_TOP,
            line_step,
            FOOTER_TOP,
        );
        assert!(
            tops.iter()
                .any(|(index, y)| *index == todo_index && has_todo_box(&frame, *y)),
            "the last to-do's box is on the last page"
        );
    }
}
