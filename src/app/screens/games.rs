//! Home › Games hub: one card per SD game, with its live state as the
//! mockup shows it.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        router::ScreenRoute,
        state::AppState,
        typography::{Text, UiTextRole},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            list_row::draw_selection_marker,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    games::{race::distance_label, sudoku::time_text},
    lua_runtime::manifest::LuaAppManifest,
    orientation::OrientedFrameBuffer,
    regional::grouped,
};

/// One card's geometry, as the mockup's 120 px cards.
const CARD_HEIGHT: i32 = 120;
const CARD_GAP: i32 = 12;
const CARD_LEFT: i32 = 16;
const CARD_RIGHT: i32 = 464;
const CARDS_TOP: i32 = STATUS_BAR_HEIGHT + 14;
const CARDS_SHOWN: usize = 4;
/// What is true today: the panel refreshes the whole screen, not cells.
const INFO_TEXT: &str =
    "Only the cells that change are refreshed (partial refresh); a full refresh every few moves cleans ghosting.";

/// Shared hint set for this screen.
pub const GAMES_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "game"),
    (KeyCap::Select, "play"),
    (KeyCap::Boot, "options"),
];

/// First card of the scrolling window that keeps `selected` in view: the
/// cards scroll one at a time, and the window never runs past the last game.
#[must_use]
fn first_card(len: usize, selected: usize) -> usize {
    selected
        .saturating_sub(CARDS_SHOWN - 1)
        .min(len.saturating_sub(CARDS_SHOWN))
}

/// The Home › Games hub.
pub fn render_games_hub(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let games = state.lua_runtime.games();
    draw_status_bar(display, state.display, ScreenRoute::Games.label())?;
    let count = games.len().to_string();
    draw_status_text(display, state.display, &count, STATUS_BAR_RIGHT)?;
    if games.is_empty() {
        let body = state.display.body_style();
        let message = "No games on the SD card";
        Text::new(message, Point::new(CARD_LEFT, CARDS_TOP + 40), body).draw(display)?;
    }
    let selected = state
        .category_selection(ScreenRoute::Games)
        .min(games.len().saturating_sub(1));
    let first = first_card(games.len(), selected);
    for (visible, index) in (first..games.len().min(first + CARDS_SHOWN)).enumerate() {
        let top = CARDS_TOP + visible as i32 * (CARD_HEIGHT + CARD_GAP);
        draw_game_card(display, state, games[index], top, index == selected)?;
    }
    let info_top = CARDS_TOP + CARDS_SHOWN as i32 * (CARD_HEIGHT + CARD_GAP);
    draw_info_box(display, state.display, INFO_TEXT, info_top)?;
    draw_bottom_bar(display, state.display, &GAMES_HINTS)
}

/// One card: icon, name, two detail lines, inverted when selected.
fn draw_game_card(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    manifest: &LuaAppManifest,
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
    if selected {
        // The mockup's `▸` before the icon, as on Home and the lists.
        draw_selection_marker(display, CARD_LEFT + 4, top + CARD_HEIGHT / 2)?;
    }
    draw_icon(
        display,
        manifest,
        Point::new(CARD_LEFT + 20, top + 30),
        selected,
    )?;
    let name_style = card_text(state.display, selected, UiTextRole::Heading);
    let detail_style = card_text(state.display, selected, UiTextRole::Body);
    let name_left = CARD_LEFT + 100;
    Text::new(
        &name_style.fit(&manifest.name, CARD_RIGHT - name_left - 12),
        Point::new(name_left, top + 38),
        name_style,
    )
    .draw(display)?;
    let (first, second) = game_details(state, manifest);
    Text::new(
        &detail_style.fit(&first, CARD_RIGHT - name_left - 12),
        Point::new(name_left, top + 72),
        detail_style,
    )
    .draw(display)?;
    Text::new(
        &detail_style.fit(&second, CARD_RIGHT - name_left - 12),
        Point::new(name_left, top + 100),
        detail_style,
    )
    .draw(display)?;
    Ok(())
}

