//! SD Lua application catalog and native-canvas preview screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
};

use crate::{
    app::reader_typography::reader_body_style,
    app::{
        state::AppState,
        typography::{Text, UiTextRole, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap, BACK_HINTS, CHOOSE_HINTS, OPEN_HINTS},
            display_numerals::{draw_numerals, Numerals},
            game_sprites,
            header::draw_header,
            list_row::{draw_list_row, ListRow},
            status_row::{draw_status_row, StatusRow},
        },
    },
    games::{
        canvas::{CanvasTextStyle, DrawCommand},
        sudoku::SudokuStep,
    },
    lua_runtime::{event_bridge::LuaEventBridge, LUA_CATALOG_PAGE_SIZE},
    orientation::OrientedFrameBuffer,
    reader::{BookFont, BookFontSize, ReadingTheme},
};

const SUDOKU_ROW_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "row"),
    (KeyCap::Select, "choose"),
    (KeyCap::Boot, "hold: back"),
];

const SUDOKU_CELL_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "cell"),
    (KeyCap::Select, "choose"),
    (KeyCap::Boot, "back"),
];

const SUDOKU_NUMBER_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "number"),
    (KeyCap::Select, "place"),
    (KeyCap::Boot, "back"),
];

const TETRIS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "rotate"),
    (KeyCap::Boot, "drop"),
];

const RACE_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "lane"),
    (KeyCap::Select, "nitro"),
    (KeyCap::Boot, "pause"),
];

const RACE_ZEN_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "lane"),
    (KeyCap::Select, "step"),
    (KeyCap::Boot, "hold: back"),
];

const RACE_CRASH_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "choose"),
    (KeyCap::Select, "select"),
    (KeyCap::Boot, "hold: back"),
];

const DICE_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::Select, "roll"),
    (KeyCap::UpDown, "dice"),
    (KeyCap::Boot, "options · hold: back"),
];

const DICE_ROLLING_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "rolling, keys wait")];

const DICE_OPTIONS_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "choose"),
    (KeyCap::Select, "change"),
    (KeyCap::Boot, "hold: back"),
];

pub fn render_lua_apps(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let catalog = &state.lua_runtime.catalog;
    let selected = state.lua_runtime.selected;
    let page_start = (selected / LUA_CATALOG_PAGE_SIZE) * LUA_CATALOG_PAGE_SIZE;
    let pages = catalog.entries.len().max(1).div_ceil(LUA_CATALOG_PAGE_SIZE);
    let page = format!("{}/{}", (page_start / LUA_CATALOG_PAGE_SIZE) + 1, pages);
    let heading = state.display.heading_style();
    let body = state.display.body_style();

    draw_header(display, state.display, "SD LUA APPS", "RUST-OWNED CANVAS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "CATALOG",
            middle: &format!("{} apps", catalog.entries.len()),
            right: &page,
        },
    )?;
    Text::new("Select an SD-loaded app", Point::new(22, 158), heading).draw(display)?;
    Text::new("Hold BOOT to go back.", Point::new(22, 188), body).draw(display)?;

    if catalog.entries.is_empty() {
        Rectangle::new(Point::new(22, 232), Size::new(436, 228))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
        Text::new("No Lua apps found.", Point::new(44, 300), heading).draw(display)?;
        Text::new("Install /RUSTMIX/APPS/SUDOKU", Point::new(44, 350), body).draw(display)?;
        Text::new(
            catalog.warning.as_deref().unwrap_or("Catalog is empty"),
            Point::new(44, 400),
            state.display.detail_style(),
        )
        .draw(display)?;
    } else {
        for (visible_index, entry) in catalog
            .entries
            .iter()
            .skip(page_start)
            .take(LUA_CATALOG_PAGE_SIZE)
            .enumerate()
        {
            let top = 214 + visible_index as i32 * 86;
            let is_selected = page_start + visible_index == selected;
            let border = if is_selected {
                PrimitiveStyle::with_stroke(BinaryColor::On, 4)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };
            Rectangle::new(Point::new(22, top), Size::new(436, 76))
                .into_styled(border)
                .draw(display)?;
            Text::new(
                if is_selected { ">" } else { " " },
                Point::new(34, top + 35),
                body,
            )
            .draw(display)?;
            Text::new(
                &entry.manifest.name,
                Point::new(56, top + 31),
                state.display.navigation_style(),
            )
            .draw(display)?;
            Text::new(
                &format!(
                    "{}  {}  {}",
                    entry.directory_name,
                    entry.manifest.kind.marker().to_ascii_uppercase(),
                    entry.manifest.version
                ),
                Point::new(56, top + 62),
                body,
            )
            .draw(display)?;
            Text::new("SD", Point::new(408, top + 62), body).draw(display)?;
        }
    }

    draw_bottom_bar(display, state.display, &OPEN_HINTS)
}

