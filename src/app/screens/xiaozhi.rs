//! Home › AI › XiaoZhi: the call as the mockup draws it, plus connecting,
//! linking the device at xiaozhi.me and the end of the call.

use core::convert::Infallible;

use embedded_graphics::{
    geometry::Angle,
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Arc, Circle, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        state::AppState,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap, BOTTOM_BAR_TOP},
            display_numerals::{draw_numerals, Numerals},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    charset::glyph_index,
    orientation::OrientedFrameBuffer,
    xiaozhi::call::{CallPhase, XiaozhiUi},
};

use super::{
    games::draw_info_box,
    home::{draw_battery, BATTERY_HEIGHT, BATTERY_WIDTH},
};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;
const FACE_SIZE: i32 = 88;
const FACE_TOP: i32 = STATUS_BAR_HEIGHT + 18;
const NAME_BASELINE: i32 = FACE_TOP + FACE_SIZE + 40;
const CHIPS_TOP: i32 = NAME_BASELINE + 18;
const CHIP_HEIGHT: i32 = 30;
const MESSAGES_TOP: i32 = CHIPS_TOP + CHIP_HEIGHT + 18;
/// The volume row sits just above the bottom bar, as in the mockup.
const VOLUME_BASELINE: i32 = BOTTOM_BAR_TOP - 26;
const TOOL_BASELINE: i32 = VOLUME_BASELINE - 42;
/// Messages end above the tool line.
const MESSAGES_BOTTOM: i32 = TOOL_BASELINE - 30;
/// Lines of the question kept on screen; the reply takes the rest.
const QUESTION_LINES: usize = 3;

const TALK_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::Select, "interrupt"),
    (KeyCap::UpDown, "volume"),
    (KeyCap::Boot, "hold: hang up"),
];
const WAIT_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "hold: cancel")];
const ENDED_HINTS: [(KeyCap, &str); 2] = [
    (KeyCap::Select, "call again"),
    (KeyCap::Boot, "hold: back"),
];

/// The XiaoZhi call screen.
pub fn render_xiaozhi(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let call = &state.xiaozhi;
    let preferences = state.display;
    draw_call_status_bar(display, state)?;
    draw_face(display, call.phase == CallPhase::Speaking)?;
    let name = preferences.large_style();
    draw_centered(display, "XiaoZhi", NAME_BASELINE, name)?;
    let hints: &[(KeyCap, &str)] = match &call.phase {
        CallPhase::Idle | CallPhase::Connecting => {
            draw_centered(
                display,
                "Connecting…",
                CHIPS_TOP + 24,
                preferences.heading_style(),
            )?;
            draw_info_box(display, preferences, connecting_text(call), MESSAGES_TOP)?;
            &WAIT_HINTS
        }
        CallPhase::Linking { code } => {
            draw_linking(display, preferences, code)?;
            &WAIT_HINTS
        }
        CallPhase::Ended { error } => {
            draw_centered(
                display,
                "Call ended",
                CHIPS_TOP + 24,
                preferences.heading_style(),
            )?;
            if let Some(error) = error {
                draw_info_box(display, preferences, &screen_text(error), MESSAGES_TOP)?;
            }
            &ENDED_HINTS
        }
        CallPhase::Listening | CallPhase::Thinking | CallPhase::Speaking => {
            draw_conversation(display, preferences, call)?;
            &TALK_HINTS
        }
    };
    draw_bottom_bar(display, preferences, hints)
}

fn connecting_text(call: &XiaozhiUi) -> &'static str {
    if call.linked {
        "Wi-Fi first, then the XiaoZhi server."
    } else {
        "Wi-Fi first, then the XiaoZhi server. The first call links this device to your \
         account at xiaozhi.me."
    }
}

/// `● On call · 02:14` during the conversation, the battery at the right.
fn draw_call_status_bar(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let preferences = state.display;
    let call = &state.xiaozhi;
    let length = call
        .duration_label(call.clock_ms)
        .filter(|_| call.phase.is_talking());
    if let Some(length) = length {
        draw_status_bar(display, preferences, "")?;
        Circle::with_center(Point::new(22, STATUS_BAR_HEIGHT / 2), 12)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(display)?;
        let style = preferences.text_style(UiTextRole::Heading, BinaryColor::Off);
        let baseline = (STATUS_BAR_HEIGHT + style.cap_height()) / 2;
        let title = format!("On call · {length}");
        Text::new(&title, Point::new(36, baseline), style).draw(display)?;
    } else {
        draw_status_bar(display, preferences, "XiaoZhi")?;
    }
    if let Some(percent) = state.board.power.and_then(|power| power.battery_percent) {
        let text = format!("{percent}%");
        let right = draw_status_text(display, preferences, &text, STATUS_BAR_RIGHT)?;
        let left = right - 4 - BATTERY_WIDTH;
        let top = (STATUS_BAR_HEIGHT - BATTERY_HEIGHT) / 2;
        draw_battery(display, Point::new(left, top), percent)?;
    }
    Ok(())
}

