//! Home › AI › XiaoZhi: the call as the mockup draws it, plus connecting,
//! linking the device at xiaozhi.me and the end of the call.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle},
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
    games::{draw_dashed_box, draw_info_box},
    home::{draw_battery, BATTERY_HEIGHT, BATTERY_WIDTH},
};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;
/// The face's ring: 96 px, as in the mockup's 150 × 107 drawing.
const FACE_DIAMETER: i32 = 96;
const FACE_CENTRE: Point = Point::new(240, STATUS_BAR_HEIGHT + 72);
const NAME_BASELINE: i32 = STATUS_BAR_HEIGHT + 148;
const CHIPS_TOP: i32 = NAME_BASELINE + 14;
/// The mockup's `.chip` height (see `widgets::chips`).
const CHIP_HEIGHT: i32 = 24;
const MESSAGES_TOP: i32 = CHIPS_TOP + CHIP_HEIGHT + 14;
const MESSAGE_GAP: i32 = 12;
/// The volume row sits above the bottom bar, as in the mockup.
const VOLUME_BASELINE: i32 = BOTTOM_BAR_TOP - 22;
/// Messages end here, leaving room for the tool line and the volume row.
const MESSAGES_BOTTOM: i32 = VOLUME_BASELINE - 84;
/// Lines of the question kept on screen; the reply takes the rest.
const QUESTION_LINES: usize = 3;
/// The mockup's left sound waves on the face grid; the right ones mirror
/// them.
const WAVES: [[(f32, f32); 3]; 2] = [
    [(-2.0, 42.0), (-12.0, 60.0), (-2.0, 78.0)],
    [(-14.0, 32.0), (-30.0, 60.0), (-14.0, 88.0)],
];

const TALK_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::Select, "interrupt"),
    (KeyCap::UpDown, "volume"),
    (KeyCap::Boot, "hold: hang up"),
];
const WAIT_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "hold: cancel")];
const ENDED_HINTS: [(KeyCap, &str); 2] =
    [(KeyCap::Select, "call again"), (KeyCap::Boot, "hold: back")];

