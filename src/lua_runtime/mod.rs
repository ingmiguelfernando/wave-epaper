//! SD Lua runtime foundation with a Rust-owned native canvas.
//!
//! Preserves the static `ui.*` subset and bounded Sudoku and Minesweeper
//! bridges. Mutable game state and panel ownership remain in Rust; unrestricted Lua
//! VM callbacks stay deferred.

use std::path::PathBuf;

use crate::{
    buttons::ButtonEvent,
    games::{
        canvas::NativeGameCanvas,
        records::GameRecords,
        refresh_policy::{GameRefreshPlan, GameRefreshPolicy, RefreshTrigger},
        sudoku::SudokuGame,
        sudoku_puzzles::SudokuDifficulty,
        sudoku_save::SudokuSave,
    },
};

pub mod bootstrap;
pub mod catalog;
pub mod event_bridge;
pub mod loader;
pub mod manifest;

use catalog::{LuaAppCatalog, LUA_APPS_DIRECTORY};
use event_bridge::LuaEventBridge;
use loader::open_entry_on_worker;
use manifest::LuaAppEntry;

pub const LUA_CATALOG_PAGE_SIZE: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaAppSession {
    pub entry: LuaAppEntry,
    pub source_bytes: usize,
    pub canvas: NativeGameCanvas,
    pub refresh_plan: GameRefreshPlan,
    pub event_bridge: LuaEventBridge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuaRuntimeUiState {
    pub catalog: LuaAppCatalog,
    pub selected: usize,
    pub session: Option<LuaAppSession>,
    pub error: Option<String>,
    diagnostics: Vec<String>,
    /// Best scores loaded at boot.
    pub records: GameRecords,
    /// Set when a closed game raised a best score, so `main.rs` saves once.
    records_changed: bool,
    /// Sudoku resume state loaded at boot; `None` when no save exists.
    pub sudoku_save: Option<SudokuSave>,
    /// Set when the Sudoku save changed, so `main.rs` writes it: either a
    /// fresh resume in `sudoku_save`, or `None` after a solved puzzle.
    sudoku_save_changed: bool,
}

impl Default for LuaRuntimeUiState {
    fn default() -> Self {
        Self {
            catalog: LuaAppCatalog::unavailable(LUA_APPS_DIRECTORY, "catalog has not been scanned"),
            selected: 0,
            session: None,
            error: None,
            diagnostics: Vec::new(),
            records: GameRecords::default(),
            records_changed: false,
            sudoku_save: None,
            sudoku_save_changed: false,
        }
    }
}

impl LuaRuntimeUiState {
    pub fn refresh_catalog(&mut self, mounted: bool) {
        self.catalog = LuaAppCatalog::scan(LUA_APPS_DIRECTORY, mounted);
        self.selected = self
            .selected
            .min(self.catalog.entries.len().saturating_sub(1));
        self.push_diagnostic(format!(
            "rustmix-wave=lua-app-scan status={} root={} apps={} raw={} rejected={} warning={}",
            if self.catalog.is_available() {
                "completed"
            } else {
                "unavailable"
            },
            self.catalog.root.display(),
            self.catalog.entries.len(),
            self.catalog.raw_entries,
            self.catalog.rejected_entries,
            self.catalog.warning.as_deref().unwrap_or("none")
        ));
    }

    pub fn refresh_catalog_from_root(&mut self, root: impl Into<PathBuf>, mounted: bool) {
        self.catalog = LuaAppCatalog::scan(root, mounted);
        self.selected = self
            .selected
            .min(self.catalog.entries.len().saturating_sub(1));
    }

    #[must_use]
    pub fn selected_entry(&self) -> Option<&LuaAppEntry> {
        self.catalog.entries.get(self.selected)
    }

    pub fn apply_catalog_button(&mut self, event: ButtonEvent) -> bool {
        if self.catalog.entries.is_empty() {
            return false;
        }
        match event {
            ButtonEvent::Up => {
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.catalog.entries.len() - 1);
                false
            }
            ButtonEvent::Down => {
                self.selected = (self.selected + 1) % self.catalog.entries.len();
                false
            }
            ButtonEvent::Select => self.open_selected(),
        }
    }

    pub fn open_selected(&mut self) -> bool {
        self.error = None;
        let Some(entry) = self.selected_entry().cloned() else {
            self.error = Some("No SD Lua application is selected".into());
            return false;
        };
        self.push_diagnostic(format!(
            "rustmix-wave=lua-app-open id={} status=starting runtime=bootstrap-static entry={}",
            entry.manifest.id, entry.manifest.entry
        ));
        let entry_id = entry.manifest.id.clone();
        let opened = self.open_entry(entry).and_then(|mut session| {
            self.prepare_session(&mut session)?;
            Ok(session)
        });
        match opened {
            Ok(session) => {
                let regions = session.canvas.dirty().regions().len();
                let command_count = session.canvas.commands().len();
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-canvas-frame commands={command_count} dirty-regions={regions} transport=existing-fullscreen-partial"
                ));
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-app-open id={} status=ready runtime=bootstrap-static source-bytes={} commands={command_count}",
                    session.entry.manifest.id, session.source_bytes
                ));
                self.session = Some(session);
                true
            }
            Err(error) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={} error={}",
                    entry_id,
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                self.session = None;
                false
            }
        }
    }

    fn open_entry(&mut self, entry: LuaAppEntry) -> Result<LuaAppSession, String> {
        open_entry_on_worker(entry)
    }

    /// Hand saved state to a freshly opened game before its first frame.
    fn prepare_session(&self, session: &mut LuaAppSession) -> Result<(), String> {
        match &mut session.event_bridge {
            LuaEventBridge::Tetris(app) => app.set_best(self.records.tetris_zen),
            LuaEventBridge::Sudoku(game) => {
                game.prepare(self.sudoku_save, self.records);
                // Redraw, so the start list shows the Continue row.
                game.render_initial(&mut session.canvas)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Manifests of the catalog's game apps, in catalog order.
    #[must_use]
    pub fn games(&self) -> Vec<&crate::lua_runtime::manifest::LuaAppManifest> {
        self.catalog
            .entries
            .iter()
            .filter(|entry| entry.manifest.kind == crate::lua_runtime::manifest::LuaAppKind::Game)
            .map(|entry| &entry.manifest)
            .collect()
    }

    /// Select the catalog entry of the game card `index`; `false` when the
    /// card does not exist.
    pub fn select_game(&mut self, index: usize) -> bool {
        let Some(entry) = self.games().get(index).map(|manifest| manifest.id.clone()) else {
            return false;
        };
        let Some(position) = self
            .catalog
            .entries
            .iter()
            .position(|candidate| candidate.manifest.id == entry)
        else {
            return false;
        };
        self.selected = position;
        true
    }

    pub fn apply_game_button(&mut self, event: ButtonEvent, now_ms: u64) -> bool {
        let outcome = {
            let Some(session) = self.session.as_mut() else {
                return false;
            };
            match session
                .event_bridge
                .apply_button(event, now_ms, &mut session.canvas)
            {
                Ok(Some(result)) => {
                    session.refresh_plan = GameRefreshPolicy::plan(
                        session.canvas.dirty(),
                        RefreshTrigger::ScriptFrame,
                    );
                    Ok(Some((session.entry.manifest.id.clone(), result)))
                }
                Ok(None) => Ok(None),
                Err(error) => Err((session.entry.manifest.id.clone(), error)),
            }
        };
        match outcome {
            Ok(Some((id, result))) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-event-bridge id={id} bridge={} event={} outcome={} row={} column={} mode={} axis={} {} completed={} dirty-regions={} refresh=partial-fullscreen transport=existing-fullscreen-partial",
                    result.bridge_marker(),
                    button_marker(event),
                    result.reason(),
                    result.row() + 1,
                    result.column() + 1,
                    result.mode_marker(),
                    result.axis_marker(),
                    result.detail_marker(),
                    result.completed(),
                    result.dirty_regions_len(),
                ));
                self.autosave_sudoku();
                true
            }
            Ok(None) => false,
            Err((id, error)) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={id} error={}",
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                false
            }
        }
    }

    pub fn apply_game_boot_short_press(&mut self, now_ms: u64) -> bool {
        let outcome = {
            let Some(session) = self.session.as_mut() else {
                return false;
            };
            match session
                .event_bridge
                .apply_boot_short_press(now_ms, &mut session.canvas)
            {
                Ok(Some(result)) => {
                    session.refresh_plan = GameRefreshPolicy::plan(
                        session.canvas.dirty(),
                        RefreshTrigger::ScriptFrame,
                    );
                    Ok(Some((session.entry.manifest.id.clone(), result)))
                }
                Ok(None) => Ok(None),
                Err(error) => Err((session.entry.manifest.id.clone(), error)),
            }
        };
        match outcome {
            Ok(Some((id, result))) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-event-bridge id={id} bridge={} event=boot-short outcome={} row={} column={} mode={} axis={} {} completed={} dirty-regions={} refresh=partial-fullscreen transport=existing-fullscreen-partial",
                    result.bridge_marker(),
                    result.reason(),
                    result.row() + 1,
                    result.column() + 1,
                    result.mode_marker(),
                    result.axis_marker(),
                    result.detail_marker(),
                    result.completed(),
                    result.dirty_regions_len(),
                ));
                true
            }
            Ok(None) => false,
            Err((id, error)) => {
                self.push_diagnostic(format!(
                    "rustmix-wave=lua-runtime-error id={id} error={}",
                    sanitize_marker(&error)
                ));
                self.error = Some(error);
                false
            }
        }
    }

    pub fn close_session(&mut self) {
        if let Some(session) = self.session.take() {
            match &session.event_bridge {
                LuaEventBridge::Tetris(app) => {
                    // The changed flag makes main.rs save once, not per piece.
                    self.records_changed |= self.records.observe_tetris_zen(app.best());
                }
                LuaEventBridge::Sudoku(game) => self.close_sudoku(game),
                _ => {}
            }
            self.push_diagnostic(format!(
                "rustmix-wave=lua-app-close id={} status=released",
                session.entry.manifest.id
            ));
        }
        self.error = None;
    }

    /// Sudoku on close: keeps the final time of unfinished progress.
    fn close_sudoku(&mut self, game: &SudokuGame) {
        self.store_sudoku(sudoku_outcome(game));
    }

    /// Autosave after a press: a changed board rewrites the save and a solve
    /// deletes it, so a restart or a flat battery keeps the game.
    fn autosave_sudoku(&mut self) {
        let outcome = match self.session.as_ref().map(|session| &session.event_bridge) {
            Some(LuaEventBridge::Sudoku(game)) => sudoku_outcome(game),
            _ => return,
        };
        let changed = match (&outcome, &self.sudoku_save) {
            (Some(SudokuOutcome::Progress(save)), Some(saved)) => {
                save.board != saved.board || save.puzzle != saved.puzzle
            }
            (Some(SudokuOutcome::Progress(save)), None) => save.board != save.puzzle,
            (Some(SudokuOutcome::Solved(..)), saved) => saved.is_some(),
            (None, _) => false,
        };
        if changed {
            self.store_sudoku(outcome);
        }
    }

    /// Progress becomes the save; a solve deletes it and raises the best time.
    fn store_sudoku(&mut self, outcome: Option<SudokuOutcome>) {
        match outcome {
            Some(SudokuOutcome::Progress(save)) => self.sudoku_save = Some(save),
            Some(SudokuOutcome::Solved(difficulty, seconds)) => {
                self.sudoku_save = None;
                self.records_changed |= self.records.observe_sudoku(difficulty, seconds);
            }
            None => return,
        }
        self.sudoku_save_changed = true;
    }

    /// Take the changed flag after the records were saved.
    pub fn take_records_changed(&mut self) -> bool {
        std::mem::take(&mut self.records_changed)
    }

    /// Take the sudoku save-changed flag after `main.rs` wrote or removed
    /// the file.
    pub fn take_sudoku_save_changed(&mut self) -> bool {
        std::mem::take(&mut self.sudoku_save_changed)
    }

    pub fn take_diagnostics(&mut self) -> Vec<String> {
        core::mem::take(&mut self.diagnostics)
    }

    fn push_diagnostic(&mut self, line: String) {
        self.diagnostics.push(line);
    }
}

