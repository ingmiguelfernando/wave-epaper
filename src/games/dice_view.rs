//! Dice screens on the native game canvas: ready, rolling and options.
//!
//! Layout follows the mockup's `Dice`, `Dice: tirando` and `Dice: opciones`
//! figures as closely as the canvas allows: the same bar, die boxes, total,
//! sum line, history row, info box and key bar. The engine holds no drawing.

use super::{
    canvas::{CanvasTextStyle, GameSprite, NativeGameCanvas},
    dice::{DiceGame, FRAMES, HISTORY_LEN, MAX_DICE},
    dirty_regions::GAME_CANVAS_WIDTH,
};

/// Status bar height (the mockup's `.sb`).
const BAR_HEIGHT: i32 = 44;
/// The dice SVG sits 14 px under the bar and 16 px from each side.
const DICE_AREA_TOP: i32 = 58;
const DICE_AREA_LEFT: i32 = 16;
const DICE_AREA_HEIGHT: i32 = 230;
/// One die in the mockup's SVG: 180 × 180 at (34, 22) and (234, 22).
const DIE_SIZE: i32 = 180;
const DIE_Y: i32 = DICE_AREA_TOP + 22;
/// Tumbling die in the mockup: 150 × 150, offset and turned inside the rest
/// outline.
const TUMBLE_SIZE: i32 = 150;
/// Height of the boxes of the frame strip and the dice-count strip: the
/// mockup's label over a 64 px picture (`.layouts`).
const STRIP_HEIGHT: i32 = 94;
/// A row of the options list: the app's list row (`draw_list_row`).
const OPTION_ROW: i32 = 72;
const OPTIONS_TOP: i32 = 54;
/// Total baseline under the dice area (`.total` starts 6 px after the SVG).
const TOTAL_BASELINE: i32 = DICE_AREA_TOP + DICE_AREA_HEIGHT + 96;
/// Sum line under the total.
const SUM_BASELINE: i32 = TOTAL_BASELINE + 36;
/// "Last rolls" label and the history chips.
const HISTORY_LABEL: i32 = SUM_BASELINE + 48;
const HISTORY_TOP: i32 = HISTORY_LABEL + 16;
const CHIP_WIDTH: i32 = 52;
const CHIP_HEIGHT: i32 = 40;
const CHIP_GAP: i32 = 6;
/// Info box under the history row.
const INFO_TOP: i32 = HISTORY_TOP + CHIP_HEIGHT + 28;

/// The options screen's cursor and values. The engine owns the dice count;
/// the rest is UI state the wiring will persist later.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiceOptions {
    /// Highlighted row of the list.
    pub selected: usize,
    pub sound: bool,
    pub shake: bool,
    /// 0 Low, 1 Medium, 2 High.
    pub shake_strength: u8,
}

impl Default for DiceOptions {
    fn default() -> Self {
        Self {
            selected: 0,
            sound: true,
            shake: true,
            shake_strength: 1,
        }
    }
}

impl DiceOptions {
    /// Row labels, in the mockup's order.
    #[must_use]
    pub const fn rows(&self) -> [&'static str; 6] {
        [
            "Dice",
            "Sound",
            "Shake to roll",
            "Shake strength",
            "Sides",
            "Back to Games",
        ]
    }

    /// Right-hand value for `row`, or empty for the exit row.
    #[must_use]
    pub fn value(&self, row: usize, dice: usize) -> String {
        match row {
            0 => dice.to_string(),
            1 => on_off(self.sound),
            2 => on_off(self.shake),
            3 => ["Low", "Medium", "High"]
                .get(usize::from(self.shake_strength))
                .copied()
                .unwrap_or("Medium")
                .to_string(),
            4 => "6".to_string(),
            _ => String::new(),
        }
    }
}

fn on_off(value: bool) -> String {
    if value { "On" } else { "Off" }.to_string()
}

