# Delegated tasks

Small, self-contained tasks for a second developer or AI working in parallel
with the main line. Read this file first, then
[architecture.md](architecture.md) and [ROADMAP.md](ROADMAP.md).

## Round 2 result (2026-10-03)

Pull request #2 (D6 to D11) was reviewed, merged into `main` and released as
v0.9.1. D7, D8 and D9 went in as they were; D10 and D11 are solid libraries
with thorough tests, and D6 stopped correctly when no setting fixed the glyph.
The `side-tasks-2` branch is deleted; round 3 is in "Round 3 tasks" below.

What the main developer changed at merge time:

- Merged `main` (v0.9.0, Weather). The only conflict was "both sides added"
  in `preview_states()`. Kept one `audio-details` preview instead of seven;
  the fit test already covers every font.
- D11: `save_to_path` renamed `STATS.TMP` over `STATS.TXT`. FAT refuses that
  once the file exists (FatFs `f_rename` returns `FR_EXIST`), so every save
  after the first would have failed on the device. The old file now moves to
  `STATS.BAK` first, as in the Reader's state files, and `load_from_path`
  falls back to the backup. The helpers also moved above the test module.
- Known Issues: dropped the "retirement pending" note (D9 already deleted
  the scripts) and rewrote the `%` entry with D6's findings.

Verdict per task:

- **D6:** blocked, as allowed; findings in Known Issues and the backlog.
- **D7:** done; close to the mockup at every font family and size.
- **D8:** done.
- **D9:** done; nothing refers to the removed scripts.
- **D10:** done; path traversal is rejected and chapter reads stay bounded.
- **D11:** done after the save fix above.

Do differently next time:

- Check what the device's filesystem allows, not only the host: FAT cannot
  rename onto an existing file. Copy a pattern already proven on the device
  (`atomic_replace_text` in `reader.rs`).
- Keep production code above `#[cfg(test)] mod tests`.
- Size tests to real ranges: reading time never reaches `u32::MAX` seconds,
  and dates past year 9999 add code without value.
- A test that compares a constant with its own literal proves nothing; test
  behaviour such as fit or rendering.
- Status lines: about eight lines, and no local `/tmp` paths, which nobody
  else can open. Evidence goes in the pull request.
- Write docs for the state after the merge: "retirement is pending" was
  false once the same pull request deleted the scripts.

## Round 1 result (2026-10-03)

Pull request #1 (D1 to D5) was reviewed, merged into `main` and released as
v0.8.1. The work was careful, well tested and well documented. The
`side-tasks` branch is deleted; round 2 is in "Round 2 tasks" below.

What the main developer did at merge time:

- Merged `main` (v0.8.0, Photos) into the branch. Every conflict was "both
  sides added": `AppState::back()` (the Display and Reader pickers next to the
  Sleep screen preview and the photo viewer), `preview_states()`,
  `screens/mod.rs`, the IMU comment in `main.rs` and `architecture.md`.
- Updated the docs for Photos, which they still described as SOON: the User
  Guide (Photos and Settings › Sleep screen), Known Issues, the smoke test,
  CHANGELOG (v0.8.0 and v0.8.1), the README backlog and the ROADMAP.
- Bumped the version to 0.8.1, milestone `option-lists`.

Verdict per task:

- **D1:** done, review fix included; the same-value tests are thorough.
- **D2 and D3:** done; the reference scans and the Games test pass.
- **D4:** done; the guides are practical and accurate.
- **D5:** done as specified. Polish against the mockup is round 2 task D7:
  - the clock has large gaps (rule at y 470, details at y 622), where the
    mockup keeps them close;
  - the weather line is not centered as one group;
  - the weather screen lacks the rule above the three days;
  - the rain row uses the Detail size, where `%` is broken.

Do differently next time:

- Write docs for `main` as it will be after the merge, not "on this branch";
  if another branch changes the same feature, say so in the Status line.
- Keep each Status to about eight lines: what changed, the checks, what to
  test on the device. Logs and evidence go in the pull request description.
- Indent Markdown continuation lines with two spaces, not tabs.
- Compare previews with the mockup side by side; spacing is part of "looks
  like the mockup".

## How to work

- **Branch.** Each round of tasks gets its own branch from the latest `main`
  (round 3: `side-tasks-3`), with one commit per task (`D12: ...`). Push often.
- **Pull request.** When done, open a pull request to `main`. Do not merge
  it; the main developer reviews it, resolves any conflicts and merges.
- **Report.** Update the **Status** line of each task below in your branch:
  what changed, what needs a test on the device, and anything left open.
- **Order.** As listed in the round. Each task stands alone, so skip a task
  rather than blocking on it.

### Tools

- Host tests and formatting need stable Rust with rustfmt:
  - run `./scripts/test-host.sh`;
  - run `cargo +stable fmt`;
  - both must pass.
- The firmware build (Xtensa) runs on GitHub Actions:
  `gh workflow run firmware.yml -R ingmiguelfernando/wave-epaper --ref <branch>`
  takes about 8 minutes and must be green before you hand over.
- `ci.yml` runs on the pull request: host tests, the `screen-previews` artifact
  (one PNG per screen) and `cargo fmt --check`.
- Without local Rust, run `ci.yml` on the branch and apply the "Diff in" lines
  from its log exactly as printed.

### Rules

- English everywhere: UI text, code, comments, commits, docs. The device UI is
  terse (`MOVE  SELECT CHANGE  HOLD BOOT BACK`).
- Follow the existing patterns; the closest reference is named in each task.
  No new dependencies unless a task says so.
- Do not bump the version (Cargo.toml, sdkconfig.defaults, build_info.rs); the
  main developer does that when merging.
- Do not add or change `.py` files.
- Comments only where the code cannot speak for itself, one short line.
- Logic changes come with host tests. UI changes come with a preview: add an
  entry at the end of `preview_states()` in `src/app/preview.rs`, then check
  the PNG in the `screen-previews` artifact.
- The main line works on a phase at the same time. Each round lists the files
  that phase changes; keep edits to them small, and leave alone the ones it
  marks off-limits.
- Hardware facts and the event loop are in `architecture.md`. The board cannot
  be tested from CI, so list what to try on the device in the Status line.

### Tips

- **Choosing the value in use does nothing:** no save, no refresh, no
  repagination. This holds for every list you add.
- **Host tests do not compile `src/main.rs`.** Before you remove or rename
  something, run `git grep -n <name> -- src` and read the `main.rs` hits;
  otherwise only the firmware build finds them.
- **Firmware build without `gh`.** Ask the user to start it: GitHub ›
  Actions › firmware › Run workflow › your branch. Hand a task over only
  when it is green.
