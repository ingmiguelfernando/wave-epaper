# Delegated tasks

Small, self-contained tasks for a second developer or AI working in parallel
with the main line. Read this file first, then
[architecture.md](architecture.md) and [ROADMAP.md](ROADMAP.md).

## Round 7 result (2026-10-09)

Branch `side-tasks-7` (D34 to D38) came as pull request #6, with `main`
merged in first and CI green. It was reviewed on `review-7`, merged and
released as v0.10.3 (milestone `reader-library`). The alarm save (D38) and
hold to repeat (D37) were wired in `main.rs` and well tested. The screens
(D34, D35) were drawn without looking at their previews, and parts of both
tasks were left out of the status.

What the main developer changed at merge time:

- **D34:**
  - The text box moved up under the new header, but the Reader's lines per
    page still fit the old one, so about 100 px stayed blank above the bar.
    The table now fills the box: two to four more lines in portrait, one in
    landscape. A test checks every size, face and orientation fits. Books
    paginate again on their first open, as after a font change.
  - The bar's percent was the index progress (`CACHE 34%` before), so page
    1 read `34%`. It is now the place in the book (`place_percent`).
  - `draw_bottom_bar` drew at y 752 in every orientation, so a landscape
    page had no bar and no page label. The bar now sits along the bottom of
    either orientation, and `reader-page-landscape` shows it.
  - The bookmark corner covered the battery's `%`. It is now a ribbon at
    the edge and the status moves left to make room.
  - The Bible header had no clock or battery and left 70 px blank above
    the title; both fixed.
- **D35:**
  - The header was removed with nothing in its place: the top 80 px were
    blank, and the count sat in a boxed status row. The Library now has the
    black status bar (`Library`, `2 books`) the mockup shows.
  - At Large size `BOOKMARKS` ran into `FILES`. The chips are sized by
    their labels and drop to the detail size when they would not fit.
  - The selected row drew its bar black on black, so it vanished. Rows draw
    in the selection's ink, titles are in the Reader's book face, and FILES
    shows file sizes as the task asked.
  - An EPUB's percent divided a text offset by the zipped file's size. EPUB
    rows show the chapter until the place stores a percent.
  - Only seven rows were drawn, so ▼ past the seventh book moved off screen
    (hold to repeat made it easy). The list now scrolls.
  - `library` and `library-all` were the same picture and no book was part
    read, so no bar was ever drawn. The previews now differ and show one.
- **D36:** `verse_page` now reads the kept chapter, but the menu asked for
  the page before opening the chapter, so the verse of the day opened on
  page 1 again. The chapter now opens first; a test covers a verse on a
  later page.
- **D37:** `boot_clock` duplicated `uptime`; it is gone.
- **Doc comments (again):** `language()` was inserted under the doc comment
  of `translation_count`, and `library_preview_state` under the one of
  `sample_result_record`.
- **Docs:** the smoke test still said alarm edits are lost at reboot.

Verdict per task:

- **D34:** done after the fixes. Open: the EPUB chapter title on a
  chapter's first page (not in the status, though the task asked for it),
  the pace (`9 min left`), and tests for the label with and without a pace.
- **D35:** done after the fixes. Open: the author line, `done`, and an EPUB
  percent.
- **D34 and D35:** the header's battery, the page label and the row
  percents use the Detail size, where Inter Standard draws `%` without its
  slash (KNOWN_ISSUES). The review missed it too; D39 and D40 fix it.
- **D36:** done after the verse fix.
- **D37:** done. The refresh-time merge is not needed: the wheel reports at
  most one event per poll, so nothing queues.
- **D38:** done.

Do differently next time:

- **Look at every preview you add, at every size.** Blank bands, chips
  that overlap and a bar that vanishes on the selected row are visible in
  the PNG.
- **Check a number's source.** `progress_percent` was the index, not the
  place; a page-1 preview reading `34%` shows it.
- **When you move a layout, move what depends on it**: lines per page,
  scroll windows, the landscape case.
- **List every part of the task in the status**, done or open.
- **Doc comments, again.** Read the three lines above every function you
  add.
- **Read KNOWN_ISSUES before drawing.** It holds rules such as percentages
  in Body size.

## Round 6 result (2026-10-08)

Branch `side-tasks-6` (D29 to D33) was reviewed, merged into `main` together
with v0.10.0 and released as v0.10.1 (milestone `bible-reader`). This round
followed the device path better: the Bible place, the Tetris tick and
`AI.TXT` are wired in `main.rs`, and the tests go through `AppState::apply`
and the catalog. No pull request was opened, and `main` was not merged in
first; the merge had no conflicts.

What the main developer changed at merge time:

- **D29:** a stale `.idx` offset that lands inside a character made the
  read fail instead of falling back to the scan.
- **D30:**
  - ▲ turned to the next page; the Reader uses ▼ for that, so the Bible
    does too.
  - The title showed the short name (`Gén 1`); it is now the full name, large
    (`Génesis 1`). Headings are bold capitals as in the mockup, and book rows
    say `50 cap.`, not `50 ch.`.
  - The menu listed `Versículo del día` and `Traducción`, which only closed
    it. They are hidden until they work.
  - The place reached the card only before sleep; leaving the reading view
    saves it too. With two translations, the one of the saved place opens.
- **D31:**
  - The mode list drew over its last frame: after one move the Zen row
    stayed black and its text disappeared. It now redraws from a clear
    canvas, under the `Tetris` title bar that Sudoku's list has.
  - A game started from the list showed `BEST 0`; it now gets its mode's
    saved best.
  - Classic kept falling while the panel was powered down (a minute without
    keys), so a game could end unseen; it now pauses with the panel.
  - The player's digits followed the Reader's font and size; they are now
    Literata at the size closest to the givens, as the task asked.
  - The flaky test: two tests could read the same clock tick and share a
    temp folder. A counter keeps them apart.
- **D32:** the group labels sat on the row lines; each has its own band.
- **Doc comments:** three new items were inserted between an existing doc
  comment and its function, so `apply_photos_boot_short_press`,
  `apply_ai_settings` and `TetrisStart` described the wrong thing.

Verdict per task:

- **D29:** done.
- **D30:** done after the paging, title, menu and save fixes. Open: the verse
  of the day, the translation switch and hyphenation.