/// Die left edges in the mockup's SVG (34 and 234) for two dice. Fewer or
/// more dice keep the same 180 px boxes, centred as a row.
fn die_lefts(count: usize) -> Vec<i32> {
    match count {
        1 => vec![(GAME_CANVAS_WIDTH - DIE_SIZE) / 2],
        2 => vec![DICE_AREA_LEFT + 34, DICE_AREA_LEFT + 234],
        _ => {
            // Three dice share the row at 130 px so they fit 480 px.
            let size = 130;
            let gap = 10;
            let total = 3 * size + 2 * gap;
            let left = (GAME_CANVAS_WIDTH - total) / 2;
            (0..3).map(|index| left + index * (size + gap)).collect()
        }
    }
}

fn die_edge(count: usize) -> i32 {
    if count >= 3 {
        130
    } else {
        DIE_SIZE
    }
}

/// The ready screen: status, dice, total, sum line, history, info box.
pub fn render_ready(
    game: &DiceGame,
    canvas: &mut NativeGameCanvas,
    _options: &DiceOptions,
) -> Result<(), String> {
    canvas.clear_frame();
    draw_status(canvas, &format!("{} dice", game.dice()))?;
    draw_dice_row(canvas, game, 0)?;
    // `.total` is the big monospace number; the sum line is under it.
    canvas.numerals(
        game.total().to_string(),
        16,
        GAME_CANVAS_WIDTH - 16,
        TOTAL_BASELINE,
    )?;
    canvas.text_center(
        GAME_CANVAS_WIDTH / 2,
        SUM_BASELINE,
        sum_line(game),
        CanvasTextStyle::Heading,
    )?;
    draw_history(canvas, game)?;
    canvas.info_box(
        INFO_TOP,
        "Shake the device to roll. Sound and shake can be turned off in Options.".to_string(),
    )?;
    canvas.request_refresh();
    Ok(())
}

/// One tumbling frame of a roll. `frame` is 1-based.
pub fn render_rolling(
    game: &DiceGame,
    canvas: &mut NativeGameCanvas,
    frame: u8,
) -> Result<(), String> {
    canvas.clear_frame();
    draw_status(canvas, "Rolling…")?;
    draw_dice_row(canvas, game, frame)?;
    // The mockup drops the total to 64 px and says the frame number.
    canvas.text_center(
        GAME_CANVAS_WIDTH / 2,
        TOTAL_BASELINE - 20,
        "Rolling…".to_string(),
        CanvasTextStyle::Heading,
    )?;
    canvas.text_center(
        GAME_CANVAS_WIDTH / 2,
        SUM_BASELINE - 8,
        format!("frame {frame} of {}", FRAMES),
        CanvasTextStyle::Body,
    )?;
    draw_frame_strip(canvas, frame)?;
    canvas.info_box(
        INFO_TOP + 40,
        "Three quick partial refreshes while the speaker plays the rattle, then a knock and the result. A full refresh every 10 rolls clears ghosting."
            .to_string(),
    )?;
    canvas.request_refresh();
    Ok(())
}

/// The options list, as the mockup's `Dice: opciones`.
pub fn render_options(
    game: &DiceGame,
    canvas: &mut NativeGameCanvas,
    options: &DiceOptions,
) -> Result<(), String> {
    canvas.clear_frame();
    canvas.rect(0, 0, GAME_CANVAS_WIDTH, BAR_HEIGHT, true)?;
    canvas.text(16, 30, "Dice · Options".into(), CanvasTextStyle::Inverse)?;
    for (row, title) in options.rows().iter().enumerate() {
        let top = OPTIONS_TOP + row as i32 * OPTION_ROW;
        let selected = row == options.selected;
        canvas.list_row(
            top,
            (*title).to_string(),
            options.value(row, game.dice()),
            selected,
        )?;
    }
    let strip_top = OPTIONS_TOP + options.rows().len() as i32 * OPTION_ROW + 18;
    draw_dice_count_strip(canvas, game.dice(), strip_top)?;
    canvas.info_box(
        strip_top + STRIP_HEIGHT + 14,
        "Shake strength is how hard a shake must be: Low rolls with a small flick, High needs a real shake. The motion sensor runs only while Dice is open."
            .to_string(),
    )?;
    canvas.request_refresh();
    Ok(())
}