pub fn render_lua_game(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let Some(session) = state.lua_runtime.session.as_ref() else {
        return render_lua_error(display, state);
    };
    for command in session.canvas.commands() {
        draw_command(display, state, command)?;
    }
    draw_bottom_bar(display, state.display, game_hints(session))
}

/// Hints for the game's current mode; game canvases leave the bar band free.
/// A Dice session answers from its screen; the previews of its three screens
/// hold a static bridge and are named by their session id.
fn game_hints(session: &crate::lua_runtime::LuaAppSession) -> &'static [(KeyCap, &'static str)] {
    if let LuaEventBridge::Dice(app) = &session.event_bridge {
        return match app.view() {
            crate::games::dice_session::View::Ready => &DICE_HINTS,
            crate::games::dice_session::View::Rolling { .. } => &DICE_ROLLING_HINTS,
            crate::games::dice_session::View::Options { .. } => &DICE_OPTIONS_HINTS,
        };
    }
    match session.entry.manifest.id.as_str() {
        "dice-options" => return &DICE_OPTIONS_HINTS,
        "dice-rolling" => return &DICE_ROLLING_HINTS,
        "dice" => return &DICE_HINTS,
        _ => {}
    }
    match &session.event_bridge {
        LuaEventBridge::Static => &BACK_HINTS,
        LuaEventBridge::Sudoku(game) => match game.step() {
            SudokuStep::Start => &CHOOSE_HINTS,
            SudokuStep::Row => &SUDOKU_ROW_HINTS,
            SudokuStep::Cell => &SUDOKU_CELL_HINTS,
            SudokuStep::Number => &SUDOKU_NUMBER_HINTS,
        },
        LuaEventBridge::Tetris(_) => &TETRIS_HINTS,
        // The mode list chooses, as Sudoku's start list does.
        LuaEventBridge::TetrisStart(_) => &CHOOSE_HINTS,
        // The crash screen's rows choose; the road plays.
        LuaEventBridge::Race(game) => {
            if game.game().is_over() {
                &RACE_CRASH_HINTS
            } else if game.game().mode() == crate::games::race::RaceMode::Zen {
                &RACE_ZEN_HINTS
            } else {
                &RACE_HINTS
            }
        }
        LuaEventBridge::RaceStart(_) => &CHOOSE_HINTS,
        // Answered above, from the screen the session is on.
        LuaEventBridge::Dice(_) => &DICE_HINTS,
    }
}

pub fn render_lua_error(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    draw_header(display, state.display, "LUA APP ERROR", "NATIVE FALLBACK")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: "SCRIPT",
            middle: "ERROR",
            right: "SAFE",
        },
    )?;
    Rectangle::new(Point::new(22, 210), Size::new(436, 270))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    Text::new("App could not open.", Point::new(44, 280), heading).draw(display)?;
    Text::new(
        state
            .lua_runtime
            .error
            .as_deref()
            .unwrap_or("No active app session"),
        Point::new(44, 350),
        body,
    )
    .draw(display)?;
    Text::new("Hold BOOT to return.", Point::new(44, 430), body).draw(display)?;
    draw_bottom_bar(display, state.display, &BACK_HINTS)
}