/// Paper-colored text when the card is selected, ink otherwise.
pub(crate) fn card_text(
    preferences: DisplayPreferences,
    selected: bool,
    role: UiTextRole,
) -> crate::app::typography::UiTextStyle {
    preferences.text_style(
        role,
        if selected {
            BinaryColor::Off
        } else {
            BinaryColor::On
        },
    )
}

/// The mockup's Dice icon: one die resting, one tilted above it
/// (`die-5` at (2, 14) 34 px, `die-3` at (26, 2) 32 px turned 14°).
fn draw_die_pair(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    color: BinaryColor,
) -> Result<(), Infallible> {
    use crate::games::die_shape::{outline, pip_centres, PIP_PERCENT, STROKE_PERCENT};
    let style = PrimitiveStyle::with_stroke(color, 3);
    let pip_style = PrimitiveStyle::with_fill(color);
    let mut one_die = |cx: i32, cy: i32, size: i32, face: u8, degrees: i32| {
        let points = outline(cx, cy, size, degrees);
        for pair in points.windows(2) {
            let _ = Line::new(
                Point::new(pair[0].0, pair[0].1),
                Point::new(pair[1].0, pair[1].1),
            )
            .into_styled(style)
            .draw(display);
        }
        if let Some(first) = points.first() {
            if let Some(last) = points.last() {
                let _ = Line::new(Point::new(last.0, last.1), Point::new(first.0, first.1))
                    .into_styled(style)
                    .draw(display);
            }
        }
        let pip = (size * PIP_PERCENT / 100).max(3) as u32;
        for (px, py) in pip_centres(face, cx, cy, size, degrees) {
            let _ = Circle::new(Point::new(px - pip as i32 / 2, py - pip as i32 / 2), pip)
                .into_styled(pip_style)
                .draw(display);
        }
        let _ = STROKE_PERCENT;
    };
    // Resting die face 5 in the lower left of the 60 × 60 icon.
    one_die(top_left.x + 19, top_left.y + 31, 34, 5, 0);
    // Tilted die face 3 above and to the right.
    one_die(top_left.x + 42, top_left.y + 18, 32, 3, 14);
    Ok(())
}

/// The mockup's two detail lines per game; other games read their manifest.
fn game_details(state: &AppState, manifest: &LuaAppManifest) -> (String, String) {
    let records = &state.lua_runtime.records;
    match manifest.id.as_str() {
        "sudoku" => {
            let Some(save) = state.lua_runtime.sudoku_save else {
                // Solving deletes the save, so the best times stay in view.
                let second = match records.sudoku_fastest() {
                    Some((difficulty, best)) => {
                        format!("Best time {} · {}", time_text(best), difficulty.label())
                    }
                    None => "No best time yet".into(),
                };
                return ("No game in progress".into(), second);
            };
            let filled = save.board.iter().filter(|&&cell| cell != 0).count();
            let first = format!("{} · in progress {filled}/81", save.difficulty.label());
            let second = match records.sudoku_best(save.difficulty) {
                0 => "Auto-saved".into(),
                best => format!("Best time {} · auto-saved", time_text(best)),
            };
            (first, second)
        }
        "tetris" => {
            // Zen and Classic each show their own best, one per line.
            let zen = match records.tetris_zen {
                0 => "Zen (no gravity)".into(),
                best => format!("Zen (no gravity) · best {}", grouped(best)),
            };
            let classic = match records.tetris_classic {
                0 => "Classic: slow gravity".into(),
                best => format!("Classic · best {}", grouped(best)),
            };
            (zen, classic)
        }
        "race" => {
            // The mockup's card: Classic over Zen, each with its best distance.
            let classic = match records.race_classic {
                0 => "Classic · no best yet".into(),
                best => format!("Classic · best {}", distance_label(best)),
            };
            let zen = match records.race_zen {
                0 => "Zen: the road moves on a key".into(),
                best => format!("Zen · best {}", distance_label(best)),
            };
            (classic, zen)
        }
        "dice" => {
            // The mockup's card: how to roll, and the last faces.
            let first = format!("Shake to roll · {} dice", crate::games::dice::DEFAULT_DICE);
            let second = dice_last_line(records.dice_last.as_deref());
            (first, second)
        }
        _ => (
            format!("Version {}", manifest.version),
            "SD card game".into(),
        ),
    }
}