- **Code shape.** Prefer an enum to a row index (`PowerSetting` is the
  model), `ALL.get(index)` to `ALL[index % len]`, and one generic helper to
  several copies of the same match arm.
- **SD files.** Replace a file with `.TMP` then `.BAK` renames, as
  `atomic_replace_text` in `reader.rs` does; FAT cannot rename onto an
  existing file.

## Round 3 tasks

Branch `side-tasks-3` from the latest `main` (v0.9.1); pull request title
`Side tasks 3`. Order: D12, D13, D14, D15, D16, D17.

The main line builds Phase 3b (sleep screen modes) at the same time.

- Do not edit: `src/sleep_mode.rs`, `src/sleep_screen.rs`,
  `src/sleep_images.rs`, `src/radio_burst.rs`, `src/app/screens/sleep_*.rs`,
  `docs/ROADMAP.md`, `docs/architecture.md`.
- Keep edits small and local: `src/main.rs`, `src/app/state.rs`,
  `src/app/router.rs`, `src/app/menu.rs`, `src/app/mod.rs`,
  `src/app/preview.rs`, `src/app/screens/home.rs`, `src/lib.rs`,
  `README.md`, `Cargo.toml`.

### D12: Tetris engine (library only)

**Why.** Phase 6 adds Tetris (ROADMAP › Phase 6, mockup "Tetris"). This task
builds and tests the game rules only: no drawing and no SD app.

**Add `src/games/tetris.rs`**, registered in `src/games/mod.rs`:

- Board 10 × 20; the seven pieces I, O, T, S, Z, J and L, four rotations each.
- `TetrisMode { Zen, Classic }`, `TetrisAction { Left, Right, Rotate, Drop }`
  and `TetrisGame::new(mode, seed: u32)`.
- Pieces come from a 7-bag (each piece once in every seven), shuffled by a
  small seeded xorshift in the module; no new dependency.
- A new piece spawns centred in the top two rows. If it overlaps, the game is
  over and every action does nothing.
- `apply(action)`: Left and Right move one column when free; Rotate turns
  clockwise and, when blocked, tries the column offsets 0, −1, +1, −2, +2;
  Drop moves the piece to the bottom and locks it.
- Locking clears full rows, adds the score and spawns the next piece.
- `step()` moves the piece down one row and locks it when it cannot move.
  Zen never calls it. Classic calls it every `GRAVITY_MS = 1000` at every
  level, because e-paper cannot refresh faster.
- Score 100, 300, 500 or 800 times the level for 1 to 4 rows; level is
  `1 + lines / 10`.
- Queries for drawing: `cell(column, row)` for locked cells,
  `active_cells()`, `ghost_cells()` (where Drop would land), `next()`,
  `score()`, `lines()`, `level()` and `is_over()`.

**Tests:** each piece rotates through four states and back; walls and locked
cells block moves; the kicks; one to four cleared rows with their scores and
the level; each piece once per bag; the same seed gives the same pieces; the
ghost; game over at spawn; Drop locks at once.

**Status:** Done on `side-tasks-3` (library only).
  Added `src/games/tetris.rs` (10 x 20 board, SRS-like rotations with the
  0/−1/+1/−2/+2 column kicks, 7-bag fed by an in-module xorshift32) and
  registered it in `src/games/mod.rs`. Queries for drawing: `cell`,
  `active_cells`, `ghost_cells`, `next`, `score`, `lines`, `level`, `is_over`;
  `GRAVITY_MS = 1000` for the future Classic timer in `main.rs`.
  Checks: `./scripts/test-host.sh` green (488 old + 12 new tests),
  `cargo +stable fmt --all -- --check` and `git diff --check` clean.
  Device: nothing to test yet, the engine draws nothing; it ships with D13.
  Left open: nothing.

### D13: Tetris Zen as an SD app

**Why.** With D12, Tetris can run through the SD game path that Sudoku and
Minesweeper use. Zen has no gravity, so it needs no timer; Classic needs one
in `main.rs` and stays with the main line.

**Change.**

- `src/lua_runtime/event_bridge.rs`: `tetris.init('zen', <seed>)` loads a
  `TetrisGame`, next to `sudoku.init` and `minesweeper.init`.
- Draw into `NativeGameCanvas` like `minesweeper.rs` does (`render_initial`,
  `apply_button_and_render`, `apply_boot_short_press_and_render`), following
  the mockup "Tetris":
  - the board in 30 px cells, the dotted ghost, Next, Score, Lines, Level and
    Best (kept in memory until Phase 6 saves it);
  - title `Tetris · Zen`, footer `▲▼ move  ● rotate  BOOT drop`;
  - ▲ moves left, ▼ right, ● rotates and a short BOOT press drops. Game over
    shows the score, and ● starts a new game.
- Stay under `MAX_GAME_DRAW_COMMANDS` (256): draw each row's run of filled
  cells as one rectangle. Invalidate only what changed (at most
  `MAX_DIRTY_REGIONS` regions), and request a full refresh every 20 locked
  pieces to clear ghosting.
- `examples/sd-card/RUSTMIX/APPS/TETRIS/` with `APP.TOM`, `MAIN.LUA` and
  `README.TXT`, like `MINES`; add Tetris to the Games section of the User
  Guide.
- A `tetris` preview.

**Done when:** the preview matches the mockup's layout; tests cover the key
mapping, the command limit on a nearly full board and the dirty regions; the
firmware build is green. Device check: play a game from Games › SD Games.

**Status:** done on `side-tasks-3`; firmware build pending.
  `TetrisApp` draws through `NativeGameCanvas` like Minesweeper: run-of-cells
  rectangles, dotted ghost, Next/Score/Lines/Level/Best panel, dirty regions
  bounded by `MAX_DIRTY_REGIONS`, full refresh every 20 locks.
  `tetris.init('zen', seed)` added to `event_bridge.rs`; the `TETRIS/` sample
  app and a `tetris` preview are in place; the User Guide lists Tetris Zen.
  Deviation: the atlas lacks `▲▼●`, so the footer uses the house wording
  `UP/DOWN move  SELECT rotate  BOOT drop` instead of the mockup's glyphs.
  Checks: host tests green (507), fmt and diff checks clean; preview reviewed.
  Device: play from Games › SD Games and confirm refresh cadence.
  Left open: nothing.

### D14: Reading Stats screen (drawing only)

**Why.** Phase 5 shows reading statistics (mockup "Reading Stats"). This task
draws the screen from a `ReadingStats`; the main line wires it to the Reader
later.