fn draw_command(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    command: &DrawCommand,
) -> Result<(), Infallible> {
    match command {
        DrawCommand::Clear => {}
        DrawCommand::Text { x, y, text, style } => {
            Text::new(text, Point::new(*x, *y), canvas_text_style(state, *style)).draw(display)?;
        }
        DrawCommand::TextRight {
            right,
            y,
            text,
            style,
        } => {
            let style = canvas_text_style(state, *style);
            let left = *right - style.text_width(text);
            Text::new(text, Point::new(left, *y), style).draw(display)?;
        }
        DrawCommand::Line { x1, y1, x2, y2 } => {
            Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
        }
        DrawCommand::Rect {
            x,
            y,
            width,
            height,
            filled,
        } => {
            let style = if *filled {
                PrimitiveStyle::with_fill(BinaryColor::On)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };
            Rectangle::new(
                Point::new(*x, *y),
                Size::new((*width).max(0) as u32, (*height).max(0) as u32),
            )
            .into_styled(style)
            .draw(display)?;
        }
        DrawCommand::Grid {
            x,
            y,
            columns,
            rows,
            cell_width,
            cell_height,
        } => {
            let width = i32::from(*columns) * *cell_width;
            let height = i32::from(*rows) * *cell_height;
            for column in 0..=i32::from(*columns) {
                let line_x = *x + column * *cell_width;
                Line::new(Point::new(line_x, *y), Point::new(line_x, *y + height))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(display)?;
            }
            for row in 0..=i32::from(*rows) {
                let line_y = *y + row * *cell_height;
                Line::new(Point::new(*x, line_y), Point::new(*x + width, line_y))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(display)?;
            }
        }
        DrawCommand::PaperRect {
            x,
            y,
            width,
            height,
            filled,
        } => {
            let style = if *filled {
                PrimitiveStyle::with_fill(BinaryColor::Off)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::Off, 2)
            };
            Rectangle::new(
                Point::new(*x, *y),
                Size::new((*width).max(0) as u32, (*height).max(0) as u32),
            )
            .into_styled(style)
            .draw(display)?;
        }
        DrawCommand::PaperLine { x1, y1, x2, y2 } => {
            Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 3))
                .draw(display)?;
        }
        DrawCommand::Sprite { x, y, sprite } => {
            game_sprites::draw_sprite(display, Point::new(*x, *y), *sprite, BinaryColor::On)?;
        }
        DrawCommand::Die {
            x,
            y,
            size,
            face,
            dashed,
            angle,
        } => {
            draw_die(display, *x, *y, *size, *face, *dashed, *angle)?;
        }
        DrawCommand::Numerals {
            text,
            left,
            right,
            baseline,
        } => {
            draw_numerals(
                display,
                text,
                Numerals::Temperature,
                (*left, *right),
                *baseline,
                BinaryColor::On,
            )?;
        }
        DrawCommand::TextCenter {
            center_x,
            baseline,
            text,
            style,
        } => {
            let style = canvas_text_style(state, *style);
            let left = *center_x - style.text_width(text) / 2;
            Text::new(text, Point::new(left, *baseline), style).draw(display)?;
        }
        DrawCommand::ListRow {
            top,
            title,
            value,
            selected,
        } => {
            draw_list_row(
                display,
                state.display,
                *top,
                ListRow {
                    title,
                    subtitle: "",
                    value,
                    selected: *selected,
                },
            )?;
        }
        DrawCommand::InfoBox { top, text } => {
            super::games::draw_info_box(display, state.display, text, *top)?;
        }
        DrawCommand::ChipRow { top, items } => {
            draw_chip_row(display, state.display, *top, items)?;
        }
    }
    Ok(())
}

/// Mockup chips with `▸` separators, measured by `widgets::chips` (Body
/// labels, 24 px tall — the figure's `.chip`).
fn draw_chip_row(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: crate::app::display::DisplayPreferences,
    top: i32,
    items: &[(String, bool)],
) -> Result<(), Infallible> {
    use crate::app::widgets::chips::{draw_chip_row_centered, Chip};

    let chips: Vec<Chip<'_>> = items
        .iter()
        .map(|(label, filled)| Chip::new(label, *filled))
        .collect();
    draw_chip_row_centered(display, top, preferences.extrabold_body_style(), &chips)?;
    Ok(())
}