/// `Last roll 4 + 6 = 10`, or `No rolls yet` when the card has no `dice_last`.
fn dice_last_line(faces: Option<&str>) -> String {
    let Some(faces) = faces else {
        return "No rolls yet".into();
    };
    let Some(parsed) = crate::games::records::parse_dice_faces(faces) else {
        return "No rolls yet".into();
    };
    if parsed.len() == 1 {
        return format!("Last roll {}", parsed[0]);
    }
    let joined = parsed
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(" + ");
    let total: u16 = parsed.iter().map(|face| u16::from(*face)).sum();
    format!("Last roll {joined} = {total}")
}

/// The 60 × 60 icon: Sudoku's 3 × 3 grid, Tetris' stacked blocks, or a
/// plain framed square for the other games.
fn draw_icon(
    display: &mut OrientedFrameBuffer<'_>,
    manifest: &LuaAppManifest,
    top_left: Point,
    selected: bool,
) -> Result<(), Infallible> {
    // Paper on the selected card's black fill, ink elsewhere.
    let color = if selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let ink = PrimitiveStyle::with_fill(color);
    const ICON: i32 = 60;
    match manifest.id.as_str() {
        "sudoku" => {
            let cell = ICON / 3;
            for column in 0..=3 {
                let x = top_left.x + column * cell;
                Rectangle::new(Point::new(x, top_left.y), Size::new(2, ICON as u32))
                    .into_styled(ink)
                    .draw(display)?;
            }
            for row in 0..=3 {
                let y = top_left.y + row * cell;
                Rectangle::new(Point::new(top_left.x, y), Size::new(ICON as u32, 2))
                    .into_styled(ink)
                    .draw(display)?;
            }
            // The mockup's sample digits in four cells.
            for (column, row, digit) in [(0, 0, "5"), (2, 0, "7"), (1, 1, "3"), (2, 2, "9")] {
                let x = top_left.x + column * cell + cell / 2 - 3;
                let y = top_left.y + row * cell + cell * 2 / 3;
                let text_style = crate::app::display::DisplayPreferences::default()
                    .text_style(UiTextRole::Detail, color);
                Text::new(digit, Point::new(x, y), text_style).draw(display)?;
            }
        }
        "tetris" => {
            // The mockup's staircase: three on the bottom, two above the
            // right, one on top (`die` figure's SVG rects at 16/60).
            let block = ICON / 3;
            for (column, row) in [(0_i32, 2_i32), (1, 2), (2, 2), (1, 1), (2, 1), (2, 0)] {
                Rectangle::new(
                    Point::new(top_left.x + column * block, top_left.y + row * block),
                    Size::new((block - 1) as u32, (block - 1) as u32),
                )
                .into_styled(ink)
                .draw(display)?;
            }
        }
        "race" => {
            // The player's car sprite, paper on the selected card.
            crate::app::widgets::game_sprites::draw_sprite(
                display,
                top_left + Point::new(-6, 4),
                crate::games::canvas::GameSprite::PlayerCar,
                color,
            )?;
        }
        "dice" => {
            // The mockup's icon: one die resting, one tilted above it.
            draw_die_pair(display, top_left, color)?;
        }
        _ => {
            Rectangle::new(top_left, Size::new(ICON as u32, ICON as u32))
                .into_styled(PrimitiveStyle::with_stroke(color, 2))
                .draw(display)?;
        }
    }
    Ok(())
}