**Change.**

- `src/reading_stats.rs`, with tests:
  - `best_streak(&self) -> u32` over the retained days;
  - `total(&self, first: u32, last: u32) -> DayStats`, both days included;
  - `book(&self, path: &str) -> Option<DayStats>`, a book's totals.
- `src/app/screens/reading_stats.rs`:
  `render_reading_stats(display, preferences, stats, today, current)`, where
  `current` is an optional `CurrentBook { title, path, percent }`. No route
  yet: previews call it directly, like the sleep layouts.
- Top to bottom, as in the mockup:
  - header `READING STATS` / `THIS WEEK`;
  - Today (minutes and pages), Streak (current and best), This week (time,
    and the change against the week before);
  - minutes per day for the last seven days as bars, labelled with weekday
    initials;
  - the last 12 weeks as 12 × 7 squares, filled for days with 5 minutes or
    more;
  - this year: pages, hours, pages per hour, and books finished (all retained
    books; finish dates are not stored);
  - the current book: time read, percent, and time left at that pace
    (`read × (100 − percent) / percent`) when the percent is known;
  - footer `HOLD BOOT BACK`; the week, month and year ranges come with the
    wiring.
- Percentages in Body size (Known Issues › Percent glyph).
- Previews `reading-stats` (a month of sample reading) and
  `reading-stats-empty`.

**Done when:** both previews look like the mockup at every font family and
size, and the tests pass.

**Status:** done on `side-tasks-3` (2026-10-04). Added `best_streak`,
  `total` and `book` to `reading_stats.rs`, and `screens/reading_stats.rs`
  with `CurrentBook` and `render_reading_stats` (no route or `main.rs`
  wiring). Layout follows the mockup: TODAY/STREAK/THIS WEEK tiles,
  hatched today bar with weekday row, 3-level 12-week heatmap with dotted
  future cells, year totals in mockup order, current book with the
  "~31 h left at your pace" line. Percentages use Body size (D6).
  Previews `reading-stats` and `reading-stats-empty` plus every font
  family/size variant are in `screen-previews`; compared side by side with
  the mockup. Host tests (519) and `cargo +stable fmt` pass.
  Device check: readability on the panel at every UI size; the Reader
  wiring and week/month/year ranges stay with the main line.

### D15: Safe SD writes for settings files

**Why.** `WEATHER.TXT`, `POWER.TXT`, `DISPLAY.TXT`, `BATTERY.TXT` and
`STARRED.TXT` are written with a plain `fs::write`, so a power cut during the
write can leave an empty file; `WEATHER.TXT` holds the user's coordinates.
The Reader already replaces its files safely.

**Change.**

- New `src/sd_file.rs`, registered in `src/lib.rs`:
  - `replace(path: &Path, text: &str) -> io::Result<()>`: write and sync
    `.TMP`, move the old file to `.BAK`, rename `.TMP` into place, delete
    `.BAK`; if that rename fails, put `.BAK` back. These are the steps of
    `atomic_replace_text` in `reader.rs`, since FAT cannot rename onto an
    existing file.
  - `read_to_string(path: &Path) -> io::Result<String>`: the file, or its
    `.BAK` when the file is missing after an interrupted write.
- Use both in `weather_config.rs`, `power_settings.rs`, `app/display.rs`,
  `battery_log.rs`, `photos/mod.rs` (starred list) and `reading_stats.rs`,
  which drops its own swap; update its save tests to the helper's steps.
- Leave `reader.rs`, `calendar.rs`, `voice_note_metadata.rs`,
  `wifi_transfer.rs` and `photos/cache.rs` as they are. `sleep_screen.rs`
  moves to the helper with Phase 3b.

**Tests:** create; replace leaving no `.TMP` or `.BAK`; the `.BAK` fallback;
a failed rename keeps the original; one round trip per settings file.

**Status:** done on `side-tasks-3` (2026-10-04). Added `src/sd_file.rs` with
  `replace` (the `atomic_replace_text` steps: write `.TMP`, old to `.BAK`,
  rename in, delete; restores `.BAK` when the last rename fails) and
  `read_to_string` with the `.BAK` fallback. `weather_config`,
  `power_settings`, `app/display`, `battery_log`, `photos` (starred) and
  `reading_stats` save and load through it; `reading_stats` dropped its own
  swap and its save tests assert no leftover `.TMP`/`.BAK`. Host tests green
  (531) with 12 new helper tests, fmt and diff checks clean. Device: change
  each settings file from its screen and pull the power mid-save; boot must
  recover from `.BAK`.

### D16: One module for calendar dates

**Why.** The same date arithmetic lives in four places: `rtc.rs` (private
helpers), `weather.rs` (`weekday_of`), `reading_stats.rs` (`parse_day`,
`day_label`) and `screens/home.rs` (weekday and month names).

**Change.**

- New `src/civil_date.rs`, registered in `src/lib.rs`:
  `days_from_civil(year, month, day) -> i64`,
  `civil_from_days(days: i64) -> (i64, u8, u8)`, `weekday(days: i64) -> u8`
  (0 is Sunday), and the `WEEKDAY_SHORT` and `MONTH_SHORT` names.
- Use it in those four places. Their public functions keep their behaviour,
  and their existing tests pass unchanged.

**Tests:** 1970-01-01, 2000-02-29, 2026-10-03 (a Saturday), the last day of
every month in a leap and a common year, and a round trip across several
centuries.

**Status:** done on `side-tasks-3` (2026-10-04). Added `src/civil_date.rs`
  (Hinnant's algorithm: `days_from_civil`, `civil_from_days`, `weekday` with
  0 = Sunday, `WEEKDAY_SHORT`/`MONTH_SHORT` and month lengths) with six tests:
  1970-01-01, the 2000 leap day, Saturday 2026-10-03, every month end of 2024
  and 2026, century round trips and weekday continuity. `rtc.rs`,
  `weather.rs`, `reading_stats.rs` and `screens/home.rs` delegate to it;
  behaviour, error strings and tests unchanged. Invalid month/day values roll
  over; call sites keep validating. Host tests green (537), checks clean.

### D17: Boot log markers that match the firmware

**Why.** `main.rs` logs about 90 `rustmix-wave=*-ready` markers at boot. Some
still describe upstream features or old counts (Known Issues › Inherited
diagnostic wording).

**Change.**

- In the boot block of `main.rs` only, remove the markers that announce
  features Wave no longer has or that only restate a design (BLE, tilt
  games, ELF-only releases, old category counts). Keep those that report
  what happened at boot (a file loaded, a device found).