- **D31:** done after the list, best and pause fixes.
- **D32:** done after the layout fix.
- **D33:** the record, labels and result screen are done. Open: the queue on
  save, the worker and the requests (main line, Phase 7), the note's
  `Process` menu and the result's ● actions.

Do differently next time:

- **New code goes after the doc comment above it.** Read the three lines
  above every function you add.
- **Redraw from a clear canvas.** A game screen that draws again must call
  `clear_frame()` first; test that a move keeps the command count.
- **Keys match the rest of the device.** ▼ is next in the Reader, so it is
  next everywhere.
- **No dead options.** A row that does nothing reads as a bug; leave it out
  and list it as open.
- **Merge `main` before handing over**, as the round's note asked.

**Follow-up branch** `side-tasks-6-open` (the verse of the day in the reading
menu, ● on a note's result opens its actions) was merged in v0.10.2. At
merge, two new functions were again inserted under the doc comment of
`translation_count`; a verse whose chapter is not on the card opened an
empty view, so the option now needs the chapter; and the verse opens on the
page that holds it, not on the chapter's first page. The owner found that
hold BOOT in the chapter grid stayed on the grid (the picker drew from its
own view, not from the route) and that short BOOT did nothing in the picker
and the grid, though their bars promise `sección ›` and `+10`; both are
fixed.

## Round 5 result (2026-10-08)

Pull request #5 (D24 to D28) was reviewed, merged into `main` and released as
v0.9.7 (milestone `hubs-and-stats`). The screens follow the mockup and the
round has many tests, but two features never ran on the device: the tested
path and the device path were different. The `side-tasks-5` branch is
deleted; round 6 is below.

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

- English everywhere: UI text, code, comments, commits, docs. One exception:
  the Bible app's screens are in Spanish (D30); its code stays English. The
  device UI is terse (bottom bar labels such as `move`, `change`, `hold:
  back`).
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
- **rustfmt and guards:** rustfmt cannot format `matches!(x, P if guard)`
  inside a closure and leaves it on one long line. Use `filter_map` with a
  `match`, or compare with a whole value.

## Round 8 tasks (branch `side-tasks-8`)

Start from `main` at v0.10.3. Five tasks, in this order; D39 and D40 finish
D34 and D35. When they are done, merge `main` into the branch and open a
pull request (`gh pr create`).

**Main line during this round.** The main developer draws the display
digits and the mockup's weather icons (Weather, the sleep screens and
Home), then sends voice notes for transcription and summary (Phase 7).
Leave these files alone:

- `src/main.rs`, except the lines a task names;
- power and sleep: `src/power*.rs`, `src/sleep_*.rs`,
  `src/app/screens/sleep_*.rs`, `src/app/screens/power*.rs`;
- weather and Home: `src/weather*.rs`, `src/app/screens/weather*.rs`,
  `src/app/screens/home.rs`;
- drawing: `src/app/widgets/icons.rs`, `src/app/widgets/big_digits.rs`,
  `src/app/typography/`, `scripts/fonts/`;
- voice notes and AI: `src/voice_note*.rs`, `src/voice_notes.rs`,
  `src/ai_*.rs`, `src/app/screens/voice_notes.rs`, `src/app/screens/ai.rs`.

Read the Round 7 result first: look at every preview you add at Compact,
Standard and Large; check where each number comes from; when a layout
moves, move what depends on it; list every part of a task in its Status,
done or open. Percentages are drawn in Body size (KNOWN_ISSUES). New code
goes after the doc comment above it. The fonts have ASCII, Latin-1 and
`– — ‘ ’ ‚ “ ” „ • … ‹ › € ™ −` only.

### D39: Reader page: chapter titles and minutes left

**Why.** Left from D34. The mockup's page opens a chapter with its title
(`Capítulo VIII`) and the bar says how long the chapter will take
(`Ch. 8 · 12% · 9 min left`). The header's battery and the page label use
the Detail size, so Inter Standard draws a broken `%`.

**Change.**

- **Chapter title.** On the first page of an EPUB chapter: the chapter's
  label (`EpubChapter::label`) in the large style, a rule, then the text.
  The room comes from pagination, not drawing: the first page of a chapter
  holds fewer lines, so no line is lost or drawn under the bar. Bump the
  page cache version so old caches rebuild. TXT books and later pages are
  unchanged.
- **Minutes left** (EPUB only): the pages left in the chapter times the
  seconds per page of the last seven days (`ReadingStats::total`), rounded
  up: `9 min left`, or `< 1 min left`. Leave it out with fewer than 20
  pages recorded in those days, and when the label would not fit beside
  the bar's hints.
- **Percent size.** The page label and the header's clock and battery in
  Body size.
