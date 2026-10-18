//! Voice note result screen state: the tab, the scroll position, and the text
//! preparation that keeps the body inside the device font set.

use crate::charset::glyph_index;

/// The tab strip under the title: summary, transcript, or audio details.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResultTab {
    #[default]
    Summary,
    Transcript,
    Audio,
}

impl ResultTab {
    /// The next tab, wrapping after audio, for a BOOT short press.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Summary => Self::Transcript,
            Self::Transcript => Self::Audio,
            Self::Audio => Self::Summary,
        }
    }

    /// The tab label, drawn in uppercase like the other strips.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Summary => "SUMMARY",
            Self::Transcript => "TRANSCRIPT",
            Self::Audio => "AUDIO",
        }
    }
}

/// The result screen's own state: the open tab and how far the body scrolled.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResultUiState {
    pub tab: ResultTab,
    /// First wrapped line shown in the body.
    pub scroll: usize,
    /// The open note's AI record, loaded once when the screen opens.
    pub record: Option<crate::voice_note_record::NoteRecord>,
}

impl ResultUiState {
    /// BOOT short press: move to the next tab and reset the scroll.
    pub fn cycle_tab(&mut self) {
        self.tab = self.tab.next();
        self.scroll = 0;
    }

    /// Move the body by `delta` lines. Input has no line count, so this only
    /// keeps the top at zero or below; the render clamps to the last page.
    pub fn step(&mut self, delta: i32) {
        let next = self.scroll as i64 + i64::from(delta);
        self.scroll = next.max(0) as usize;
    }

    /// Move the body by `delta` lines, clamped so the last `visible` lines
    /// stay reachable and the top never goes negative. A body shorter than
    /// the visible area stays at the top.
    pub fn scroll_by(&mut self, delta: i32, total_lines: usize, visible: usize) {
        let last_start = total_lines.saturating_sub(visible) as i64;
        let next = self.scroll as i64 + i64::from(delta);
        self.scroll = next.clamp(0, last_start.max(0)) as usize;
    }
}

/// The audio tab's text: what the saved file is and where it stays. A `.WAV`
/// is the recorder's own 16 kHz, mono, 16-bit PCM; a `.OGG` is Opus. Any
/// other name keeps the WAV wording, as before Ogg notes existed. `bytes` is
/// the PCM bytes for a WAV and the file size for an Ogg.
#[must_use]
pub fn audio_text(file_name: &str, duration_seconds: u32, bytes: u32) -> String {
    let ogg = file_name.to_ascii_uppercase().ends_with(".OGG");
    // Whole kilobytes in integers: the device path avoids float formatting.
    let kilobytes = bytes / 1_000;
    let format = if ogg {
        "Opus \u{b7} mono"
    } else {
        "16 kHz \u{b7} mono \u{b7} 16-bit"
    };
    let size = if ogg {
        format!("{kilobytes} KB")
    } else {
        format!("{kilobytes} KB of PCM")
    };
    format!(
        "{file_name}\n\
         {minutes}:{seconds:02} \u{b7} {format}\n\
         {size}\n\
         Kept on the SD card. Sent only to the providers you configure.",
        minutes = duration_seconds / 60,
        seconds = duration_seconds % 60,
    )
}

/// What the summary tab says while a note is not summarized yet, or why it
/// failed. `None` once the summary is there.
#[must_use]
pub fn status_text(record: &crate::voice_note_record::NoteRecord) -> Option<String> {
    use crate::voice_note_record::NoteState;
    match record.state {
        NoteState::Done => None,
        NoteState::Queued => {
            Some("Queued. The note goes to the providers when Wi-Fi is on.".into())
        }
        NoteState::Transcribing => Some("Transcribing\u{2026}".into()),
        NoteState::Summarizing => Some("Summarizing\u{2026}".into()),
        NoteState::Failed => Some(format!(
            "Failed: {}\nSelect opens the actions; Retry sends the note again.",
            record.error
        )),
    }
}

/// The line under the title: when, the language if known, and who
/// transcribed and summarized, leaving out what is not known yet.
#[must_use]
pub fn meta_line(when: &str, record: &crate::voice_note_record::NoteRecord) -> String {
    let providers = match (
        record.transcribed_by.as_str(),
        record.summarized_by.as_str(),
    ) {
        ("", "") => String::new(),
        (only, "") | ("", only) => only.to_string(),
        (transcribed, summarized) => format!("{transcribed} \u{2192} {summarized}"),
    };
    [when, record.language.as_str(), providers.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// Make text drawable with the device fonts: `☐` becomes `•`, and any
/// character the font set cannot draw becomes `?`. Spanish accents stay.
#[must_use]
pub fn prepare_text(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\u{2610}' => '\u{2022}',
            '\u{2192}' => '>',
            other if glyph_index(other).is_some() || other == '\n' => other,
            _ => '?',
        })
        .collect()
}