- Nothing in `scripts/`, `.github/` or `docs/` reads them today; check again
  with `git grep` before removing one.
- Remove the Known Issues section once nothing stale is left.

**Done when:** no boot marker names a removed feature or a wrong count, and
the firmware build is green. Device check: the serial log at boot.

**Status:** not started.

## Round 2 tasks (done in v0.9.1)

Kept as the reference for the code they added.

### D6: Fix the `%` glyph at the Detail size

**Why.** In Inter at the Standard size the Detail strike (12 px) draws `%`
broken, for example the battery on the sleep card and the rain row of the
`sleep-weather` preview. Compact (11 px) and Large look right.

**Where.**

- `scripts/fonts/fonts.toml`: strike `INTER_STANDARD_DETAIL` uses the face
  `inter-medium` (gray, threshold 128).
- Pushing a change to `scripts/fonts/` runs the `fonts` workflow, which
  uploads the `generated-fonts` artifact; commit the atlas `.rs` files from
  it. Do not change `generate.py`.

**Change.**

1. Compare `%` in `INTER_STANDARD_DETAIL` with the 11 px strike in the
   generated atlas, and check the Atkinson Detail strikes too.
2. Fix it in `fonts.toml` only, for example with a face for that one strike
   that has its own `threshold`, `render` or `tracking`.
3. Other glyphs and strikes must not change: check the atlas diff.
4. If no setting fixes it, stop and write what you found in the Status line.

**Done when:** `%` reads clearly in the `sleep-card` and `sleep-weather`
previews at Inter Standard, and the other previews are unchanged.

**Status:** blocked (`b1fa110`). No threshold, render mode or axis value fixes
`%` at Inter 12 px without changing other glyphs; fonts are unchanged.
Percentages use Body size instead (Known Issues › Percent glyph).

### D7: Sleep layouts closer to the mockup

**Why.** The D5 layouts work, but their spacing differs from "Reposo: reloj y
fecha" and "Reposo: clima" in `mockups/index.html`. Change only
`src/app/screens/sleep_screens.rs` and its tests.

**Clock** (portrait 480 × 800):

- Keep the time at y 180–330.
- Date (Large) right under it, top about 340.
- The rule about 40 px under the date, x 70 to 410.
- The weather line about 28 px under the rule: the 2× icon and the summary
  (Large) centered as one group, the summary vertically centered on the icon.
- The details (Body) centered, right under the weather line.

**Weather:**

- Center the big icon with the temperature and condition as one group.
- Add a 3 px rule above the three days, x 42 to 438, as in the mockup.
- Rain row: Detail if D6 fixed `%`, otherwise Body, with a box tall enough
  for Body at the Large size.

**Done when:**

- `sleep-clock`, `sleep-clock-weather` and `sleep-weather` match the
  mockup's proportions when compared side by side;
- the fit and region tests pass, updated for the new positions;
- every font family and size still fits.

**Status:** done in v0.9.1 (`7b22583`). Positions come from measured text
heights, so every font family and size fits; rain uses Body in a 34 px box.
Device check: the layouts become visible with Phase 3b.

### D8: Audio details describe the real audio path

**Why.** Settings › Audio › Audio details still says `I2S mode: TX ONLY` and
`RX input: DIN GPIO21 deferred`, but the I2S driver is bidirectional and
Voice Notes record through that input.

**Change.**

- `src/app/screens/audio.rs`: describe both directions in the terse style,
  for example `TX + RX / S16 STEREO` and `DIN GPIO21 · voice notes`. Take the
  facts from `src/audio/espidf.rs` and the `I2sDriver::new_std_bidir` call in
  `main.rs`.
- Remove the paragraph about it from `docs/KNOWN_ISSUES.md`.
- Add an `audio-details` preview.

**Done when:** the preview shows the corrected lines and the tests pass.

**Status:** done in v0.9.1 (`e03d6e9`). Device check: open Audio details,
play the test chime, then record and replay a Voice Note.

### D9: Retire the upstream release helpers

**Why.** Wave ships the merged `.bin` built by `firmware.yml` (README › Flash
the board). `scripts/build-release-firmware.sh`, `scripts/flash-release.sh`
and their regression `scripts/test-release-flash-workflow.sh` come from
upstream's ELF releases, and the regression fails on its v1.0.0 file names.

**Change.**

- Search for each of them, and for `scripts/build.sh`, `scripts/flash.sh` and
  `scripts/validate.sh`, in `.github/`, `scripts/`, `README.md` and `docs/`.
- Remove what nothing current uses. Keep what README or a workflow uses, and
  make it read the version from `Cargo.toml` instead of a fixed one.
- Update `docs/KNOWN_ISSUES.md` and any doc that names a removed script.

**Done when:** nothing refers to a removed script, and every kept script
passes `bash -n`.

**Status:** done in v0.9.1 (`dec9ee4`). Removed `build-release-firmware.sh`,
`flash-release.sh`, `test-release-flash-workflow.sh`, `build.sh` and
`flash.sh`; kept `validate.sh` (VS Code task) and `test-host.sh` (README, CI).

### D10: Bible text module (library only)

**Why.** Phase 5 adds a Bible reader (ROADMAP › Phase 5 › Bible). This task
builds and tests the data layer only: no UI, routes or `main.rs`.

**Add `src/bible.rs`**, registered in `src/lib.rs`:

- `pub const BIBLE_ROOT: &str = "/sdcard/RUSTMIX/BIBLE";`
- `Testament { Old, New }`: books 1–39 are Old, 40–66 New.
- `BibleBook { number: u8, name: String, short_name: String, chapters: u16 }`
  and `parse_books(text: &str) -> Result<Vec<BibleBook>>` for `BOOKS.TXT`:
  - lines `number|name|short name|chapters`, fields trimmed;
  - skip blank lines, lines starting with `#` and a UTF-8 BOM;
  - errors name the line: bad number, missing field, number outside 1–66,
    duplicate number, zero chapters.
- `book_file_name(number: u8) -> String`: `01.TXT` to `66.TXT`.
- `Verse { number: u16, text: String }` and
  `read_chapter(reader: impl BufRead, chapter: u16) -> Result<Vec<Verse>>`:
  - lines `chapter:verse<TAB>text`;
  - read line by line and stop after the chapter, so a book is never fully
    in memory;
  - skip blank lines and a BOM; a malformed line is an error naming its line.
- `load_chapter(root: &Path, code: &str, book: u8, chapter: u16)` opens
  `root/code/NN.TXT` and calls `read_chapter`.
- `translations(root: &Path) -> io::Result<Vec<String>>`: the sub-folders that
  hold a `BOOKS.TXT`, sorted, ignoring names that start with `.`.