- **Previews:** `reader-page-epub` (a chapter's first page with its title)
  and `reader-page-epub-pace` (a later page with minutes left), from
  `epub::sample_epub` with reading stats set; `reader-page` stays TXT.

**Tests:** the first page of a chapter holds the title and fewer lines, at
every Reader size in both orientations, and the lines still end above the
bar; the label with and without a pace (19 and 20 pages recorded); the
label fits beside the hints at Large; a page turn from Home › Library
through `AppState::apply` crosses into a new chapter and shows its title.

**Status:** done on `side-tasks-8`. Built: the chapter title in the large style with its rule on a chapter first page only, its room kept by pagination (three body lines fewer there; `READER_CACHE_VERSION` bumped so old caches rebuild), and a leading copy of the label dropped from the text so it is not said twice; minutes left on the label (EPUB only: the chapter pages left times the week seconds per page, rounded up, `< 1 min left` under a minute, left out below 20 recorded pages or when it would run into the hints); the page label and the header clock and battery in Body size (the KNOWN_ISSUES percent). Previews `reader-page-epub`, `reader-page-epub-pace` (`sample_epub` gained a `dc:title`; both looked at Compact, Standard and Large) and `reader-page` stays TXT. Tests: the title band and fewer lines at every Reader size in both orientations with the lines above the bar; the label at 19 and 20 pages; the fit beside the hints at Large; a crossing into chapter 2 through `AppState::apply` from Home; and a pixel check that the rule draws on a chapter first page only. Open: nothing known.

### D40: Library: authors, finished books and EPUB percent

**Why.** Left from D35. The mockup's rows show the author under the title,
a check for a finished book, and a percent for EPUBs; today an EPUB row
shows `Ch. 8`, because the saved place has no percent. Row percents use
the Detail size (broken `%`).

**Change.**

- **Author.** The Library scan already opens each EPUB's OPF for its title
  (`read_epub_title_on_worker`); read the first `dc:creator` in the same
  pass. `ReaderBook` gets an `author`, empty for TXT. RECENT and BOOKMARKS
  rows take it from the scanned book with the same path.
- **Rows** as in the mockup: the title, the author under it, and on the
  right the format chip over the percent, a drawn check (no `✓` glyph) or
  `new`; the bar under the author for a book in progress. Fit seven rows,
  or fewer at Large, and keep the scrolling.
- **Finished.** Reaching the last page of a book (index complete) calls
  `ReadingStats::mark_finished`; `main.rs` already saves the stats. A
  finished book shows the check, also after it is opened again.
- **EPUB percent.** The saved place keeps the percent of the book
  (`ReaderSession::place_percent`) in STATE.TXT and RECENT.TXT, as an
  optional field: old files still read (7, 10 or 11 fields). EPUB rows draw
  the bar from it, and `Ch. N` only for an old place without one.
- Row percents in Body size.
- **Previews:** `library` (an EPUB in progress with its author, a finished
  book, a new book), `library-all`, `library-large-font`.

**Tests:** the author from a sample OPF (`epub::sample_epub` with a
`dc:creator`); `done` after the last page through `AppState::apply`; the
percent round trip in STATE.TXT and RECENT.TXT, and an old line without
it; every row fits at Large.

**Status:** done on `side-tasks-8`. Built: the scan reads the first `dc:creator` in the same OPF pass (`read_epub_meta_on_worker`) into `ReaderBook.author`, empty for TXT, and RECENT and BOOKMARKS rows take it from the scanned book with the same path; rows show the title, the author under it, the format chip over a state slot (a drawn check for a finished book, the percent in Body size, `Ch. N` for an old EPUB place or `new`), and a bar under the author for a book in progress, with BOOKMARKS rows showing their place in the slot; reaching a book last page (index complete; an EPUB last chapter) marks it finished and the check shows from the stats after reopen; the saved place stores `place_percent` in STATE.TXT and RECENT.TXT (both the field and key=value records round-trip it) and 7 and 10 field lines still read with no percent. Previews `library` (an EPUB in progress with its author, a finished book, a new book), `library-all` (the finished row under the cursor), `library-large-font` and `library-files`, checked at Standard and Large. Tests: the scan author from `sample_epub`; the percent round trip and an old line; `done` after the last page through `AppState::apply`; the row author lookup; every row fits at Large. Open: `library` and `library-all` are one list differing by selection; bookmark rows moved from the compact list to the book rows so they show the author (seven rows now, scrolling kept); a book resumed at its end is not re-marked until a turn.

### D41: Settings › Reading

**Why.** Mockup "Settings": a `Reading` group (`Hyphenation, margins,
stats`, value `Auto ES/EN`) between Display and Sleep screen. Reader
preferences open only from inside a book today, and margins and
hyphenation cannot be changed at all.

**Change.**

- **Settings row** `Reading`, subtitle `Hyphenation, margins, stats`,
  value `Auto ES/EN` or `Hyphenation off`. Nine rows must still fit.
- **Screen** `SettingsReading` (parent Settings), an option list like
  Settings › Display: Book font, Size, Alignment, Margins (`Narrow`,
  `Normal`, `Wide`), Hyphenation (`Auto ES/EN`, `Off`), Show progress,
  Reading stats (`On`, `Off`).
- **Margins and hyphenation** are new `ReaderPreferences` fields saved in
  `PREFS.TXT`; a file without them reads as Normal and Auto. Margins set
  the text inset (16, 24 or 40 px) for `line_width` and the page box;
  lines per page stay as they are. Hyphenation Off wraps whole words. Both
  are part of the layout: a change repaginates the open book the next
  time it shows, the way `finish_preferences_edit` does.
- **Reading stats Off** records no time and no pages and writes nothing.
- Reader Options › Reading Preferences lists the new options too.
- **Previews:** `settings` (nine rows), `settings-reading`,
  `reader-page-wide-margins`.

**Tests:** Home › Settings › Reading through `AppState::apply`; the PREFS
round trip with and without the new fields; margins change `line_width`
and the cache fingerprint; Off leaves no hyphen at a line's end; stats Off
records nothing; the row's value.

**Status:** done on `side-tasks-8`. Built: a `Reading` row between Display and Sleep screen (nine rows fit at 44..692 above the bar), subtitle `Hyphenation, margins, stats`, value `Auto ES/EN` or `Hyphenation off`; the `SettingsReading` screen (parent Settings) with Book font, Size, Alignment, Margins (`Narrow`/`Normal`/`Wide`), Hyphenation (`Auto ES/EN`/`Off`), Show progress and Reading stats, each opening an option list at the value in use; `PageMargins`, `HyphenationMode` and `record_stats` in `PREFS.TXT` (old files read as Normal and Auto and On) whose margins set the text inset (16, 24 or 40 px) for `line_width` and the page box and both margins and hyphenation join the cache fingerprint; Hyphenation Off wraps whole words; stats Off records no time, no pages and no finished book. A change goes through the in-book preference path, so it persists and stages the rebuild for the next book open; choosing the value in use does nothing. Reader Options › Reading Preferences lists the new options too. Previews `settings` (nine rows), `settings-reading`, `reader-page-wide-margins` (text ink at 41 px vs 24), all checked at Standard and Large. Tests: Settings › Reading through `AppState::apply` with two edits and their values; the PREFS round trip with and without the new fields; margins in `line_width` and the fingerprint; Hyphenation Off leaves no hyphen at a line end; stats Off records nothing; nine rows fit above the bar. Open: nothing known.

### D42: Add and delete alarms on the device

**Why.** Since D38 edits are saved, but the list holds only the alarms in
`ALARMS.TXT`: a new alarm or a deletion still needs a computer. Without
the file, nothing can be saved at all.

**Change.**

- The Alarms list ends with a `New alarm` row while fewer than
  `MAX_ALARMS` exist. It opens the editor on a new alarm (07:00, on,
  weekdays, named `Alarm N`); Save adds it and writes the file.
- The editor's last field chooses with ▲▼ between `Save alarm` and
  `Delete alarm`; ● runs the choice. Delete removes the alarm, writes the
  file and recomputes the next alarm for the RTC; on a new alarm it only
  discards it.
- A missing `ALARMS.TXT` is an empty list that can be saved; a file that
  does not parse still blocks saving. `main.rs` loads through a new
  `AlarmEngine::load_or_empty` (only that call).
- **Previews:** `alarms-new`, `alarms-delete`.

**Tests:** add up to six and no more; delete and save; a missing file and
a new alarm write the file; a broken file is never overwritten; the next
alarm after a delete.

**Status:**

### D43: Bible: no card reads while drawing

**Why.** Each draw of the reading menu lists the translation folders
(`translation_count`) and reads `VERSES.TXT` (`verse_list`), and so does
each key press in it.

**Change.**

- `BibleUiState` keeps the translation list and the verse list, read when
  the state is built; `bible_menu_items` and the picker use the kept ones.
- The Traducción list leaves out a translation whose `index.tsv` does not
  read, so choosing one never lands on the old translation's book picker.

**Tests:** count card reads as D36's `loads()` does: drawing the menu and
pressing keys in it read nothing; a translation with a broken index is not
listed.

**Status:**

## Round 7 tasks (done in v0.10.3)

Start from `main` at v0.10.2. Five tasks, in this order; D35 and D36 reuse
D34's reading header. When they are done, merge `main` into the branch and
open a pull request (`gh pr create`).

**Main line during this round.** The main developer draws the display
digits and the mockup's weather icons (Weather, the sleep screens and
Home), then sends voice notes for transcription and summary (Phase 7).
Leave these files alone:

- `src/main.rs`, except the lines a task names;
- power and sleep: `src/power*.rs`, `src/sleep_*.rs`,
  `src/app/screens/sleep_*.rs`, `src/app/screens/power*.rs`;
- weather: `src/weather*.rs`, `src/app/screens/weather*.rs`;
- drawing: `src/app/widgets/icons.rs`, `src/app/widgets/big_digits.rs`,
  `src/app/typography/`, `scripts/fonts/`;
- voice notes and AI: `src/voice_note*.rs`, `src/voice_notes.rs`,
  `src/ai_*.rs`, `src/app/screens/voice_notes.rs`, `src/app/screens/ai.rs`.

Read the Round 6 result first: new code goes after the doc comment above
it, a screen that draws again starts from a clear canvas, ▼ is "next"
everywhere, and a menu row that does nothing stays out. The fonts have
ASCII, Latin-1 and `– — ‘ ’ ‚ “ ” „ • … ‹ › € ™ −` only.

### D34: Reader page like the mockup

**Why.** Mockup "Lector: libro en español". The most used screen still has
rustmix's layout: a black header with the format name, a status box
(`UTF-8 PAGE 1+ CACHE 34%`) and a text footer (`UP previous DOWN next
SELECT options`) instead of the shared bottom bar.

**Change.**

- **Header.** A reading header as the mockup's: the book title in capitals
  on the left, the time and battery on the right, a rule under it, no black
  bar. Put it in `src/app/widgets/reading_header.rs`; the Bible reading view
  uses it too, with `BIBLIA · RVR1960` on the left.
- **Chapter title.** On the first page of an EPUB chapter, the chapter's
  title from the table of contents, large, then a rule. Other pages and TXT
  books start with the text.
- **Bottom bar.** `draw_bottom_bar` with `▲▼ page` and `● menu`, and on the
  right `Ch. 8 · 12% · 9 min left` for an EPUB, `p. 41 · 12%` for a TXT.
  The percent is the place in the whole book. Minutes left count the pages
  left in the chapter at the reader's own pace from `reading_stats` (seconds
  per page over the last seven days); without 20 recorded pages, leave them
  out. With `show_progress` off the right side stays empty.
- **Bookmark.** A bookmarked page shows a small filled corner at the top
  right of the text (no `★` glyph).
- The text takes the room of the old status box. Portrait and landscape.
- **Previews:** `reader-page` (EPUB with a chapter title), `reader-page-txt`,
  `reader-page-landscape`, `bible-reading` with the new header.

**Tests:** the geometry at every Reader size and UI size in both
orientations (the text never reaches the header or the bar); the right-side
label for EPUB, TXT, with and without a pace; the corner on a bookmarked
page; a page turn through `AppState::apply` from Home › Library.

**Status:** done on `side-tasks-7` (commit `0e39b32`). The Reader page has the shared reading header (`src/app/widgets/reading_header.rs`: title in capitals, clock and battery, a rule) in place of the black bar and the status box. The footer is the shared bottom bar with `▲▼ page` and `● menu`, and the page label on the right: chapter for EPUB, page for TXT, with the percent. A bookmarked page shows a filled corner. The Bible reading view uses the same header with `BIBLIA · RVR1960`. Tests: the body stays between the rule and the bar in both orientations; a TXT page from Home turns with ▼. Open: pace minutes (`9 min left`) until the reading stats have 20 pages; the landscape preview needs the panel, since the host cannot repaginate a book.

### D35: Library like the mockup

**Why.** Mockup "Library". Home › Library opens a list of three rows
(Continue Reading, Books, Bookmarks); the mockup shows the books.

**Change.**

- Status bar `Library` with `24 books` on the right, then tab chips
  `RECENT ALL BOOKMARKS FILES`. Short BOOT moves to the next tab; the
  `Change tab` row goes.
- **Book rows** (RECENT and ALL): the title in the Reader's book face, the
  author under it (EPUB `dc:creator`; nothing for TXT), then the format chip
  (`EPUB`, `TXT`), a progress bar and the percent, `done` for a finished
  book (draw a check mark: there is no `✓` glyph) or `new` for a book never
  opened. ● opens the book at its place.
- BOOKMARKS lists the saved bookmarks as today; FILES lists the files of
  `/RUSTMIX/BOOKS` with their sizes.
- Bottom bar `▲▼ book ● read BOOT tab`. Home's Continue card stays.
- **Previews:** `library`, `library-all`, `library-files`,
  `library-large-font`.

**Tests:** from Home through `AppState::apply`: short BOOT cycles the tabs,
● on a recent book opens it at its place, `new` and `done`, the author from
a sample OPF, every row fits at Large size.