fn canvas_text_style(state: &AppState, style: CanvasTextStyle) -> UiTextStyle {
    match style {
        CanvasTextStyle::Body => state.display.body_style(),
        CanvasTextStyle::Heading => state.display.heading_style(),
        CanvasTextStyle::Detail => state.display.detail_style(),
        CanvasTextStyle::Inverse => state.display.text_style(UiTextRole::Body, BinaryColor::Off),
        CanvasTextStyle::InverseHeading => state
            .display
            .text_style(UiTextRole::Heading, BinaryColor::Off),
        CanvasTextStyle::Player => player_digit_style(givens_style(state.display)),
        CanvasTextStyle::Givens => givens_style(state.display),
    }
}

/// Sudoku givens: Inter ExtraBold at the cell's digit size (mockup 800).
fn givens_style(preferences: crate::app::display::DisplayPreferences) -> UiTextStyle {
    preferences.extrabold_style()
}

/// One die as the mockup's `die-N` symbols: a rounded square with a thick
/// stroke and round pips, or its dashed rest outline, turned by `angle`
/// degrees about its centre.
fn draw_die(
    display: &mut OrientedFrameBuffer<'_>,
    x: i32,
    y: i32,
    size: i32,
    face: u8,
    dashed: bool,
    angle: i32,
) -> Result<(), Infallible> {
    use crate::games::die_shape::{
        outline, pip_centres, DASH_PERCENT, PIP_PERCENT, REST_STROKE_PERCENT, STROKE_PERCENT,
    };
    let (cx, cy) = (x + size / 2, y + size / 2);
    // The mockup's symbols draw a 100-unit square and its stroke inside a
    // 108-unit box, so the box `size` holds a square a little smaller.
    let side = size * 100 / 108;
    let points = outline(cx, cy, side, angle);
    if dashed {
        let width = (side * REST_STROKE_PERCENT / 100).max(2) as u32;
        let dash = side * DASH_PERCENT.0 / 100;
        let gap = side * DASH_PERCENT.1 / 100;
        stroke_path(display, &points, width, Some((dash.max(2), gap.max(2))))?;
        return Ok(());
    }
    let width = (side * STROKE_PERCENT / 100).max(2) as u32;
    stroke_path(display, &points, width, None)?;
    let pip = (side * PIP_PERCENT / 100).max(3) as u32;
    for (px, py) in pip_centres(face, cx, cy, side, angle) {
        Circle::new(Point::new(px - pip as i32 / 2, py - pip as i32 / 2), pip)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
    }
    Ok(())
}

/// A closed path drawn with a round-capped stroke of `width`. With
/// `dashes` as `(dash, gap)` lengths it draws a dashed line along the path.
fn stroke_path(
    display: &mut OrientedFrameBuffer<'_>,
    points: &[(i32, i32)],
    width: u32,
    dashes: Option<(i32, i32)>,
) -> Result<(), Infallible> {
    let ink = PrimitiveStyle::with_stroke(BinaryColor::On, width);
    let cap = PrimitiveStyle::with_fill(BinaryColor::On);
    let dot = |display: &mut OrientedFrameBuffer<'_>, at: Point| {
        Circle::new(at - Point::new(width as i32 / 2, width as i32 / 2), width)
            .into_styled(cap)
            .draw(display)
    };
    // `along` is how far the path has gone; a dash is drawn while
    // `along % (dash + gap)` is under `dash`.
    let mut along = 0_i32;
    for index in 0..points.len() {
        let (x1, y1) = points[index];
        let (x2, y2) = points[(index + 1) % points.len()];
        let (dx, dy) = (x2 - x1, y2 - y1);
        let length = isqrt(dx * dx + dy * dy).max(1);
        let Some((dash, gap)) = dashes else {
            Line::new(Point::new(x1, y1), Point::new(x2, y2))
                .into_styled(ink)
                .draw(display)?;
            dot(display, Point::new(x1, y1))?;
            continue;
        };
        // Walk the edge in steps of one pixel's worth of its length, drawing
        // the pieces that fall inside a dash.
        let period = dash + gap;
        let mut start = 0;
        while start < length {
            let phase = (along + start) % period;
            if phase < dash {
                let run = (dash - phase).min(length - start);
                let a = Point::new(x1 + dx * start / length, y1 + dy * start / length);
                let b = Point::new(
                    x1 + dx * (start + run) / length,
                    y1 + dy * (start + run) / length,
                );
                Line::new(a, b).into_styled(ink).draw(display)?;
                dot(display, a)?;
                dot(display, b)?;
                start += run;
            } else {
                start += period - phase;
            }
        }
        along += length;
    }
    Ok(())
}

