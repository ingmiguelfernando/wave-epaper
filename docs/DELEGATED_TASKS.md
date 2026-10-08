# Delegated tasks

Small, self-contained tasks for a second developer or AI working in parallel
with the main line. Read this file first, then
[architecture.md](architecture.md) and [ROADMAP.md](ROADMAP.md).

## Round 5 result (2026-10-08)

Pull request #5 (D24 to D28) was reviewed, merged into `main` and released as
v0.9.7 (milestone `hubs-and-stats`). The screens follow the mockup and the
round has many tests, but two features never ran on the device: the tested
path and the device path were different. The `side-tasks-5` branch is
deleted. Round 6 will be added here with the next main-line release.

What the main developer changed at merge time:

- **D24:** the device never showed the start list. The event bridge still
  built the fixed puzzle of `sudoku.init("…")`, `sudoku.init()` did not
  parse, and nothing handed the save to the game. Now the bridge opens the
  start list, the runtime passes in the save and the best times, and the
  sample `MAIN.LUA` is `sudoku.init()`. Every `New` game used one constant
  seed, so it was always the same puzzle; the press time now seeds it. The
  solver walked the cells in order, millions of steps for some Hard puzzles
  (seconds on the device, a watchdog risk); it now fills the cell with the
  fewest candidates first. The time is right-aligned on the title bar (new
  `text_right` canvas command), and a solve says `Solved in … · best …`.
  Removing the save also removes its `.BAK`, which loading would read back.
- **Clocks:** `main.rs` passed `last_activity.elapsed()` as the event clock.
  It is about 0 at every press, so neither the Sudoku time nor the reading
  time would ever grow. It now passes the time since boot.
- **D25:** hold BOOT in a game went back to the old catalog list, not the
  hub. The hub now has the mockup's status bar with the game count; the
  selected card's plain icon is visible; the info box wraps instead of
  cutting its text; and the Sudoku card keeps the best time after a solve
  deletes the save. One `grouped()` in `regional.rs` replaces the two copies,
  and the hub uses Sudoku's `time_text()`.
- **D26:** the rows repeated the menu's titles and subtitles; they now come
  from `category_entries`, with one value per route. Values read like the
  mockup (`Off`, `Not set up`, `Manual`), the version is on the status bar,
  and the fit test measures title and value side by side as the row draws
  them.
- **D27:** the day came from the raw RTC, which keeps its own time-zone
  basis, so late reading could land on the wrong day; `AppState::local_day()`
  serves Home, the screen and the saves. Without a clock the screen was
  blank. A press on the last page counted as a page turn. `main.rs` dropped a
  `#[must_use]` result, a warning in the firmware build.
- **D28:** the short BOOT queued a recording that started only at the next
  key press; `main.rs` now applies it at once. The status bar says `Online`
  or `Offline`.

Verdict per task:

- **D24:** done after the wiring, seed and solver fixes.
- **D25:** done after the back route and the hub fixes.
- **D26:** done after the rows moved to the menu entries.
- **D27:** done after the clock and local-day fixes.
- **D28:** done after the BOOT fix.

Do differently next time:

- **Test the device path.** A test that builds the game with
  `start_list(…)` cannot show that the bridge never calls it. Add one test
  through the entry the device uses (the catalog, `LuaEventBridge::load`,
  `open_selected`), and read `main.rs` for every value the feature needs.
- **Clocks.** Elapsed time needs a clock that keeps running between presses.
- **Local dates.** Dates shown to the user or keyed by day go through
  `regional.localize_rtc` (or `AppState::local_day`).
- **Cost on the device.** A loop that is quick on the host can take seconds
  on the ESP32-S3; count the steps of the worst case before handing over.
- **Status lines.** "Done" for a start list the device never showed is not
  honest; the User Guide also missed the new Sudoku behaviour.

## Lessons from earlier rounds

- No warnings: read the `warning:` lines of the test and firmware builds.
- Look at previews like a user: "which row is selected?" needs an answer.
- Test what the other side reads: parse a request body and compare fields;
  `contains` hides double escaping.
- Only what the installer creates exists on the card: create the parent
  folder before the first write, and test without it.