**Status:** built on `side-tasks-7`. Tabs are `RECENT ALL BOOKMARKS FILES` in that order, and short BOOT moves through them and wraps. The Library status shows `Library` with the book count (`1 book` or `N books`). Book rows show the title, the format chip, a bar with the percent of the saved place, or `new` for a book never opened. The `Change tab` row is gone. Select on a recent book opens it at its saved place. Open: the author line (the EPUB reader reads `dc:title` only; `dc:creator` needs a parser change) and `done` (nothing marks a book finished on the device yet; `reading_stats::mark_finished` has no caller). Device check: the four previews and a real book folder with a saved place.

### D36: Bible translations, hyphenation and one load per chapter

**Why.** Left from D30: the menu has no `Traducción`, verses wrap without
hyphenation, and every page turn reads the chapter from the card three
times (`chapter_pages` twice, then the drawing).

**Change.**

- **Traducción** in the reading menu when the card has two or more
  translations: an option list of their `meta.txt` titles with the one in
  use marked. Choosing another loads its books, keeps the place when that
  book is there, and otherwise opens the picker.
- Verses wrap and hyphenate by the translation's `language` (`es`, `en`)
  the way Reader pages do.
- `BibleUiState` keeps the open chapter's items and loads a chapter once,
  when the place moves to it; drawing and paging use the kept items.
- **Previews:** `bible-translation`, `bible-reading` hyphenated.

**Tests:** the switch through `AppState::apply` with two translations in a
temp folder (place kept, place missing); a hyphenated line; one load per
chapter across page turns (count the loads).

**Status:** built on `side-tasks-7`. Menu › Traducción appears when the card has two or more translations: an option list of their `meta.txt` titles with the one in use marked. Choosing another keeps the place when its book is there, and otherwise opens the book picker. Verses wrap with a hyphen by the translation's language (`es`, `en`), as Reader pages do. The chapter open in the reading view is read once when the place moves to it, and page turns inside it read nothing. Tests: the switch through `AppState::apply` (place kept, place missing); a hyphenated line; one load per chapter. Previews: `bible-translation`, `bible-reading`. Device check: two translations on the card; a hyphenated line on the panel.

### D37: Hold ▲▼ to repeat

**Why.** Backlog. Long lists (150 Psalms, a long Library, the Dictionary)
take one press per row.

**Change.**

- `buttons.rs` reports ▲ or ▼ when it goes down, then repeats it after
  500 ms and every 200 ms while it is held. ● and BOOT do not repeat.
- Repeats count only on lists and grids: `ScreenRoute::repeats_keys()` is
  true for Settings lists and option lists, the Library, the Bible picker
  and grid, the Photos gallery and the Dictionary; false for Reader and
  Bible pages, games, the photo viewer and every screen not listed.
- **main.rs** (only the wheel polling): a repeat that arrives while a
  refresh runs replaces the one waiting, so one refresh shows the latest
  cursor and repeats never queue up.

**Tests:** the timing with a fake clock (nothing before 500 ms, then every
200 ms, nothing after the release); the route list; merging during a
refresh.

**Status:** built on `side-tasks-7`. Holding ▲ or ▼ repeats after 500 ms and every 200 ms, on the screens `ScreenRoute::repeats_keys()` lists (settings, option lists, the Library, the Bible picker and grid, the Dictionary, and others). Reading pages, games and the photo viewer take one press per key. `main.rs` changed on the wheel poll only: it passes the boot clock and the route's flag. Tests: the timing with a fake clock (nothing before 500 ms, then every 200 ms, nothing after release, one repeat after a long pause); the route list. Open: the refresh-time merge (a repeat during a refresh replacing the one waiting) is not built; the wheel poll reports at most one event per call, so the queue stays short without it. Device check: hold ▼ in Settings and in the Bible picker.

### D38: Alarm edits survive a reboot

**Why.** Known issue: the Alarms editor changes only the running engine;
a reboot reloads `/RUSTMIX/ALARMS.TXT`.

**Change.**

- Saving an alarm in the editor writes `ALARMS.TXT` through `sd_file`, in
  the format the loader reads, once per saved edit (a changed flag that
  `main.rs` takes and saves next to the `AI.TXT` save; only those lines).
  The RTC alarm is programmed as today.
- Comments in a hand-written file are not kept; the User Guide says so.
- Remove the known issue; update the User Guide and the smoke test.

**Tests:** a round trip of every schedule kind; an edit through
`AppState::apply` marks one save and a cancel marks none; the folder is
created when missing.

**Status:** built on `side-tasks-7`. A saved editor edit writes `ALARMS.TXT` through `sd_file`, once per save, in the format the loader reads, and the folder is created when missing. A config that was never read is not overwritten. `main.rs` saves on the same outcome path as the RTC alarm. The known issue is removed, and the User Guide and the smoke test say the edit survives a reboot. Comments in a hand-written file are not kept, as the User Guide says. Tests: a round trip of every schedule kind (recurring, named sets, once, a weekday list); one save per saved edit and none before it; the folder creation; the never-read guard. Device check: change an alarm, Save, reboot, and read the time back.

## Round 6 tasks (done in v0.10.1)

Start from `main` at v0.9.7. Five tasks, in this order; D30 uses D29, and
D33 uses D32's `AI.TXT`.

**v0.10.0 on main.** Phase 3b and a Sudoku autosave landed during this
round. Merge `main` into `side-tasks-6` before opening the pull request: it
changed `src/lua_runtime/mod.rs` (Sudoku saves go through `store_sudoku`),
`src/app/state.rs`, `src/app/screens/settings.rs` and `src/main.rs`.

**Main line during this round.** The main developer builds Phase 3b (clock
and weather sleep screens, refreshed while the device sleeps) and redraws
Weather and the sleep screens like the mockup. Leave these files alone:

- `src/main.rs`, except the lines a task names;
- power and sleep: `src/power*.rs`, `src/sleep_*.rs`,
  `src/app/screens/sleep_*.rs`, `src/app/screens/power*.rs`;
- weather: `src/weather*.rs`, `src/app/screens/weather*.rs`;
- drawing: `src/app/widgets/icons.rs`, `src/app/widgets/big_digits.rs`,
  `src/app/typography/`, `scripts/fonts/`;