/// One typed line of an AI summary, drawn by type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SummaryLine {
    /// A `•` bullet and its text.
    Bullet(String),
    /// A `☐` to-do and its text; drawn with its box.
    Todo(String),
    /// A plain line right before a to-do: its heading.
    Heading(String),
    /// Any other line.
    Text(String),
}

/// Parse the summary once into typed lines: `☐` and `•` lines keep their
/// marker, a plain line right before a to-do becomes its heading, and the
/// rest is text. A `☐` inside a sentence stays text; CRLF line ends count
/// as one.
#[must_use]
pub fn parse_summary(text: &str) -> Vec<SummaryLine> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let raw: Vec<&str> = normalized.lines().collect();
    let mut typed = Vec::new();
    for (index, line) in raw.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = marker_text(trimmed, '\u{2610}') {
            typed.push(SummaryLine::Todo(rest.to_string()));
        } else if let Some(rest) = marker_text(trimmed, '\u{2022}') {
            typed.push(SummaryLine::Bullet(rest.to_string()));
        } else {
            // A clean plain line right before a to-do is its heading; a line
            // with a stray marker inside stays text.
            let next_is_todo = raw
                .get(index + 1)
                .is_some_and(|next| marker_text(next.trim(), '\u{2610}').is_some());
            let clean = !trimmed.contains('\u{2610}') && !trimmed.contains('\u{2022}');
            if next_is_todo && clean && !trimmed.is_empty() {
                typed.push(SummaryLine::Heading(trimmed.to_string()));
            } else {
                typed.push(SummaryLine::Text(line.trim_end().to_string()));
            }
        }
    }
    typed
}

