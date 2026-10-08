//! Home › AI: XiaoZhi and Voice Notes on one page with the recent notes.
//! Drawing and selection only; transcription, summaries and provider
//! settings stay with Phase 7.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        router::ScreenRoute,
        state::AppState,
        typography::{Text, UiTextRole},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
    voice_notes::VoiceNoteEntry,
};

use super::games::{card_text, draw_info_box};

const CARD_LEFT: i32 = 16;
const CARD_RIGHT: i32 = 464;
const CARD_TOP: i32 = STATUS_BAR_HEIGHT + 12;
const CARD_HEIGHT: i32 = 96;
const CARD_GAP: i32 = 12;
const LABEL_BASELINE: i32 = CARD_TOP + 2 * (CARD_HEIGHT + CARD_GAP) + 26;
const NOTE_TOP: i32 = LABEL_BASELINE + 12;
const NOTE_HEIGHT: i32 = 72;
const INFO_TOP: i32 = NOTE_TOP + 3 * NOTE_HEIGHT + 20;
/// Newest notes shown under `RECENT NOTES`.
const RECENT_COUNT: usize = 3;

pub const AI_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "open"),
    (KeyCap::Boot, "new note"),
];

/// The Home › AI hub.
pub fn render_ai_hub(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_status_bar(display, state.display, ScreenRoute::Ai.label())?;
    draw_status_text(
        display,
        state.display,
        &network_label(state),
        STATUS_BAR_RIGHT,
    )?;
    let selected = state.category_selection(ScreenRoute::Ai);
    draw_xiaozhi_card(display, state, selected == 0)?;
    draw_voice_notes_card(display, state, selected == 1)?;
    let label = state.display.detail_style();
    Text::new("RECENT NOTES", Point::new(CARD_LEFT, LABEL_BASELINE), label).draw(display)?;
    let notes = recent_notes(state);
    for (offset, note) in notes.iter().enumerate() {
        draw_note_row(
            display,
            state,
            note,
            NOTE_TOP + offset as i32 * NOTE_HEIGHT,
            selected == 2 + offset,
        )?;
    }
    if notes.is_empty() {
        let body = state.display.body_style();
        Text::new(
            "No notes yet",
            Point::new(CARD_LEFT + 12, NOTE_TOP + 40),
            body,
        )
        .draw(display)?;
    }
    draw_info_box(
        display,
        state.display,
        "Recordings stay on the SD card.",
        INFO_TOP,
    )?;
    draw_bottom_bar(display, state.display, &AI_HINTS)
}

/// Hub rows: two cards plus up to three notes, as selection counts them.
#[must_use]
pub fn ai_row_count(state: &AppState) -> usize {
    2 + recent_notes(state).len()
}

/// The status bar's right text for the hub.
#[must_use]
pub fn network_label(state: &AppState) -> String {
    match state.network.wifi_state {
        crate::network::WifiConnectionState::Connected => "Online".into(),
        other => other.label().to_owned(),
    }
}

/// The catalog's newest three: entries sort by file name, so the newest
/// are the last ones reversed.
#[must_use]
pub fn recent_notes(state: &AppState) -> Vec<&VoiceNoteEntry> {
    state
        .voice_notes
        .notes
        .iter()
        .rev()
        .take(RECENT_COUNT)
        .collect()
}

/// `Oct 2 · 12:04 · 18 min` from a note's RTC stamp and duration.
#[must_use]
pub fn note_meta(note: &VoiceNoteEntry) -> String {
    let (date, time) = split_stamp(&note.recorded_at);
    let duration = if note.duration_seconds >= 60 {
        format!("{} min", note.duration_seconds / 60)
    } else {
        format!("{} s", note.duration_seconds)
    };
    match (date, time) {
        (Some(date), Some(time)) => format!("{date} · {time} · {duration}"),
        (Some(date), None) => format!("{date} · {duration}"),
        _ => duration,
    }
}

/// `2026-06-06  11:43:24` into (`Jun 6`, `11:43`); unknown stamps drop out.
#[must_use]
pub fn split_stamp(stamp: &str) -> (Option<String>, Option<String>) {
    let mut parts = stamp.split_whitespace();
    let date = parts.next().map(str::to_owned);
    let time = parts.next().map(str::to_owned);
    let date = date.and_then(|date| {
        let fields: Vec<_> = date.split('-').collect();
        if fields.len() != 3 {
            return None;
        }
        let month: usize = fields[1].parse().ok()?;
        let day: u8 = fields[2].parse().ok()?;
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let month = MONTHS.get(month.checked_sub(1)?)?;
        Some(format!("{month} {day}"))
    });
    let time = time.and_then(|time| {
        let fields: Vec<_> = time.split(':').collect();
        if fields.len() < 2 {
            return None;
        }
        Some(format!("{}:{}", fields[0], fields[1]))
    });
    (date, time)
}