- Verse of the day, from `/RUSTMIX/BIBLE/VERSES.TXT`:
  - `VerseRef { book: u8, chapter: u16, first: u16, last: u16 }`, written
    `19 23:1` or `19 23:1-3` (book number, chapter, verse or range);
  - `parse_verse_list(text: &str) -> Result<Vec<VerseRef>>`, with the same
    skipping and line-numbered errors;
  - `VerseRef::label(&self, books: &[BibleBook]) -> String`, e.g. `Sal 23:1-3`;
  - `verse_of_the_day(list: &[VerseRef], epoch_day: u32) -> Option<&VerseRef>`
    returns `list[epoch_day % len]`.

**Tests:** every parser on good and bad input (fixtures as strings), chapter
boundaries and the last chapter, a BOM, the daily rotation, and
`translations` on a temporary directory.

**Status:** done in v0.9.1 (`3d2aa1e`). Translation codes must be a single
path component. Device check after Phase 5 wiring: UTF-8 text from the SD and
the last chapter of a book.

### D11: Reading stats module (library only)

**Why.** Phase 5 adds Reading Stats (ROADMAP › Phase 5 › Reading Stats). This
task builds and tests the counting and the file only: no UI or `main.rs`.

**Add `src/reading_stats.rs`**, registered in `src/lib.rs`:

- `pub const READING_STATS_PATH: &str = "/sdcard/RUSTMIX/READER/STATS.TXT";`
- Days are `epoch_day: u32` (days since 1970-01-01), with a `2026-10-03`
  label and its parser; reuse `rtc.rs` helpers where they fit.
- `ReadingStats`:
  - `record(&mut self, day: u32, seconds: u32, pages: u32, book: Option<&str>)`;
  - `mark_finished(&mut self, book: &str)`;
  - `day(&self, day: u32) -> DayStats` with `seconds` and `pages`;
  - `week(&self, today: u32) -> [u32; 7]`: minutes for the seven days ending
    today, oldest first, for the chart;
  - `streak(&self, today: u32) -> u32`: consecutive days with at least 5
    minutes, counting back from today, or from yesterday while today is still
    under 5 minutes;
  - `books_finished(&self) -> usize`;
  - `parse`, `serialized`, `load_from_path`, and `save_to_path` that writes a
    `.TMP` file and renames it;
  - `has_unsaved` and `mark_saved`, so `main.rs` can batch writes.
- The file has one record per line, separated by `|` (FAT names cannot
  contain it):
  - `# Wave reading stats v1`;
  - `day|2026-10-03|1520|34`: date, seconds, pages;
  - `book|<book path>|5400|210|0`: seconds, pages, finished (0 or 1);
  - keep the latest 400 days and 200 books; skip unknown lines.
- `ReadingClock` turns key presses into reading time: `on_key(now_ms)` and
  `take_seconds(now_ms) -> u32`. Time counts only while the last key was less
  than 2 minutes ago, and is never counted twice.

**Tests:** recording across days, the week, the streak rules (including the
morning case), finished books, the file round trip and trimming, and the clock
with gaps shorter and longer than 2 minutes.

**Status:** done in v0.9.1 (`e7900ce`, save made FAT-safe at merge). Books
are kept in last-touched order. Device check after Phase 5 wiring: two saves
in a row replace `STATS.TXT`, and saves are batched.

## Round 1 tasks (done in v0.8.1)

Kept for reference; the verdict is in "Round 1 result" above.

### D1: Option lists instead of cycling values

**Why.** Pressing Select on a setting steps to the next value, so reaching a
value can take several presses and refreshes. Settings › Power already opens a
list; do the same everywhere else.

**Where values cycle today:**

- **Settings › Display**, rows "UI font" and "UI size". These go through
  `AppState::apply_display` in `src/app/state.rs`, which calls
  `DisplayPreferences::cycle_font_family` and `cycle_font_size`
  (`src/app/display.rs`). The screen is `src/app/screens/display.rs`.
- **Reader › Options › Reading preferences**, all six `ReadingPreference` rows
  in `src/reader.rs`:
  - theme, orientation, book font size, book font, paragraph alignment, show
    progress;
  - each cycles with `.next()` in `ReaderUiState`;
  - state handling is the `ScreenRoute::ReaderPreferences` arm in `state.rs`;
  - the screen is `render_preferences` in `src/app/screens/reader.rs`.

**Change:**

1. Select opens a list of every value for the highlighted row.
2. ▲▼ move through the list, wrapping around.
3. Select applies the value and closes the list.
4. Holding BOOT closes the list without changing anything. Handle this at the
   top of `AppState::back()`, like the `ScreenRoute::Power` case.

Other details:

- Draw the list with `widgets::option_list::draw_option_list` (it marks the
  value in use and scrolls long lists). Put the row name above it, and use the
  footer `MOVE  SELECT CHOOSE  HOLD BOOT CANCEL`.
- Give each enum an `ALL` array and a `label()` if it lacks them (`UiFontFamily`,
  `UiFontSize`, `ReadingTheme`, `ReaderOrientation`, `BookFontSize`,
  `BookFont`, `ParagraphAlignment`). Show progress is On/Off.
- Reference implementation: `PowerUiState`, `AppState::apply_power` and
  `screens/power.rs` (`draw_picker`).
- Keep what happens after a change: Display preferences are saved to
  `DISPLAY.TXT` by `main.rs` (unchanged). Reader preference changes still go
  through `finish_preferences_edit`, so a layout change repaginates the book
  as it does today.
- Remove the cycle functions once nothing calls them.

**Done when:**

- No setting cycles any more.
- Tests cover open, move, apply, cancel and back for both screens.
- Previews `display-picker` and `reader-preferences-picker` exist.
- Existing reader tests still pass.

**Status:** done on `side-tasks` (2026-10-03). Settings › Display and Reader
preferences now open option lists: SELECT opens the list at the value in use,
MOVE wraps, SELECT applies and closes, HOLD BOOT cancels. Both pickers draw
with `widgets::option_list::draw_option_list` and the footer
`MOVE  SELECT CHOOSE  HOLD BOOT CANCEL`. The value enums got `ALL` arrays;
`DisplayPreferences::cycle_font_family/size`, `activate_selected_preference`
and the `ReaderUiState` value-cycling functions are gone (tests that called
them now use the picker API). Tests cover open, move, apply, cancel and back
for both screens; previews `display-picker` and `reader-preferences-picker`
are in the `screen-previews` artifact. Host tests (378) and
`cargo +stable fmt` pass locally. Needs on-device test: both pickers on the
panel (open, scroll, apply, cancel), Display changes still reach
`DISPLAY.TXT`, and layout-sensitive Reader choices still repaginate.
The original firmware build passed; physical checks remain with the owner.