/// Integer square root, rounded down.
fn isqrt(value: i32) -> i32 {
    let mut root = 0;
    while (root + 1) * (root + 1) <= value {
        root += 1;
    }
    root
}

/// Literata at the size whose capitals come closest to the givens', so the
/// player's digits match them in size but not in weight.
fn player_digit_style(givens: UiTextStyle) -> UiTextStyle {
    let style = |size| reader_body_style(BookFont::Literata, size, ReadingTheme::Classic);
    BookFontSize::ALL
        .into_iter()
        .map(style)
        .min_by_key(|candidate| (candidate.cap_height() - givens.cap_height()).abs())
        .unwrap_or_else(|| style(BookFontSize::Medium))
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        framebuffer::FrameBuffer,
        games::{canvas::NativeGameCanvas, refresh_policy::GameRefreshPlan},
        lua_runtime::{
            event_bridge::LuaEventBridge,
            manifest::{LuaAppEntry, LuaAppKind, LuaAppManifest},
            LuaAppSession,
        },
    };

    #[test]
    fn renders_native_canvas_without_exposing_panel_transport() {
        let mut canvas = NativeGameCanvas::default();
        canvas.grid(80, 220, 4, 4, 64, 64).unwrap();
        let mut state = AppState::default();
        state.lua_runtime.session = Some(LuaAppSession {
            entry: LuaAppEntry {
                directory_name: "HGRID".into(),
                directory: std::path::PathBuf::from("/tmp/HGRID"),
                manifest: LuaAppManifest {
                    id: "hello_grid".into(),
                    name: "Hello Grid".into(),
                    kind: LuaAppKind::Game,
                    entry: "MAIN.LUA".into(),
                    version: "1.0".into(),
                    input: vec![],
                },
            },
            source_bytes: 10,
            canvas,
            refresh_plan: GameRefreshPlan::PartialFullscreen { regions: vec![] },
            event_bridge: LuaEventBridge::Static,
        });
        state.router.navigate_to(ScreenRoute::LuaGame);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        assert!(frame.is_black(Point::new(220, 399)).is_some());
    }

    /// Zen's bottom bar says what its keys do: lane, step and hold: back.
    #[test]
    fn zen_race_shows_its_own_key_hints() {
        let mut canvas = NativeGameCanvas::default();
        let bridge = LuaEventBridge::load("race.init('zen', 3)", &mut canvas).unwrap();
        let session = crate::lua_runtime::LuaAppSession {
            entry: LuaAppEntry {
                directory_name: "RACE".into(),
                directory: std::path::PathBuf::from("/tmp/RACE"),
                manifest: LuaAppManifest {
                    id: "race".into(),
                    name: "Race".into(),
                    kind: LuaAppKind::Game,
                    entry: "MAIN.LUA".into(),
                    version: "1.0".into(),
                    input: vec![],
                },
            },
            source_bytes: 0,
            canvas,
            refresh_plan: GameRefreshPlan::PartialFullscreen { regions: vec![] },
            event_bridge: bridge,
        };
        let hints = super::game_hints(&session);
        let labels: Vec<&str> = hints.iter().map(|(_, label)| *label).collect();
        assert_eq!(labels, ["lane", "step", "hold: back"]);
    }

    /// Ink columns of `character` that carry any pixel, and the thickest
    /// horizontal run of ink (a stem) in the glyph.
    fn ink_metrics(font: &crate::app::typography::BitmapFont, character: char) -> (usize, usize) {
        use crate::charset::glyph_index;
        let glyph = font.glyphs[glyph_index(character).expect("charset has the digit")];
        let stride = (usize::from(glyph.width) + 7) / 8;
        let offset = glyph.offset as usize;
        let ink = |row: usize, column: usize| {
            let byte = font.bitmap[offset + row * stride + column / 8];
            byte & (0x80 >> (column % 8)) != 0
        };
        let mut columns = 0_usize;
        for column in 0..usize::from(glyph.width) {
            if (0..usize::from(glyph.height)).any(|row| ink(row, column)) {
                columns += 1;
            }
        }
        let mut stem = 0_usize;
        for row in 0..usize::from(glyph.height) {
            let mut run = 0_usize;
            for column in 0..usize::from(glyph.width) {
                if ink(row, column) {
                    run += 1;
                    stem = stem.max(run);
                } else {
                    run = 0;
                }
            }
        }
        (columns, stem)
    }

    /// Mockup `.c.g` is weight 800; the player's digits stay a book face.
    #[test]
    fn given_digits_have_thicker_stems_than_the_players() {
        let preferences = crate::app::display::DisplayPreferences::default();
        let givens = super::givens_style(preferences);
        let player = super::player_digit_style(givens);
        let (given_columns, given_stem) = ink_metrics(givens.font(), '8');
        let (player_columns, player_stem) = ink_metrics(player.font(), '8');
        assert!(
            given_stem > player_stem,
            "givens '8' stem {given_stem} must beat the player's {player_stem} \
             (ink columns {given_columns} vs {player_columns})"
        );
    }

    /// D84: chip labels and the status-bar title are heavier than the old
    /// Medium / SemiBold faces (ink run of one letter).
    #[test]
    fn chip_labels_and_bar_titles_are_extrabold() {
        use crate::app::display::DisplayPreferences;
        let preferences = DisplayPreferences::default();
        let letter = 'B';
        let chip = ink_metrics(preferences.extrabold_body_style().font(), letter);
        let body = ink_metrics(preferences.body_style().font(), letter);
        let bar = ink_metrics(preferences.extrabold_bar_style().font(), letter);
        let heading = ink_metrics(preferences.heading_style().font(), letter);
        assert!(
            chip.1 > body.1,
            "chip stem {} must beat Body {}",
            chip.1,
            body.1
        );
        assert!(
            bar.1 > heading.1,
            "bar stem {} must beat Heading {}",
            bar.1,
            heading.1
        );
    }

    /// D84: the widest chip rows and the longest bar title still fit.
    #[test]
    fn extrabold_chips_and_bar_titles_fit_every_ui_size() {
        use crate::app::display::{DisplayPreferences, UiFontSize};
        use crate::app::widgets::chips::{chip_row_width, Chip};
        const SUDOKU: [&str; 3] = ["1 · ROW", "2 · CELL", "3 · NUMBER"];
        const XIAOZHI: [&str; 3] = ["LISTEN", "THINK", "SPEAK"];
        const BAR: &str = "Settings › Sleep screen";
        for size in UiFontSize::ALL {
            let preferences = DisplayPreferences {
                font_size: size,
                ..DisplayPreferences::default()
            };
            let style = preferences.extrabold_body_style();
            for labels in [SUDOKU, XIAOZHI] {
                let chips: Vec<Chip<'_>> = labels.iter().map(|l| Chip::new(l, false)).collect();
                // Two 20 px arrow gaps between three chips.
                let total = chip_row_width(style, &chips, 0) + 40;
                assert!(total <= 480, "{size:?} {labels:?} row is {total}");
            }
            let bar = preferences.extrabold_bar_style();
            assert!(
                bar.text_width(BAR) <= 400,
                "{size:?}: bar title {} px",
                bar.text_width(BAR)
            );
        }
    }
}