fn draw_xiaozhi_card(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    selected: bool,
) -> Result<(), Infallible> {
    draw_card_frame(display, CARD_TOP, selected)?;
    draw_face_icon(display, Point::new(CARD_LEFT + 20, CARD_TOP + 20), selected)?;
    let name_style = card_text(state.display, selected, UiTextRole::Heading);
    let detail_style = card_text(state.display, selected, UiTextRole::Body);
    let name_left = CARD_LEFT + 100;
    Text::new("XiaoZhi", Point::new(name_left, CARD_TOP + 40), name_style).draw(display)?;
    Text::new(
        "Voice chat · xiaozhi.me",
        Point::new(name_left, CARD_TOP + 74),
        detail_style,
    )
    .draw(display)?;
    let badge_style = card_text(state.display, selected, UiTextRole::Body);
    let badge = "SOON";
    let badge_left = CARD_RIGHT - 16 - badge_style.text_width(badge);
    Text::new(badge, Point::new(badge_left, CARD_TOP + 40), badge_style).draw(display)?;
    Ok(())
}

fn draw_voice_notes_card(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    selected: bool,
) -> Result<(), Infallible> {
    let top = CARD_TOP + CARD_HEIGHT + CARD_GAP;
    draw_card_frame(display, top, selected)?;
    draw_mic_icon(display, Point::new(CARD_LEFT + 20, top + 20), selected)?;
    let name_style = card_text(state.display, selected, UiTextRole::Heading);
    let detail_style = card_text(state.display, selected, UiTextRole::Body);
    let name_left = CARD_LEFT + 100;
    Text::new("Voice Notes", Point::new(name_left, top + 40), name_style).draw(display)?;
    Text::new(
        "Record › transcript › summary",
        Point::new(name_left, top + 74),
        detail_style,
    )
    .draw(display)?;
    Ok(())
}

fn draw_note_row(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    note: &VoiceNoteEntry,
    top: i32,
    selected: bool,
) -> Result<(), Infallible> {
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(
        Point::new(CARD_LEFT, top),
        Size::new((CARD_RIGHT - CARD_LEFT) as u32, (NOTE_HEIGHT - 8) as u32),
    )
    .into_styled(style)
    .draw(display)?;
    let title_style = card_text(state.display, selected, UiTextRole::Heading);
    let meta_style = card_text(state.display, selected, UiTextRole::Body);
    let left = CARD_LEFT + 12;
    let width = CARD_RIGHT - left - 12;
    Text::new(
        &title_style.fit(&note.title, width),
        Point::new(left, top + 30),
        title_style,
    )
    .draw(display)?;
    Text::new(
        &meta_style.fit(&note_meta(note), width),
        Point::new(left, top + 54),
        meta_style,
    )
    .draw(display)?;
    Ok(())
}

fn draw_card_frame(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    selected: bool,
) -> Result<(), Infallible> {
    let style = if selected {
        PrimitiveStyle::with_fill(BinaryColor::On)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 2)
    };
    Rectangle::new(
        Point::new(CARD_LEFT, top),
        Size::new((CARD_RIGHT - CARD_LEFT) as u32, CARD_HEIGHT as u32),
    )
    .into_styled(style)
    .draw(display)?;
    Ok(())
}

/// The mockup's face: a frame, two eyes and a mouth bar.
fn draw_face_icon(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    selected: bool,
) -> Result<(), Infallible> {
    let ink = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::Off, 3)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 3)
    };
    let fill = if selected {
        PrimitiveStyle::with_fill(BinaryColor::Off)
    } else {
        PrimitiveStyle::with_fill(BinaryColor::On)
    };
    const SIZE: u32 = 56;
    Rectangle::new(top_left, Size::new(SIZE, SIZE))
        .into_styled(ink)
        .draw(display)?;
    for eye in [12_i32, 36_i32] {
        Rectangle::new(
            Point::new(top_left.x + eye, top_left.y + 16),
            Size::new(8, 8),
        )
        .into_styled(fill)
        .draw(display)?;
    }
    Rectangle::new(
        Point::new(top_left.x + 16, top_left.y + 38),
        Size::new(24, 4),
    )
    .into_styled(fill)
    .draw(display)?;
    Ok(())
}