**D1 follow-up status (2026-10-03):** committed and pushed as `27bd2bf`.
Choosing any current Reader value closes the picker
without saving, requesting ghost clearing, or rebuilding the book. Added the
shared `option_labels` helper and typed `DisplaySetting::{Font, Size}` API;
all D1 option choices use checked `.get()` lookups, including On/Off, and
invalid indices leave preferences unchanged. Removed unused Reader preference
enum `next`/`previous` methods after checking tracked call sites, including
`main.rs`; compatibility tests now check `ALL` order and persisted markers.
Host tests: 386 passed; stable formatting check and `git diff --check` passed.
No changes to D5 files, previews, module registration, or firmware runtime.
Covered by green CI `37082267617` and firmware `37082265451` on `dc5baec`.
Panel smoke tests remain with the owner.

**Review (main developer, 2026-10-03):** good work; merge after the fix
below. `ci.yml` (378 host tests, fmt, no warnings) and the firmware build
are green on 1773deb, which closes the open item above. Nothing calls the
removed functions, `main.rs` included, and the previews match the Power
picker.

- **Fix before merge: choosing the value in use must do nothing.**
  - The list opens on the value in use, so pressing Select at once is the
    natural "keep it".
  - `ReaderUiState::choose_preference` treats it as a change. Theme and
    show progress are saved again, and the theme flashes the panel to clear
    ghosting. Orientation, font size, font and alignment reopen the book
    through `ReaderLoading` and repaginate it: seconds of waiting and
    battery on a large EPUB, for nothing.
  - Return early at the top of `choose_preference`:

    ```rust
    if index == self.preference_options().1 {
        self.preferences_picker = None;
        return false;
    }
    ```

  - Add a test: choosing the orientation in use returns `false`, closes the
    list and leaves the preferences unchanged.
  - Display needs nothing: `main.rs` only saves `DISPLAY.TXT` when
    `state.display` changed.
- **Optional polish**, in the same follow-up commit if it is cheap:
  - Five arms of `preference_options` and both branches of
    `DisplayPreferences::options` repeat one pattern. One helper in
    `widgets/option_list.rs` replaces them:

    ```rust
    pub fn option_labels<T: Copy + PartialEq>(
        all: &[T],
        current: T,
        label: fn(T) -> &'static str,
    ) -> (Vec<&'static str>, usize)
    ```

    Call it as
    `option_labels(&BookFont::ALL, self.preferences.book_font, BookFont::label)`.
  - `DisplayPreferences::options` and `choose` take `action: usize`, where 0
    means font and anything else size. A `DisplaySetting { Font, Size }`
    enum like `PowerSetting`, mapped once from the row index in
    `apply_display`, makes every call read clearly.
  - `ALL[index % len]` hides a wrong index by wrapping. Use
    `if let Some(&value) = ALL.get(index)`, as `PowerSettings::choose` does.
- Commit the fix as `D1 fix: ...`; do not amend or force-push the D1 commit.

**Follow-up review results (2026-10-03):** regression tests exercise SELECT
on all six current preferences with an actual open TXT Reader session,
including orientation, and verify unchanged session/page/cache state, no
`ReaderLoading` route, no ghost-clearing request, no persistence event, and
an untouched preferences-file sentinel. Every supported current value and
invalid index is also covered, as are helper fallbacks and Display current
choices. Ordinary UI redraw still closes the picker; no extra preference
refresh is requested. No host failures or editor errors in the changed files.
The main developer review above is retained unchanged.

### D2: Remove the BLE remote build

**Why.** The inherited `rustmix-remote-ble` feature (a BLE page turner for a
Wear OS watch) is not used by Wave and complicates `main.rs`.

**Remove:**

- `src/rustmix_remote/` and `pub mod rustmix_remote;` in `src/lib.rs`.
- The `rustmix-remote-ble` feature in `Cargo.toml`, and its optional dependency
  if nothing else uses it. Let cargo update `Cargo.lock`.
- In `src/main.rs`:
  - every `#[cfg(feature = "rustmix-remote-ble")]` item: the import, the event
    queue, the BLE service start, the queue drain in the loop, and the early
    return in `light_sleep_until_wake`;
  - `#[cfg(not(feature = "rustmix-remote-ble"))]` code becomes unconditional.
- Files: `sdkconfig.defaults.rustmix-remote-ble`, `README_RUSTMIX_REMOTE_R1.md`,
  `README_RUSTMIX_REMOTE_BLE_R1.md`, `docs/rustmix-remote/`.
- Scripts: `build_release_ble_v1_2_0.sh`, `build_rustmix_remote_ble_r1.sh`,
  `validate_rustmix_remote_ble_r1.sh`, `validate_rustmix_remote_rrbp.sh`,
  `release_v1_2_0_ble.sh`, `prepare_dual_release_v1_2_0_docs.sh`.
  - First `grep -r` the name in `.github/` and `scripts/`.
  - Keep any script a workflow still calls, and say so in the Status line.

**Done when:**

- `grep -rni "rustmix.remote\|rrbp" --exclude-dir=target --exclude-dir=.git .`
  finds nothing outside `CHANGELOG.md`.
- Host tests and fmt pass.
- The firmware build is green.

**Status:** implemented on `side-tasks` (2026-10-03), pending firmware
validation. Removed the library module
registration, the optional feature and direct `enumset` dependency, and all
remote runtime imports, queue setup/drain, BLE startup and light-sleep bypass
from `main.rs`. The former non-feature Wi-Fi path is unchanged and now
unconditional. Cargo updated `Cargo.lock` through stable host tests/checks;
`enumset` remains transitively required by the ESP service/HAL dependencies.
Removed the architecture removal note, the Wi-Fi release's BLE recommendation,
and the Wi-Fi builder's obsolete config-backup restore.

All tracked references, including `main.rs`, workflow/script callers and
filename links, were enumerated before deletion. Removed all 24 candidates:
the five live remote module files, three duplicate assistant scaffold files,
two remote READMEs, optional SDK defaults, four remote docs, BLE release notes,
two patch overlays and six task-specified scripts. The reference scan passes
with necessary scan
exceptions are `CHANGELOG.md` history and this task specification, whose
original text is retained unchanged.