- Write docs for `main` as it will be after the merge, not "on this branch";
  if another branch changes the same feature, say so in the Status line.
- Compare previews with the mockup side by side; spacing is part of "looks
  like the mockup".
- Check what the device allows, not only the host: FAT cannot rename onto an
  existing file. Copy a pattern already proven on the device.
- Keep production code above `#[cfg(test)] mod tests`, and test helpers
  inside it.
- Size tests to real ranges. A test that compares a constant with its own
  literal proves nothing; test behaviour such as fit or rendering.
- Status lines: about eight lines, no local `/tmp` paths. Logs and evidence
  go in the pull request description.
- Indent Markdown continuation lines with two spaces, not tabs.
- A change to a shared draw path needs a look at every screen that goes
  through it, not only yours.
- Do not key behaviour on text from the SD card; match on types.
- Pin directions in tests; "four turns return to the start" passes both ways.
- When a later task adds a shared module, use it in the earlier tasks of the
  same round.
- A new SD sample needs its lines in `install-sd-examples.sh` and
  SD_CARD_SETUP.
- Labels: singular and plural ("1 book"), no half labels without a number.
- Power safety: walk through two interruptions in a row, not only one.

## How to work

- **Branch.** Each round of tasks gets its own branch from the latest `main`
  (round 6: `side-tasks-6`), with one commit per task. Push often.
- **Pull request.** When done, open a pull request to `main` with a
  description: per task what changed, the checks, the previews to look at and
  the device checks. Do not merge it; the main developer reviews it, resolves
  any conflicts and merges.
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
  terse (bottom bar labels such as `move`, `change`, `hold: back`).
- Follow the existing patterns; the closest reference is named in each task.
  No new dependencies unless a task says so.
- Do not bump the version (Cargo.toml, sdkconfig.defaults, build_info.rs); the
  main developer does that when merging.
- Do not add or change `.py` files.
- Comments only where the code cannot speak for itself, one short line.
- Logic changes come with host tests. UI changes come with a preview: add an
  entry at the end of `preview_states()` in `src/app/preview.rs`, then check
  the PNG in the `screen-previews` artifact.
- **No warnings.** The test build must print no `warning:` lines. CI stays
  green with warnings, so read the log yourself.
- **Bottom bar.** Every screen ends with
  `draw_bottom_bar(display, preferences, &HINTS)` from
  `widgets/bottom_bar.rs`. Use a shared set (`BACK_HINTS`, `OPEN_HINTS`,
  `RUN_HINTS`, `CHANGE_HINTS`, `CHOOSE_HINTS`, `KEYBOARD_HINTS`) or a local
  `const` of `(KeyCap, "label")` pairs: lowercase labels, and `hold: ` for a
  long BOOT press. Content stays above `BOTTOM_BAR_TOP` (752). Never draw
  footer text yourself.
- **SD games** leave y 752 and below free. `game_hints` in
  `screens/lua_game.rs` picks the hints for the game's mode, and the game
  adds `GAME_BOTTOM_BAR_RECT` to its dirty regions when the mode changes.
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
- **SD files.** Settings files go through `sd_file::replace` and
  `sd_file::read_to_string` (`.TMP` then `.BAK`); FAT cannot rename onto an
  existing file. A new folder does not exist until code creates it
  (`fs::create_dir_all` before the first write).
- **JSON.** Read with `json_lite`, write with `json_lite::escape` once per
  value. No serde: the Xtensa backend cannot build its float visitor.
- **Game dirty regions** are bookkeeping today: the panel refreshes the
  whole screen with a partial refresh. Keep them right and at most
  `MAX_DIRTY_REGIONS`; a whole-canvas region is fine when a step redraws
  most of the screen.
- **Shared modules.** Dates: `civil_date`; the user's day:
  `AppState::local_day`; thousands separators: `regional::grouped`; play
  time: `games::sudoku::time_text`. Look for a helper before writing one.
- **rustfmt** (CI is the judge): an array whose items fit in 60 characters
  stays on one line, after a break at `=` when the line would pass 100;
  wider arrays go one item per line. `a.field.method(..)` chains break past
  60 characters.