fn draw_status(canvas: &mut NativeGameCanvas, right: &str) -> Result<(), String> {
    canvas.rect(0, 0, GAME_CANVAS_WIDTH, BAR_HEIGHT, true)?;
    canvas.text(16, 30, "Dice".into(), CanvasTextStyle::Inverse)?;
    if !right.is_empty() {
        canvas.text_right(
            GAME_CANVAS_WIDTH - 16,
            30,
            right.to_string(),
            CanvasTextStyle::Inverse,
        )?;
        // The mockup's speaker (`#spk`) sits left of the label, which is
        // about 8 px a character in the bar's face.
        let label = right.chars().count() as i32 * 8;
        let (width, height) = GameSprite::Speaker.size();
        let left = GAME_CANVAS_WIDTH - 16 - label - 8 - width;
        canvas.sprite(left, (BAR_HEIGHT - height) / 2, GameSprite::Speaker)?;
    }
    Ok(())
}

/// The dice row. `frame` 0 is settled; 1..=FRAMES is a tumble.
fn draw_dice_row(canvas: &mut NativeGameCanvas, game: &DiceGame, frame: u8) -> Result<(), String> {
    let count = game.dice();
    let edge = die_edge(count);
    let lefts = die_lefts(count);
    for (index, x) in lefts.iter().enumerate() {
        let face = game.faces().get(index).copied().unwrap_or(1);
        if frame == 0 {
            canvas.die(*x, DIE_Y, edge, face, false, 0)?;
        } else {
            // The dashed rest outline at the seat, the die turned inside it.
            canvas.die(*x, DIE_Y, edge, 0, true, 0)?;
            let tumble = if count >= 3 {
                TUMBLE_SIZE * 130 / DIE_SIZE
            } else {
                TUMBLE_SIZE
            };
            // Dice in the left half of the row are thrown to the right and
            // the others to the left, so none leaves the screen.
            let (dx, dy, angle) = tumble_pose(frame, index * 2 < count);
            let (dx, dy) = (dx * tumble / TUMBLE_SIZE, dy * tumble / TUMBLE_SIZE);
            let inset = (edge - tumble) / 2;
            canvas.die(
                *x + inset + dx,
                DIE_Y + inset + dy,
                tumble,
                face,
                false,
                angle,
            )?;
            draw_motion_lines(canvas, *x, edge)?;
        }
    }
    Ok(())
}

/// Offset from the centre of its seat, and turn in degrees, of a tumbling
/// die. Frame 2 is the mockup's (58, 40, -24°) and (250, 30, +31°) relative
/// to the seats (34, 22) and (234, 22); frame 1 is thrown further and
/// frame 3 has nearly settled.
fn tumble_pose(frame: u8, left: bool) -> (i32, i32, i32) {
    match (frame, left) {
        (1, true) => (30, 6, -42),
        (1, false) => (-6, -8, 52),
        (2, true) => (9, 3, -24),
        (2, false) => (1, 8, 31),
        (_, true) => (3, 0, -9),
        (_, false) => (-2, 3, 8),
    }
}

/// Motion lines of the mockup's rolling figure (`M10 92h30…`).
fn draw_motion_lines(canvas: &mut NativeGameCanvas, die_x: i32, edge: i32) -> Result<(), String> {
    let left_side = die_x + edge / 2 < GAME_CANVAS_WIDTH / 2;
    // The mockup's three strokes at y 92, 118, 144 in the SVG (top 58).
    for (svg_y, length) in [(92, 30), (118, 36), (144, 30)] {
        let y = DICE_AREA_TOP + svg_y;
        if left_side {
            canvas.line(10, y, 10 + length, y)?;
        } else {
            canvas.line(
                GAME_CANVAS_WIDTH - 12 - length,
                y,
                GAME_CANVAS_WIDTH - 12,
                y,
            )?;
        }
    }
    Ok(())
}