Retained `scripts/build_release_wifi_v1_2_0.sh` and
`scripts/release_v1_2_0_wifi.sh` (the latter calls the former), along with the
current build/validation/flash/package helpers at the D2 checkpoint. D4 later
removed the unused upstream Wi-Fi release scripts and source packager.
No workflow calls a BLE script;
CI calls `scripts/test-host.sh`, and firmware builds directly with Cargo.
Tests: 389 host tests passed; `cargo +stable fmt --all -- --check`, native
`cargo +stable check --all-targets --all-features`, locked ESP-target Cargo
metadata, and `git diff --check` passed. No version bump, new dependency,
Python edit, protected-file change or unrelated task work.
Committed and pushed as `2e30703`; firmware run `37083296561` passed.
On-device needs:
boot with and without Wi-Fi config, TXT/EPUB page turns using wheel keys,
weather/NTP/file-transfer bursts, battery idle light-sleep, Power/wheel wake
and RTC alarm wake. Deletions and reference scans passed; physical checks
remain with the owner.

### D3: Remove the IMU tilt games

**Why.** Wave keeps button games only (Phase 6 adds Sudoku and Tetris). The
IMU should only run on the Motion diagnostic screens.

**Remove:**

- The three games:
  - Motion 2048: `src/games/motion_2048.rs`, `examples/sd-card/RUSTMIX/APPS/M2048/`;
  - Sokoban Tilt: `src/games/sokoban_tilt.rs`, `APPS/SOKOBAN/`;
  - Tilt Maze: `src/games/tilt_maze.rs`, `APPS/TILTMAZE/`.
- Their install scripts (`install-motion-2048-sd-sample.sh`,
  `install-sokoban-tilt-sd-sample.sh`, `install-tilt-maze-sd-sample.sh`) and
  their lines in `install-sd-examples.sh`.
- The Lua `imu` input path: apps with `input = ["imu"]`,
  `AppState::lua_game_needs_imu_events()`, and the tilt events in
  `lua_runtime/event_bridge.rs`. Remove whatever else becomes unused.
- In `main.rs`, `imu_sampling` should then depend only on
  `ScreenRoute::MotionEvents`. Keep that edit minimal.
- Mentions in `docs/USER_GUIDE.md` and the tree in `docs/SD_CARD_SETUP.md`.

**Keep:**

- Settings › Motion and its event diagnostics (`imu.rs`, `imu_events.rs`,
  `screens/motion.rs`).
- Sudoku, Minesweeper and Hello Grid.

**Done when:**

- `grep -rni "tilt\|motion_2048\|sokoban" --exclude-dir=target --exclude-dir=.git .`
  only finds history in `CHANGELOG.md`.
- The Games list still opens Sudoku, Minesweeper and Hello Grid (add a test).
- Host tests, fmt and the firmware build pass.

**Status:** D3 implemented on `side-tasks` (2026-10-03), pending firmware
validation. Removed
the three game registrations and native bridges, Lua motion dispatch and
IMU input capability. `main.rs` samples events only on `MotionEvents`;
Motion/Details power and diagnostics remain unchanged. Removed game entries
from the generic installer, User Guide and SD tree. Added a Games-route test
that scans the real sample manifests and opens Hello Grid, Minesweeper and
Sudoku through the worker/native bootstrap, exercises SELECT/BOOT and back,
and a manifest test rejecting sensor input.

Checks: `./scripts/test-host.sh` passed (378 tests),
`cargo +stable fmt --all -- --check`, `git diff --check` and
`bash -n scripts/install-sd-examples.sh` passed; editor error checks found no
errors in changed files. Reviewed runtime references including `main.rs`,
manifest/catalog schemas, workflow/script callers and the final diff.
No protected files, dependencies, Python files or versions changed.

Removed all 12 obsolete files:
`src/games/{motion_2048,sokoban_tilt,tilt_maze}.rs`,
`examples/sd-card/RUSTMIX/APPS/{M2048,SOKOBAN,TILTMAZE}/{APP.TOM,MAIN.LUA}`,
and `scripts/install-{motion-2048,sokoban-tilt,tilt-maze}-sd-sample.sh`.
Reference scans preserve the diagnostic exceptions below.

Remaining reference exceptions beyond CHANGELOG and this task document:
diagnostic tilt events in `src/imu_events.rs`, `src/app/screens/motion.rs`,
`src/app/state.rs`, `src/main.rs` and the User Guide's Motion event list;
the manifest rejection test deliberately names the removed input capability.
README's removal note, upstream `RELEASE_NOTES-v1.0.0.md`, the architecture's
old IMU-games statement and protected ROADMAP text are left for the planned
documentation follow-up. Protected ROADMAP text remains unchanged.
Committed and pushed as `cd9dd3f`; firmware run `37083864113` passed.
Device needs: open and use
all three remaining samples, SELECT/short BOOT/hold BOOT, Motion/Details live
readings, MotionEvents tilt/shake/rotate/level and threshold/reset controls,
and IMU off outside diagnostics including Games. Handover requires a green
Xtensa firmware build; physical checks remain with the owner.

### D4: Documentation refresh

Do this last so it describes the code after D1 to D3.

**Write or update:**

- **`docs/USER_GUIDE.md`.** Rewrite it for Wave as it is now.
  - Cover keys, Home, Library and the Reader, Settings (Display, Power, Network,
    Clock, Alarms, Audio), sleep and wake, and the SD card layout (link
    `SD_CARD_SETUP.md`).
  - Use the `screen-previews` artifact and the code as the source of truth.
  - Keep it practical: what a key does on each screen.
- **`CHANGELOG.md`.** Replace the upstream content with Wave's history, newest
  first. Take the versions and commits from the Status table in `ROADMAP.md`
  and `git log`. Keep one line linking to upstream Rustmix Wave for older
  history.
- **`docs/KNOWN_ISSUES.md`.** Only real, current issues: the `%` sign at the
  Detail font size, flashing needs USB power or BOOT held (see README), and
  anything you find.
- **`docs/PHYSICAL_SMOKE_TEST.md`.** A checklist for each release:
  - boot to Home;
  - open a book and turn pages;
  - Settings › Power lists;
  - hold Power to sleep, then wake with Power and with a wheel key;
  - the sleep card when `/RUSTMIX/SLEEP` is empty;
  - weather refresh (Wi-Fi burst);
  - an RTC alarm while asleep;
  - flashing over USB.
- **Upstream leftovers.** Remove `RELEASE_NOTES-v1.0.0.md`, `docs/releases/`,
  and upstream-only release docs and scripts (`docs/RELEASE.md`,
  `*_v1_2_0*.sh`, `package-release.sh`, `release*.sh`).
  - Only remove what no workflow or remaining script uses; check with
    `grep -r <name> .github scripts`.
  - Update the "Documentation" section of `README.md` to match. Leave the
    rest of the README alone.