/// The marker at the start of a line: `☐ text` or a bare `☐`. A marker
/// mid-sentence, or one welded to its text, is not a marker.
fn marker_text<'a>(line: &'a str, marker: char) -> Option<&'a str> {
    let rest = line.strip_prefix(marker)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix(' ').map(str::trim_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_cycles_summary_transcript_audio_and_wraps() {
        let mut ui = ResultUiState::default();
        assert_eq!(ui.tab, ResultTab::Summary);
        ui.cycle_tab();
        assert_eq!(ui.tab, ResultTab::Transcript);
        ui.cycle_tab();
        assert_eq!(ui.tab, ResultTab::Audio);
        ui.cycle_tab();
        assert_eq!(ui.tab, ResultTab::Summary);
    }

    #[test]
    fn switching_tabs_returns_the_body_to_the_top() {
        let mut ui = ResultUiState::default();
        ui.scroll_by(5, 40, 10);
        assert_eq!(ui.scroll, 5);
        ui.cycle_tab();
        assert_eq!(ui.scroll, 0);
    }

    #[test]
    fn scroll_stays_between_the_first_and_last_page() {
        let mut ui = ResultUiState::default();
        ui.scroll_by(-3, 40, 10);
        assert_eq!(ui.scroll, 0, "never above the top");
        ui.scroll_by(100, 40, 10);
        assert_eq!(ui.scroll, 30, "last visible line stays reachable");
    }

    #[test]
    fn short_bodies_do_not_scroll() {
        let mut ui = ResultUiState::default();
        ui.scroll_by(4, 6, 10);
        assert_eq!(ui.scroll, 0);
    }

    #[test]
    fn input_steps_never_go_above_the_top_or_panic() {
        let mut ui = ResultUiState::default();
        ui.step(-5);
        assert_eq!(ui.scroll, 0);
        ui.step(3);
        assert_eq!(ui.scroll, 3);
        ui.step(-1);
        assert_eq!(ui.scroll, 2);
    }

    #[test]
    fn checkbox_becomes_a_bullet_and_accents_stay() {
        assert_eq!(
            prepare_text("☐ Carlos: enviar el resumen"),
            "• Carlos: enviar el resumen"
        );
        assert_eq!(prepare_text("Reunión · año"), "Reunión · año");
    }

    #[test]
    fn the_arrow_between_providers_becomes_a_greater_than() {
        assert_eq!(prepare_text("Groq → OpenRouter"), "Groq > OpenRouter");
    }

    #[test]
    fn characters_outside_the_font_become_question_marks() {
        assert_eq!(prepare_text("ok ✓ fin"), "ok ? fin");
        assert_eq!(prepare_text("hola 🙂"), "hola ?");
    }

    #[test]
    fn audio_tab_states_the_file_length_format_and_storage() {
        assert_eq!(
            audio_text("VOICE001.WAV", 185, 5_920_000),
            "VOICE001.WAV\n3:05 \u{b7} 16 kHz \u{b7} mono \u{b7} 16-bit\n5920 KB of PCM\n\
             Kept on the SD card. Sent only to the providers you configure."
        );
        assert_eq!(
            audio_text("VOICE007.OGG", 185, 228_000),
            "VOICE007.OGG\n3:05 \u{b7} Opus \u{b7} mono\n228 KB\n\
             Kept on the SD card. Sent only to the providers you configure."
        );
        // A lower-case name is the same note (the file name line stays as
        // written; the format and size follow the suffix).
        let lower = audio_text("voice007.ogg", 185, 228_000);
        assert!(lower.starts_with("voice007.ogg\n3:05 \u{b7} Opus \u{b7} mono\n228 KB\n"));
        // A name that is neither keeps the WAV text, as before Ogg notes.
        let mp3 = audio_text("NOTE.MP3", 185, 5_920_000);
        assert!(mp3.contains("16 kHz \u{b7} mono \u{b7} 16-bit"));
        assert!(mp3.contains("5920 KB of PCM"));
    }

    #[test]
    fn line_breaks_are_kept() {
        assert_eq!(prepare_text("uno\ndos"), "uno\ndos");
    }

    #[test]
    fn each_unfinished_state_says_where_the_note_is() {
        use crate::voice_note_record::{NoteRecord, NoteState};
        let queued = NoteRecord::queued();
        assert!(status_text(&queued).unwrap().starts_with("Queued."));
        let failed = NoteRecord {
            state: NoteState::Failed,
            error: "HTTP 401: Invalid API Key".into(),
            ..NoteRecord::queued()
        };
        let text = status_text(&failed).unwrap();
        assert!(text.starts_with("Failed: HTTP 401: Invalid API Key\n"));
        let done = NoteRecord {
            state: NoteState::Done,
            ..NoteRecord::queued()
        };
        assert_eq!(status_text(&done), None);
    }

    #[test]
    fn the_meta_line_leaves_out_what_is_unknown() {
        use crate::voice_note_record::NoteRecord;
        let queued = NoteRecord::queued();
        assert_eq!(
            meta_line("Oct 2 \u{b7} 12:04", &queued),
            "Oct 2 \u{b7} 12:04"
        );
        let done = NoteRecord {
            language: "Spanish".into(),
            transcribed_by: "Groq whisper".into(),
            summarized_by: "OpenRouter llama".into(),
            ..NoteRecord::queued()
        };
        assert_eq!(
            meta_line("Oct 2", &done),
            "Oct 2 \u{b7} Spanish \u{b7} Groq whisper \u{2192} OpenRouter llama"
        );
        let half = NoteRecord {
            transcribed_by: "Groq whisper".into(),
            ..NoteRecord::queued()
        };
        assert_eq!(meta_line("Oct 2", &half), "Oct 2 \u{b7} Groq whisper");
    }

    /// Bullets, to-dos with and without a heading line, a `☐` inside a
    /// sentence staying text, and CRLF line ends.
    #[test]
    fn the_summary_parses_into_typed_lines() {
        let summary = "• Se aprobó el presupuesto.\r\n\
                       \r\n\
                       Pendientes\r\n\
                       \u{2610} Carlos: enviar el resumen.\r\n\
                       \u{2610} Ana: plan de contratación.\r\n\
                       Hay que comprar leche \u{2610} y pan.\r\n\
                       \u{2610} Revisar proveedores.";
        let lines = parse_summary(summary);
        assert_eq!(
            lines[0],
            SummaryLine::Bullet("Se aprobó el presupuesto.".into())
        );
        assert_eq!(lines[1], SummaryLine::Text(String::new()), "the blank line");
        assert_eq!(lines[2], SummaryLine::Heading("Pendientes".into()));
        assert_eq!(
            lines[3],
            SummaryLine::Todo("Carlos: enviar el resumen.".into())
        );
        assert_eq!(
            lines[4],
            SummaryLine::Todo("Ana: plan de contratación.".into())
        );
        assert_eq!(
            lines[5],
            SummaryLine::Text("Hay que comprar leche \u{2610} y pan.".into()),
            "a marker inside a sentence stays text"
        );
        assert_eq!(
            lines[6],
            SummaryLine::Todo("Revisar proveedores.".into()),
            "a to-do without a heading line is still a to-do"
        );
    }

    /// A plain line right before a to-do is its heading; one before a
    /// bullet is not.
    #[test]
    fn only_a_line_before_a_todo_is_a_heading() {
        let lines = parse_summary("Notas\n• Un bullet\nPendientes\n\u{2610} Una tarea");
        assert_eq!(lines[0], SummaryLine::Text("Notas".into()));
        assert_eq!(lines[1], SummaryLine::Bullet("Un bullet".into()));
        assert_eq!(lines[2], SummaryLine::Heading("Pendientes".into()));
        assert_eq!(lines[3], SummaryLine::Todo("Una tarea".into()));
    }
}
