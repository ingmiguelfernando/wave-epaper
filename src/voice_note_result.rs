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
    fn line_breaks_are_kept() {
        assert_eq!(prepare_text("uno\ndos"), "uno\ndos");
    }
}
