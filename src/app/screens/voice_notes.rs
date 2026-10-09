//! SD-backed voice notes list, details, title editor and recording screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            bottom_bar::{
                draw_bottom_bar, KeyCap, BACK_HINTS, KEYBOARD_HINTS, OPEN_HINTS, RUN_HINTS,
            },
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    voice_note_metadata::format_storage_bytes,
    voice_notes::{format_duration, VoiceNotesMode, VOICE_TITLE_EDITOR_KEY_ROWS},
};

const DELETE_HINTS: [(KeyCap, &str); 2] = [(KeyCap::UpDown, "move"), (KeyCap::Select, "confirm")];

const RECORDING_HINTS: [(KeyCap, &str); 2] = [
    (KeyCap::UpDown, "pause / resume"),
    (KeyCap::Select, "stop + save"),
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
        "Duration",
        &format_duration(note.duration_seconds),
        metadata,
    )?;
    line(
        display,
        312,
        "Available",
        &format_storage_bytes(state.voice_notes.available_storage_bytes),
        metadata,
    )?;
    line(
        display,
        348,
        "Playback",
        &format!(
            "{} / {}",
            format_duration(state.voice_notes.playback_elapsed_seconds()),
            format_duration(note.duration_seconds)
        ),
        metadata,
    )?;
    draw_action(
        display,
        386,
        if state.voice_notes.is_playing_selected() {
            "Stop playback"
        } else {
            "Play note"
        },
        state.voice_notes.detail_selected == 0,
        body,
    )?;
    draw_action(
        display,
        436,
        "Edit friendly title",
        state.voice_notes.detail_selected == 1,
        body,
    )?;
    draw_action(
        display,
        486,
        "Export / download",
        state.voice_notes.detail_selected == 2,
        body,
    )?;
    draw_action(
        display,
        536,
        "Delete note",
        state.voice_notes.detail_selected == 3,
        body,
    )?;
    draw_action(
        display,
        586,
        "Return to Voice Notes",
        state.voice_notes.detail_selected == 4,
        body,
    )?;
    if state.voice_notes.export_file.as_deref() == Some(note.file_name.as_str()) {
        line(
            display,
            650,
            "LAN",
            state.wifi_transfer.url_label(),
            metadata,
        )?;
        line(
            display,
            682,
            "Code",
            state.wifi_transfer.code_label(),
            metadata,
        )?;
        line(
            display,
            714,
            "Path",
            &format!("VOICE/{}", note.file_name),
            metadata,
        )?;
    } else if let Some(error) = state.voice_notes.error.as_deref() {
        Text::new(error, Point::new(22, 682), detail).draw(display)?;
    }
    draw_bottom_bar(display, state.display, &RUN_HINTS)?;
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
    for paragraph in prepare_text(text).split('\n') {
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
        Some(record) => format!(
            "{} · {} · {} → {}",
            when, record.language, record.transcribed_by, record.summarized_by
        ),
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

pub fn render_voice_note_recording(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let body = state.display.body_style();
    let heading = state.display.heading_style();
    let detail = state.display.detail_style();
    let voice = &state.voice_notes;
    let elapsed = format_duration(voice.elapsed_seconds);
    let file = voice.active_file.as_deref().unwrap_or("VOICE---.WAV");
    draw_header(
        display,
        state.display,
        "RECORD VOICE NOTE",
        "ES8311 MICROPHONE TO SD",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: voice.mode.label(),
            middle: &elapsed,
            right: "MONO",
        },
    )?;
    Text::new(file, Point::new(22, 176), heading).draw(display)?;
    line(display, 222, "Elapsed", &elapsed, body)?;
    line(
        display,
        270,
        "PCM bytes",
        &voice.pcm_bytes.to_string(),
        body,
    )?;
    line(display, 318, "Peak", &voice.peak.to_string(), body)?;
    line(
        display,
        366,
        "Started",
        voice
            .active_recorded_at
            .as_deref()
            .unwrap_or("DATE UNKNOWN"),
        detail,
    )?;
    line(
        display,
        414,
        "Mic gain",
        &format!("{} ({})", voice.mic_gain.label(), voice.mic_gain.db_label()),
        body,
    )?;
    line(
        display,
        462,
        "Clipped",
        &voice.clipped_samples.to_string(),
        body,
    )?;
    match voice.mode {
        VoiceNotesMode::Recording => {
            Text::new("Streaming VOICE###.TMP", Point::new(22, 546), body).draw(display)?;
            Text::new(
                "UP / DOWN pause. SELECT stop + save.",
                Point::new(22, 598),
                detail,
            )
            .draw(display)?;
        }
        VoiceNotesMode::Paused => {
            Text::new(
                "Recording paused. WAV remains open.",
                Point::new(22, 546),
                body,
            )
            .draw(display)?;
            Text::new(
                "UP / DOWN resume. SELECT stop + save.",
                Point::new(22, 598),
                detail,
            )
            .draw(display)?;
        }
        VoiceNotesMode::Playing => {
            Text::new("Saved WAV playback active.", Point::new(22, 546), body).draw(display)?;
            Text::new("Hold BOOT to stop and return.", Point::new(22, 598), body).draw(display)?;
        }
        VoiceNotesMode::Saved => {
            Text::new("Saved as recovery-safe WAV.", Point::new(22, 546), body).draw(display)?;
            Text::new("Hold BOOT to return.", Point::new(22, 598), body).draw(display)?;
        }
        VoiceNotesMode::Error => {
            Text::new(
                voice.error.as_deref().unwrap_or("Recording failed"),
                Point::new(22, 546),
                detail,
            )
            .draw(display)?;
            Text::new("Hold BOOT to return.", Point::new(22, 598), body).draw(display)?;
        }
        VoiceNotesMode::Idle => {
            Text::new("Preparing microphone capture...", Point::new(22, 546), body)
                .draw(display)?;
        }
    }
    draw_bottom_bar(display, state.display, &RECORDING_HINTS)?;
    Ok(())
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
}