/// The mockup's microphone: a capsule, a stem and a base.
fn draw_mic_icon(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    selected: bool,
) -> Result<(), Infallible> {
    let fill = if selected {
        PrimitiveStyle::with_fill(BinaryColor::Off)
    } else {
        PrimitiveStyle::with_fill(BinaryColor::On)
    };
    let stroke = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::Off, 3)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 3)
    };
    Rectangle::new(
        Point::new(top_left.x + 18, top_left.y + 2),
        Size::new(20, 30),
    )
    .into_styled(fill)
    .draw(display)?;
    Rectangle::new(
        Point::new(top_left.x + 27, top_left.y + 32),
        Size::new(3, 12),
    )
    .into_styled(fill)
    .draw(display)?;
    Rectangle::new(
        Point::new(top_left.x + 14, top_left.y + 46),
        Size::new(28, 3),
    )
    .into_styled(stroke)
    .draw(display)?;
    Ok(())
}

#[cfg(test)]
mod d28_tests {
    use super::{ai_row_count, network_label, note_meta, recent_notes, split_stamp};
    use crate::{
        app::{render_current_screen, router::ScreenRoute, AppState},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        voice_notes::VoiceNoteEntry,
    };

    fn note(name: &str, title: &str, stamp: &str, seconds: u32) -> VoiceNoteEntry {
        VoiceNoteEntry {
            file_name: name.into(),
            title: title.into(),
            recorded_at: stamp.into(),
            wav_bytes: 1_000_000,
            pcm_bytes: 500_000,
            duration_seconds: seconds,
        }
    }

    fn hub_state(notes: Vec<VoiceNoteEntry>) -> AppState {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Ai);
        state.voice_notes.notes = notes;
        state
    }

    #[test]
    fn recent_notes_takes_the_newest_three_reversed() {
        let state = hub_state(vec![
            note("NOTE_001.WAV", "Oldest", "2026-09-28  08:00:00", 60),
            note("NOTE_002.WAV", "Middle", "2026-09-30  09:15:00", 180),
            note("NOTE_003.WAV", "Later", "2026-10-01  07:30:00", 120),
            note("NOTE_004.WAV", "Newest", "2026-10-02  12:04:00", 1_080),
        ]);
        let recent = recent_notes(&state);
        let titles: Vec<_> = recent.iter().map(|entry| entry.title.as_str()).collect();
        assert_eq!(titles, ["Newest", "Later", "Middle"]);
        assert_eq!(ai_row_count(&state), 5);
    }

    #[test]
    fn note_meta_formats_stamp_and_duration() {
        let long = note("NOTE_001.WAV", "T", "2026-10-02  12:04:33", 1_080);
        assert_eq!(note_meta(&long), "Oct 2 · 12:04 · 18 min");
        let short = note("NOTE_002.WAV", "T", "2026-09-30  09:15:00", 45);
        assert_eq!(note_meta(&short), "Sep 30 · 09:15 · 45 s");
        let unknown = note("NOTE_003.WAV", "T", "unknown", 120);
        assert_eq!(note_meta(&unknown), "2 min");
    }

    #[test]
    fn split_stamp_drops_malformed_parts() {
        assert_eq!(
            split_stamp("2026-10-02  12:04:33"),
            (Some("Oct 2".into()), Some("12:04".into()))
        );
        assert_eq!(split_stamp("nope"), (None, None));
        assert_eq!(
            split_stamp("2026-13-40  99:99"),
            (None, Some("99:99".into()))
        );
    }

    #[test]
    fn network_label_follows_the_wifi_state() {
        let mut state = AppState::default();
        assert_eq!(network_label(&state), "NO CONFIG");
        state.network.wifi_state = crate::network::WifiConnectionState::Connected;
        assert_eq!(network_label(&state), "Online");
    }

    #[test]
    fn empty_catalog_renders_two_rows_without_panic() {
        let state = hub_state(Vec::new());
        assert_eq!(ai_row_count(&state), 2);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn full_hub_renders_with_selection_on_a_note() {
        let mut state = hub_state(vec![
            note("NOTE_001.WAV", "Oldest", "2026-09-28  08:00:00", 60),
            note("NOTE_002.WAV", "Newest", "2026-10-02  12:04:00", 1_080),
        ]);
        // Two Downs move from XiaoZhi onto the newest note.
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        assert_eq!(state.category_selection(ScreenRoute::Ai), 2);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }
}