- photos: `src/photos/`, `src/app/screens/photos.rs`.

The fonts have ASCII, Latin-1 and `– — ‘ ’ ‚ “ ” „ • … ‹ › € ™ −` only: no
`→`, `✓`, `☐`, `★` or `⌫`. Write `›`, draw the shape, or leave it out.

Every feature needs at least one host test through the path the device
uses (`AppState::apply` from Home, the catalog and `open_selected`), not
only through its new type; see the Round 5 result.

### D29: Bible text from the SD card

**Why.** Phase 5. `bible.rs` (D10) reads a `BOOKS.TXT` layout that no card
has. The owner's Bible is already converted by the reference project's
`referencias/folloup-waveshare/scripts/bible_json_to_sd.py`; its layout has
the section headings the mockup shows ("Jehová es mi pastor") and a chapter
index that makes reads cheap. Decision: Wave reads that layout, and the
`BOOKS.TXT` reader goes.

**Change.**

- **Files.** `/RUSTMIX/BIBLE/<ABBR>/` holds what the script writes for one
  translation; the owner copies the script's `bible/<ABBR>/` folder there.
  Read the script's docstring for the exact rules; do not change it.
  - `meta.txt`: `key=value` lines (`format`, `abbreviation`, `title`,
    `language`, `copyright`);
  - `index.tsv`: one book per line, `usfm<TAB>name<TAB>chapters<TAB>file`;
  - `<USFM>.txt`: one record per line: `C<TAB>chapter`, `H<TAB>heading` or
    `V<TAB>label<TAB>paragraph<TAB>text`, with labels like `3` or `3-4` and
    paragraph `1` when the verse starts one;
  - `<USFM>.idx`: `chapter<TAB>byte offset of its C line<TAB>verse count`.
- **Books.** Map USFM codes to 1 to 66 in canonical order (GEN to REV), so
  the `bible_nav.rs` sections keep working; skip other codes. The short name
  for labels comes from the name: the first three letters of the first word,
  keeping a leading number (`Sal`, `Gén`, `1 Cor`).
- **Chapters.** `load_chapter` seeks to the `.idx` offset and reads records
  until the next `C` line, returning headings and verses in order (for
  example `enum ChapterItem { Heading(String), Verse { label, paragraph,
  text } }`). Without a `.idx`, or when its offset does not land on
  `C<TAB>chapter`, it scans the file instead. Only that chapter stays in RAM.
- **Translations.** `translations(root)` lists the folders that hold an
  `index.tsv`, with the title and language from `meta.txt`.
- **Verse of the day.** `VERSES.TXT` lines accept a USFM code (`PSA 23:1-3`)
  besides the book number, and a helper returns the text of one reference
  for the sleep screen's verse mode (main line).
- Update `bible_nav.rs`'s sample books and the D20 previews to the new types.
- **Docs.** `SD_CARD_SETUP.md` gains a Bible section: run the script on a
  computer, copy the folder, what the device reads. The Bible text never goes
  into this repo; fixtures use a few public-domain verses (Reina-Valera 1909
  or the KJV).

**Tests:** a fixture written to a temp folder with two books: headings, a
`3-4` label, paragraph flags and CRLF lines; reads with and without `.idx`
and with a stale offset; a malformed record names its file and line; USFM
and numbered references in `VERSES.TXT`; `translations` skips folders
without `index.tsv`.

**Status:** done on `side-tasks-6`, data layer and docs. `bible.rs` now reads
the script's layout: `index.tsv` for books (USFM codes mapped to 1 to 66,
unknown codes skipped), `.idx` seeks with a scan fallback, `meta.txt`,
`translations` on `index.tsv`, and `VERSES.TXT` with USFM codes; the old
`BOOKS.TXT` reader is gone. `reference_text` serves the sleep screen's verse
mode. Deviation: a bad record names its file and byte offset, not its line,
because an index seek cannot count lines. SD_CARD_SETUP has a Bible
section. Open: the fixture with a few public-domain verses is not in the repo
yet; the parser tests use inline text. Device check after D30: a book's last
chapter and a heading read from the owner's card.

### D30: Bible reading view, in Spanish

**Why.** Phase 5, mockups "Bible: lectura" and "Bible: elegir libro". Home ›
Bible says SOON, and the book and chapter pickers (D20) are drawn only in
previews, in English. The owner wants this app in Spanish.

**Change.**

- **Language.** Every screen from Home › Bible on is in Spanish, bottom bars
  included; the mockup has the texts. Home's row stays `Bible`. Code,
  comments and tests stay English.