/// "4 + 6", as the mockup's `.sumline`.
fn sum_line(game: &DiceGame) -> String {
    game.faces()
        .iter()
        .map(|face| face.to_string())
        .collect::<Vec<_>>()
        .join(" + ")
}

fn draw_history(canvas: &mut NativeGameCanvas, game: &DiceGame) -> Result<(), String> {
    canvas.text(
        16,
        HISTORY_LABEL,
        "LAST ROLLS".into(),
        CanvasTextStyle::Detail,
    )?;
    let history = game.history();
    let shown = HISTORY_LEN.min(history.len());
    let total_width = shown as i32 * CHIP_WIDTH + shown.saturating_sub(1) as i32 * CHIP_GAP;
    let left = (GAME_CANVAS_WIDTH - total_width) / 2;
    for index in 0..shown {
        let chip_left = left + index as i32 * (CHIP_WIDTH + CHIP_GAP);
        let text = history[index].to_string();
        let selected = index == 0;
        canvas.rect(chip_left, HISTORY_TOP, CHIP_WIDTH, CHIP_HEIGHT, selected)?;
        let style = if selected {
            CanvasTextStyle::Inverse
        } else {
            CanvasTextStyle::Body
        };
        canvas.text_center(chip_left + CHIP_WIDTH / 2, HISTORY_TOP + 28, text, style)?;
    }
    Ok(())
}

/// One box of a strip: its label over a row of small dice (`size`, face,
/// turn). The chosen box has a second outline around it, as the mockup's
/// `.layouts .on`.
fn draw_strip_box(
    canvas: &mut NativeGameCanvas,
    left: i32,
    top: i32,
    label: &str,
    chosen: bool,
    dice: &[(i32, u8, i32)],
) -> Result<(), String> {
    const WIDTH: i32 = 144;
    canvas.rect(left, top, WIDTH, STRIP_HEIGHT, false)?;
    if chosen {
        canvas.rect(left - 4, top - 4, WIDTH + 8, STRIP_HEIGHT + 8, false)?;
    }
    canvas.text_center(
        left + WIDTH / 2,
        top + 22,
        label.to_string(),
        CanvasTextStyle::Detail,
    )?;
    // The dice share the box, centred, with a small gap.
    let gap = 8;
    let total: i32 = dice.iter().map(|d| d.0).sum::<i32>() + gap * (dice.len() as i32 - 1);
    let mut x = left + (WIDTH - total) / 2;
    for &(size, face, angle) in dice {
        let y = top + 30 + (STRIP_HEIGHT - 30 - size) / 2;
        canvas.die(x, y, size, face, false, angle)?;
        x += size + gap;
    }
    Ok(())
}

/// The three-frame strip of the rolling figure (`.layouts`): tilted,
/// tumbling, result.
fn draw_frame_strip(canvas: &mut NativeGameCanvas, active: u8) -> Result<(), String> {
    const BOXES: [(&str, u8, i32); 3] = [
        ("1 · tilted", 3, -30),
        ("2 · tumbling", 2, -24),
        ("3 · result", 4, 0),
    ];
    let top = SUM_BASELINE + 36;
    for (index, (label, face, angle)) in BOXES.iter().enumerate() {
        let left = 16 + index as i32 * 152;
        let chosen = u8::try_from(index).unwrap_or(0) + 1 == active;
        draw_strip_box(canvas, left, top, label, chosen, &[(46, *face, *angle)])?;
    }
    Ok(())
}