- **rustfmt call width:** when the arguments of one call add up to more than
  60 characters, they go one per line, even if the line fits in 100. A
  `draw_header(…)` with a 30-character subtitle did.

## Round 5 tasks (done in v0.9.7)

Start from `main` at v0.9.4 or later. Five tasks, in this order; D25 uses
D24's records.

**Main line during this round.** The main developer builds Phase 3b (clock
and weather sleep screens, refreshed while the device sleeps) and redraws
Weather and the sleep screens like the mockup (bold display digits, the
mockup's weather icons). Leave these files alone:

- `src/main.rs`, except the lines a task names;
- power and sleep: `src/power*.rs`, `src/sleep_*.rs`,
  `src/app/screens/sleep_*.rs`, `src/app/screens/power*.rs`;
- weather: `src/weather*.rs`, `src/app/screens/weather*.rs`;
- drawing: `src/app/widgets/icons.rs`, `src/app/widgets/big_digits.rs`,
  `src/app/typography/`, `scripts/fonts/`;
- photos: `src/photos/`, `src/app/screens/photos.rs`.

The fonts have no `→`, `★` or `⌫`; write `›`, draw a star shape, or use
`×`, as the existing screens do.

### D24: Sudoku difficulty, timer and resume

**Why.** Phase 6. Mockup "Sudoku" shows `Sudoku · Medium` with the play
time on the right, and mockup "Games" shows `Medium · in progress 35/81` and
`Best time 12:41 · auto-saved`. Today the SD app declares one fixed puzzle,
there is no difficulty, and leaving the game loses it.

**Change.**

- **Puzzles.** New `src/games/sudoku_puzzles.rs`: a seeded generator that
  fills a solution grid by shuffled backtracking, then removes cells in
  random order while the puzzle keeps exactly one solution. Targets: Easy 40
  givens, Medium 32, Hard 26; if uniqueness stops the removal earlier, keep
  the puzzle. Use a small xorshift like `tetris.rs`; no new dependency. The
  caller passes the seed.
- **Start.** Opening Sudoku shows an option list (`widgets/option_list.rs`):
  `Continue · Medium · 35/81` when a saved game exists, then `New · Easy`,
  `New · Medium`, `New · Hard`. `APPS/SUDOKU/MAIN.LUA` becomes
  `sudoku.init()`; an old card's `sudoku.init("…")` puzzle still parses and
  appears as a last option, `SD puzzle`.
- **Timer.** Play time in seconds: each key press adds the time since the
  previous one, at most 60 s, so a game left open does not run up the clock.
  The title bar shows `Sudoku · Medium` on the left and `7:41` (or
  `1:02:03`) on the right, updated whenever the screen redraws; no ticking.
  Give the game the elapsed milliseconds with each event, so tests choose
  the times.
- **Resume.** Hold BOOT saves `/RUSTMIX/GAMES/SUDOKU.TXT` (`key=value`:
  `difficulty`, `puzzle` and `board` as 81 digits each, `seconds`) through
  `sd_file`, creating the folder first. Save only on leaving, never per
  move. Follow the records pattern: the runtime sets a flag and `main.rs`
  writes the file next to the records save. Solving the puzzle deletes it.
- **Best times.** `GameRecords` gains `sudoku_easy`, `sudoku_medium` and
  `sudoku_hard` (seconds, 0 = none), written once when a solve beats them.
  The solved screen says `Solved in 12:41 · best 11:02`.

**Tests:** the generator (same seed, same puzzle; givens per difficulty;
exactly one solution, with a solver that stops counting at two), capped
timer gaps, the save file round trip and a missing or malformed file, best
times only on improvement, the start list with and without a save.
**Previews:** `sudoku-start` (with a saved game); `sudoku-row` and
`sudoku-number` show the difficulty and a time. **main.rs:** the save next
to the records save only.

**Status:** done in v0.9.7 (`2ec0ff1`). At merge, the start list was wired
on the device path, new games seeded by the press time, the solver made
fast and the clock right-aligned; see the Round 5 result.

### D25: Games hub as in the mockup

**Why.** Home › Games shows a single `SD Games` row, then a generic list of
SD apps with author and version. Mockup "Games" lists the games themselves,
each with its state.

**Change.**

- Home › Games opens the hub: the status bar `Games` with the number of
  games, then one card per SD app of kind `game`, with an icon, the name and
  two detail lines:
  - Sudoku: `Medium · in progress 35/81` or `No game in progress`, then
    `Best time 12:41 · auto-saved` or `No best time yet` (from D24);
  - Tetris: `Zen · best 18,950`, then `No gravity: pieces move when you
    press`. This is the third user of the private `grouped()` helpers;
    move one to a shared place (see Tips);
  - any other game: its manifest description on two lines.
- Icons: the Sudoku grid and the Tetris blocks of the mockup's 60 × 60
  drawings, made with primitives; other games get a plain framed square.
- A dashed info box with what is true today: `Moves use the fast partial
  refresh; a full refresh now and then cleans ghosting.` (the mockup's
  "only the cells that change" is not how the panel refreshes yet).
- ● opens the selected game directly; bottom bar `▲▼ game ● play BOOT
  hold: back`.
- Category screens say `1 items` today; use singular and plural there too.

**Tests:** cards for the sample catalog, Sudoku lines with and without a
save and a best time, Tetris thousands separator, plural labels.
**Preview:** `games` with a Sudoku save and both records.

**Status:** done in v0.9.7 (`eae988d`). At merge, games return to the hub,
the hub got the mockup's status bar, a visible selected icon and a wrapped
info box, and `grouped()` moved to `regional.rs`.

### D26: Settings as in the mockup

**Why.** Mockup "Settings": a handful of groups on one page, each row with a
short description and its current value. Today Settings is an 11-row list
over two pages.

**Change.**

- Rows in this order, each with description and value:
  - Display: `Font, size, ghost cleanup`, value like `Inter · M`;
  - Sleep screen: `Photo, clock, weather`, value like `Photo · 12 starred`
    (Phase 3b adds a mode on the main line; compute the value in one
    function so the merge changes only that function);
  - Weather: `Service, interval, location`, value `On · 2 h`, `Manual` or
    `Off`;
  - Wi-Fi & transfer: `Network, file portal`, value from the network state;
  - Clock & alarms: `Time, date, alarms`, value `2 alarms` or `No alarms`;
    opens a list with Clock and Alarms;
  - Power: `Auto-sleep, battery log`, value the battery percentage;
  - System: `Version, SD, diagnostics`, value `v0.9.4`; opens a list with
    Device Info, Audio, Environment and Motion.
- Reading and AI from the mockup come later; leave them out rather than
  adding SOON rows.
- Row style as the mockup's `.set` rows: title and description on the left,
  value on the right, the selected row inverted with the marker the other
  lists use. Everything fits at every font size.
- The two sub-lists are category routes like Home's, with `parent()` so
  hold BOOT walks back. Every existing screen stays reachable.
- Update the User Guide's Settings row and the smoke test.

**Tests:** row order and routes, values from state, back from every
sub-screen, fit at every font family and size. **Previews:** `settings`,
`settings-system`, `settings-clock-alarms`, and Settings at the Large size.

**Status:** done in v0.9.7 (`fa72151`). At merge, the rows came from the
menu entries, the values were reworded like the mockup and the fit test
measures each row as drawn.

### D27: Reading Stats wiring

**Why.** Phase 5. `reading_stats.rs` (D11) and its screen (D14) exist, but
nothing records reading time, nothing loads or saves `STATS.TXT`, and
`ScreenRoute::ReadingStats` cannot be reached.

**Change.**

- A host-tested reading session: while a Reader page is open, each key press
  adds the time since the previous press, at most 2 minutes, and each page
  turn adds a page. The day comes from the RTC's local date (`civil_date`);
  without a set clock, record nothing.
- Load `/RUSTMIX/READER/STATS.TXT` at boot with the other settings files.
- Save when there is unsaved data and five minutes have passed since the
  last save, when the Reader closes, and before sleep. Never per page.
- Home › Reading Stats opens the screen (the row shows SOON today). The
  Home row shows `5-day streak` when there is a streak; the Continue reading
  card adds `· 25 min today` when today has reading time.

**Tests:** session time with capped gaps, page counts, the day change at
midnight, no save without changes, the Home labels. **Previews:** `home`
with a streak and reading time. **main.rs:** the boot load, the five-minute
save, and one line before sleep next to the battery-log save.

**Status:** done in v0.9.7 (`ddca716`). At merge, `main.rs` passes the time
since boot as the event clock, days are local, and the screen explains a
missing clock.

### D28: AI hub (drawing and navigation)

**Why.** Phase 7, mockup "AI": XiaoZhi and Voice Notes on one page with the
recent notes. Today Home › AI is a two-row category.

**Change.**

- Home › AI opens the hub: status bar `AI` with the network state; a
  XiaoZhi card (the mockup's face icon, `Voice chat · xiaozhi.me`, a `SOON`
  badge); a Voice Notes card (microphone icon, `Record › transcript ›
  summary`); `RECENT NOTES` with the three newest notes from the Voice Notes
  catalog (title, then `Oct 2 · 12:04 · 18 min`); a dashed info box
  `Recordings stay on the SD card.`; bottom bar `▲▼ move ● open BOOT new
  note`.
- ● opens XiaoZhi (its placeholder), Voice Notes, or the selected note's
  details; short BOOT starts a recording the way the Voice Notes screen does
  today, through the same request.
- No network code and no provider settings: transcription, summaries and
  Settings › AI are Phase 7.

**Tests:** rows and selection, the three newest notes, an empty catalog.
**Preview:** `ai` with sample notes.

**Status:** done in v0.9.7 (`482d639`). At merge, `main.rs` starts the
recording on the short BOOT itself, and the status bar says `Online` or
`Offline`.

## Round 4 tasks (done in v0.9.3)

Kept as the reference for the code they added.

### D18: Sudoku three-step entry

**Why.** Phase 6, mockup "Sudoku": choose a row, then a cell, then a number.
It reads without instructions, so it replaces Sudoku's H/V axis mode
(Minesweeper keeps its own).

**Change** (`src/games/sudoku.rs`, `game_hints` in `screens/lua_game.rs`):

- **Row:** ▲▼ moves the row highlight and wraps, skipping rows with no
  editable cell; ● confirms.
- **Cell:** ▲▼ moves between the row's editable cells (givens are skipped);
  ● confirms.
- **Number:** ▲▼ walks 1 to 9 and ⌫ (erase); ● places it and returns to Cell
  in the same row.
- Short BOOT goes back one step and does nothing in Row; hold BOOT still
  leaves the game.
- Draw as in the mockup: the strip `1 · ROW ▸ 2 · CELL ▸ 3 · NUMBER` with the
  current step inverted, the number strip `1 … 9 ⌫` with the choice inverted,
  and `Row 5 · Col 3 · options: 2 · 6 · 9` (numbers not yet in that row,
  column or box).
- Bottom bar per step: Row `▲▼ row ● choose BOOT hold: back`, Cell
  `▲▼ cell ● choose BOOT back`, Number `▲▼ number ● place BOOT back`. Expose
  the step with a public method and refresh `GAME_BOTTOM_BAR_RECT` when it
  changes.
- Keep the conflict and completion checks, `MAX_GAME_DRAW_COMMANDS` and
  `MAX_DIRTY_REGIONS`. The title stays `Sudoku`; the timer and difficulty
  come with Phase 6.
- Update the Sudoku rows of the User Guide and the smoke test.

**Tests:** each step's keys, skipped givens and rows, options, erase, BOOT
back from each step, completion, and the command and region limits.
**Previews:** `sudoku-row` and `sudoku-number`, built like `tetris`.

**Status:** done in v0.9.3 (`5dd70ff`). At merge, the mockup pass: title bar,
  visible row band, inverted cursor cell, boxed number strip, no key
  instructions in the status line. Device check: play through all three
  steps and BOOT back from each.

### D19: Keep the Tetris best score

**Why.** Best lives in memory and is lost when Tetris closes. Phase 6 shows
best scores in the Games hub.

**Change.**

- New `src/games/records.rs`: `GameRecords { tetris_zen: u32 }` in
  `/sdcard/RUSTMIX/GAMES/RECORDS.TXT`, `key=value` lines under a header
  comment like `POWER.TXT`, read and written through `sd_file`. A missing
  file means no records; unknown keys are ignored.
- `TetrisApp::set_best(u32)` and `best()`.
- Wiring, kept small: load the records at boot with the other settings; set
  the best when a Tetris session opens; when it closes (hold BOOT) with a
  higher best, mark the records changed so `main.rs` saves them once, like
  `take_starred_changed()` for starred photos. Never save per piece: every SD
  write costs battery.

**Tests:** round trip, missing and malformed files, one save after a new
best, no save after a lower score. **Device check:** score, leave and reopen
(Best shows it), then reboot (still there).

**Status:** done in v0.9.3 (`5a57d3f`). At merge, the first save creates
  `/RUSTMIX/GAMES/`. Device check: beat the best, leave and reopen, then
  reboot; the log shows one save.

### D20: Bible book and chapter pickers (drawing and state only)

**Why.** Phase 5, mockup "Bible: elegir libro". The main line adds the
routes, the reading view and the wiring later.

**Change.**

- New `src/bible_nav.rs`: the picker state over `Vec<BibleBook>`.
  - Eight sections by book number: Pentateuch 1–5, History 6–17, Poetry &
    Wisdom 18–22, Major Prophets 23–27, Minor Prophets 28–39, Gospels & Acts
    40–44, Paul's Letters 45–57, General Letters & Revelation 58–66; tabs
    `PEN HIS POE MAJ MIN GOS PAU REV`. Books missing from `BOOKS.TXT` and
    empty sections are skipped.
  - Books: ▲▼ moves within the section, short BOOT jumps to the next section
    (wrapping), ● opens the chapter grid.
  - Chapters: ▲▼ moves one chapter, short BOOT moves ten, ● returns
    `Open { book, chapter }`; `back()` returns to the books.
- New `src/app/screens/bible.rs`, following the mockup:
  - books: header `Go to · Book` with the translation code,
    `Old Testament · section 3 of 8`, the section name, the tab strip with
    the current tab inverted, rows `Job` … `42 ch.`, the line
    `Next section (BOOT): Major Prophets · Isaías, …` fitted to the width,
    and the bar `▲▼ book ● chapter BOOT section`;
  - chapters: the book name, a grid of chapter numbers with the current one
    inverted, paged when it does not fit, and the bar
    `▲▼ chapter ● read BOOT +10`.
- A `#[cfg(test)]` sample `BOOKS.TXT` with the 66 Reina-Valera names, like
  `SAMPLE_CONFIG` in `weather_config.rs`. No route and no `main.rs` changes.

**Tests:** sections and skipped books, section wrap, chapter moves and wrap,
the Open outcome, fit at every font family and size. **Previews:**
`bible-books` and `bible-chapters` (Psalms, 150 chapters).

**Status:** done in v0.9.3 (`c3e25e5`). ● returns the choice as
  `open() -> Option<(book, chapter)>`. At merge: text sizes of the selected
  row and tab, the list window, the next-section line and the chapter grid.
  Routes and the reading view come with Phase 5.

### D21: A small JSON reader

**Why.** Phase 7 talks JSON to OpenAI-compatible APIs and to XiaoZhi. Wave has
no JSON dependency, and the Xtensa backend cannot reliably build serde's
float visitor, so `weather.rs`, `dictionary.rs` and `wifi_transfer.rs` each
hand-roll pieces.

**Change.**

- New `src/json_lite.rs`:
  - `parse(text) -> Result<JsonValue>` with `Null`, `Bool`, `Number(String)`,
    `String`, `Array` and `Object(Vec<(String, JsonValue)>)`. Numbers stay
    text; `as_i64()` parses integers on demand, and nothing parses floats.
  - `get(key)`, `index(i)`, `as_str()`, `as_bool()` and `as_i64()`.
  - `escape(text) -> String` for requests: quotes, backslash and control
    characters escaped, other Unicode kept.
  - `\uXXXX` escapes including surrogate pairs; lone surrogates are errors.
  - Limits: 64 KiB of input and nesting depth 32; errors give the byte
    offset.
- Leave the three hand-rolled parsers as they are; list in the Status which
  of their helpers could move to `json_lite` later.

**Tests:** every value type, nesting, escapes both ways with Spanish text and
an emoji, the limits, malformed input (trailing comma, unterminated string,
bad escape).

**Status:** done in v0.9.3 (`7d67ad8`). Later candidates to move onto it:
  `weather.rs`, `dictionary.rs` and `wifi_transfer.rs`.

### D22: OpenAI-compatible requests and responses (library only)

**Why.** Voice Notes v2 (ROADMAP › Phase 7) transcribes with
`POST /audio/transcriptions` and summarizes with `POST /chat/completions`.
This task builds and reads the messages; Phase 7 adds HTTPS, keys and
screens.

**Change.** New `src/ai_client.rs`, with no network access and no keys:

- `transcription_request(model, language, wav, boundary)` returns the
  `multipart/form-data` content type and body: `model`, `language`,
  `response_format=json` and `file` (`note.wav`, `audio/wav`).
- `parse_transcription(json)`: the `text` field.
- `summary_request(model, transcript)`: chat JSON whose system prompt asks
  for a title line, then a short summary in the transcript's language;
  `temperature` 0.2 and a `max_tokens` limit.
- `parse_chat_completion(json)`: `choices[0].message.content`; then
  `split_title(content)` returns a title of at most 40 characters and the
  summary.
- `api_error(status, json)`: the provider's `error.message` when present.

**Tests:** a byte-exact multipart body for a tiny WAV, a transcript with
quotes and accents, success and error responses taken from the Groq and
OpenRouter documentation, title splitting.

**Status:** done in v0.9.3 (`1e2fd22`). At merge, the transcript is escaped
  once (it was escaped twice) and the model id is escaped.

### D23: XiaoZhi messages (library only)

**Why.** XiaoZhi voice chat is the largest Phase 7 item. xiaozhi-esp32 needs
ESP-IDF 6, so Wave ports its documented protocol to Rust. This task models
the messages; Phase 7 adds WebSocket, Opus and the screens.

**Change.** New `src/xiaozhi/` with `protocol.rs`, from the WebSocket protocol
document of the xiaozhi-esp32 repository (`docs/websocket.md`) and its
`main/protocols/protocol.h`. Name the commit you read in the module doc.

- Device to server: `hello` (version, `websocket` transport, Opus 16 kHz mono
  60 ms), `listen` (`start`, `stop`, `detect`; modes `auto`, `manual`,
  `realtime`), `abort`, and `mcp` with the JSON-RPC payload passed through.
- Server to device: `hello` (session id, audio parameters), `stt`, `llm`
  (emotion, text), `tts` (`start`, `stop`, `sentence_start` with text),
  `mcp`, and `Unknown(type)` for anything else.
- Binary audio frames: the versioned headers (type, timestamp, payload size),
  encoded and decoded with size checks.

**Tests:** each message both ways against the document's examples, the frame
headers, malformed and unknown input.

**Status:** done in v0.9.3 (`bd7b52c`), from xiaozhi-esp32 `af5a8c5`.

## Round 3 tasks (done in v0.9.2)

Kept as the reference for the code they added.

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

**Status:** done in v0.9.2 (`132a16f`). At merge, Rotate was made clockwise
  (the engine turned counter-clockwise) and the kick tests updated.

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

**Status:** done in v0.9.2 (`53877be`, `7809768`, `cf9b1e2`). At merge, the
  key caps became the shared bottom bar of every screen, and the SD
  installer copies `APPS/TETRIS`. Device check: play from Games › SD Games.

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

**Status:** done in v0.9.2 (`fbbad98`). At merge: "1 book finished", a blank
  change label without a previous week, and `civil_date` for weeks and
  years. The route and Reader wiring come with Phase 5.

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

**Status:** done in v0.9.2 (`075e512`). At merge, a `.BAK` left as the only
  copy is renamed back before the cleanup. Device check: pull the power
  right after a settings change; the next boot keeps the old or new value.

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

**Status:** done in v0.9.2 (`4624eba`).

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

**Status:** done in v0.9.2 (`248826b`). Device check: the serial log at boot.

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