/// The hub's face, larger; sound waves while XiaoZhi speaks.
fn draw_face(display: &mut OrientedFrameBuffer<'_>, speaking: bool) -> Result<(), Infallible> {
    let left = (480 - FACE_SIZE) / 2;
    let ink = PrimitiveStyle::with_stroke(BinaryColor::On, 4);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);
    Rectangle::new(
        Point::new(left, FACE_TOP),
        Size::new(FACE_SIZE as u32, FACE_SIZE as u32),
    )
    .into_styled(ink)
    .draw(display)?;
    for eye in [22, 54] {
        Rectangle::new(Point::new(left + eye, FACE_TOP + 26), Size::new(12, 12))
            .into_styled(fill)
            .draw(display)?;
    }
    Rectangle::new(Point::new(left + 26, FACE_TOP + 60), Size::new(36, 6))
        .into_styled(fill)
        .draw(display)?;
    if speaking {
        let middle = FACE_TOP + FACE_SIZE / 2;
        for (offset, diameter) in [(14, 40), (30, 72)] {
            let right_centre = Point::new(left + FACE_SIZE + offset - diameter / 2, middle);
            let left_centre = Point::new(left - offset + diameter / 2, middle);
            for (centre, start) in [(right_centre, -40.0), (left_centre, 140.0)] {
                Arc::with_center(
                    centre,
                    diameter as u32,
                    Angle::from_degrees(start),
                    Angle::from_degrees(80.0),
                )
                .into_styled(ink)
                .draw(display)?;
            }
        }
    }
    Ok(())
}

/// The code to type at xiaozhi.me, in large digits.
fn draw_linking(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    code: &str,
) -> Result<(), Infallible> {
    draw_centered(
        display,
        "Link this device",
        CHIPS_TOP + 24,
        preferences.heading_style(),
    )?;
    draw_info_box(
        display,
        preferences,
        "On xiaozhi.me, sign in, choose Add device and type this code.",
        MESSAGES_TOP,
    )?;
    let digits = screen_text(code);
    let baseline = MESSAGES_TOP + 110 + Numerals::Temperature.digit_height();
    draw_numerals(
        display,
        &digits,
        Numerals::Temperature,
        (LEFT, RIGHT),
        baseline,
        BinaryColor::On,
    )?;
    draw_centered(
        display,
        "Waiting for the code…",
        baseline + 60,
        preferences.body_style(),
    )
}

/// The turn chips, the question and the reply, the tool and the volume.
fn draw_conversation(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    call: &XiaozhiUi,
) -> Result<(), Infallible> {
    draw_chips(display, preferences, call)?;
    let mut top = MESSAGES_TOP;
    let question = if call.heard.is_empty() && call.phase == CallPhase::Listening {
        if call.muted {
            "The microphone is off. Short BOOT turns it on."
        } else {
            "Listening…"
        }
    } else {
        call.heard.as_str()
    };
    if !question.is_empty() {
        top = draw_message(display, preferences, "You", question, top, false)? + 12;
    }
    if !call.said.is_empty() {
        draw_message(display, preferences, "XiaoZhi", &call.said, top, true)?;
    }
    let body = preferences.body_style();
    if let Some(tool) = call.tool.as_deref() {
        let tool = format!("Device tool: {}", screen_text(tool));
        let line = body.fit(&tool, RIGHT - LEFT);
        Text::new(&line, Point::new(LEFT, TOOL_BASELINE), body).draw(display)?;
    }
    draw_volume(display, preferences, call.volume)
}

/// LISTEN › THINK › SPEAK, the current turn filled.
fn draw_chips(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    call: &XiaozhiUi,
) -> Result<(), Infallible> {
    let listen = if call.muted { "MIC OFF" } else { "LISTEN" };
    let chips = [
        (listen, CallPhase::Listening),
        ("THINK", CallPhase::Thinking),
        ("SPEAK", CallPhase::Speaking),
    ];
    let label = preferences.detail_style();
    let separator_width = label.text_width("›") + 20;
    let widths: Vec<i32> = chips
        .iter()
        .map(|(text, _)| label.text_width(text) + 24)
        .collect();
    let total = widths.iter().sum::<i32>() + 2 * separator_width;
    let mut left = (480 - total) / 2;
    let baseline = CHIPS_TOP + (CHIP_HEIGHT + label.cap_height()) / 2;
    for (index, ((text, phase), &width)) in chips.iter().zip(&widths).enumerate() {
        let on = call.phase == *phase;
        let (style, ink) = if on {
            (PrimitiveStyle::with_fill(BinaryColor::On), BinaryColor::Off)
        } else {
            (PrimitiveStyle::with_stroke(BinaryColor::On, 2), BinaryColor::On)
        };
        Rectangle::new(
            Point::new(left, CHIPS_TOP),
            Size::new(width as u32, CHIP_HEIGHT as u32),
        )
        .into_styled(style)
        .draw(display)?;
        let text_style = label.with_color(ink);
        Text::new(text, Point::new(left + 12, baseline), text_style).draw(display)?;
        left += width;
        if index + 1 < chips.len() {
            Text::new("›", Point::new(left + 10, baseline), label).draw(display)?;
            left += separator_width;
        }
    }
    Ok(())
}

