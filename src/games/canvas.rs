//! Bounded native draw-command canvas for removable-storage applications.

use super::dirty_regions::{DirtyRect, DirtyRegions, GAME_CANVAS_HEIGHT, GAME_CANVAS_WIDTH};

pub const MAX_GAME_DRAW_COMMANDS: usize = 256;
pub const MAX_GAME_TEXT_BYTES: usize = 160;
/// The mockup's `.chip` height (2 px border + 2 px padding + a 13 px label).
pub const CHIP_ROW_HEIGHT: i32 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasTextStyle {
    Body,
    Heading,
    Detail,
    /// Paper-colored text for the black status bar and key chips.
    Inverse,
    /// Paper-colored Heading text, for digits on a black cell.
    InverseHeading,
    /// The player's digits: a lighter book face, mapped to the Reader body style.
    Player,
    /// Sudoku givens: the mockup's weight 800.
    Givens,
}

/// Named sprite shapes the game canvas draws in one command, from the
/// mockup's Race symbols: cars seen from above, a cone, the impact star and
/// the player's speed streaks. Each stays inside its own size, so the cells
/// a step dirties cover what changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameSprite {
    /// Another car: a 3 px outline with paper inside (`race-traffic`).
    Traffic,
    /// The player's car: solid with paper windows (`race-car`).
    PlayerCar,
    /// One cone; a cone obstacle draws two (`race-cone`).
    Cone,
    /// The impact star: an ink star with a paper star inside (`race-burst`).
    Burst,
    /// The dashed rounded outline of the cell Up would move to.
    MoveHint,
    /// Three speed streaks in the cell behind the player's car.
    Streaks,
    /// The player's car turned 14 degrees, for the crash picture.
    CrashCar,
    /// A small speaker with two sound waves, in paper color for the black
    /// status bar (`#spk`). Unlike the others it ignores the body color.
    Speaker,
}

impl GameSprite {
    /// The sprite's bounding size; it draws no pixel outside it.
    #[must_use]
    pub const fn size(self) -> (i32, i32) {
        match self {
            Self::Traffic | Self::PlayerCar | Self::MoveHint => (72, 53),
            Self::Cone => (34, 42),
            Self::Burst => (60, 60),
            Self::Streaks => (38, 34),
            Self::CrashCar => (84, 70),
            Self::Speaker => (24, 20),
        }
    }
}