/// The dice-count strip of the options figure (`.layouts`): one, two and
/// three dice.
fn draw_dice_count_strip(
    canvas: &mut NativeGameCanvas,
    dice: usize,
    top: i32,
) -> Result<(), String> {
    let boxes: [(&str, Vec<(i32, u8, i32)>); 3] = [
        ("1 die", vec![(50, 5, 0)]),
        ("2 dice", vec![(46, 4, 0), (46, 6, 0)]),
        ("3 dice", vec![(36, 1, 0), (36, 3, 0), (36, 6, 0)]),
    ];
    for (index, (label, faces)) in boxes.iter().enumerate() {
        let left = 16 + index as i32 * 152;
        draw_strip_box(canvas, left, top, label, index + 1 == dice, faces)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::canvas::MAX_GAME_DRAW_COMMANDS;

    fn ready_game(dice: usize, seed: u32) -> DiceGame {
        let mut game = DiceGame::new(seed);
        game.set_dice(dice);
        game
    }

    #[test]
    fn ready_draws_every_die_inside_the_screen() {
        for dice in 1..=MAX_DICE {
            let game = ready_game(dice, 42);
            let mut canvas = NativeGameCanvas::default();
            render_ready(&game, &mut canvas, &DiceOptions::default()).unwrap();
            let faces = canvas
                .commands()
                .iter()
                .filter(|command| {
                    matches!(
                        command,
                        crate::games::canvas::DrawCommand::Die { dashed: false, .. }
                    )
                })
                .count();
            assert_eq!(faces, dice, "{dice} dice draw {faces} faces");
            assert!(
                canvas.commands().len() < MAX_GAME_DRAW_COMMANDS,
                "{dice} dice use {} commands",
                canvas.commands().len()
            );
        }
    }

    #[test]
    fn two_dice_sit_where_the_mockup_puts_them() {
        let game = ready_game(2, 42);
        let mut canvas = NativeGameCanvas::default();
        render_ready(&game, &mut canvas, &DiceOptions::default()).unwrap();
        let lefts: Vec<(i32, i32)> = canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                crate::games::canvas::DrawCommand::Die {
                    x,
                    size,
                    dashed: false,
                    ..
                } => Some((*x, *size)),
                _ => None,
            })
            .collect();
        assert_eq!(lefts, vec![(50, 180), (250, 180)]);
    }

    #[test]
    fn ready_is_a_pure_function_of_the_state() {
        let game = ready_game(2, 7);
        let options = DiceOptions::default();
        let mut left = NativeGameCanvas::default();
        let mut right = NativeGameCanvas::default();
        render_ready(&game, &mut left, &options).unwrap();
        render_ready(&game, &mut right, &options).unwrap();
        assert_eq!(left.commands(), right.commands());
    }

    #[test]
    fn the_three_frames_of_a_roll_are_not_equal() {
        let mut game = ready_game(3, 21);
        assert!(game.request_roll());
        let mut frames = Vec::new();
        for frame in 1..=FRAMES {
            let mut canvas = NativeGameCanvas::default();
            render_rolling(&game, &mut canvas, frame).unwrap();
            frames.push(canvas.commands().to_vec());
            assert!(game.advance());
        }
        assert_ne!(frames[0], frames[1], "frames 1 and 2 match");
        assert_ne!(frames[1], frames[2], "frames 2 and 3 match");
        assert_ne!(frames[0], frames[2], "frames 1 and 3 match");
    }

    #[test]
    fn a_roll_stays_inside_the_screen_and_under_the_command_limit() {
        let mut game = ready_game(3, 5);
        assert!(game.request_roll());
        let mut canvas = NativeGameCanvas::default();
        render_rolling(&game, &mut canvas, 1).unwrap();
        assert!(
            canvas.commands().len() < MAX_GAME_DRAW_COMMANDS,
            "rolling uses {} commands",
            canvas.commands().len()
        );
        for command in canvas.commands() {
            if let crate::games::canvas::DrawCommand::Die {
                x, y, size, angle, ..
            } = command
            {
                let pad = crate::games::die_shape::tilt_padding(*size, *angle);
                assert!(*x - pad >= 0, "die at {x} turned {angle} leaves the left");
                assert!(
                    *x + size + pad <= GAME_CANVAS_WIDTH,
                    "die at {x} leaves the right"
                );
                assert!(*y - pad >= 0 && *y + size + pad <= 800);
            }
        }
    }

    #[test]
    fn a_roll_turns_every_die_a_different_way_on_each_frame() {
        let mut game = ready_game(2, 9);
        assert!(game.request_roll());
        let mut angles = Vec::new();
        for frame in 1..=FRAMES {
            let mut canvas = NativeGameCanvas::default();
            render_rolling(&game, &mut canvas, frame).unwrap();
            let turned: Vec<i32> = canvas
                .commands()
                .iter()
                .filter_map(|command| match command {
                    crate::games::canvas::DrawCommand::Die {
                        dashed: false,
                        angle,
                        ..
                    } => Some(*angle),
                    _ => None,
                })
                .collect();
            // The two dice never lie at the same angle, and a tumbling die
            // is tilted at least a little.
            assert_eq!(turned.len(), 2 + 3, "two dice plus the strip's three");
            assert_ne!(turned[0], turned[1]);
            assert!(turned[0] != 0 && turned[1] != 0);
            angles.push((turned[0], turned[1]));
        }
        // Frame 2 is the mockup's -24 and +31 degrees.
        assert_eq!(angles[1], (-24, 31));
        assert!(angles[0].0.abs() > angles[2].0.abs(), "it settles");
    }

    #[test]
    fn the_status_bar_has_the_speaker_beside_its_label() {
        let game = ready_game(2, 1);
        let mut canvas = NativeGameCanvas::default();
        render_ready(&game, &mut canvas, &DiceOptions::default()).unwrap();
        let speaker = canvas.commands().iter().find_map(|command| match command {
            crate::games::canvas::DrawCommand::Sprite {
                x,
                y,
                sprite: GameSprite::Speaker,
            } => Some((*x, *y)),
            _ => None,
        });
        let (x, y) = speaker.expect("the ready screen draws the speaker");
        // Left of the right-hand label, inside the 44 px bar.
        assert!(x > 300 && x < 440, "x {x}");
        assert!(y >= 0 && y + GameSprite::Speaker.size().1 <= 44, "y {y}");
    }

    #[test]
    fn the_strips_show_dice_and_the_options_rows_do_not_overlap() {
        let game = ready_game(2, 3);
        let mut canvas = NativeGameCanvas::default();
        render_options(&game, &mut canvas, &DiceOptions::default()).unwrap();
        let dice: Vec<_> = canvas
            .commands()
            .iter()
            .filter(|command| matches!(command, crate::games::canvas::DrawCommand::Die { .. }))
            .collect();
        // One die, two and three in the strip: six small dice.
        assert_eq!(dice.len(), 6);
        let row_tops: Vec<i32> = canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                crate::games::canvas::DrawCommand::ListRow { top, .. } => Some(*top),
                _ => None,
            })
            .collect();
        assert_eq!(row_tops.len(), 6);
        for pair in row_tops.windows(2) {
            assert!(
                pair[1] - pair[0] >= OPTION_ROW,
                "rows overlap: {row_tops:?}"
            );
        }
        let strip_top = *row_tops.last().unwrap() + OPTION_ROW;
        let box_top = canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                crate::games::canvas::DrawCommand::Rect {
                    y,
                    height,
                    filled: false,
                    ..
                } if *height == STRIP_HEIGHT => Some(*y),
                _ => None,
            })
            .min()
            .unwrap();
        assert!(box_top >= strip_top, "the strip starts under the last row");
    }
}