/// A labelled message, filled black for XiaoZhi. Text that does not fit
/// keeps the start of a question and the end of a reply. Returns its bottom.
fn draw_message(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    label: &str,
    text: &str,
    top: i32,
    reply: bool,
) -> Result<i32, Infallible> {
    let (ink, style) = if reply {
        (BinaryColor::Off, PrimitiveStyle::with_fill(BinaryColor::On))
    } else {
        (BinaryColor::On, PrimitiveStyle::with_stroke(BinaryColor::On, 2))
    };
    let label_style = preferences.text_style(UiTextRole::Detail, ink);
    let body = preferences.text_style(UiTextRole::Body, ink);
    let label_height = i32::from(label_style.line_height());
    let line_height = i32::from(body.line_height()) + 2;
    let room = usize::try_from((MESSAGES_BOTTOM - top - 18 - label_height) / line_height)
        .unwrap_or(0)
        .max(1);
    let limit = if reply { room } else { room.min(QUESTION_LINES) };
    let mut lines = body.wrap(&screen_text(text), RIGHT - LEFT - 24);
    if lines.len() > limit {
        if reply {
            lines.drain(..lines.len() - limit);
            lines[0].insert_str(0, "… ");
        } else {
            lines.truncate(limit);
            if let Some(last) = lines.last_mut() {
                last.push_str(" …");
            }
        }
    }
    let height = 18 + label_height + lines.len() as i32 * line_height;
    Rectangle::new(
        Point::new(LEFT, top),
        Size::new((RIGHT - LEFT) as u32, height as u32),
    )
    .into_styled(style)
    .draw(display)?;
    let label_baseline = top + 10 + label_style.cap_height();
    Text::new(label, Point::new(LEFT + 12, label_baseline), label_style).draw(display)?;
    let mut baseline = top + 12 + label_height + body.cap_height();
    for line in &lines {
        Text::new(line, Point::new(LEFT + 12, baseline), body).draw(display)?;
        baseline += line_height;
    }
    Ok(top + height)
}

/// `Volume` and ten segments, filled up to the level.
fn draw_volume(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    volume: u8,
) -> Result<(), Infallible> {
    let label = preferences.heading_style();
    let end = Text::new("Volume", Point::new(LEFT, VOLUME_BASELINE), label).draw(display)?;
    let filled = i32::from(volume.min(100) + 5) / 10;
    let height = 16;
    let top = VOLUME_BASELINE - height;
    for segment in 0..10 {
        let style = if segment < filled {
            PrimitiveStyle::with_fill(BinaryColor::On)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 2)
        };
        Rectangle::new(
            Point::new(end.x + 16 + segment * 26, top),
            Size::new(20, height as u32),
        )
        .into_styled(style)
        .draw(display)?;
    }
    Ok(())
}

fn draw_centered(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    baseline: i32,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let width = style.text_width(text);
    Text::new(text, Point::new((480 - width) / 2, baseline), style).draw(display)?;
    Ok(())
}

/// The text without the characters the fonts lack, such as Chinese or
/// emoji, which would otherwise draw as `?`.
#[must_use]
pub fn screen_text(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_whitespace() || glyph_index(*character).is_some())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::screen_text;
    use crate::{
        app::{render_current_screen, router::ScreenRoute, AppState},
        framebuffer::FrameBuffer,
        xiaozhi::call::{CallEvent, CallPhase},
    };

    #[test]
    fn screen_text_drops_what_the_fonts_cannot_draw() {
        assert_eq!(screen_text("Hola 你好 «Salmo» 😀"), "Hola  «Salmo» ");
    }

    #[test]
    fn every_phase_renders() {
        let phases = [
            CallPhase::Idle,
            CallPhase::Connecting,
            CallPhase::Linking {
                code: "482913".into(),
            },
            CallPhase::Listening,
            CallPhase::Thinking,
            CallPhase::Speaking,
            CallPhase::Ended {
                error: Some("Could not reach the XiaoZhi server.".into()),
            },
        ];
        for phase in phases {
            let mut state = AppState::default();
            state.router.navigate_to(ScreenRoute::XiaoZhi);
            state.xiaozhi.apply(CallEvent::Phase(phase), 0);
            state.xiaozhi.apply(CallEvent::Heard("A question".repeat(30)), 1_000);
            state.xiaozhi.apply(CallEvent::Said("A long reply.".repeat(90)), 2_000);
            let mut frame = FrameBuffer::new_white();
            render_current_screen(&mut frame, &state).unwrap();
        }
    }
}
