//! Home › Games hub: one card per SD game, with its live state as the
//! mockup shows it.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        router::ScreenRoute,
        state::AppState,
        typography::{Text, UiTextRole},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    games::sudoku::time_text,
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
    "Moves use the fast partial refresh; a full refresh now and then cleans ghosting.";

/// Shared hint set for this screen.
pub const GAMES_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "game"),
    (KeyCap::Select, "play"),
    (KeyCap::Boot, "hold: back"),
];

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
    let first = selected.saturating_sub(CARDS_SHOWN - 1) / CARDS_SHOWN * CARDS_SHOWN;
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

/// The mockup's two detail lines per game; other games read their manifest.
fn game_details(state: &AppState, manifest: &LuaAppManifest) -> (String, String) {
    let records = state.lua_runtime.records;
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
            let first = match records.tetris_zen {
                0 => "Zen".into(),
                best => format!("Zen · best {}", grouped(best)),
            };
            (first, "No gravity: pieces move when you press".into())
        }
        _ => (
            format!("Version {}", manifest.version),
            "SD card game".into(),
        ),
    }
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
            // One filled cell reads as a puzzle in progress.
            Rectangle::new(
                Point::new(top_left.x + 2 * cell + 2, top_left.y + cell + 2),
                Size::new((cell - 4) as u32, (cell - 4) as u32),
            )
            .into_styled(ink)
            .draw(display)?;
        }
        "tetris" => {
            // A 2×2 piece over a floor row, the mockup's stack.
            let block = ICON / 4;
            for (column, row) in [(1_i32, 0_i32), (2, 0), (1, 1), (2, 1), (0, 2), (1, 2)] {
                Rectangle::new(
                    Point::new(top_left.x + column * block, top_left.y + row * block),
                    Size::new((block - 2) as u32, (block - 2) as u32),
                )
                .into_styled(ink)
                .draw(display)?;
            }
        }
        _ => {
            Rectangle::new(top_left, Size::new(ICON as u32, ICON as u32))
                .into_styled(PrimitiveStyle::with_stroke(color, 2))
                .draw(display)?;
        }
    }
    Ok(())
}

/// The mockup's framed info box, as tall as its wrapped text.
pub(crate) fn draw_info_box(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    text: &str,
    top: i32,
) -> Result<(), Infallible> {
    let body = preferences.body_style();
    let lines = body.wrap(text, CARD_RIGHT - CARD_LEFT - 24);
    let line_height = i32::from(body.line_height()) + 2;
    let height = lines.len() as i32 * line_height + 18;
    Rectangle::new(
        Point::new(CARD_LEFT, top),
        Size::new((CARD_RIGHT - CARD_LEFT) as u32, height as u32),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
    .draw(display)?;
    let mut baseline = top + 10 + body.cap_height();
    for line in lines {
        Text::new(&line, Point::new(CARD_LEFT + 12, baseline), body).draw(display)?;
        baseline += line_height;
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
            ("hgrid", "Hello Grid"),
            ("mines", "Minesweeper"),
            ("sudoku", "Sudoku"),
            ("tetris", "Tetris"),
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
        // No save, no best.
        let manifest_id = state.lua_runtime.games()[2].id.clone();
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
        assert_eq!(second, "Best time 6:41 · Easy");
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
        let manifest_id = state.lua_runtime.games()[3].id.clone();
        let manifest = state
            .lua_runtime
            .catalog
            .entries
            .iter()
            .find(|entry| entry.manifest.id == manifest_id)
            .map(|entry| &entry.manifest)
            .unwrap();
        let (first, _) = super::game_details(&state, manifest);
        assert_eq!(first, "Zen");
        state.lua_runtime.records.tetris_zen = 18_950;
        let (first, second) = super::game_details(&state, manifest);
        assert_eq!(first, "Zen · best 18,950");
        assert_eq!(second, "No gravity: pieces move when you press");
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
        // Move to Tetris (4th card) and play it.
        state.apply(crate::buttons::ButtonEvent::Down);
        state.apply(crate::buttons::ButtonEvent::Down);
        state.apply(crate::buttons::ButtonEvent::Down);
        assert_eq!(state.category_selection(ScreenRoute::Games), 3);
        state.apply(crate::buttons::ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::LuaGame);
        assert!(state.lua_runtime.session.is_some());
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
            sudoku_easy: 0,
            sudoku_medium: 761,
            sudoku_hard: 0,
        };
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }
}