/// The XiaoZhi call screen.
pub fn render_xiaozhi(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let call = &state.xiaozhi;
    let preferences = state.display;
    draw_call_status_bar(display, state)?;
    draw_face(display, FACE_CENTRE, FACE_DIAMETER, BinaryColor::On)?;
    if call.phase == CallPhase::Speaking {
        draw_waves(display)?;
    }
    let name = preferences.extrabold_style();
    draw_centered(display, "XiaoZhi", NAME_BASELINE, name)?;
    let hints: &[(KeyCap, &str)] = match &call.phase {
        CallPhase::Idle | CallPhase::Connecting => {
            draw_centered(
                display,
                "Connecting…",
                CHIPS_TOP + 24,
                preferences.heading_style(),
            )?;
            draw_info_box(display, preferences, &connecting_text(call), MESSAGES_TOP)?;
            &WAIT_HINTS
        }
        CallPhase::Linking { code } => {
            draw_linking(display, preferences, code, call.own_server.as_deref())?;
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

fn connecting_text(call: &XiaozhiUi) -> String {
    if let Some(host) = &call.own_server {
        format!("Wi-Fi first, then your server at {}.", screen_text(host))
    } else if call.linked {
        "Wi-Fi first, then the XiaoZhi server.".into()
    } else {
        "Wi-Fi first, then the XiaoZhi server. The first call links this device to your \
         account at xiaozhi.me."
            .into()
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

/// Places on the mockup's face drawing: a 120-unit grid whose ring, 108
/// units wide, is drawn `diameter` pixels wide around `centre`.
#[derive(Clone, Copy)]
struct FaceGrid {
    centre: Point,
    diameter: i32,
}

impl FaceGrid {
    fn length(self, units: f32) -> i32 {
        (units * self.diameter as f32 / 108.0).round() as i32
    }

    fn point(self, (x, y): (f32, f32)) -> Point {
        self.centre + Point::new(self.length(x - 60.0), self.length(y - 60.0))
    }

    /// A quadratic curve as round dots `stroke` wide, like the mockup's
    /// round-capped paths.
    fn curve(
        self,
        display: &mut OrientedFrameBuffer<'_>,
        [start, control, end]: [(f32, f32); 3],
        stroke: i32,
        color: BinaryColor,
    ) -> Result<(), Infallible> {
        let dot = PrimitiveStyle::with_fill(color);
        for step in 0..=32 {
            let t = step as f32 / 32.0;
            let u = 1.0 - t;
            let x = u * u * start.0 + 2.0 * u * t * control.0 + t * t * end.0;
            let y = u * u * start.1 + 2.0 * u * t * control.1 + t * t * end.1;
            Circle::with_center(self.point((x, y)), stroke as u32)
                .into_styled(dot)
                .draw(display)?;
        }
        Ok(())
    }
}

/// The mockup's face: a ring, two eyes and a smile.
pub(crate) fn draw_face(
    display: &mut OrientedFrameBuffer<'_>,
    centre: Point,
    diameter: i32,
    color: BinaryColor,
) -> Result<(), Infallible> {
    let grid = FaceGrid { centre, diameter };
    let stroke = grid.length(6.0).max(2);
    Circle::with_center(centre, diameter as u32)
        .into_styled(PrimitiveStyle::with_stroke(color, stroke as u32))
        .draw(display)?;
    for eye in [42.0, 78.0] {
        Circle::with_center(grid.point((eye, 50.0)), grid.length(15.0) as u32)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
    }
    let smile = [(38.0, 73.0), (60.0, 95.0), (82.0, 73.0)];
    grid.curve(display, smile, stroke, color)
}

/// Sound waves at both sides of the face while XiaoZhi speaks.
fn draw_waves(display: &mut OrientedFrameBuffer<'_>) -> Result<(), Infallible> {
    let grid = FaceGrid {
        centre: FACE_CENTRE,
        diameter: FACE_DIAMETER,
    };
    let stroke = grid.length(5.0);
    for wave in WAVES {
        grid.curve(display, wave, stroke, BinaryColor::On)?;
        let mirrored = wave.map(|(x, y)| (120.0 - x, y));
        grid.curve(display, mirrored, stroke, BinaryColor::On)?;
    }
    Ok(())
}

/// The code to type at xiaozhi.me, in large digits.
fn draw_linking(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    code: &str,
    own_server: Option<&str>,
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
        &if own_server.is_some() {
            "Type this code in your server's console to add the device.".to_owned()
        } else {
            "On xiaozhi.me, sign in, choose Add device and type this code.".to_owned()
        },
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
        top = draw_message(display, preferences, "YOU", question, top, false)? + MESSAGE_GAP;
    }
    if !call.said.is_empty() {
        top = draw_message(display, preferences, "XIAOZHI", &call.said, top, true)? + MESSAGE_GAP;
    }
    if let Some(tool) = call.tool.as_deref() {
        draw_tool(display, preferences, tool, top)?;
    }
    draw_volume(display, preferences, call.volume)
}

/// LISTEN ▸ THINK ▸ SPEAK, the current turn filled — the mockup's chip row.
fn draw_chips(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    call: &XiaozhiUi,
) -> Result<(), Infallible> {
    use crate::app::widgets::chips::{draw_chip_row_centered, Chip};

    let listen = if call.muted { "MIC OFF" } else { "LISTEN" };
    let chips = [
        (listen, CallPhase::Listening),
        ("THINK", CallPhase::Thinking),
        ("SPEAK", CallPhase::Speaking),
    ];
    let row: Vec<Chip<'_>> = chips
        .iter()
        .map(|(text, phase)| Chip::new(text, call.phase == *phase))
        .collect();
    draw_chip_row_centered(display, CHIPS_TOP, preferences.extrabold_body_style(), &row)?;
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
        (
            BinaryColor::On,
            PrimitiveStyle::with_stroke(BinaryColor::On, 2),
        )
    };
    // Mockup `.msg .lbl` is 12 px weight 800 letter-spaced; `.msg` is 21 px
    // regular (`message_style`).
    let label_style = preferences.extrabold_body_style().with_color(ink);
    let body = preferences.message_style().with_color(ink);
    let label_height = i32::from(label_style.line_height());
    let line_height = i32::from(body.line_height()) + 2;
    let room = usize::try_from((MESSAGES_BOTTOM - top - 27 - label_height) / line_height)
        .unwrap_or(0)
        .max(1);
    let limit = if reply {
        room
    } else {
        room.min(QUESTION_LINES)
    };
    let mut lines = body.wrap(&screen_text(text), RIGHT - LEFT - 32);
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
    // Border and padding as the mockup's: 12 px above and below, 16 at the
    // sides, 3 px under the label.
    let height = 27 + label_height + lines.len() as i32 * line_height;
    Rectangle::new(
        Point::new(LEFT, top),
        Size::new((RIGHT - LEFT) as u32, height as u32),
    )
    .into_styled(style)
    .draw(display)?;
    let label_baseline = top + 12 + label_style.cap_height();
    draw_spaced_text(
        display,
        label,
        Point::new(LEFT + 16, label_baseline),
        label_style,
    )?;
    let mut baseline = top + 15 + label_height + (line_height + body.cap_height()) / 2;
    for line in &lines {
        Text::new(line, Point::new(LEFT + 16, baseline), body).draw(display)?;
        baseline += line_height;
    }
    Ok(top + height)
}

/// Capitals with 1 px between letters, the mockup's `.lbl` letter-spacing
/// (the fonts have no tracking).
fn draw_spaced_text(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    at: Point,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let mut x = at.x;
    for character in text.chars() {
        Text::new(&character.to_string(), Point::new(x, at.y), style).draw(display)?;
        x += style.char_advance(character) + 1;
    }
    Ok(())
}

/// The device tool XiaoZhi used, in a dashed box under the messages. The
/// mockup's `⚙` is not in the fonts; a small ring stands in for it.
fn draw_tool(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    tool: &str,
    top: i32,
) -> Result<(), Infallible> {
    let style = preferences.detail_style();
    let height = i32::from(style.line_height()) + 16;
    let size = Size::new((RIGHT - LEFT) as u32, height as u32);
    draw_dashed_box(display, Point::new(LEFT, top), size)?;
    // Ring gear mark (the mockup's ⚙), 12 px, before the words.
    let cy = top + height / 2;
    Circle::with_center(Point::new(LEFT + 20, cy), 12)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    Circle::with_center(Point::new(LEFT + 20, cy), 4)
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    let mut text = format!("Device tool: {}", screen_text(tool));
    if tool.starts_with("bible.") {
        text.push_str(" from your SD");
    }
    let line = style.fit(&text, RIGHT - LEFT - 48);
    let baseline = top + (height + style.cap_height()) / 2;
    Text::new(&line, Point::new(LEFT + 34, baseline), style).draw(display)?;
    Ok(())
}

/// `Volume` and ten segments, filled up to the level.
fn draw_volume(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    volume: u8,
) -> Result<(), Infallible> {
    let label = preferences.body_style();
    let end = Text::new("Volume", Point::new(LEFT, VOLUME_BASELINE), label).draw(display)?;
    let filled = i32::from(volume.min(100) + 5) / 10;
    let top = VOLUME_BASELINE - 14;
    for segment in 0..10 {
        let style = if segment < filled {
            PrimitiveStyle::with_fill(BinaryColor::On)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 2)
        };
        let left = end.x + 10 + segment * 18;
        Rectangle::new(Point::new(left, top), Size::new(14, 16))
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
    use super::{connecting_text, screen_text};
    use crate::xiaozhi::call::XiaozhiUi;
    use crate::{
        app::{render_current_screen, router::ScreenRoute, AppState},
        framebuffer::FrameBuffer,
        xiaozhi::call::{CallEvent, CallPhase},
    };
    use embedded_graphics::prelude::Point;

    #[test]
    fn an_own_server_is_named_while_connecting() {
        let mut call = XiaozhiUi::default();
        assert!(connecting_text(&call).contains("xiaozhi.me"));
        call.own_server = Some("192.168.1.20:8002".into());
        let text = connecting_text(&call);
        assert!(text.contains("192.168.1.20:8002") && !text.contains("xiaozhi.me"));
    }

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
            state
                .xiaozhi
                .apply(CallEvent::Heard("A question".repeat(30)), 1_000);
            state
                .xiaozhi
                .apply(CallEvent::Said("A long reply.".repeat(90)), 2_000);
            state
                .xiaozhi
                .apply(CallEvent::Tool("bible.read(PSA 23)".into()), 3_000);
            let mut frame = FrameBuffer::new_white();
            render_current_screen(&mut frame, &state).unwrap();
        }
    }

    /// The mockup's call screen: ExtraBold name, a chip row with `▸`, the
    /// YOU/XIAOZHI boxes, a gear mark on the tool line and the volume bar.
    #[test]
    fn call_screen_draws_the_mockup_elements() {
        use crate::app::display::{DisplayPreferences, UiFontSize};
        use crate::app::typography::UiTextStyle;
        use crate::xiaozhi::call::CallPhase;

        // Name is weight 800 (`extrabold_style` is thicker than Heading).
        let preferences = DisplayPreferences::default();
        let name = preferences.extrabold_style();
        let heading = preferences.heading_style();
        let width = |style: UiTextStyle, text: &str| style.text_width(text);
        assert!(
            width(name, "XiaoZhi") > width(heading, "XiaoZhi"),
            "ExtraBold name must be wider than SemiBold at the same words"
        );

        // The call paints at every UI size with protocol-length text.
        for size in UiFontSize::ALL {
            let mut state = AppState::default();
            state.display.font_size = size;
            state.router.navigate_to(ScreenRoute::XiaoZhi);
            state
                .xiaozhi
                .apply(CallEvent::Phase(CallPhase::Speaking), 0);
            state
                .xiaozhi
                .apply(CallEvent::Heard("Léeme el Salmo 23".into()), 1_000);
            state.xiaozhi.apply(
                CallEvent::Said("«Jehová es mi pastor; nada me faltará…».".repeat(8)),
                2_000,
            );
            state
                .xiaozhi
                .apply(CallEvent::Tool("bible.read(\"Sal 23\")".into()), 3_000);
            let mut frame = FrameBuffer::new_white();
            render_current_screen(&mut frame, &state).unwrap();
            // The call dot is paper on the black status bar.
            let paper = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(false);
            let ink = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(true);
            assert!(paper(22, 22), "{size:?}: call dot");
            assert!(
                (super::CHIPS_TOP..super::CHIPS_TOP + super::CHIP_HEIGHT)
                    .any(|y| (200..280).any(|x| ink(x, y))),
                "{size:?}: chip row"
            );
            assert!(
                (super::VOLUME_BASELINE - 14..super::VOLUME_BASELINE + 4)
                    .any(|y| (30..200).any(|x| ink(x, y))),
                "{size:?}: volume row"
            );
        }
    }

    /// The tool line stands in for `⚙` with a ring at a known `top`.
    #[test]
    fn tool_line_has_the_gear_mark() {
        use crate::app::display::DisplayPreferences;
        use crate::orientation::{DisplayOrientation, OrientedFrameBuffer};

        let mut frame = FrameBuffer::new_white();
        let top = 400;
        {
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            super::draw_tool(
                &mut display,
                DisplayPreferences::default(),
                "bible.read(\"Sal 23\")",
                top,
            )
            .unwrap();
        }
        let black = |x: i32, y: i32| frame.is_black(Point::new(y, 479 - x)) == Some(true);
        let style = DisplayPreferences::default().detail_style();
        let height = i32::from(style.line_height()) + 16;
        let cy = top + height / 2;
        let cx = 16 + 20;
        // Circle::with_center takes a diameter: ring 12 (stroke 2), hub 4.
        let mut ring_ink = 0_u32;
        for dy in -7..=7 {
            for dx in -7..=7 {
                if black(cx + dx, cy + dy) {
                    ring_ink += 1;
                }
            }
        }
        assert!(ring_ink > 12, "gear mark ink, found {ring_ink}");
        assert!(black(cx, cy), "hub");
    }

    /// D85: body text is the regular ~21 px face; labels are ExtraBold and
    /// letter-spaced (wider than the same word in Detail).
    #[test]
    fn message_body_is_regular_and_labels_are_spaced_extrabold() {
        use crate::app::display::DisplayPreferences;
        let preferences = DisplayPreferences::default();
        let message = preferences.message_style();
        let heading = preferences.heading_style();
        // Regular message carries less ink than the old Heading SemiBold
        // (total pixels of a round letter, not just the stem run).
        let ink = |style: crate::app::typography::UiTextStyle, character: char| {
            let font = style.font();
            let glyph = font.glyphs[crate::charset::glyph_index(character).unwrap()];
            let stride = (usize::from(glyph.width) + 7) / 8;
            let offset = glyph.offset as usize;
            let mut count = 0_usize;
            for row in 0..usize::from(glyph.height) {
                for column in 0..usize::from(glyph.width) {
                    let byte = font.bitmap[offset + row * stride + column / 8];
                    if byte & (0x80 >> (column % 8)) != 0 {
                        count += 1;
                    }
                }
            }
            count
        };
        assert!(
            ink(message, 'e') < ink(heading, 'e'),
            "message 'e' ink {} must be lighter than Heading {}",
            ink(message, 'e'),
            ink(heading, 'e')
        );
        // Letter-spaced ExtraBold label is wider than Detail without gaps.
        let label = preferences.extrabold_body_style();
        let detail = preferences.detail_style();
        let spaced = "YOU"
            .chars()
            .map(|c| label.char_advance(c) + 1)
            .sum::<i32>()
            - 1;
        assert!(
            spaced > detail.text_width("YOU"),
            "spaced ExtraBold {spaced} must beat Detail {}",
            detail.text_width("YOU")
        );
    }
}