impl CanvasTextStyle {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "body" => Some(Self::Body),
            "heading" => Some(Self::Heading),
            "detail" => Some(Self::Detail),
            "inverse" => Some(Self::Inverse),
            "inverse-heading" => Some(Self::InverseHeading),
            "player" => Some(Self::Player),
            "givens" => Some(Self::Givens),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawCommand {
    Clear,
    Text {
        x: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    },
    /// Text that ends at `right`; the screen measures it with the real font.
    TextRight {
        right: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    },
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    Rect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    },
    Grid {
        x: i32,
        y: i32,
        columns: u8,
        rows: u8,
        cell_width: i32,
        cell_height: i32,
    },
    /// Paper-colored rectangle: `filled` is a solid erase/paint, otherwise a
    /// 2 px ring like the mockup's block outlines.
    PaperRect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    },
    /// Paper-colored 3 px diagonal line, for the hatched active piece.
    PaperLine {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    /// One named sprite at its top-left corner.
    Sprite {
        x: i32,
        y: i32,
        sprite: GameSprite,
    },
    /// One die: a rounded square with round pips, as the mockup's `die-1`
    /// to `die-6`. `(x, y)` is the top-left of the untilted square. `face`
    /// is 0 for the empty rest outline, or 1 to 6 for pips. `dashed` draws
    /// the rest outline of a die that is tumbling. `angle` turns the die
    /// about its centre, in degrees (0 is square).
    Die {
        x: i32,
        y: i32,
        size: i32,
        face: u8,
        dashed: bool,
        angle: i32,
    },
    /// Large display numerals, centred between `left` and `right`.
    Numerals {
        text: String,
        left: i32,
        right: i32,
        baseline: i32,
    },
    /// Text centred on `center_x`; the screen measures it with the real font.
    TextCenter {
        center_x: i32,
        baseline: i32,
        text: String,
        style: CanvasTextStyle,
    },
    /// One settings-style list row (rule, marker and right value included).
    ListRow {
        top: i32,
        title: String,
        value: String,
        selected: bool,
    },
    /// The framed info box: the screen wraps `text` with the real font.
    InfoBox {
        top: i32,
        text: String,
    },
    /// A centred row of mockup chips with `▸` separators (the active one
    /// filled). The screen measures each label with the real font.
    ChipRow {
        top: i32,
        items: Vec<(String, bool)>,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeGameCanvas {
    commands: Vec<DrawCommand>,
    dirty: DirtyRegions,
    refresh_requested: bool,
}

impl NativeGameCanvas {
    #[must_use]
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    #[must_use]
    pub const fn dirty(&self) -> &DirtyRegions {
        &self.dirty
    }

    #[must_use]
    pub const fn refresh_requested(&self) -> bool {
        self.refresh_requested
    }

    pub fn clear_frame(&mut self) {
        self.commands.clear();
        self.dirty.clear();
        self.refresh_requested = false;
    }

    /// Keep the rebuilt full command list but replace broad render-time dirty
    /// geometry with one bounded event-specific invalidation set.
    pub fn reset_dirty_regions(&mut self) {
        self.dirty.clear();
        self.refresh_requested = false;
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.push(DrawCommand::Clear)?;
        self.dirty.invalidate_full_canvas();
        Ok(())
    }

    pub fn text(
        &mut self,
        x: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    ) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(x, y)?;
        let width = (text.len() as i32).saturating_mul(14).max(8);
        self.push(DrawCommand::Text { x, y, text, style })?;
        self.dirty.invalidate(DirtyRect::new(
            x,
            (y - 28).max(0),
            width.min(GAME_CANVAS_WIDTH - x),
            36,
        ));
        Ok(())
    }

    /// Text whose end sits at `right`, such as a clock on a title bar.
    pub fn text_right(
        &mut self,
        right: i32,
        y: i32,
        text: String,
        style: CanvasTextStyle,
    ) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(right, y)?;
        let width = (text.len() as i32).saturating_mul(14).max(8).min(right);
        self.push(DrawCommand::TextRight {
            right,
            y,
            text,
            style,
        })?;
        let area = DirtyRect::new(right - width, (y - 28).max(0), width, 36);
        self.dirty.invalidate(area);
        Ok(())
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), String> {
        self.validate_point(x1, y1)?;
        self.validate_point(x2, y2)?;
        self.push(DrawCommand::Line { x1, y1, x2, y2 })?;
        let left = x1.min(x2);
        let top = y1.min(y2);
        self.dirty.invalidate(DirtyRect::new(
            left,
            top,
            (x1.max(x2) - left + 1).max(1),
            (y1.max(y2) - top + 1).max(1),
        ));
        Ok(())
    }

    pub fn rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    ) -> Result<(), String> {
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::Rect {
            x,
            y,
            width,
            height,
            filled,
        })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    pub fn grid(
        &mut self,
        x: i32,
        y: i32,
        columns: u8,
        rows: u8,
        cell_width: i32,
        cell_height: i32,
    ) -> Result<(), String> {
        if columns == 0 || rows == 0 || columns > 16 || rows > 16 {
            return Err("game grid dimensions must be between 1 and 16".into());
        }
        let width = i32::from(columns).saturating_mul(cell_width);
        let height = i32::from(rows).saturating_mul(cell_height);
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::Grid {
            x,
            y,
            columns,
            rows,
            cell_width,
            cell_height,
        })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    pub fn invalidate_rect(&mut self, rect: DirtyRect) {
        self.dirty.invalidate(rect);
    }

    /// Paper-colored rectangle: filled paint, or a 2 px ring when hollow.
    pub fn paper_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        filled: bool,
    ) -> Result<(), String> {
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::PaperRect {
            x,
            y,
            width,
            height,
            filled,
        })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    /// Paper-colored thick diagonal, for hatch fills on locked-in pieces.
    pub fn paper_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), String> {
        self.validate_point(x1, y1)?;
        self.validate_point(x2, y2)?;
        self.push(DrawCommand::PaperLine { x1, y1, x2, y2 })?;
        let left = x1.min(x2);
        let top = y1.min(y2);
        self.dirty.invalidate(DirtyRect::new(
            left,
            top,
            (x1.max(x2) - left + 1).max(1),
            (y1.max(y2) - top + 1).max(1),
        ));
        Ok(())
    }

    /// One named sprite at `(x, y)`, its top-left corner. The sprite draws
    /// nothing outside its own size, which the dirty area matches.
    pub fn sprite(&mut self, x: i32, y: i32, sprite: GameSprite) -> Result<(), String> {
        let (width, height) = sprite.size();
        self.validate_rect(DirtyRect::new(x, y, width, height))?;
        self.push(DrawCommand::Sprite { x, y, sprite })?;
        self.dirty.invalidate(DirtyRect::new(x, y, width, height));
        Ok(())
    }

    /// One die whose untilted square has its top-left at `(x, y)`, turned
    /// by `angle` degrees about its centre. The dirty rectangle grows by
    /// what the tilt and the stroke reach.
    pub fn die(
        &mut self,
        x: i32,
        y: i32,
        size: i32,
        face: u8,
        dashed: bool,
        angle: i32,
    ) -> Result<(), String> {
        if face > 6 {
            return Err(format!("die face {face} is out of range 0-6"));
        }
        let pad = super::die_shape::tilt_padding(size, angle);
        let rect = DirtyRect::new(x - pad, y - pad, size + 2 * pad, size + 2 * pad);
        self.validate_rect(rect)?;
        self.push(DrawCommand::Die {
            x,
            y,
            size,
            face,
            dashed,
            angle,
        })?;
        self.dirty.invalidate(rect);
        Ok(())
    }

    /// Large display numerals, centred between `left` and `right` at
    /// `baseline`.
    pub fn numerals(
        &mut self,
        text: String,
        left: i32,
        right: i32,
        baseline: i32,
    ) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(left, baseline)?;
        self.validate_point(right, baseline)?;
        self.push(DrawCommand::Numerals {
            text,
            left,
            right,
            baseline,
        })?;
        self.dirty.invalidate(DirtyRect::new(
            left,
            (baseline - 84).max(0),
            (right - left).max(1),
            96,
        ));
        Ok(())
    }

    /// Text centred on `center_x`, measured by the screen's real font.
    pub fn text_center(
        &mut self,
        center_x: i32,
        baseline: i32,
        text: String,
        style: CanvasTextStyle,
    ) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(center_x, baseline)?;
        let width = (text.len() as i32).saturating_mul(14).max(8);
        self.push(DrawCommand::TextCenter {
            center_x,
            baseline,
            text,
            style,
        })?;
        let left = (center_x - width / 2).max(0);
        self.dirty.invalidate(DirtyRect::new(
            left,
            (baseline - 28).max(0),
            width.min(GAME_CANVAS_WIDTH - left),
            36,
        ));
        Ok(())
    }

    /// One settings-style list row at `top`, drawn like every list on the
    /// device: full width, a rule under it and the marker when selected.
    pub fn list_row(
        &mut self,
        top: i32,
        title: String,
        value: String,
        selected: bool,
    ) -> Result<(), String> {
        if title.len() + value.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        // Matches `widgets::list_row::LIST_ROW_HEIGHT`.
        const ROW_HEIGHT: i32 = 72;
        self.validate_rect(DirtyRect::new(0, top, GAME_CANVAS_WIDTH, ROW_HEIGHT))?;
        self.push(DrawCommand::ListRow {
            top,
            title,
            value,
            selected,
        })?;
        self.dirty
            .invalidate(DirtyRect::new(0, top, GAME_CANVAS_WIDTH, ROW_HEIGHT));
        Ok(())
    }

    /// The framed info box under the road; the screen wraps `text`.
    pub fn info_box(&mut self, top: i32, text: String) -> Result<(), String> {
        if text.len() > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game text exceeds {MAX_GAME_TEXT_BYTES}-byte command limit"
            ));
        }
        self.validate_point(16, top)?;
        self.push(DrawCommand::InfoBox { top, text })?;
        self.dirty.invalidate(DirtyRect::new(16, top, 448, 96));
        Ok(())
    }

    /// A centred row of mockup chips (`1 · ROW`, `2 · CELL`, …) with `▸`
    /// separators. The screen sizes each box from the real font; `top` is the
    /// box's top edge (the mockup's chip height).
    pub fn chip_row(&mut self, top: i32, items: Vec<(String, bool)>) -> Result<(), String> {
        let text_bytes: usize = items.iter().map(|(label, _)| label.len()).sum();
        if items.is_empty() || text_bytes > MAX_GAME_TEXT_BYTES {
            return Err(format!(
                "game chip row needs 1..={MAX_GAME_TEXT_BYTES} label bytes"
            ));
        }
        self.validate_rect(DirtyRect::new(0, top, GAME_CANVAS_WIDTH, CHIP_ROW_HEIGHT))?;
        self.push(DrawCommand::ChipRow { top, items })?;
        self.dirty
            .invalidate(DirtyRect::new(0, top, GAME_CANVAS_WIDTH, CHIP_ROW_HEIGHT));
        Ok(())
    }

    pub fn request_refresh(&mut self) {
        self.refresh_requested = true;
    }

    fn push(&mut self, command: DrawCommand) -> Result<(), String> {
        if self.commands.len() >= MAX_GAME_DRAW_COMMANDS {
            return Err(format!(
                "game frame exceeds {MAX_GAME_DRAW_COMMANDS}-command limit"
            ));
        }
        self.commands.push(command);
        Ok(())
    }

    fn validate_point(&self, x: i32, y: i32) -> Result<(), String> {
        if (0..GAME_CANVAS_WIDTH).contains(&x) && (0..GAME_CANVAS_HEIGHT).contains(&y) {
            Ok(())
        } else {
            Err(format!("game point ({x},{y}) is outside logical canvas"))
        }
    }

    fn validate_rect(&self, rect: DirtyRect) -> Result<(), String> {
        if rect.is_valid() {
            Ok(())
        } else {
            Err(format!(
                "game rectangle ({},{},{},{}) is outside logical canvas",
                rect.x, rect.y, rect.width, rect.height
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasTextStyle, DrawCommand, NativeGameCanvas};

    #[test]
    fn canvas_bounds_commands_and_tracks_dirty_regions() {
        let mut canvas = NativeGameCanvas::default();
        canvas
            .text(24, 90, "Hello Grid".into(), CanvasTextStyle::Heading)
            .unwrap();
        canvas.grid(40, 180, 4, 4, 64, 64).unwrap();
        canvas.request_refresh();
        assert_eq!(canvas.commands().len(), 2);
        assert!(!canvas.dirty().is_empty());
        assert!(canvas.refresh_requested());
        assert!(matches!(canvas.commands()[1], DrawCommand::Grid { .. }));
    }

    #[test]
    fn canvas_rejects_out_of_bounds_commands() {
        let mut canvas = NativeGameCanvas::default();
        assert!(canvas.rect(470, 20, 20, 20, false).is_err());
    }

    #[test]
    fn paper_commands_are_bounded_and_dirty() {
        let mut canvas = NativeGameCanvas::default();
        canvas.paper_rect(20, 20, 30, 30, true).unwrap();
        canvas.paper_line(20, 20, 40, 40).unwrap();
        assert!(matches!(
            canvas.commands()[0],
            DrawCommand::PaperRect { filled: true, .. }
        ));
        assert!(matches!(
            canvas.commands()[1],
            DrawCommand::PaperLine { .. }
        ));
        assert!(!canvas.dirty().is_empty());
        assert!(canvas.paper_rect(470, 20, 20, 20, false).is_err());
        assert!(canvas.paper_line(0, 0, 500, 10).is_err());
        assert_eq!(
            CanvasTextStyle::parse("inverse"),
            Some(CanvasTextStyle::Inverse)
        );
        assert_eq!(
            CanvasTextStyle::parse("inverse-heading"),
            Some(CanvasTextStyle::InverseHeading)
        );
    }
}