fn button_marker(event: ButtonEvent) -> &'static str {
    match event {
        ButtonEvent::Up => "up",
        ButtonEvent::Select => "select",
        ButtonEvent::Down => "down",
    }
}

/// What a Sudoku game means for the save.
enum SudokuOutcome {
    Progress(SudokuSave),
    Solved(SudokuDifficulty, u32),
}

/// `None` on the start list, for the card's own puzzle and for a new game
/// nobody has touched.
fn sudoku_outcome(game: &SudokuGame) -> Option<SudokuOutcome> {
    let difficulty = game.difficulty()?;
    if game.completed() {
        return Some(SudokuOutcome::Solved(difficulty, game.seconds()));
    }
    (game.seconds() > 0 || game.board() != game.puzzle()).then(|| {
        SudokuOutcome::Progress(SudokuSave {
            difficulty,
            puzzle: *game.puzzle(),
            board: *game.board(),
            seconds: game.seconds(),
        })
    })
}

fn sanitize_marker(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() {
                '-'
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{
        buttons::ButtonEvent,
        games::{
            canvas::DrawCommand,
            sudoku::{StartKind, SudokuGame, SudokuStep},
            sudoku_puzzles::{generate, SudokuDifficulty},
            sudoku_save::SudokuSave,
        },
    };

    use super::{event_bridge::LuaEventBridge, LuaRuntimeUiState};

    fn temp_directory() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-lua-runtime-{nonce}"))
    }

    #[test]
    fn opens_sd_bootstrap_app_into_native_dirty_canvas() {
        let root = temp_directory();
        let app = root.join("HGRID");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"hello_grid\"\nname=\"Hello Grid\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(
            app.join("MAIN.LUA"),
            "ui.clear()\nui.grid(80, 220, 4, 4, 64, 64)\nui.request_refresh()\n",
        )
        .unwrap();
        let mut runtime = LuaRuntimeUiState::default();
        runtime.refresh_catalog_from_root(&root, true);
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        assert!(runtime.session.is_some());
        assert!(!runtime.take_diagnostics().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tetris_sessions_seed_the_saved_best_and_close_marks_one_save() {
        // The open path seeds the best; drive it through a real catalog.
        let root = temp_directory();
        let app = root.join("TETRIS");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"tetris\"\nname=\"Tetris\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(app.join("MAIN.LUA"), "tetris.init('zen', 7)\n").unwrap();
        let mut runtime = LuaRuntimeUiState::default();
        runtime.records.tetris_zen = 500;
        runtime.refresh_catalog_from_root(&root, true);
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        assert!(runtime.session.is_some());
        if let LuaEventBridge::Tetris(app) = &runtime.session.as_ref().unwrap().event_bridge {
            assert_eq!(app.best(), 500, "opening seeds the saved best");
        }
        // A zero-score close saves nothing.
        runtime.close_session();
        assert!(!runtime.take_records_changed(), "a zero best saves nothing");
        assert_eq!(runtime.records.tetris_zen, 500);

        // A session that ends above the saved best marks exactly one save.
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        if let LuaEventBridge::Tetris(app) = &mut runtime.session.as_mut().unwrap().event_bridge {
            app.set_best(9001);
        }
        runtime.close_session();
        assert!(runtime.take_records_changed(), "a new best marks one save");
        assert_eq!(runtime.records.tetris_zen, 9001);
        assert!(!runtime.take_records_changed(), "the flag clears after use");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sudoku_opens_on_its_start_list_with_the_saved_game() {
        let root = temp_directory();
        let app = root.join("SUDOKU");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"sudoku\"\nname=\"Sudoku\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(app.join("MAIN.LUA"), "sudoku.init()\n").unwrap();
        let mut runtime = LuaRuntimeUiState::default();
        let generated = generate(SudokuDifficulty::Medium, 3);
        runtime.sudoku_save = Some(SudokuSave {
            difficulty: SudokuDifficulty::Medium,
            puzzle: generated.puzzle,
            board: generated.puzzle,
            seconds: 61,
        });
        runtime.refresh_catalog_from_root(&root, true);
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        let session = runtime.session.as_ref().unwrap();
        let LuaEventBridge::Sudoku(game) = &session.event_bridge else {
            panic!("the card opens the Sudoku bridge");
        };
        assert_eq!(game.step(), SudokuStep::Start);
        assert_eq!(game.start_options()[0].kind, StartKind::Continue);
        // The first frame was redrawn with the Continue row.
        let texts: Vec<&str> = session
            .canvas
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text.starts_with("Continue")));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn placing_a_sudoku_number_saves_without_leaving_the_game() {
        let root = temp_directory();
        let app = root.join("SUDOKU");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("APP.TOM"),
            "id=\"sudoku\"\nname=\"Sudoku\"\nkind=\"game\"\nentry=\"MAIN.LUA\"\n",
        )
        .unwrap();
        std::fs::write(app.join("MAIN.LUA"), "sudoku.init()\n").unwrap();
        let mut runtime = LuaRuntimeUiState::default();
        runtime.refresh_catalog_from_root(&root, true);
        assert!(runtime.apply_catalog_button(ButtonEvent::Select));
        // New · Easy, then the row, the cell and the first allowed number.
        for now_ms in [1_000, 2_000, 3_000] {
            assert!(runtime.apply_game_button(ButtonEvent::Select, now_ms));
            assert!(!runtime.take_sudoku_save_changed());
        }
        assert!(runtime.apply_game_button(ButtonEvent::Select, 4_000));
        assert!(runtime.take_sudoku_save_changed());
        let save = runtime.sudoku_save.unwrap();
        assert_eq!(save.difficulty, SudokuDifficulty::Easy);
        assert_ne!(save.board, save.puzzle);

        assert!(runtime.apply_game_button(ButtonEvent::Down, 5_000));
        assert!(!runtime.take_sudoku_save_changed(), "moving writes nothing");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_sudoku_saves_progress_and_a_solve_clears_the_save() {
        let mut runtime = LuaRuntimeUiState::default();
        let generated = generate(SudokuDifficulty::Easy, 7);
        let givens = generated.puzzle.map(|cell| cell != 0);
        let difficulty = Some(SudokuDifficulty::Easy);
        let puzzle = generated.puzzle;
        let unfinished = SudokuGame::new(puzzle, givens, puzzle, difficulty, 95);
        runtime.close_sudoku(&unfinished);
        assert!(runtime.take_sudoku_save_changed());
        assert_eq!(runtime.sudoku_save.map(|save| save.seconds), Some(95));
        let solved = SudokuGame::new(generated.solution, givens, puzzle, difficulty, 300);
        runtime.close_sudoku(&solved);
        assert!(runtime.take_sudoku_save_changed());
        assert_eq!(runtime.sudoku_save, None);
        assert_eq!(runtime.records.sudoku_easy, 300);
        assert!(runtime.take_records_changed());
        // The card's own puzzle keeps no save and no best.
        let own = SudokuGame::new(puzzle, givens, puzzle, None, 40);
        runtime.close_sudoku(&own);
        assert!(!runtime.take_sudoku_save_changed());
    }
}