/// The mockup's info box: a dashed frame as tall as its wrapped text. A
/// `\n` starts a new line.
pub(crate) fn draw_info_box(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    text: &str,
    top: i32,
) -> Result<(), Infallible> {
    let body = preferences.body_style();
    let lines: Vec<String> = text
        .split('\n')
        .flat_map(|paragraph| body.wrap(paragraph, CARD_RIGHT - CARD_LEFT - 24))
        .collect();
    let line_height = i32::from(body.line_height()) + 2;
    let height = lines.len() as i32 * line_height + 18;
    let size = Size::new((CARD_RIGHT - CARD_LEFT) as u32, height as u32);
    draw_dashed_box(display, Point::new(CARD_LEFT, top), size)?;
    let mut baseline = top + 10 + body.cap_height();
    for line in lines {
        Text::new(&line, Point::new(CARD_LEFT + 12, baseline), body).draw(display)?;
        baseline += line_height;
    }
    Ok(())
}

/// A 2 px dashed outline: 6 px dashes with 4 px gaps.
pub(crate) fn draw_dashed_box(
    display: &mut OrientedFrameBuffer<'_>,
    top_left: Point,
    size: Size,
) -> Result<(), Infallible> {
    let ink = PrimitiveStyle::with_fill(BinaryColor::On);
    let (width, height) = (size.width as i32, size.height as i32);
    for x in (0..width).step_by(10) {
        let dash = Size::new((width - x).min(6) as u32, 2);
        for y in [0, height - 2] {
            Rectangle::new(top_left + Point::new(x, y), dash)
                .into_styled(ink)
                .draw(display)?;
        }
    }
    for y in (0..height).step_by(10) {
        let dash = Size::new(2, (height - y).min(6) as u32);
        for x in [0, width - 2] {
            Rectangle::new(top_left + Point::new(x, y), dash)
                .into_styled(ink)
                .draw(display)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod d25_tests {
    use crate::{
        app::{menu::category_entries, render_current_screen, router::ScreenRoute, AppState},
        framebuffer::FrameBuffer,
        games::{records::GameRecords, sudoku_puzzles::SudokuDifficulty, sudoku_save::SudokuSave},
        lua_runtime::manifest::LuaAppKind,
    };

    fn hub_state_with_games() -> AppState {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Games);
        let manifest = |id: &str, name: &str| crate::lua_runtime::manifest::LuaAppManifest {
            id: id.into(),
            name: name.into(),
            kind: LuaAppKind::Game,
            entry: "MAIN.LUA".into(),
            version: "1.0".into(),
            input: vec![],
        };
        for (id, name) in [
            ("sudoku", "Sudoku"),
            ("tetris", "Tetris"),
            ("race", "Race"),
            ("dice", "Dice"),
        ] {
            state
                .lua_runtime
                .catalog
                .entries
                .push(crate::lua_runtime::manifest::LuaAppEntry {
                    directory_name: id.to_ascii_uppercase(),
                    directory: std::path::PathBuf::from("/sdcard/RUSTMIX/APPS"),
                    manifest: manifest(id, name),
                });
        }
        state.lua_runtime.catalog.warning = None;
        state
    }

    #[test]
    fn sudoku_card_reads_the_save_and_the_best_time() {
        let mut state = hub_state_with_games();
        // No save, no best. Sudoku is the first card in the mockup's order.
        let manifest_id = state.lua_runtime.games()[0].id.clone();
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap()
            .clone();
        let (first, second) = super::game_details(&state, &manifest);
        assert_eq!(first, "No game in progress");
        assert_eq!(second, "No best time yet");
        // A solve deleted the save; the fastest best stays on the card.
        state.lua_runtime.records.sudoku_hard = 1_500;
        state.lua_runtime.records.sudoku_easy = 401;
        let (_, second) = super::game_details(&state, &manifest);
        assert_eq!(second, "Best time 06:41 · Easy");
        // A running game and a best time.
        state.lua_runtime.sudoku_save = Some(SudokuSave {
            difficulty: SudokuDifficulty::Medium,
            puzzle: [0; 81],
            board: {
                let mut board = [0_u8; 81];
                for cell in board.iter_mut().take(35) {
                    *cell = 1;
                }
                board
            },
            seconds: 300,
        });
        state.lua_runtime.records.sudoku_medium = 761;
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap();
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Medium · in progress 35/81");
        assert_eq!(second, "Best time 12:41 · auto-saved");
    }

    #[test]
    fn tetris_card_groups_the_best_score() {
        let mut state = hub_state_with_games();
        let manifest_id = state.lua_runtime.games()[1].id.clone();
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap();
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Zen (no gravity)");
        assert_eq!(second, "Classic: slow gravity", "no Classic best yet");
        state.lua_runtime.records.tetris_zen = 18_950;
        state.lua_runtime.records.tetris_classic = 4_200;
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Zen (no gravity) · best 18,950");
        assert_eq!(second, "Classic · best 4,200");
    }

    /// The mockup's Race card: Classic over Zen, each with its best distance.
    #[test]
    fn race_card_shows_the_best_distances() {
        let mut state = hub_state_with_games();
        let manifest_id = state.lua_runtime.games()[2].id.clone();
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap();
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Classic · no best yet");
        assert_eq!(second, "Zen: the road moves on a key");
        state.lua_runtime.records.race_classic = 5_100;
        state.lua_runtime.records.race_zen = 2_400;
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Classic · best 5.1 km");
        assert_eq!(second, "Zen · best 2.4 km");
    }

    #[test]
    fn the_hub_selects_a_card_and_plays_it() {
        // Real samples, because opening a session reads MAIN.LUA from the
        // entry's directory.
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Games);
        state.lua_runtime.refresh_catalog_from_root(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sd-card/RUSTMIX/APPS"),
            true,
        );
        assert!(state.lua_runtime.catalog.is_available());
        // The cards sort by name: Hello Grid, Minesweeper, Race, Sudoku,
        // Tetris. Move to Tetris (5th card) and play it.
        for _ in 0..4 {
            state.apply(crate::buttons::ButtonEvent::Down);
        }
        assert_eq!(state.category_selection(ScreenRoute::Games), 4);
        state.apply(crate::buttons::ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::LuaGame);
        assert!(state.lua_runtime.session.is_some());
    }

    /// Games › Race › Classic through `AppState::apply` and `tick_lua_game`:
    /// the timer steps the road, the interval shrinks with the speed, a crash
    /// opens the crash screen, Race again starts a new run and the best
    /// distance is kept for `RECORDS.TXT`.
    #[test]
    fn race_plays_on_the_timer_crashes_and_races_again() {
        use crate::buttons::ButtonEvent;
        use crate::lua_runtime::event_bridge::LuaEventBridge;
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Games);
        state.lua_runtime.refresh_catalog_from_root(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sd-card/RUSTMIX/APPS"),
            true,
        );
        // Race is the fourth card in the sample catalog (Dice, Hello Grid,
        // Minesweeper, Race, Sudoku, Tetris).
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::LuaGame);
        // The mode list: Classic is the first row. The press time seeds the
        // game, so the run is the same every time.
        state.apply_with_clock(ButtonEvent::Select, 3);
        let mut clock = 0_u64;
        let mut stepped = false;
        let mut interval_at_start = None;
        let mut interval_after_km = None;
        // Careful driving: dodge into a lane that survives the next column,
        // then let the timer step the road.
        for _ in 0..3_000 {
            let (over, threatened) = match &state.lua_runtime.session {
                Some(session) => match &session.event_bridge {
                    LuaEventBridge::Race(app) => {
                        let game = app.game();
                        (game.is_over(), game.cell(game.player_lane(), 2).is_some())
                    }
                    _ => break,
                },
                None => break,
            };
            if over {
                break;
            }
            if threatened {
                let target = match &state.lua_runtime.session {
                    Some(session) => match &session.event_bridge {
                        LuaEventBridge::Race(app) => {
                            let game = app.game();
                            (0..4_usize)
                                .filter(|lane| {
                                    *lane != game.player_lane()
                                        && game.cell(*lane, 1).is_none()
                                        && game.cell(*lane, 2).is_none()
                                })
                                .min_by_key(|lane| lane.abs_diff(game.player_lane()))
                        }
                        _ => break,
                    },
                    None => break,
                };
                if let Some(target) = target {
                    let lane = match &state.lua_runtime.session {
                        Some(session) => match &session.event_bridge {
                            LuaEventBridge::Race(app) => app.game().player_lane(),
                            _ => break,
                        },
                        None => break,
                    };
                    if target < lane {
                        state.apply(ButtonEvent::Up);
                    } else if target > lane {
                        state.apply(ButtonEvent::Down);
                    }
                }
            }
            clock += 1_000;
            if state.tick_lua_game(clock) {
                stepped = true;
                // The runtime's interval, measured right after a step ran.
                // `None` on the step that crashes: the game is over.
                if let Some(due) = state.next_lua_game_tick_ms() {
                    let interval = due - clock;
                    let speed = match &state.lua_runtime.session {
                        Some(session) => match &session.event_bridge {
                            LuaEventBridge::Race(app) => app.game().speed(),
                            _ => break,
                        },
                        None => break,
                    };
                    if speed <= 1 {
                        interval_at_start = Some(interval);
                    } else if interval_after_km.is_none() {
                        interval_after_km = Some(interval);
                    }
                }
            }
            if interval_after_km.is_some() {
                break;
            }
        }
        assert!(stepped, "the timer stepped the road");
        assert_eq!(
            interval_at_start,
            Some(1_000),
            "one step a second at speed 1"
        );
        assert_eq!(
            interval_after_km,
            Some(900),
            "100 ms less after a kilometre"
        );
        // Stop dodging: the next obstacle in the lane ends the run.
        for _ in 0..300 {
            let over = match &state.lua_runtime.session {
                Some(session) => match &session.event_bridge {
                    LuaEventBridge::Race(app) => app.game().is_over(),
                    _ => false,
                },
                None => false,
            };
            if over {
                break;
            }
            clock += 1_000;
            state.tick_lua_game(clock);
        }
        // The crash screen's rows: Race again is the first one.
        state.apply(ButtonEvent::Select);
        let distance = match &state.lua_runtime.session {
            Some(session) => match &session.event_bridge {
                LuaEventBridge::Race(app) => {
                    assert!(!app.game().is_over(), "Race again starts a new run");
                    app.game().distance_m()
                }
                _ => panic!("the race is open"),
            },
            None => panic!("the race is open"),
        };
        assert_eq!(distance, 0, "the new run starts at the start line");
        // Leaving keeps the best distance for RECORDS.TXT.
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Games);
        assert!(
            state.lua_runtime.records.race_classic > 0,
            "the best is kept"
        );
    }

    /// The crash screen's Mode row keeps the run's best before the mode list
    /// replaces the game, and Back to Games leaves as hold BOOT does.
    #[test]
    fn race_crash_rows_keep_the_best_and_leave() {
        use crate::buttons::ButtonEvent;
        use crate::lua_runtime::event_bridge::LuaEventBridge;
        fn crash(state: &mut AppState, clock: &mut u64) {
            for _ in 0..3_000 {
                let over = match &state.lua_runtime.session {
                    Some(session) => match &session.event_bridge {
                        LuaEventBridge::Race(app) => app.game().is_over(),
                        _ => panic!("the race is open"),
                    },
                    None => panic!("the race is open"),
                };
                if over {
                    return;
                }
                *clock += 1_000;
                state.tick_lua_game(*clock);
            }
            panic!("the run never crashed");
        }
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Games);
        state.lua_runtime.refresh_catalog_from_root(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sd-card/RUSTMIX/APPS"),
            true,
        );
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        state.apply_with_clock(ButtonEvent::Select, 3);
        let mut clock = 1_000_u64;
        state.tick_lua_game(clock);
        assert!(state.next_lua_game_tick_ms().is_some());
        state.apply_lua_game_boot_short_press();
        assert_eq!(state.next_lua_game_tick_ms(), None, "no timer while paused");
        state.apply_lua_game_boot_short_press();
        crash(&mut state, &mut clock);

        // Mode: the list replaces the game, and the best is already kept.
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert!(matches!(
            state
                .lua_runtime
                .session
                .as_ref()
                .map(|session| &session.event_bridge),
            Some(LuaEventBridge::RaceStart(_))
        ));
        assert!(
            state.lua_runtime.records.race_classic > 0,
            "the best is kept"
        );

        // Classic again, a crash, then Back to Games (row 2).
        state.apply_with_clock(ButtonEvent::Select, clock + 7);
        crash(&mut state, &mut clock);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Games);
        assert!(state.lua_runtime.session.is_none());
    }

    #[test]
    fn item_counts_use_singular_and_plural() {
        assert_eq!(crate::app::screens::category::item_count_text(1), "1 item");
        assert_eq!(crate::app::screens::category::item_count_text(4), "4 items");
        assert_eq!(category_entries(ScreenRoute::Games).len(), 0);
    }

    #[test]
    fn the_hub_renders_a_full_screen() {
        let mut state = hub_state_with_games();
        state.lua_runtime.sudoku_save = Some(SudokuSave {
            difficulty: SudokuDifficulty::Medium,
            puzzle: [0; 81],
            board: [0; 81],
            seconds: 10,
        });
        state.lua_runtime.records = GameRecords {
            tetris_zen: 18_950,
            tetris_classic: 4_200,
            race_classic: 5_100,
            race_zen: 2_400,
            sudoku_easy: 0,
            sudoku_medium: 761,
            sudoku_hard: 0,
            dice_last: None,
            dice_options: None,
        };
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }

    /// The cards scroll one at a time: for one to nine games and every
    /// selection the window holds the selected card and never runs past the
    /// last game.
    #[test]
    fn the_card_window_keeps_the_selection_and_stays_in_range() {
        for len in 1..=9_usize {
            for selected in 0..len {
                let first = super::first_card(len, selected);
                assert!(first <= selected, "{len} games, selection {selected}");
                assert!(
                    selected < first + super::CARDS_SHOWN,
                    "{len} games, selection {selected}: the selection is drawn"
                );
                if len >= super::CARDS_SHOWN {
                    assert!(
                        first + super::CARDS_SHOWN <= len,
                        "{len} games, selection {selected}: the window stops at the last game"
                    );
                } else {
                    assert_eq!(first, 0, "a short list never scrolls");
                }
            }
        }
        // ▼ on the last card shows it at the bottom; ▲ on the first at the top.
        assert_eq!(
            super::first_card(5, 4),
            1,
            "the last card sits in the last slot"
        );
        assert_eq!(
            super::first_card(5, 0),
            0,
            "the first card sits in the first slot"
        );
    }

    /// The hub with the fourth of four games (Dice) selected draws its
    /// inverted card in the fourth slot, as the mockup's figure.
    #[test]
    fn the_fourth_of_four_games_shows_in_the_fourth_slot() {
        use embedded_graphics::prelude::Point;
        let mut state = hub_state_with_games();
        for _ in 0..3 {
            state.apply(crate::buttons::ButtonEvent::Down);
        }
        assert_eq!(state.category_selection(ScreenRoute::Games), 3);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        // The frame stores the portrait screen rotated: (logical y, 479 - x).
        // The fourth slot's card is filled black where nothing is written.
        assert_eq!(
            frame.is_black(Point::new(566, 479 - 455)),
            Some(true),
            "the selected card fills the fourth slot"
        );
        // The mockup's `▸` leads the selected card: paper inside the marker,
        // ink just above it, at the card's vertical middle (y 454 + 60).
        let cy = 454 + super::CARD_HEIGHT / 2;
        let left = super::CARD_LEFT + 4;
        assert_eq!(
            frame.is_black(Point::new(cy, 479 - (left + 2))),
            Some(false),
            "the marker is paper"
        );
        assert_eq!(
            frame.is_black(Point::new(cy - 12, 479 - (left + 2))),
            Some(true),
            "ink above the marker"
        );
        // The first slot's card keeps its white interior.
        assert_eq!(
            frame.is_black(Point::new(170, 479 - 455)),
            Some(false),
            "the first slot holds an unselected card"
        );
    }

    /// The mockup's Dice card: how to roll, and the last faces with the total.
    #[test]
    fn dice_card_shows_the_shake_line_and_the_last_roll() {
        let mut state = hub_state_with_games();
        let manifest_id = state.lua_runtime.games()[3].id.clone();
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap()
            .clone();
        let (first, second) = super::game_details(&state, &manifest);
        assert_eq!(first, "Shake to roll · 2 dice");
        assert_eq!(second, "No rolls yet", "no dice_last yet");
        state.lua_runtime.records.dice_last = Some("4+6".into());
        let (_, second) = super::game_details(&state, &manifest);
        assert_eq!(second, "Last roll 4 + 6 = 10");
        state.lua_runtime.records.dice_last = Some("5".into());
        let (_, second) = super::game_details(&state, &manifest);
        assert_eq!(second, "Last roll 5", "one die needs no sum");
        state.lua_runtime.records.dice_last = Some("1+2+3".into());
        let (_, second) = super::game_details(&state, &manifest);
        assert_eq!(second, "Last roll 1 + 2 + 3 = 6");
    }

    /// The bundled `DICE/APP.TOM` parses with the loader's own parser.
    #[test]
    fn the_dice_app_manifest_parses() {
        let source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sd-card/RUSTMIX/APPS/DICE/APP.TOM"
        ))
        .expect("examples/sd-card/RUSTMIX/APPS/DICE/APP.TOM");
        let manifest = crate::lua_runtime::manifest::LuaAppManifest::parse(&source).unwrap();
        assert_eq!(manifest.id, "dice");
        assert_eq!(manifest.name, "Dice");
        assert_eq!(manifest.entry, "MAIN.LUA");
    }

    /// The Tetris hub icon is the mockup's staircase: three blocks on the
    /// bottom row, two above the right, one on top, and nothing in the three
    /// other cells of its 3 x 3 grid.
    #[test]
    fn the_tetris_icon_is_a_staircase_of_six_blocks() {
        use embedded_graphics::prelude::Point;
        let state = hub_state_with_games();
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        // Sudoku is selected and first; Tetris is the unselected second card.
        use crate::app::screens::games::{CARDS_TOP, CARD_GAP, CARD_HEIGHT, CARD_LEFT};
        let top = CARDS_TOP + CARD_HEIGHT + CARD_GAP;
        let (left, up) = (CARD_LEFT + 20, top + 30);
        let filled = [(0, 2), (1, 2), (2, 2), (1, 1), (2, 1), (2, 0)];
        for row in 0..3 {
            for column in 0..3 {
                // The frame stores the portrait screen as (y, 479 - x).
                let x = left + column * 20 + 9;
                let y = up + row * 20 + 9;
                assert_eq!(
                    frame.is_black(Point::new(y, 479 - x)),
                    Some(filled.contains(&(column, row))),
                    "cell column {column}, row {row}"
                );
            }
        }
    }
}