**Done when:** links between docs work, and nothing removed is still referenced.

**Status:** implemented on `side-tasks` (2026-10-03). Rewrote the User Guide,
Known Issues and release smoke checklist for current firmware, replaced the
upstream changelog with Wave history, and updated only README's Documentation
section. D5 previews are explicitly drawing demonstrations, not runtime modes.
Removed upstream release notes, release docs, Wi-Fi v1.2.0 builders and the
unused source packager after checking workflow and remaining script callers.
Kept the current build-release-firmware, flash-release and regression helpers.
Updated architecture's stale motion-game statement. README's backlog and the
protected ROADMAP still describe removed work; these remain unchanged as
required by file ownership. Owner must run the physical checklist before a
release; no hardware test is claimed here. Host tests (378), stable format
check and relative documentation file-link checks pass. The retained flash
regression fails its pre-existing hard-coded v1.0.0 artifact expectation
against v0.7.0; recorded in Known Issues rather than changing unrelated code.

### D5: Sleep screen layouts (drawing only)

**Why.** Phase 3b adds clock and weather sleep screens. The main developer
wires up when they show and how they refresh; this task only draws them.
Mockup: "Reposo: reloj y fecha" and "Reposo: clima" in `mockups/index.html`.

**Add:**

- `src/app/screens/sleep_screens.rs`, registered in `src/app/screens/mod.rs`,
  with exactly this API. Field examples are in the comments.

  ```rust
  pub struct SleepClock<'a> {
      pub time: &'a str,                        // "13:42"
      pub date: &'a str,                        // "Friday, October 2"
      pub weather: Option<SleepWeatherLine<'a>>,
      pub battery_percent: Option<u8>,
      pub wake_hint: &'a str,                   // "Press any key to wake"
  }

  pub struct SleepWeatherLine<'a> {
      pub weather_code: u16,                    // WMO code
      pub summary: &'a str,                     // "18° · Partly cloudy"
      pub details: &'a str,                     // "H 21° · L 11° · Rain 10%"
  }

  pub struct SleepWeather<'a> {
      pub place: &'a str,                       // "Madrid"
      pub updated: &'a str,                     // "Fri, Oct 2 · updated 13:30"
      pub weather_code: u16,
      pub temperature: &'a str,                 // "18°"
      pub condition: &'a str,                   // "Partly cloudy"
      pub details: &'a str,                     // "H 21° · L 11° · Wind 12 km/h · Rain 10%"
      pub days: &'a [SleepWeatherDay<'a>],      // up to 3
      pub battery_percent: Option<u8>,
      pub wake_hint: &'a str,
  }

  pub struct SleepWeatherDay<'a> {
      pub name: &'a str,                        // "Sat"
      pub weather_code: u16,
      pub range: &'a str,                       // "23° / 12°"
      pub rain: &'a str,                        // "0%"
  }

  pub fn render_sleep_clock(
      display: &mut OrientedFrameBuffer<'_>,
      preferences: DisplayPreferences,
      clock: &SleepClock<'_>,
  ) -> Result<(), Infallible>;

  pub fn render_sleep_weather(
      display: &mut OrientedFrameBuffer<'_>,
      preferences: DisplayPreferences,
      weather: &SleepWeather<'_>,
  ) -> Result<(), Infallible>;
  ```

- Frame-level wrappers in `src/app/mod.rs` next to `render_sleep_card`
  (`render_sleep_clock` and `render_sleep_weather` taking `&mut FrameBuffer`).
  They clear the frame and draw in portrait.
- `src/app/widgets/big_digits.rs`: large numerals drawn with embedded-graphics
  primitives, because the fonts stop at about 30 px.
  - Seven-segment style with slightly rounded or beveled segments, stroke
    about 1/8 of the height.
  - Supports `0-9`, `:`, `°` and `-`.
  - API: `draw_big_text(display, text, center_x, top, height)` returning the
    drawn width, plus `big_text_width(text, height)`.
  - The clock uses about 150 px digits, the weather temperature about 110 px.
- `Icon::draw_scaled(display, top_left, scale, color)` in
  `src/app/widgets/icons.rs` (nearest neighbour), for weather icons at 2× to 5×.

**Layout** (portrait 480 × 800, black on white):

- Both screens have the same 3 px frame as the sleep card (inset 16 px).
- Both have a bottom row inside the frame: the wake hint on the left, battery
  % on the right.
- **Clock screen**, top to bottom:
  1. the time, centered, around y 180–330;
  2. the date in the Large style, centered;
  3. a horizontal rule;
  4. if `weather` is set: the 2× icon with the summary (Large), and the details
     (Body) below.
- **Weather screen**, top to bottom:
  1. place (Detail) and updated line (Body), top left;
  2. a big icon (about 5×) beside the big temperature, with the condition
     under it;
  3. the details centered;
  4. three day columns (name, 2× icon, range, rain).

**Tests and previews:**

- Previews `sleep-clock`, `sleep-clock-weather` and `sleep-weather`. Add them
  in `render_screen_previews` next to `sleep-card`, the same way.
- One render test per layout that checks a few pixels (frame, a digit
  segment).
- A test of `big_text_width`.

**Done when:** the previews look like the mockup, all text fits at every
Display size (Compact, Standard, Large), and the tests pass.

**Status:** committed and pushed as `dc5baec` (2026-10-03).
Added the exact drawing API and portrait frame wrappers, rounded
seven-segment digits (`0-9`, `:`, `°`, `-`), allocation-free width measurement,
and transparent nearest-neighbour icon scaling (zero scale is a no-op).
Both layouts use the inset-16, 3 px frame and measured, bounded text; the wake
hint and battery percent have separate footer budgets. Long temperatures shrink
to fit beside the 5× icon; forecasts show at most three 2× icon columns.
Host tests: 399 passed (13 D5 tests added), including frame/digit pixels,
glyph widths, icon scaling, all six size/family combinations and extreme text.
Stable full formatting check, native library check, `git diff --check` and
editor diagnostics pass. Required previews and 18 typography variants are in
`/tmp/wave-d5-previews`; required images and Compact/Large family samples were
visually inspected without overlaps. No settings, routes, runtime, dependencies,
versions or prohibited files changed; D1 review remains unchanged.
CI `37082267617` and firmware `37082265451` passed. Still needs on-device
checks of frame/digit clarity,
both font families at all sizes, negative temperatures and footer readability.
Sleep-mode selection, refresh scheduling and wake behavior remain Phase 3b work.