- **Book picker** (D20's, translated and easier to read):
  - status bar `Ir a · Libro` with the translation on the right, then
    `Antiguo Testamento · sección 3 de 8` (or `Nuevo Testamento`) and the
    section name large;
  - sections: `Pentateuco`, `Libros históricos`, `Poesía y sabiduría`,
    `Profetas mayores`, `Profetas menores`, `Evangelios y Hechos`, `Cartas
    de Pablo`, `Cartas generales y Apocalipsis`;
  - the tabs become a thumb index, like the edge of a printed Bible: each
    shows its section's first book as a D29 short name in capitals (`GÉN JOS
    JOB ISA OSE MAT ROM HEB`), so no English code (`HIS`, `GOS`) remains;
  - rows `Salmos` with `150 cap.`, the line `Siguiente (BOOT): Profetas
    mayores · Isaías, Jeremías…`, and the bottom bar `▲▼ libro ● capítulos
    BOOT sección ›`;
  - it opens on the book being read, in its section.
- **Chapter grid:** `Ir a · Capítulo`, `Capítulos 1–60 de 150`, bottom bar
  `▲▼ capítulo ● leer BOOT +10`; it opens on the chapter being read.
- **Routes.** Home › Bible opens the reading view at the last place; on
  first use, or when that place is gone, the book picker opens. ● on a
  chapter opens it. Hold BOOT walks back: chapters, books, then the reading
  view or Home.
- **Reading view** as the mockup:
  - a header `BIBLIA · RVR1960` with the time and battery, like Reader pages;
  - the chapter title (`Salmos 23`) large, then a rule;
  - headings in bold; each verse starts a line with its number small and
    raised; verse text in the Reader's book font and size
    (`reader_body_style` with the Reader preferences), wrapped and
    hyphenated by language like Reader pages;
  - the bottom bar `▲▼ página ● menú BOOT capítulos`, and `Sal 23 · 1/2`
    above it on the right.
- **Paging.** ▲▼ turn pages. Past the last page the next chapter opens (the
  next book after the last chapter); before the first page, the previous
  chapter's last page. Pages are laid out per chapter. Short BOOT jumps to
  the chapter grid of the book being read.
- **Menu.** ● opens an option list (`widgets/option_list.rs`): `Ir a libro`,
  `Ir a capítulo`, `Versículo del día` when `VERSES.TXT` exists (today's
  verse through D29's helper), and `Traducción` when the card has more than
  one.
- **Place.** `/RUSTMIX/BIBLE/STATE.TXT` (`translation`, `book` as USFM,
  `chapter`, `page`) through `sd_file`, saved when the view closes and before
  sleep, never per page. The Home row shows `Sal 23` the way Reading Stats
  shows its streak, and `Bible` leaves the placeholder set.
- **No Bible on the card.** The view says, in Spanish, how to add one (a
  line pointing to SD_CARD_SETUP), with a `BOOT mantener: volver` bar.
- The Bible state takes its root from a constructor, so tests use a temp
  folder (as `PhotosUiState::with_roots` does).
- **main.rs:** one line before sleep, next to the reading-stats save, to
  save the place.

**Tests:** through `AppState::apply` from Home: first use opens the picker,
choosing a book and chapter opens the view, paging crosses into the next
and the previous chapter, short BOOT opens the chapter grid on the current
chapter, the place survives a reload, the Home label; the thumb index for
the sample books; the layout fits at every Reader and UI size.
**Previews:** `bible-reading` (Psalm 23 from the fixture),
`bible-reading-large`, `bible-books`, `bible-chapters`, `bible-menu`,
`bible-missing`.

**Status:** done on `side-tasks-6`, except two menu options. Built: the
reading view (header, title, verses in the Reader's body style, wrapped to
the width, the page indicator right-aligned), the Spanish thumb index from
each section's first book, Home routing (saved place, else picker, else the
missing view), paging across chapters, hold-BOOT walk-back, short BOOT to the
chapter grid, the reading menu, the place saved on close and before sleep,
and six previews. `Versículo del día` and `Traducción` are listed in the
menu but only close it for now. Deviation: verse text wraps by words, not by
Reader hyphenation. Checked: host suite green with no warnings; firmware
green on the branch head (see the pull request). Device checks: the smoke-test Bible steps, and that a verse's
first line starts after its number on the panel.

### D31: Tetris Classic

**Why.** Phase 6, mockup "Tetris" and the Games card: Zen has no gravity,
Classic has slow gravity. `TetrisGame::step()` exists, but nothing calls it.

**Change.**

- **Start.** Tetris opens a mode list like Sudoku's start list: `Zen · best
  18,950` and `Classic · best 4,200` (`no best yet` before the first).
  `APPS/TETRIS/MAIN.LUA` becomes `tetris.init()`; an old card's
  `tetris.init('zen', 1803)` still loads and highlights Zen. The press time
  seeds every new game.
- **Gravity.** Classic moves the piece down one row every `GRAVITY_MS`
  (1 s) and locks it when it cannot move. A tick carries the clock through
  the same layers as a key: `TetrisApp`, `LuaEventBridge`,
  `LuaRuntimeUiState::tick_game(now_ms) -> bool` (true when the screen
  changed) with `next_game_tick_ms()`, and `AppState::tick_lua_game`. Zen,
  a finished game and the other games never tick; after a long pause one
  tick steps once.
- **View.** Title bar `Tetris · Classic`; the Mode block reads `Classic`
  and `Slow gravity: one row a second.`
- **Records.** `tetris_classic` in `RECORDS.TXT`, written once on leaving,
  like Zen's. The Games card: `Zen · best 18,950`, then `Classic · best
  4,200` (or `Classic: slow gravity` without a best).
- **main.rs** (only these lines):
  - next to the Reader tick, when `next_game_tick_ms()` is due, call
    `state.tick_lua_game(uptime_ms)` and refresh as after a key;
  - add the time to that tick to `light_sleep_budget(&[…])`.
  A tick is not a key press: leave `last_activity` alone.
- **Sudoku digits.** The player's digits draw in the smaller Body size; the
  mockup draws them as large as the givens, in a lighter face. Add a canvas
  style that `lua_game.rs` maps to Literata (`reader_body_style`) at the
  size closest to the givens.

**Tests:** ticks at chosen times (none before 1 s, one per second, none in
Zen or after game over), a lock and a line clear by gravity, best per mode,
the mode list, and a tick through `LuaRuntimeUiState` after
`open_selected`. **Previews:** `tetris-start`, `tetris-classic`, and
`sudoku-row` with the new digits.

**Status:** done on `side-tasks-6`. Built: the mode list on `tetris.init()`
with the press time as the seed; Classic gravity on a one-second clock
(tick, sleep budget and refresh in `main.rs`); a separate `tetris_classic`
best saved on leaving; the Games card's two lines; the title and Mode block
read the mode; the `tetris-start` and `tetris-classic` previews; the Sudoku
player digits in a lighter Reader face (`sudoku-row`). The mode list reports
its start as `start-choose`, so the device log shows it. Checked: host suite
683 passed, no warnings; local Xtensa build clean on the branch head. Device
check: Classic falls with no key pressed, the piece stays put in Zen, a long
pause steps once, and the card shows both bests after a reboot. The player
digits are the Reader face at the Reader size, close to the givens; confirm
the size on the panel.

### D32: Settings › AI

**Why.** Phase 7, mockup "Settings › AI". Transcription and summary
providers need a home before the main line sends any request.

**Change.**

- **Settings.** An `AI` row between Weather and Wi-Fi & transfer:
  `XiaoZhi, transcription, summary`, value `Ready` when both providers have
  a URL and a model, otherwise `Not set up`.
- **Settings › AI** as the mockup: the groups `Transcription ·
  /audio/transcriptions` (Provider, Model, Language), `Summary ·
  /chat/completions` (Provider, Model, Style) and `General` (Process, API
  keys), its info box, and the bottom bar `▲▼ move ● change BOOT hold:
  back`.
- **File.** `/RUSTMIX/AI.TXT`, `key=value` through `sd_file`:
  `transcription_url`, `transcription_model`, `language` (`auto`, `es`,
  `en`), `summary_url`, `summary_model`, `style` (`bullets-todos`,
  `bullets`, `paragraph`) and `process` (`online`, `manual`). Unknown keys
  are ignored; no file means not set up. Add `AI.TXT.example` (Groq for
  transcription, OpenRouter for summaries) with its installer and
  SD_CARD_SETUP lines.
- **Values.** The provider comes from the URL host: `api.groq.com` is Groq,
  `openrouter.ai` OpenRouter, `api.openai.com` OpenAI, a local address
  Local, anything else the host itself. ● on Language, Style or Process
  opens an option list (as in Settings › Weather) and saves; URLs and models
  are edited in the file. API keys read `Not set` from a state flag that the
  main line will set: keys never come from the SD card.
- The AI hub's Voice Notes card shows the two providers on its right, as
  in the mockup (`GROQ` over `OPENROUTER`), once both are set.

**Tests:** file round trip and defaults, provider names, option lists that
mark the value in use, the Settings row value, Settings › AI reached from
Home and back. **Previews:** `settings` with eight rows, `settings-ai`,
`settings-ai-picker`.

**Status:** done on `side-tasks-6`, except what is listed as open below. Built:
`AI.TXT` (`ai_config.rs`: parse, defaults, readiness, provider names, save
with the folder created first), loaded at boot and saved once per change
from `main.rs`; the Settings row (`Ready` or `Not set up`); the Settings ›
AI screen with its three groups and the Language, Style and Process lists
(the value in use changes nothing); the Voice Notes card's providers
(`GROQ` over `OPENROUTER`) once both are set; `AI.TXT.example` with its
installer line and SD_CARD_SETUP section. Deviations: the eight rows at 72 px
leave the group labels close to the rows above them, tighter than the
mockup; the Process value reads `Online` or `Manual`, not the mockup's
`When online`; the API keys row reads `Not set` (keys come in the browser,
Phase 7). Checked: host suite 699 passed, no warnings; the device build is
clean on the branch. Open: one host test,
`tetris_sessions_seed_the_saved_best_and_close_marks_one_save`, failed once in
about 45 full runs and never alone (60 of 60 passed); the cause is not found,
so it should be fixed or re-run before the pull request. Device checks: the
Settings › AI smoke test, and `AI.TXT` written on a real card.

### D33: Voice Notes results

**Why.** Phase 7, mockups "AI" and "Voice Notes: resultado". Each note
shows whether it is queued, being transcribed or summarized, and a
summarized note opens on its summary. The requests themselves
(`ai_client.rs`, D22) go out with the main line's network work; this task
builds everything around them.

**Change.**

- **Record.** `VOICE###.AI` next to the WAV, through `sd_file`: `state`
  (`queued`, `transcribing`, `summarizing`, `done`, `failed`), `error`,
  `title` (UTF-8, from the summary), `language`, `transcribed_by`,
  `summarized_by`, then a summary block and a transcript block. No file
  means not processed; deleting a note deletes its record.
- **Queue.** A saved recording is queued when `AI.TXT` (D32) has
  `process=online`; with `manual`, the note's menu gains `Process`.
  `VoiceNotesUiState` gains the steps the network code will call:
  `next_job()` (the oldest queued note), `begin_transcription`,
  `finish_transcription(text, language)`, `finish_summary(content)` (title
  through `ai_client::split_title`) and `fail(error)`.
- **Labels.** AI hub rows and the Voice Notes list show the state as in the
  mockup: `SUMMARY`, `TRANSCRIBING…`, `SUMMARIZING…`, `QUEUED · OFFLINE`
  (queued while Wi-Fi is off), `QUEUED` or `FAILED`. A summarized note shows
  its summary title. The hub's info box becomes the mockup's text once
  `AI.TXT` exists.
- **Result screen** as the mockup: the title, a meta line (`Oct 2 · 12:04 ·
  Spanish · Groq whisper-large-v3-turbo › OpenRouter llama-3.3-70b`), tabs
  `SUMMARY`, `TRANSCRIPT` and `AUDIO` (short BOOT switches), ▲▼ scroll, ●
  the note's existing actions (play, rename, export, delete), and the line
  `AI-generated · may contain mistakes · audio kept on SD`. A note without
  a record opens today's details screen.
- Text the fonts cannot draw (`☐`, `✓`, emoji) is replaced before layout
  (`☐` becomes `•`).

**Tests:** record round trip (blank lines, a block marker inside the text),
the steps and the queue order, labels per state, delete removes the record,
opening a summarized note from the AI hub through `AppState::apply`.
**Previews:** `voice-note-summary`, `voice-note-transcript`, and `ai` with
the three states.

**Status:** done on `side-tasks-6` for the device-side screens and records, except the open items below. Built: the record (`voice_note_record.rs`, `VOICE###.AI` beside the WAV, round trip, block markers, states, delete removes it); the queue steps (`next_job`, `advance`, `queues_on_save` for `process=online`); the state labels (`state_label`: `SUMMARY`, `TRANSCRIBING…`, `SUMMARIZING…`, `QUEUED`, `QUEUED · OFFLINE`, `FAILED`) on the AI hub chips and the Voice Notes list; the hub info box (mockup text once `AI.TXT` is ready); and the result screen (summary title, meta line, SUMMARY / TRANSCRIPT / AUDIO tabs, scroll, the AI-generated footer; a note without a record opens its details). Text is prepared for the fonts (`☐` becomes `•`, `→` becomes `>`). Previews: `voice-note-summary`, `voice-note-transcript`, `voice-note-audio`, `ai-states`, `ai-states-offline`, `voice-notes-states`.

Open, not built: (1) the device wiring: queued save on recording, the worker that calls `next_job` and the steps, and the network requests from `ai_client.rs` (the main line's network work); (2) the manual `Process` menu on a note; (3) the result's `●` actions (play, rename, export, delete): `●` only goes back; (4) the audio tab shows the file facts, not more. Test gap: a summarized note is opened from the list in the tests, not from the AI hub. Device check: put a hand-made `VOICE###.AI` on the card and check the label, the result and the tabs. Host 729 passed, fmt clean, firmware green.

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
