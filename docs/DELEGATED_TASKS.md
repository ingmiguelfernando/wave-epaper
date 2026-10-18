# Delegated tasks

Small, self-contained tasks for a second developer or AI working in parallel
with the main line. Read this file first, then
[architecture.md](architecture.md) and [ROADMAP.md](ROADMAP.md).

## Round 17 result (2026-10-18)

Branch `side-tasks-17` (D72 to D74) came as pull request #16 from v0.12.7,
with CI and the firmware build green. It was reviewed on `side-tasks-17`
itself and merged without a release: the Dice engine, screens and rattle
are libraries and previews until the main line wires them. The engine and
the rattle are what the tasks asked for: seeded, constants in one place,
statistical tests for the dice, byte-exact tests for the sound. The screens
(D73) were not close to the mockup.

What the main developer changed at review time:

- **The Dice screens did not look like the mockup.** The agent listed the
  differences in the Status (tilt by shear, `RoundedRectangle`, a speaker of
  paper shapes) and handed the task over as done. In the previews the dice
  were thin-lined and tiny-pipped, the tilted dice were flat parallelograms,
  the speaker was a bracket, the two strips under the dice held no dice, and
  the options rows overlapped the strip. The mockup's `die-N` symbols draw a
  100-unit square with a 6-unit stroke, 17-unit corners and 9.5-unit pips in
  a 108-unit box; `die_shape.rs` now holds that geometry in integers
  (rotation by a rational sine, no float trigonometry), the `Die` command
  turns by an angle about its centre, the dashed rest outline is a rounded
  dashed square, a `Speaker` sprite replaces the bracket, the strips draw
  their dice, and the options list uses the app's 72 px rows. The pip test
  of the first version only checked that a field equalled the value it had
  been given; the new tests probe the geometry.

Verdict: D72 and D74 done; D73 done after the fix. Open: `Rolling…` at the
mockup's 64 px (the canvas has no large letters), the owner's approval of
the mockup, and the wiring (main line).

Do differently next time:

- A preview "looked at enlarged next to the mockup" must be checked for
  every difference you can see, and each one fixed or named as a blocker;
  a difference you list and ship is not looked at. Render the mockup's
  figure and the preview side by side (headless Chrome renders the
  mockup) and compare before writing the Status.
- A test that restates its input (`assert_eq!(face, face)`) proves nothing.
  Probe what the user sees: pixel positions, a point inside a shape, a
  count of what was drawn.
- If the canvas lacks a primitive the mockup needs, add it (the round
  allowed it) instead of approximating with another one.

## Round 16 result (2026-10-18)

Branch `side-tasks-16` (D68 to D70) came as pull request #15 from `main`
after the Dice mockup, with CI, the `fonts` workflow and the firmware
build green. It was reviewed on `side-tasks-16` itself, merged and
released as v0.12.7 (milestone `percent-detail`). D71 (the Dice mockup)
was drawn by the main developer. The generator override is clean and
checked (the committed atlas is byte for byte what the `fonts` workflow
generates), the wording hits are all listed, and the deep sleep study
names the file and line of every claim about this repo, and says what it
could not find.

What the main developer changed at review time:

- **The `%` was two solid 2 × 2 blobs and a slash**, which reads as
  `0/0` at 12 px. The rings are open 3 × 3 now (a paper pixel in the
  middle of each), and the test checks that. The atlas was patched by
  hand to match the TOML; the `fonts` workflow generates the same bytes.

Verdict: D68 to D70 done after the fix. Open for the owner: D70's
question (a timer for the alarm, or a board rewire, study §7) and the
approval of the Dice mockup (D71).

Do differently next time:

- Look at a hand-drawn glyph at the size it is used, among the other
  glyphs, not only its bitmap: a shape that follows the right outline can
  still read as another character.
- When a task's acceptance is "the owner decides", write the options and
  your recommendation first, as D70's section 7 does.

## Round 15 result (2026-10-18)

Branch `side-tasks-15` (D65 to D67) came as pull request #14 from `main`
after Round 14, with CI and the firmware build green. It was reviewed on
`side-tasks-15` itself, merged and released as v0.12.6 (milestone
`books-this-year`). The year count, the streamed split and the Ogg text are
what the tasks asked for: the split is checked byte for byte against
`split_parts` at every edge count the task named, the errors of
`make_part` and of a writer come back, and the Status lines say what was
looked at, including the one thing left open (the result header).

What the main developer changed at review time:

- **A downgrade lost the whole of `STATS.TXT`.** The finish date went in
  the `book` line's last field, where firmware up to v0.12.5 accepts only
  `0` or `1`. An older image (a USB flash back, or the OTA rollback Phase
  8 adds) failed the parse, started with empty stats and overwrote the
  file on its next save: every day of reading history gone. Older
  firmware skips a line kind it does not know, so the day now goes on its
  own `done|<path>|<date>` line after the books, and `book` lines keep
  `0` or `1`. A downgrade keeps everything but the days. The task's text
  asked for the date in the `book` line; the task was wrong, not the
  work.
- **The packet walk had no bound.** `walk_packets` joined continued pages
  into one packet for as long as the file said so; a damaged file could
  fill the memory before the writer refused the packet. A packet longer
  than one page holds is now an error while reading.
- `balanced_shape` returned a part count nobody read; it returns the size.

Verdict: D65 to D67 done after the fixes. Open: the result screen's
header `SAVED WAV DETAILS` (D69).

Do differently next time:

- A file format change has two readers: this firmware and the one before
  it. Read the old parser (`git show <tag>:path`) and check what it does
  with the new file before you change a line it reads. Add a field where
  the old code ignores it, not where it refuses it.
- A loop that grows a buffer from file data needs a limit taken from what
  the format allows, not only from what a good file holds.

## Round 14 result (2026-10-18)

Branch `side-tasks-14` (D62 to D64) came as pull request #13 from v0.12.5,
with CI and the firmware build green. It was reviewed on `side-tasks-14`
itself and merged without a release: nothing the device does changes
until the encoder is wired. The form, the record name and the split are
what the tasks asked for, the split balances its parts as `wav_parts`
does, and the four previews show what their names say.

What the main developer changed at review time:

- **The cached-forecast previews' clock:** `weather_cached_preview_state`
  set the RTC to the local 18:20. The RTC keeps its own basis (UTC+8), so
  Home's status bar said 23:20 while the cache had dropped the hours by
  18:20; on a device the two always agree, because `main.rs` localizes
  first. The preview stores `regional.local_to_rtc(now)` now.
- **`split_parts` panicked** through `expect` on a packet too big for one
  Ogg page. `read_stream` joins pages, so a damaged file can hold one; on
  the device that is a reboot. It returns `io::Result` now, with a test.
- **The Ogg form's test** compared `transcription_request` with
  `transcription_form_for`, one function with itself. It checks the
  exact bytes now, as the WAV test does.

Verdict: D62 to D64 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- Build test and preview state through the same conversions the device
  uses: a clock goes in as the RTC stores it, not as the screen shows it.
  Look at every number on a preview, the status bar's time included.
- No `expect` or `unwrap` on a path the device runs with data from a file
  or the network. "Cannot fail" is true for the writer, not for the
  packet; return the error.
- "Against the exact bytes" means the expected bytes written out in the
  test, not one function compared with another that shares its code.

## Round 13 result (2026-10-17)

Branch `side-tasks-13` (D58 to D61) came as pull request #12 from v0.12.3,
with `main` merged in, CI and the firmware build green. It was reviewed
on `side-tasks-13` itself, merged and released as v0.12.5 (milestone
`forecast-kept`). The cache file, the Opus pages and the stats wiring are
what the tasks asked for: the formats follow the spec byte for byte, the
`main.rs` edits stay on the named lines, and the Status lines are honest
about what only the device can show.

What the main developer changed at review time:

- **The verse bar across a page break:** `current_span` started empty on
  every page, so a page that opened on the wrapped lines of a verse begun
  on the page before drew no bar there. It starts from the last numbered
  line before the page now. The page-break test used one-line verses, so
  no verse ever wrapped over the break; the new test wraps them.
- **Book and Bible time:** `reading_on_bible` was set by the next key, so
  minutes accrued in a book but not yet saved (up to five minutes) went to
  the Bible, or the other way round, when the reader switched. Switching
  settles the pending time first.
- **A cached forecast said `update failed`:** the Weather screen's footer
  read `Stale` as a failed update; a forecast adopted at boot has no error,
  so it says `stale` now, as the sleep screens do.
- **The Ogg writer** looped forever on a packet that needs more than 255
  segments (`flush_page` kept none and recursed); `write_packet` refuses it
  now. Dead code in `read_page` and a doc comment for a parameter that does
  not exist are gone.

Verdict: D58 to D61 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- Test an item that begins before the page, window or frame you draw: a
  verse, a row or a word that wraps over the edge. Short items hide it.
- Walk a new state through every screen that matches on it. `Stale` meant
  "the update failed" on one screen and "old" on another; a new way into a
  state (the cache) needs a grep for each `match` on it.
- State shared by two modes (a book and the Bible, two games) needs a
  rule for the switch: settle what is pending before the mode changes.
- A loop that ends only when it makes progress needs a test with input
  that cannot: the biggest packet, the empty list, the full page.
- Read your own diff for leftovers (`let _ = x;`, a buffer built and never
  read, a comment that names a removed parameter) before you hand over.

## Round 12 result (2026-10-16)

Branch `side-tasks-12` (D54 to D57) came as pull request #11 from v0.12.1,
with CI and the firmware build green. It was reviewed on `review-12`,
merged and released as v0.12.2 (milestone `verse-sleep`). The result
screen, the Home details and the verse mode needed little: the summary
parser is typed, its tests probe the frame, and the verse loads once a
day without reading the card while drawing.

What the main developer changed at merge time:

- **The meter after 30 seconds:** the bars were indexed by the second of
  audio, so past the 30th second every 4 KB chunk (about eight a second)
  pushed a new bar and the meter showed the last four seconds. The test
  sent one chunk a second, so it passed; it sends 4 KB chunks now.
- **The meter's place:** it started at x 58, so its last bars ran past the
  rules and off the screen, and until 30 seconds had passed the newest bar
  was at the left. It is centred now, with silence before the first bar.
- **The info box** ran its three lines together (`draw_info_box` wrapped a
  `\n` as a space; it breaks the line now) and drew `≈` as `?`: the fonts
  have no `≈`. The task asked for it; it reads `~132 h` now.
- The chips are centred, as the mockup's. After ● the screen said
  `PAUSED · Voice Notes` with the recording keys; it says `Voice Notes`
  with `hold: back`.
- **Verse screen:** the reference sat at a fixed height, far under a short
  verse. The verse and its reference are one centred block now, under the
  date and a rule as on the clock screen.
- The Home preview's Bible card used the Bible previews' temp folder,
  which they rewrite; it has its own.

Verdict: D54 to D57 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- Feed state the way the device does: a meter fed one chunk a second
  hides a bug that eight chunks a second show.
- Check the edges of a new drawing: the first and last bar, the right
  margin, and every mode the screen can show, not only the one in the
  preview.
- Look up every character you draw in `src/charset.rs`, even when the task
  text has it.

## Round 11 result (2026-10-12)

Branch `side-tasks-11` (D51 to D53) came as pull request #10 from v0.11.3,
with `main` (XiaoZhi, v0.12.0) merged in, CI and the firmware build green.
It was reviewed on `review-11`, merged and released as v0.12.1 (milestone
`race-look`). The screens now read like the mockup: real car shapes, the
block rumble strips, the counters, the crash picture and list rows. The
hub scrolls one card at a time, the doc-comment check was clean, and the
new canvas commands keep a full road far below the command limit.

What the main developer changed at merge time:

- **The move hint** drew a few dots at its corners instead of a dashed
  outline: each dash's end was divided by the edge length twice, so every
  dash shrank onto the start of its edge. The previews were reported as
  inspected, but the hint in `race-play` shows the dots.
- **Cars:** the nose is rounder than the back and the traffic car's
  windshield is a curve, as the mockup's `race-car` and `race-traffic`.
- **Info boxes** have the mockup's 2 px dashed frame (`.infobox`), on the
  Race screens and on every other screen that draws one.

Verdict: D51 to D53 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- Look at a new shape enlarged next to the mockup's, not only through pixel
  probes: a probe on one dash passes while the outline is missing.

## Round 10 result (2026-10-11)

Branch `side-tasks-10` (D49 and D50) came as pull request #9 from v0.11.2,
with CI and the firmware build green. It was reviewed on `review-10`,
merged and released as v0.11.3 (milestone `race`). The engine is clean and
well tested, the drawing follows the mockup, and the step redraws only the
cells that changed. One choice differs from the task and stays: a car or
cone that reaches the player's column in another lane is counted as passed
and leaves the road, so the cells beside the player are always clear and a
lane change never crashes. A careful driver can always get through.

What the main developer changed at merge time:

- **Previews:** `race-play` showed the crash screen. Its sample driver
  pressed once toward a free lane two lanes away, stayed in the wrong lane
  and crashed at 1.4 km. It now presses until it reaches the lane. The
  previews were reported as inspected.
- **Back to Games** on the crash screen did nothing: `exit_requested` was
  set and never read. It now leaves as hold BOOT does.
- **Mode** replaced the game with the mode list before the run's best was
  kept, so a new best was lost. It is kept first now.
- **Race again** kept the last run's top speed; it starts at 1 now.
- The mode list's help line drew `?? change a lane, ? nitro`: ▲▼● are not
  in the fonts. It says it in words now.
- The crash rows read `Race again  classic`; the mode is now a
  right-aligned value, as in the other lists.
- A paused race still asked the main loop for a step every second; it
  needs no timer now, so the device can sleep between keys.
- The guide said driving into a car crashes; it now says what ends a run.

Verdict: D49 and D50 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- Open every preview image you add and check it shows the state its name
  says.
- For every request flag a screen sets, find the code that reads it.
- Draw only the characters the fonts have; ▲▼● are key caps in the
  bottom bar, not text.

## Round 9 result (2026-10-11)

Branch `side-tasks-9` (D44 to D48) came as pull request #8 from v0.11.1,
with CI and the firmware build green. It was reviewed on `review-9`, merged
and released as v0.11.2 (milestone `reader-style`). Every task is wired to
the device, each screen has its preview (and Large where asked), and the
doc-comment check came back clean for the first time. D48 also fixed a real
loader gap: a place outside the cached pages now opens at its byte offset
instead of the start.

What the main developer changed at merge time:

- **D44:** `main.rs` passed the key clock (`event_clock_ms`) with
  `force = false` at all three sites. The key clock stops between presses,
  so the five-minute cadence never saw two minutes pass, and sleep did not
  write a place younger than two minutes. `main.rs` now uses the running
  uptime clock, and sleep and leaving the reading view force the write. A
  place still unsaved after its session closed reported a write without
  writing; it now writes the place kept in memory.
- **D46:** Reader Options still showed rustmix's `ADD`, `LIST`, `NONE`,
  `RUN` and `>>>`. It now says Add or Remove Bookmark, the number of
  bookmarks and of contents entries, `None`, nothing for an action and `›`
  for a screen. `draw_list_row` drew a title without a description high in
  the row, off the value's line; it now centres it, which every settings
  list shows.

Verdict: D44 to D48 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- For a new function that takes a time, check the clock every caller
  passes. The key clock only moves when a key is pressed.
- When a screen changes style, read its values as well as its layout, and
  replace old placeholders.

## Round 8 result (2026-10-10)

Branch `side-tasks-8` (D39 to D43) came as pull request #7 from v0.10.3,
with CI and the firmware build green. It was reviewed on `review-8`, merged
and released as v0.10.4 (milestone `reading-settings`). This was the best
round so far: every task is wired to the device, the previews show the new
states at Standard and Large, percentages are in Body size, and the old
`library` preview name clash was found and fixed.

What the main developer changed at merge time:

- **D39:** the leading copy of a chapter's label was dropped from the text
  without a word boundary, so a chapter labelled `I` that starts "In a
  hole" would lose its `I`. Only a whole label followed by a space or a
  line break is dropped now; a test covers it.
- **D41:** hold BOOT in an option list of Settings › Reading left the
  screen, though the bar says `hold: cancel`, and the list was still open
  on the next visit. It now closes the list first, as Display does.
  Settings › AI had the same gap (main line); fixed too.
- **D42:** the editor still said `Runtime editor`; it says `Edit alarm` or
  `New alarm`. The empty list said "New alarm creates one; ALARMS.TXT holds
  more", which reads as if the file held more than six.
- **Home** (main line): the Continue card now shows an EPUB's percent from
  the saved place.
- **Doc comments, a fourth time:** `leading_label_bytes` went under the doc
  comment of `paginate_epub_chapter`, `apply_reading_settings` under the
  one of `apply_display`, and the new doc of `draw_book_row` was stacked
  under the old one, which still said the author line was not done.
- **Docs:** the User Guide now covers chapter titles, the minutes left,
  authors and checks, Settings › Reading, and adding and deleting alarms.

Verdict: D39 to D43 done after the fixes. Open: nothing for these tasks.

Do differently next time:

- **Before each commit, run `git diff` and read the three lines above every
  added function.** The same mistake in four rounds means the check has to
  be a habit, not a memory.
- **Cutting user text needs a boundary test.** Anything that removes text
  (a label, a prefix, whitespace) needs a test where the text almost
  matches.
- **Every option list closes on hold BOOT before the screen does.** Check
  `AppState::back` when a screen gets a picker.

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
- An item that wraps over the edge of a page or window needs its own test;
  one-line items pass without exercising it.
- A new state needs a look at every `match` on it, in screens and in
  `main.rs`.
- Pending time or counts are settled when the mode that owns them changes.

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

## Round 18 tasks (branch `side-tasks-18`)

Start from `main` after pull request #16, where Round 17 is merged; read its
result at the top of this file first (it is about looking at your own
output), then the lists of Rounds 13 to 16. Create `side-tasks-18` from
`main`. Three tasks. The owner has not yet approved the Dice mockup; these
tasks do not depend on that. When they are done, merge `main` into the
branch and open a pull request.

**How to compare with the mockup.** For every screen you draw or change,
render the mockup's figure and your preview side by side before you write
the Status. Headless Chrome renders the mockup: copy `mockups/index.html`
to a scratch folder outside the repo, append a script that sets `--z` and
hides every `body > *` except `<h2 class="sec">` of your section and its
`.wall`, and run `"/Applications/Google Chrome.app/Contents/MacOS/Google
Chrome" --headless=new --hide-scrollbars --window-size=1250,760
--screenshot=out.png file:///path/to/copy.html` (the Round 17 review did
exactly this). List every difference you see; fix each one, or say why it
cannot be fixed with this canvas and which primitive it needs.

**Main line during this round.** The main developer works on OTA updates
(Phase 8), then the Opus encoder, then wires Dice. Leave alone: `src/main.rs`,
`src/lua_runtime/`; updates and network (`partitions.csv`,
`sdkconfig.defaults`, `Cargo.toml`, `.github/`, `src/ota*.rs`,
`src/network*.rs`, `src/radio_burst.rs`, `src/wifi_transfer.rs`,
`src/build_info.rs`, `src/app/screens/settings.rs`,
`src/app/screens/device_info.rs`, `src/app/screens/network.rs`); XiaoZhi
and audio (`src/xiaozhi/`, `src/app/screens/xiaozhi.rs`, `src/audio/`,
`src/voice_notes.rs`, `src/ai_jobs.rs`, `src/ai_worker.rs`), `src/imu*.rs`.
`src/lib.rs` and `src/games/mod.rs` are shared: add only your `pub mod`
lines. Run the doc-comment check of Round 9 before you hand over.

### D75: The Games hub as in the mockup, with Race and Dice

**Why.** The mockup's Games figure now has four cards: Sudoku, Tetris, Race
and Dice (Dice selected). The hub scrolls one card at a time since Round
11; its previews were made with three cards.

**Change.**

- `src/app/screens/games.rs` and its previews: a Dice card with the
  mockup's icon (two dice, one tilted: use the `die_shape` geometry through
  a sprite or the `Die` command, whichever keeps the card's command budget)
  over `Shake to roll · 2 dice` and `Last roll 4 + 6 = 10`; the last roll
  comes from `RECORDS.TXT` through a new key `dice_last` in
  `games/records.rs` (a string `"4+6"`; absent when never rolled; a bad
  value is an error that leaves the card without the line, not a panic).
- `examples/sd-card/RUSTMIX/APPS/DICE/` with `APP.TOM`, a `MAIN.LUA` that
  calls `dice.init()` (the bridge is the main line's; say so in the
  `README.TXT`, which reads like `RACE/`'s) and the SD guide's tree and the
  User Guide's Games section (say that Dice is not playable until the
  firmware that wires it).
- The status bar count in the hub follows the number of cards.
- Previews: `games` (four cards, Dice selected, as the figure), `games-race`
  (Race selected, hub scrolled so Race and Dice are visible), each side by
  side with the mockup as above.

**Tests:** four cards, the selected one scrolls into view; a missing, empty
and malformed `dice_last`; the card text for one die and for three; the
manifest parses with the loader's own parser.

**Status:** done on `side-tasks-18`. Built: a Dice card in
`src/app/screens/games.rs` (icon: `die_shape` pair, `Shake to roll · 2
dice`, `Last roll 4 + 6 = 10` from `dice_last`); `games/records.rs` gains
`dice_last` (`4+6`, one to three faces of 1..=6; missing/empty/malformed
leave the card without the line and do not fail the rest of the file);
`examples/sd-card/RUSTMIX/APPS/DICE/` with `APP.TOM`, `MAIN.LUA`
(`dice.init()`), `README.TXT` (says the wiring is not released); SD
guide tree and User Guide Games table (Dice not playable yet). Hub count
follows the card count. Previews `games` (four cards, Dice selected) and
`games-race` compared side by side with headless Chrome. Fixed to match:
Tetris `Zen (no gravity) · best …`, Race `Zen: the road moves on a key`
when no best, Sudoku icon digits, info-box text, key bar `● options`.
Tests: four cards in mockup order, dice card one/two/three faces, missing
empty and malformed `dice_last`, `DICE/APP.TOM` through the loader's
parser, fourth card selected in the fourth slot. Open: nothing known.

### D76: Games screens against the mockup

**Why.** Round 17 showed a screen can be handed over with its differences
listed and not fixed. The Sudoku, Tetris and Race screens are older and
have not been put next to the mockup since their rounds.

**Change.** For each of these previews, render the mockup's figure and the
preview side by side, write a table of every visible difference (layout,
sizes, weights, borders, text, icons) and fix those that need only code
outside the files this round leaves alone: `sudoku-*`, `tetris-*` (Zen and
Classic), `race-play`, `race-crash`, and `games`. Put the table in the
Status: screen, difference, fixed (commit) or blocked (the primitive it
needs). Do not change a game's rules or the wiring. If a fix needs a font
or a widget in `src/app/widgets/`, add it: the main line is not editing
those.

**Tests:** one test for each fix that changes pixels (a probe of where the
new thing is), none for a layout that only moved by a few pixels.

**Status:** to do.

### D77: A Dice session, so the wiring is a few lines

**Why.** The main developer will wire Dice into `main.rs` and `lua_runtime/`
once OTA and the encoder are done. The less that wiring has to decide, the
less can go wrong on a device nobody can step through.

**Change.** `src/games/dice_session.rs` with `pub mod dice_session;`: a pure
state machine that ties the engine, the screens and the rattle together,
driven by the events the device has and a clock the caller passes in:

- `DiceSession::new(seed, options)`; `on_shake(now_ms)`, `on_key(key,
  now_ms)` (`Select` rolls, `UpDown` changes the number of dice, short
  `Boot` opens or closes Options, hold `Boot` leaves) and `tick(now_ms)`,
  each returning an `Output` that says what the caller must do: redraw (and
  whether the refresh is partial or full, from `full_refresh_due`), start
  the rattle (with the seed and dice count `Rattle::new` needs), stop it,
  the next time it wants a tick (`Option<u64>` in ms), and whether the
  session ends.
- A shake rolls only on the ready screen with Shake to roll on, outside the
  900 ms cooldown after a roll ends (`imu_events.rs`'s `SHAKE_COOLDOWN_MS`
  is 900 and not public: take the number as a named constant here, with a
  comment naming it, and a test that both agree if it becomes public
  later); a shake while rolling or in Options does nothing. A key rolls
  even with shaking off.
- The roll's frames are spaced to fit `dice_sound::duration_ms(dice)`: the
  engine's `FRAMES` frames and the result inside the rattle, the last
  frame landing with the knock. With Sound off the same timing is kept.
- The options (dice count, sound, shake, strength) live in the session and
  are returned as a small `Options` value the caller can save and give
  back; saving them is the caller's job. Strength maps to the
  `shake_delta_mg` the IMU detector should use, as three named constants
  around its default 420.
- No drawing, no audio, no IMU, no clock of its own.

**Tests:** a shake rolls, a second shake 300 ms later does nothing, one
after the rattle plus 900 ms does; a key roll with shaking off; the frame
times are strictly increasing and the last is at the knock; sound off
still yields the frame times and no rattle; Options swallows shakes;
changing the dice count mid-roll waits; hold Boot ends from every screen;
a tick with nothing due returns the next due time and no redraw; the
tenth roll asks for a full refresh and the ninth does not; every `Output`
for a fixed script of events is the same on two runs.

**Status:** to do.

## Round 17 tasks (done, merged without a release)

Start from `main` at v0.12.7, where Round 16 is merged; read its result at
the top of this file first, and the "Do differently next time" lists of
Rounds 13 to 16. Create `side-tasks-17` from `main`. Three tasks, in this
order. The owner asked for a Dice game (a die, rolled by shaking the
device, with the sound of real dice); the mockup is the `Dice` figures in
`mockups/index.html` (D71, waiting for the owner's approval). This round
builds the parts that need neither the device nor `main.rs`, as Race's
D49 did. When they are done, merge `main` into the branch and open a pull
request.

**Parts that may still change.** The owner has not approved the mockup's
choices: up to three dice, three frames a roll, a shake strength option,
six sides only. Keep the number of dice, the number of sides and the
frame count as parameters or constants in one place each, so changing
one is a one-line edit and a test follows it.

**Main line during this round.** The main developer works on OTA updates
(Phase 8), then wires the Opus encoder, then wires Dice: the Lua or
native entry in `main.rs` and `lua_runtime/`, the shake events and the
motion sensor's switching, and the playback. Leave alone: `src/main.rs`,
`src/lua_runtime/`; updates and network (`partitions.csv`,
`sdkconfig.defaults`, `Cargo.toml`, `.github/`, `src/ota*.rs`,
`src/network*.rs`, `src/radio_burst.rs`, `src/wifi_transfer.rs`,
`src/build_info.rs`, `src/app/screens/settings.rs`,
`src/app/screens/device_info.rs`, `src/app/screens/network.rs`); XiaoZhi
and audio (`src/xiaozhi/`, `src/app/screens/xiaozhi.rs`, `src/audio/`,
`src/voice_notes.rs`, `src/ai_jobs.rs`, `src/ai_worker.rs`); the
widgets in `src/app/widgets/`, `src/app/display_assets.rs`,
`src/imu*.rs`. `src/lib.rs` is shared: add only your `pub mod` lines.

Run the doc-comment check of Round 9 before you hand over. Test the
edges, as Round 13 and 14's lists say. No `expect` or `unwrap` outside
tests on data from the card.

### D72: The dice engine (library only)

**Why.** Everything the game does that is not drawing: rolling, the dice
count, the history. It must be testable on the host, as `race.rs`'s
engine is.

**Change.** `src/games/dice.rs`, a pure engine, with `pub mod dice;` in
`src/games/mod.rs`:

- `DiceGame::new(seed)`: the same seeded xorshift as `race.rs` and
  `tetris.rs` (a zero seed starts from a constant; copy the rule, or
  share a helper if one exists, without changing theirs).
- State: `dice` (1 to 3, default 2), `sides` (6), the current faces, the
  history of the last six totals (newest first), `rolls` since the last
  full refresh, and a `phase` (`Ready`, `Rolling { frame }`).
- `request_roll()`: from `Ready`, picks the final faces now (so a test
  can see them) and goes to `Rolling { frame: 1 }`; while rolling it
  does nothing and says so (a shake during a roll must not restart it).
  A shake that arrives within the cooldown after a roll ends is the
  caller's business, not the engine's.
- `advance()`: one frame on; the last frame ends in `Ready` with the
  final faces, the total pushed on the history, and `rolls + 1`. Frames
  before it show tumbling faces that are not the final ones when more
  than one face is possible, and are never the same two frames in a row.
- `set_dice(n)` clamps to 1..=3, takes effect from `Ready` only, and
  keeps the history. `full_refresh_due()` is true every tenth roll.
- Every constant (`MAX_DICE`, `DEFAULT_DICE`, `SIDES`, `FRAMES`,
  `HISTORY_LEN`, `FULL_REFRESH_EVERY`) is a named `const` at the top.

**Tests:** the same seed rolls the same faces; over 6 000 rolls of one
die every face 1 to 6 comes up, each between 14% and 19%; two dice total
2 to 12 and the totals are not uniform (7 is the most common); a roll
takes exactly `FRAMES` calls to `advance`; `request_roll` while rolling
changes nothing; `set_dice(0)` and `set_dice(9)` clamp; changing the
count mid-roll waits for `Ready`; the history keeps six totals, newest
first; a full refresh is due on the tenth roll and not the ninth;
`advance` in `Ready` does nothing.

**Status:** done on `side-tasks-17`. Built `src/games/dice.rs` with
`pub mod dice;` in `src/games/mod.rs`. `DiceGame::new(seed)` uses the
same xorshift and zero-seed rule as `race.rs`/`tetris.rs` (copied, theirs
unchanged). Constants `MAX_DICE`, `DEFAULT_DICE`, `SIDES`, `FRAMES`,
`HISTORY_LEN`, `FULL_REFRESH_EVERY` at the top. `request_roll` fixes
`final_faces` at once and enters `Rolling { frame: 1 }`; while rolling it
is a no-op. `FRAMES` advances play tumbling faces (never the final set,
never the same set twice in a row) and the last one lands in `Ready`,
pushes the total and increments `rolls`. `set_dice` clamps to 1..=3 and
takes effect from `Ready` (a mid-roll change waits; new dice are dealt,
kept ones stay). `full_refresh_due` on every tenth roll. Ten host tests
cover the list, including a tumbling-frame test and a 6 000-roll fair die
check. Open: nothing known.

### D73: The Dice screens and previews (no wiring)

**Why.** The mockup's three screens, drawn from the engine on the game
canvas, with previews to compare with the mockup.

**Change.** In `src/games/dice.rs` (or `src/games/dice_view.rs` if it
grows past the file's size limits; say which) and
`src/app/preview.rs`, following `race.rs`'s `render` and its previews:

- `render_ready`, `render_rolling` and `render_options` on
  `NativeGameCanvas` at 480 × 800, from the mockup's `Dice`, `Dice:
  tirando` and `Dice: opciones` figures: status bar, the dice drawn as
  rounded squares with round pips (`GameSprite` shapes if they fit, else
  new commands in `src/games/canvas.rs`, which this round may extend),
  the total, the last rolls, the info box, the key bar. A rolling frame
  draws the dice tilted and offset with the dashed rest outlines and the
  motion lines; frame 1 to `FRAMES` differ.
- A roll redraws only what changed (`dirty_regions.rs`); the full
  refresh comes when `full_refresh_due()` says so. Show in a test how
  many commands and what dirty area a roll's frames produce, and keep
  every screen far under `MAX_GAME_DRAW_COMMANDS`.
- Text uses only characters in `src/charset.rs`; `●▲▼` only in key caps.
- Previews: `dice-ready` (one, two and three dice), `dice-rolling-1`,
  `dice-rolling-2`, `dice-rolling-3`, `dice-options`, each at the
  Standard size and `dice-ready` also at Large UI. Open each, look at
  it enlarged next to the mockup, and list in the Status what differs
  and why.
- The Games hub's Dice card is not part of this task (it needs the SD
  app and `main.rs`).

**Tests:** the pips of each face 1 to 6 are where the mockup puts them
(probe the canvas); two dice do not overlap and stay inside the screen
when tilted; the three frames of a roll are not equal; the command count
stays under the limit for three dice; `render_ready` is a pure function
of the state (render twice, same commands).

**Status:** done on `side-tasks-17`. Built `src/games/dice_view.rs`
(engine stays pure). New canvas command `Die` (`src/games/canvas.rs`)
and its drawing in `src/app/screens/lua_game.rs` (rounded square, round
pips at the mockup's `die-1`..`die-6` centres, dashed rest outline, shear
for the tilt). `render_ready`, `render_rolling` and `render_options` at
the mockup's geometry: two 180 px dice at x 50 and 250, the big total,
`4 + 6` sum line, `Last rolls` chips (52 × 40), the info box and the key
bars (`● roll · ▲▼ dice · BOOT options · hold: back` and `rolling, keys
wait`). Rolling keeps the dashed rest seats, the offset sheared dice and
the motion lines; frames differ; the three-frame strip is on the figure.
Options has the six rows and the 1/2/3-dice strip. Previews `dice-ready`
(also `-1`, `-3`, `-large`), `dice-rolling-1..3`, `dice-options` looked at
enlarged next to the mockup. **Differs and why:** (1) die corners are
`RoundedRectangle`, not the SVG `rx=17` curve; (2) tilt is a shear plus
offset, not a rotate transform; (3) the ready preview rolls six times
first so the history chips show (a fresh engine's history is empty);
(4) the speaker mark in the status bar is drawn as paper shapes, not the
mockup's SVG; (5) `LuaEventBridge` is `Static` and the key hints come
from the session id — the Games hub card and `main.rs` wiring are the
next round. Command count is well under `MAX_GAME_DRAW_COMMANDS`. Open:
nothing known.

**Main developer, at review:** the five differences above were the
screens' real differences from the mockup, and the task said to list what
differs "and why", not to accept it. The shear drew flat parallelograms,
the stroke was a third of the mockup's, the pips half its size, the speaker
a broken bracket, the strips had no dice in them and the options rows
overlapped. Fixed in the review (`src/games/die_shape.rs`, the `Die`
command turns by an `angle`, a `Speaker` sprite, dice in the strips,
rows at the app's 72 px). What still differs: `Rolling…` is the Heading
size where the mockup has 64 px monospace (the canvas has no large
letters).

### D74: The dice rattle (library only)

**Why.** The owner wants the sound of real dice. The firmware has no
sound files and plays 16-bit stereo PCM at 16 kHz (`src/audio/tone.rs`:
`PCM_CHANNELS`, `PCM_BYTES_PER_FRAME`, `AUDIO_SAMPLE_RATE_HZ`); a
rattle can be made from short decaying noise clicks with a small
fixed-point generator, with no float trigonometry and no large arrays,
as `tone.rs` says.

**Change.** `src/dice_sound.rs` with `pub mod dice_sound;` in
`src/lib.rs`; `src/audio/` stays untouched and unread-only:

- A `Rattle` generator seeded like the engine, giving PCM in chunks of
  `PCM_CHUNK_FRAMES` frames so the playback loop the main developer
  writes can pull a chunk at a time: `new(seed, dice, frames)` and
  `next_chunk(&mut self, out: &mut [u8]) -> usize` (bytes written, `0`
  when finished).
- The sound: a rattle of about 600 ms made of 12 to 18 clicks (more for
  more dice) whose gaps grow and whose amplitude falls, each click a few
  milliseconds of band-limited noise with an exponential decay, then a
  knock about 80 ms long, a low decaying sine-like thump, at the end
  where the result lands. Total amplitude stays under half of full scale
  (the speaker is small and the chime uses 12 000).
- One function, `duration_ms(dice)`, says how long it plays, so a screen
  can show the result when the sound ends; the roll's three frames fit
  in it.
- Document in the module doc how the clicks and the knock are made and
  how a different timing is chosen (constants at the top).

**Tests:** the same seed gives the same bytes; a different seed gives
different bytes; the peak is under the limit and over silence; the
output ends in silence (the last 20 ms) and starts within 10 ms of a
click; the total length equals `duration_ms`; the chunks join to exactly
the length, the last one is short, and `next_chunk` returns `0` after
that; more dice give more clicks (count them by the peaks); a buffer
that is not a whole number of frames is refused or truncated to whole
frames, and the test says which.

**Status:** done on `side-tasks-17`. Built `src/dice_sound.rs` with
`pub mod dice_sound;` in `src/lib.rs`; `src/audio/` is unchanged.
`Rattle::new(seed, dice, frames)` (xorshift, zero seed as `race.rs`) and
`next_chunk(&mut self, out) -> usize` in `PCM_CHUNK_FRAMES` chunks of
16-bit stereo PCM at 16 kHz. About 620 ms: 12 to 18 band-limited noise
clicks (4 ms, growing gaps, falling amplitude) then an 80 ms 120 Hz
triangle knock, then a 20 ms silent tail. Peak is 12 000 (under half of
full scale). `duration_ms(dice)` reports the length so a screen can wait
for the result. A buffer that is not a whole number of frames is
**truncated** to whole frames (the test says so). Module doc explains the
clicks, the knock and how to retime them (constants at the top). Tests:
same seed / different seed, peak bounds, silent tail and early start,
length equals `duration_ms`, chunks join exactly with a short last one
and `0` after, more dice more peaks, partial-frame truncation, empty
buffer. Open: nothing known. Device check: play a rattle through the
speaker once the main line wires the playback loop.

## Round 16 tasks (done in v0.12.7)

Start from `main` at v0.12.6, where Round 15 is merged; read its result at
the top of this file first, and the "Do differently next time" lists of
Rounds 13 to 15. Create `side-tasks-16` from `main`. Three tasks, in any
order; D70 is a written study, no code. D71 (the Dice mockup) was drawn by
the main developer; leave `mockups/index.html` alone. When they are done, merge `main`
into the branch and open a pull request.

**Main line during this round.** The main developer works on OTA updates
(Phase 8) and then wires the Opus encoder into recording and the upload.
Leave alone: `src/main.rs`; updates and network (`partitions.csv`,
`sdkconfig.defaults`, `Cargo.toml`, `.github/`, `src/ota*.rs`,
`src/network*.rs`, `src/radio_burst.rs`, `src/wifi_transfer.rs`,
`src/build_info.rs`, `src/app/screens/settings.rs`,
`src/app/screens/device_info.rs`, `src/app/screens/network.rs`); XiaoZhi
and audio (`src/xiaozhi/`, `src/app/screens/xiaozhi.rs`, `src/audio/`,
`src/voice_notes.rs`, `src/ai_jobs.rs`, `src/ai_worker.rs`); the widgets
in `src/app/widgets/` and `src/app/display_assets.rs`. This round D68 may
change `scripts/fonts/` and the atlases it generates in
`src/app/typography/`, and nothing else in that folder.

Run the doc-comment check of Round 9 before you hand over. No `expect` or
`unwrap` outside tests on data from the card or the network.

### D68: `%` at the Detail size

**Why.** Inter's Detail strike at the Standard size (12 px) draws `%`
without its slash (Known Issues › Percent glyph). D6 found no
`fonts.toml` setting that fixes it alone, so the screens show
percentages in Body size, one size larger than the mockup.

**Change.**

- `scripts/fonts/generate.py` and `fonts.toml`: a per-glyph override, a
  small bitmap written in the TOML (rows of `#` and `.`) for one
  character in one face at one pixel size, with its advance. The
  generator uses it instead of the rasterized glyph and fails, with the
  face and size in the message, if the bitmap's height does not fit the
  strike's line height or a row has the wrong width. Document the key in
  the comment at the top of `fonts.toml`.
- Draw `%` for Inter at 12 px by hand: two rings and a slash, as wide as
  the rasterized `%` of the strike, on its baseline. Regenerate the
  atlases.
- Then move the percentages that went to Body size because of this back
  to Detail where the mockup has them (the sleep card's battery, the
  weather rain chances; grep for the comments that name the glyph). Not
  in the files this round leaves alone; list those in the Status.

**Tests:** the atlas diff changes only `%` in the Inter 12 px strike (say
how you checked it); a test in the generator's own check, or a host test,
that the `%` bitmap of `INTER_STANDARD_DETAIL` has ink on the diagonal
between its rings; the previews `sleep-card` and `sleep-weather` at Inter
Standard, looked at enlarged. Update Known Issues and the roadmap's
Backlog line.

**Status:** done on `side-tasks-16`. Built: `fonts.toml` documents the
`overrides` key (a character map of `advance` and a `#`/`.` bitmap) and
`generate.py` replaces the rasterized glyph, failing with the face and
size when the bitmap is too tall for the line height or a row has the
wrong width. `%` for Inter 12 px is two rings and a diagonal, 9 × 9 as
the rasterized glyph, advance 12. The atlas was patched by hand to only
that glyph (regenerating under local Pillow/fontTools rewrote hundreds of
unrelated Inter and display glyphs; the `fonts` workflow's toolchain is
the one that matches the committed atlases). Checked the atlas diff with
`git diff`: only `INTER_STANDARD_DETAIL`'s `%` metrics (`left` 1 → 2),
its 18 bitmap bytes and the strike summary line. Percentages moved back
to Detail: the sleep card's battery, the sleep-weather rain row, the
Reader page label and library row percents, and Reading Stats' change
label and current-book summary. Not in the files this round leaves alone:
`src/app/widgets/reading_header.rs` (the reading header's clock and
battery). Host test `standard_detail_percent_has_ink_on_the_diagonal_between_its_rings`.
Previews `sleep-card` and `sleep-weather` looked at enlarged: the slash
reads in `Battery 78%`, `0%`/`80%`/`60%` and the footer `78%`. Known
Issues' Percent glyph section is gone; the roadmap's Backlog line is
gone. Open: nothing known. Device check: the sleep card and the sleep
weather screen on the panel.

### D69: Voice Notes screens say audio, not WAV

**Why.** The result screen's header says `SAVED WAV DETAILS` (Round 15's
open item); once the encoder is wired a note can be Ogg. Other WAV words
may be left in the Voice Notes and AI screens.

**Change.**

- `src/app/screens/voice_notes.rs`: the header says `SAVED AUDIO`. Grep
  this file and `src/app/screens/ai.rs` for `WAV`, `wav` and `PCM` in
  text the user sees (not file names in tests) and change each to words
  that are true for both kinds, or that read the note's own kind as
  `voice_note_result::audio_text` does. List every hit and what you did
  with it in the Status.
- Do not change the free-space estimate (`~132 h`): it depends on the
  format the recorder writes, which the main line decides.

**Tests:** the header text; one test for each other string you change;
the `voice-note-audio` and `voice-note-result-audio-ogg` previews looked
at.

**Status:** done. Built: the result header is `SAVED AUDIO`. Every other
user-visible `WAV`/`PCM` hit in `src/app/screens/voice_notes.rs` and
`src/app/screens/ai.rs`: `SD PCM WAV RECORDINGS` → `SD AUDIO
RECORDINGS`; `Internal WAV filename remains unchanged.` → `Internal
filename remains unchanged.`; `This permanently removes the WAV file.` →
`… the audio file.`; `Saved as recovery-safe WAV.` → `… recovery-safe
audio.`; `WAV · 16 kHz mono · … so far` → `Audio · 16 kHz mono · …`;
the placeholder `VOICE---.WAV` → `VOICE---`. `ai.rs` had no
user-visible WAV/PCM text (only test file names and `wav_bytes` /
`pcm_bytes` fields). The free-space line `SD: … free (~132 h)` is
unchanged. Tests: the header text and each other string in
`screens_say_audio_not_wav`, plus the updated info-box test. Looked at
`voice-note-audio` and `voice-note-result-audio-ogg` (Opus text still
reads). Open: nothing known. Device check: open a note's result and the
delete confirm on the panel.

### D70: Deep sleep, a study

**Why.** Sleep mode now uses light sleep; the mockup's 8 µA needs deep
sleep (Backlog › Deeper sleep). Deep sleep restarts the chip on wake, so
it touches the wake keys, the alarms, the sleep screen and how fast the
device is back. Before any code the main developer needs the facts and a
plan.

**Change.** A new `docs/DEEP_SLEEP.md`, no code. From this repo,
`docs/architecture.md`, the board's schematic notes in the repo and the
ESP-IDF 5.5 documentation:

- Which wake sources deep sleep has on the ESP32-S3 (`ext0`, `ext1`,
  timer, ULP) and which of the device's inputs reach an RTC GPIO: BOOT,
  the wheel, the Power key through GPIO1 (`PWR_OUT`), the RTC alarm. Say
  for each where in this repo the pin is named.
- What a wake from deep sleep loses (RAM, PSRAM, Wi-Fi, the panel's
  driver state) and what survives (RTC memory, the panel's image, the
  SD card's files, NVS), and what the firmware keeps today only in RAM
  that would have to be saved first: list the fields of `AppState` that
  are not on the card, with the file that would hold each.
- What boot does today before the first screen, from the boot log
  markers (`rustmix-wave=…`) and `main.rs`: which steps a wake to the
  sleep screen could skip, and a time estimate for each with how you got
  it.
- How alarms, the weather sleep screens' updates and the clock sleep
  screen's minute would wake the chip.
- A plan in small steps, each one testable on the device, with the risks
  (a wake key that does not reach an RTC GPIO, a panel that needs a full
  refresh after a reset) and what the owner should measure.

**Tests:** none; every claim about this repo names its file and line,
and every claim about the chip names its documentation page. Say what
you could not find.

**Status:** done on `side-tasks-16`. Built `docs/DEEP_SLEEP.md`, no code.
Covered: ESP-IDF v5.5 wake sources (`ext0`, `ext1`, timer, ULP; GPIO and
UART are light-sleep only) and which of the device's inputs reach an RTC
GPIO — BOOT GPIO0, wheel GPIO4/5/6 and Power GPIO1 are RTC GPIOs 0–21;
**GPIO45 (RTC alarm) and GPIO38 (PMIC IRQ) are not**, so the alarm cannot
use `ext0`/`ext1` (the main hardware gap; timer-to-next-alarm or a board
rewire). What a wake loses (RAM, PSRAM, Wi-Fi, panel driver state) vs
survives (RTC memory, panel image, SD files, NVS), plus the `AppState`
fields that are RAM-only with the file that would hold each. Boot steps
from the `rustmix-wave=` markers in `src/main.rs` with a skip list for a
wake-to-sleep path and a rough 1.5–4 s cold / 200–800 ms warm budget to
measure. How alarms, weather slots and the clock minute would wake the
chip (timer; alarm needs the GPIO45 workaround). An eight-step plan with
risks and measurements. Could not find: measured boot timings (the table
says "owner must measure"), the exact ALDO3 behaviour for a partial
refresh after deep sleep in clock/weather modes, and what the mockup's
8 µA assumes for the PMIC/SD rails. Open: the owner's choice between
timer alarms and a board rewire (study §7).

### D71: Dice, the mockup

**Why.** The owner wants a new app in Games: a die on the screen that
rolls when the device is shaken, with the sound of real dice. Before any
code the screens are drawn in `mockups/index.html`, as every other app
was, so the owner can approve the look and the keys. This task is the
mockup only; the engine, the shake and the sound are later rounds.

Two facts to design around. First, Round 1 (D3) removed the tilt games
and kept the IMU for the Motion diagnostics only, so Dice is the first
game to use motion again: it must also roll with a key, and the IMU runs
only while Dice is open. Second, e-paper cannot animate: a roll is two
or three quick partial refreshes of the die tumbling (tilted, blurred
pips or a motion outline), then the result, while the speaker plays the
rattle and a last knock. The detector in `src/imu_events.rs` already
reports a `Shake` (80 ms polling, 900 ms cooldown); the tone code is in
`src/audio/tone.rs`.

**Change.** `mockups/index.html` only, in the `Juegos` section, after the
two Race figures, following the existing pattern exactly: one `<figure>`
with a `bezel`, an `s` screen in `lang="en"`, the status bar (`sb`), the
key bar (`ft` with `<i class="k">` caps) and a Spanish `figcaption` with a
bold title, as the other figures. Reuse the existing classes (`sb`, `ft`,
`k`, `kv`, `sel`, `infobox`, `chips`, `lbl`, `rstats`, `timer`) and add
new CSS only for the die itself; draw the die and its pips in inline SVG
(or an `<svg><symbol>` with `<use>`, as the Race cars do), black and
white only, no grey fills, at the mockup's 480 × 800 scale.

1. **Games hub:** add a Dice card to the hub figure (an icon in the
   60 × 60 `viewBox` like the others, title `Dice`, a line such as `Shake
   to roll · 2 dice` and `Last roll 4 + 6 = 10`), and update the count
   in its status bar. If the hub figure still lacks Race, add it too.
2. **Dice: ready.** One die large in the centre (two and three dice also
   fit: show how), the total under it, a short history of the last rolls
   (for example `Last: 10 · 7 · 3`), and the key bar: `●` roll, `▲▼`
   number of dice, `BOOT` hold: back. A one-line hint that shaking rolls.
3. **Dice: rolling.** The frame shown while it rolls: the dice tilted
   and offset from their rest place, with a motion mark (dashed outline
   or speed lines, like Race's streaks), a `Rolling…` line and a small
   speaker mark so the sound is visible in the mockup. The caption says
   how many frames and that the panel uses partial refresh.
4. **Dice: options** (a `kv` list): number of dice (1 to 3), sound (On,
   Off), shake to roll (On, Off), sides (6; say in the caption if other
   dice are worth it later).

Keep every character in text to what `src/charset.rs` has: `…` and `·`
are there, `●▲▼` are not and appear only inside key caps (`<i class="k">`).
Keep the English on screen and Spanish in the
captions, as the rest of the file.

**Tests:** none. Open the file in a browser and look at the new figures
next to Race and Sudoku at the same zoom: same frame, same status and key
bars, same type sizes. Say in the Status which choices you made that the
owner should confirm (number of dice, frames per roll, the options).

**Status:** done by the main developer (2026-10-18), waiting for the
owner's approval. Drawn: a Dice card (and the missing Race card) in the
Games hub, `Dice` ready with two dice, the total and the last rolls,
`Dice` rolling (frame 2 of 3: tilted dice, dashed rest outlines, motion
lines, the speaker mark) and `Dice · Options` (dice 1 to 3, sound, shake
to roll, shake strength, sides, a preview of the 1, 2 and 3 dice
layouts). The die faces are `die-1` to `die-6` and `die-rest` in the
file's `<defs>`. For the owner to confirm: up to three dice, three frames
a roll, a shake strength option, six sides only.

## Round 15 tasks (done in v0.12.6)

Start from `main` after pull request #13, where Round 14 is merged; read
its result at the top of this file first, and the "Do differently next
time" lists of Rounds 13 and 14 before you start. Create `side-tasks-15`
from `main`. Three tasks, in this order. When they are done, merge `main`
into the branch and open a pull request.

**Main line during this round.** The main developer keeps working on OTA
updates (Phase 8) and then wires the Opus encoder into recording and the
upload. The same files as in Round 14 stay alone: `src/main.rs`; updates
and network (`partitions.csv`, `sdkconfig.defaults`, `Cargo.toml`,
`.github/`, `src/ota*.rs`, `src/network*.rs`, `src/radio_burst.rs`,
`src/wifi_transfer.rs`, `src/build_info.rs`, `src/app/screens/settings.rs`,
`src/app/screens/device_info.rs`, `src/app/screens/network.rs`); XiaoZhi
and audio (`src/xiaozhi/`, `src/app/screens/xiaozhi.rs`, `src/audio/`,
`src/voice_notes.rs`, `src/ai_jobs.rs`, `src/ai_worker.rs`); drawing (the
files already in `src/app/widgets/`, `src/app/typography/`,
`src/app/display_assets.rs`, `scripts/fonts/`). `src/app/state.rs` is
shared: change only the line D65 names.

Run the doc-comment check of Round 9 before you hand over. No `expect` or
`unwrap` outside tests on data read from the card.

### D65: Books finished this year

**Why.** Reading Stats shows `books finished` beside this year's pages and
hours, but counts every finished book it keeps, from any year, because
`STATS.TXT` stores no finish date (see the roadmap, Reading Stats).

**Change.**

- `src/reading_stats.rs`: `BookStats` keeps the finish day (`Option<u32>`,
  an epoch day) instead of `finished: bool`. `mark_finished(book, day)`
  takes the day as `Option<u32>`; finishing a book already finished keeps
  its first day. The book line's last field is `0` (not finished), `1`
  (finished, day unknown: what older files hold) or the date as
  `day_label` writes it (`2026-10-18`). Old files load unchanged; a file
  saved again keeps `1` for a book whose day is unknown.
  `books_finished_between(first, last)` counts the books finished in that
  range of days; `is_finished` still says yes for every finished book,
  dated or not. Keep `books_finished` only if something still needs it.
- `src/app/state.rs`, `note_reading_page_turn` only: pass
  `self.local_day()`.
- `src/app/screens/reading_stats.rs`: the count beside this year's rows is
  the books finished between `first_of_year(today)` and `today`.
- `src/app/preview.rs`: the Reading Stats previews still show a count
  (give the sample books dates in this year).

**Tests:** a `1` line loads, counts as finished, is not counted in any
year and saves back as `1`; a dated line round-trips; finishing twice
keeps the first day; a book finished on 31 December is not counted on 1
January of the next year, and one finished on 1 January is; a malformed
date is an error, as other malformed fields are; without a clock
(`None`) the book is still marked finished.

**Status:** done. Built: `BookStats` keeps the finish day as
`Option<Option<u32>>` (`None` not finished, `Some(None)` finished with
the day unknown, `Some(Some(day))` the finish day). The book line's last
field is `0`, `1` or `day_label`; old `1` lines load and save unchanged;
a malformed date is an error. `mark_finished(book, day)` takes
`Option<u32>` and a second finish keeps the first day.
`books_finished_between(first, last)` counts only dated finishes in the
range; `is_finished` still covers every finished book.
`note_reading_page_turn` passes `self.local_day()`. The Reading Stats
year row uses `books_finished_between(first_of_year(today), today)`.
The Reading Stats and Library previews give their sample books dates in
this year so the count still shows. Tests: a `1` line loads, counts as
finished, is in no year and saves as `1`; a dated line round-trips and
counts in its year; finishing twice keeps the first day; 31 December
counts in its year and not the next, 1 January counts in the new year;
a malformed date is an error; without a clock the book is finished with
no day. Open: nothing known. Device check: finish a book and see this
year's `books finished` go up in Reading Stats.

### D66: Split an Ogg file into parts without holding it in memory

**Why.** `ogg_opus::split_parts` (D63) needs the whole stream in memory.
A one-hour note at 24 kbps is about 11 MB of packets, more than the
PSRAM the upload can have. The upload reads WAV parts straight from the
file; Ogg needs the same.

**Change.**

- `src/ogg_opus.rs`: `count_packets(reader) -> io::Result<(OpusHead,
  usize)>` reads the pages once and counts the audio packets (the head
  and tags are not counted), with the same rules as `read_stream` for a
  cut or damaged page.
- `split_file(reader, max_seconds, make_part)`, where `make_part(index)`
  gives the `Write` for part `index`: it counts first, rewinds (`Read +
  Seek`), then reads the packets again page by page and writes them to
  balanced parts as `split_parts` does, each a complete Ogg Opus file
  with the stream's head, finished before the next part starts. It
  returns the number of parts. It holds at most one page and one packet
  in memory, never the stream.
- `split_parts` stays (the tests use it as the reference); keep one
  balancing rule, used by both, not a copy.

**Tests:** for 1, 29 999, 30 000, 30 001 and 60 001 packets at 600 s,
`split_file` into `Vec<u8>` writers gives byte for byte what
`split_parts` gives; a stream whose packets cross page boundaries (a
packet over 255 bytes at the end of a page) splits the same way; a file
cut in the middle of a page gives the parts of the complete packets
before the cut; an empty stream calls `make_part` zero times; an error
from `make_part` or from a writer is returned, not swallowed.

**Status:** done. Built: `count_packets(reader)` walks the pages once and
counts audio packets (head and tags excluded), stopping at a cut or
damaged page as `read_stream` does. `split_file(reader, max_seconds,
make_part)` counts, rewinds, then writes each packet to a complete Ogg
Opus file through `make_part(index)`; each part is finished before the
next starts, at most one page and one packet are held, and it returns
the number of parts. An empty stream calls `make_part` zero times.
Errors from `make_part` or a writer are returned (no `expect`).
`split_parts` stays as the reference; both use one `balanced_shape`.
`read_stream` and the new walk share one packet path. Tests: `split_file`
into `Vec<u8>` writers is byte for byte `split_parts` for 1, 29 999,
30 000, 30 001 and 60 001 packets at 600 s; packets that cross a page
split the same way; a cut file gives the complete packets before the
cut; empty calls `make_part` zero times; `make_part` and writer errors
are returned. Open: nothing known. Device check: none yet (the encoder
is wired later).

### D67: Voice note text for an Ogg note

**Why.** The result screen's Audio tab says `16 kHz · mono · 16-bit` and
`KB of PCM` for every note, and `default_voice_title` gives `VOICE NOTE`
without the number for a `VOICE###.OGG`. When the encoder is wired, both
would be wrong for an Ogg note.

**Change.**

- `src/voice_note_result.rs`: `audio_text(file_name, duration_seconds,
  bytes)` decides from the name: a `.WAV` says what it says now; a `.OGG`
  says `{m}:{ss} · Opus · mono` and `{kilobytes} KB`. Rename the parameter
  `pcm_bytes` to `bytes` and say in its doc what it is for each kind (the
  PCM bytes for a WAV, the file size for an Ogg); the call in
  `src/app/screens/voice_notes.rs` stays as it is.
- `src/voice_note_metadata.rs`: `default_voice_title` gives `VOICE NOTE
  001` for `VOICE001.OGG` as well. Look for every other `.WAV` test in
  this file and in `voice_note_result.rs` and say in the Status what you
  found.
- `src/app/preview.rs`: `voice-note-result-audio-ogg`, the Audio tab of a
  note whose file is `VOICE007.OGG`, built by hand next to the other
  result previews.

**Tests:** both texts against the exact strings (a WAV and an Ogg of
3:05); `voice007.ogg` in lower case; a name that is neither (`NOTE.MP3`)
keeps the WAV text, as today; the titles for `VOICE001.WAV`,
`VOICE001.OGG` and `VOICE001.OPUS` (the last is `VOICE NOTE`). Check
every character you draw in `src/charset.rs`.

**Status:** done. Built: `audio_text(file_name, duration_seconds, bytes)`
decides from the suffix: `.WAV` keeps `16 kHz · mono · 16-bit` and
`KB of PCM`; `.OGG` (any case) says `{m}:{ss} · Opus · mono` and
`{kilobytes} KB`; any other name keeps the WAV text. `bytes` is the PCM
bytes for a WAV and the file size for an Ogg. `default_voice_title`
gives `VOICE NOTE 001` for `VOICE001.WAV` or `VOICE001.OGG` and
`VOICE NOTE` for `VOICE001.OPUS`. Preview `voice-note-result-audio-ogg`
is the Audio tab of `VOICE007.OGG`. Other `.WAV` checks in these two
files: `metadata_sidecar_keeps_fat83_wav_filename_and_friendly_title`
only names `VOICE001.WAV` as a sidecar file (not a title or format
text); no other test in `voice_note_result.rs` or
`voice_note_metadata.rs` hard-codes the WAV audio wording. The result
screen's header still says `SAVED WAV DETAILS`
(`src/app/screens/voice_notes.rs`, off-limits this round). Tests: both
texts against the exact strings for a 3:05 WAV and Ogg; lower-case
`voice007.ogg`; `NOTE.MP3` keeps the WAV text; the titles for
`VOICE001.WAV`, `VOICE001.OGG`, `voice001.ogg`, `VOICE001.OPUS` and
`NOTE001.OGG`. Every character is in `charset.rs` (Latin-1 `·`). Open:
the result header wording. Device check: open an Ogg note's Audio tab
once the encoder is wired.

## Round 14 tasks (done, merged without a release)

Start from `main` at v0.12.5, where Round 13 is merged; read its result at
the top of this file first, and its "Do differently next time" list before
you start. The Ogg Opus file exists but nothing sends it: this round
prepares the upload so that wiring the encoder later is one small step, and
looks at what Round 13 drew on a device with a different setup than the
previews. Three tasks, in this order. When they are done, merge `main`
into the branch and open a pull request.

**Main line during this round.** The main developer keeps working on OTA
updates (Phase 8) and, after it, wires the Opus encoder into recording.
Leave these files alone:

- `src/main.rs`; none of these tasks needs it;
- updates and network: `partitions.csv`, `sdkconfig.defaults`,
  `Cargo.toml`, `.github/`, `src/ota*.rs`, `src/network*.rs`,
  `src/radio_burst.rs`, `src/wifi_transfer.rs`, `src/build_info.rs`,
  `src/app/screens/settings.rs`, `src/app/screens/device_info.rs`,
  `src/app/screens/network.rs`;
- XiaoZhi and audio: `src/xiaozhi/`, `src/app/screens/xiaozhi.rs`,
  `src/audio/`, the voice note recorder in `src/voice_notes.rs`, and
  `src/ai_jobs.rs`, `src/ai_worker.rs` (the upload loop);
- drawing: the files already in `src/app/widgets/`, `src/app/typography/`,
  `src/app/display_assets.rs`, `scripts/fonts/`.

Put new previews in `src/app/preview.rs` next to the screen's other
previews. Run the doc-comment check of Round 9 before you hand over. Test
the edges, as Round 13's list says: an item that wraps over a boundary, the
largest input, every state a screen can be in.

### D62: The transcription form can carry an Ogg file

**Why.** `ai_client::transcription_form` always says `filename="note.wav"`
and `Content-Type: audio/wav`. The providers decide the format from the
file name and type, so an Ogg Opus body under those headers is refused.
`voice_note_record::record_file_name` accepts only `VOICE###.WAV`, so a
note recorded as Ogg would have no AI record.

**Change.**

- `src/ai_client.rs`: an `AudioKind` enum (`Wav`, `Ogg`) with the file
  name (`note.wav`, `note.ogg`) and the content type (`audio/wav`,
  `audio/ogg`) as methods. A new `transcription_form_for(kind, model,
  language, boundary)` builds the form; `transcription_form` stays and
  calls it with `Wav`, so `ai_jobs.rs` does not change. Use the enum in
  `transcription_request` too (a `kind` argument; update its callers in
  this file's tests).
- `src/voice_note_record.rs`: `record_file_name` accepts `VOICE###.OGG`
  as well and gives the same `VOICE###.AI`. Anything else is `None`.
  `VOICE001.WAV` and `VOICE001.OGG` share one record on purpose: a note
  is one or the other.

**Tests:** the form for each kind against the exact bytes, as the existing
test does for WAV; `transcription_form` still gives the WAV bytes; both
names map to one record, in upper and lower case; `VOICE001.OPUS`,
`VOICE1.OGG` and `NOTE001.OGG` give `None`.

**Status:** done. Built: `AudioKind` (`Wav`, `Ogg`) with `file_name`
(`note.wav`, `note.ogg`) and `content_type` (`audio/wav`, `audio/ogg`).
`transcription_form_for` builds the form; `transcription_form` stays the
WAV form and `ai_jobs.rs` is unchanged. `transcription_request` takes a
`kind`. `record_file_name` accepts `VOICE###.OGG` and gives the same
`VOICE###.AI` as `VOICE###.WAV` (upper and lower case); `VOICE001.OPUS`,
`VOICE1.OGG` and `NOTE001.OGG` give `None`. Tests: the Ogg form against
the exact bytes, `transcription_form` still the WAV bytes, both names map
to one record, the refused names. Open: nothing known. Device check:
none (the encoder is wired later).

### D63: Split an Ogg stream into parts

**Why.** A provider takes at most 25 MB, and `MAX_PART_SECONDS` keeps a
WAV part to ten minutes. An Ogg Opus note of the same length is about 2 MB,
so the limit is rarely reached, but a part that is an Ogg file of its own
is what the providers accept, and the main developer needs it for the
longest notes. Cutting the bytes of an Ogg file in the middle gives a part
the provider cannot read.

**Change.** `src/ogg_opus.rs`:

- `split_parts(stream: &OggOpusStream, max_seconds: u32) -> Vec<Vec<u8>>`:
  each part is a complete Ogg Opus file (head, tags, pages, end of stream)
  holding at most `max_seconds` of packets (50 a second), written with
  `OggOpusWriter` and the stream's own head. The parts are equal, the way
  `ai_client::wav_parts` makes them, so none is a short tail. An empty
  stream gives no parts.
- `ai_client::MAX_PART_SECONDS` is the limit a caller passes; do not
  copy the number.

**Tests:** a stream of 30 001 packets with a 600 s limit gives two parts
of equal size that read back to the same packets in order; exactly 30 000
gives one; an empty stream gives none; every part ends with the
end-of-stream flag and its first page is the head; each part's
`duration_ms` is under the limit.

**Status:** done. Built: `split_parts(stream, max_seconds)` in
`src/ogg_opus.rs`. Each part is a complete Ogg Opus file (head, tags,
pages, end of stream) written with `OggOpusWriter` and the stream's own
head, holding at most `max_seconds` × 50 packets. The parts are balanced
the way `wav_parts` splits (15 001 and 15 000 packets for 30 001 at a
600 s limit), so none is a short tail. An empty stream gives no parts.
The caller passes `ai_client::MAX_PART_SECONDS`; the number is not
copied here. Tests: 30 001 packets give two balanced parts that read
back to the same packets in order with the stream's head, BOS on the
first page, EOS on the last and `duration_ms` under the limit; exactly
30 000 gives one part; an empty stream gives none. Open: nothing known.
Device check: none yet (the encoder is wired later).

### D64: Look at the cached forecast and the verse bar the way a user sees them

**Why.** Round 13's previews show one setup each. A forecast adopted from
the card is `Stale` without an error, and the verse bar has only been seen
at the default font. Both went wrong on a case the preview did not show.

**Change.** `src/app/preview.rs` only, and fixes in the screens if a
preview shows a problem:

- `weather-cached`: Weather page one for a state made the way boot makes
  it (`adopt_cache` on parsed `FORECAST.TXT` text, a clock after the
  forecast's hours so some are dropped). Its footer reads `Updated … ·
  stale · Open-Meteo`.
- `home-weather-cached`: Home with the same state.
- `bible-verse-of-day-large`: the verse of the day preview at the Large
  UI and Large book size, so a verse wraps to more lines and a range
  crosses the page.
- `bible-verse-of-day-page-2`: the same Bible state one page on, opening
  on the wrapped lines of a marked verse (the case fixed in review).

Check each PNG: the bar's top and bottom against the text lines, nothing
past the bottom bar, `stale` readable at Large. Say in the Status line what
you looked at and what you changed.

**Tests:** none beyond the previews rendering, unless you change a screen;
then a test for that change.

**Status:** done. Built: `weather-cached` (Weather page one after boot
loads `/RUSTMIX/FORECAST.TXT`: `adopt_cache` on the parsed text, local
clock 18:20 so hours 13:00 to 17:00 are dropped), `home-weather-cached`
(Home with that state), `bible-verse-of-day-large` (the verse-of-day
card at Large UI and Large book size) and `bible-verse-of-day-page-2`
(the same card one page on). No screen change: the footers already say
`stale` without an error and the verse bar already follows a verse
across a page break. Looked at: the weather footer reads
`Updated 13:30 · stale · Open-Meteo` and `stale` is still readable when
the UI is Large; the cached Weather page shows only the hours from 18:00
and the days left; Home's strip still shows the cached conditions; the
verse bar on page two starts at the top of the text (the wrapped lines
of verse 2) and stops with that verse, nothing past the bottom bar on
any of the four. Open: nothing known. Device check: `weather-cached`
and `bible-verse-of-day-page-2` on the panel.

## Round 13 tasks (done in v0.12.5)

Start from `main` at v0.12.2, where Round 12 is merged; read its result at
the top of this file first. The screens match the mockup now, so this
round is about what the device remembers and counts: the forecast across
a restart, the verse of the day on its page, Bible reading in Reading
Stats, and the file format the Opus voice notes will use. Four tasks, in
this order. When they are done, merge `main` into the branch and open a
pull request.

**Main line during this round.** The main developer works on OTA updates
(Phase 8): a second app partition, the download over Wi-Fi and its row in
Settings › System. An update restarts the device, which is one more reason
for D58. Leave these files alone:

- `src/main.rs`, except the lines each task names;
- updates and network: `partitions.csv`, `sdkconfig.defaults`,
  `Cargo.toml`, `.github/`, `src/ota*.rs`, `src/network*.rs`,
  `src/radio_burst.rs`, `src/wifi_transfer.rs`, `src/build_info.rs`,
  `src/app/screens/settings.rs`, `src/app/screens/device_info.rs`,
  `src/app/screens/network.rs`;
- XiaoZhi and audio: `src/xiaozhi/`, `src/app/screens/xiaozhi.rs`,
  `src/audio/`, and the voice note recorder in `src/voice_notes.rs`;
- drawing: the files already in `src/app/widgets/` (call them; a new file
  there and its line in `widgets/mod.rs` are fine), `src/app/typography/`,
  `src/app/display_assets.rs`, `scripts/fonts/`.

Put new previews in `src/app/preview.rs` next to the screen's other
previews. Run the doc-comment check of Round 9 before you hand over. As
Round 12's result asks: feed state in tests the way the device does, and
check the edges and every mode of what you draw.

### D58: Keep the last forecast across a restart

**Why.** The forecast lives in memory only. After a restart (an update, a
flat battery, a crash), Home, Weather and the weather sleep screens show
nothing until the next fetch, which can be hours away with Wi-Fi off
between bursts (KNOWN_ISSUES, `Weather provider reliability`).

**Change.**

- `src/weather.rs`, or a new `src/weather_cache.rs`: write a `WeatherData`
  to `/RUSTMIX/FORECAST.TXT` and read it back. Text as the other settings
  files: `# Wave forecast v1` on the first line, then one `key=value` per
  line, with the location it was fetched for; `current` once, `day` and
  `hour` once per entry, the numbers as the integers `WeatherData` keeps.
  Write with `sd_file::replace`, read with `sd_file::read_to_string`.
- Reading drops what has passed: hours before the current local hour and
  days before today, when the clock is set.
- `WeatherSnapshot`: a way to adopt cached data that keeps its
  `observed_at` and the state `Stale`, not `Ready`, until a fetch
  succeeds. The screens then say `Updated 13:45 · stale` as they do after
  failed fetches; `sleep-live-weather-stale` shows that state, so no new
  preview.
- `main.rs`, named lines: at boot, after the first `update_board_snapshot`
  (the clock is read there), load the cache when the weather is set up and
  on, and the cache's location is the config's; after
  `state.weather.record_success(data)` in the weather fetch, save it (log a
  failure, do not retry).
- A cache from another location, a broken line, another version or an
  empty file is ignored, never an error on screen.

**Tests:** a round trip of a full `WeatherData` (negative temperatures, a
day and an hour without a rain chance); past hours and days dropped by
the clock and kept without one; another location, another version, a
broken line and an empty file give no cache; a snapshot from a cache is
`Stale` with its `observed_at`, and a fetch afterwards makes it `Ready`.

**Status:** done in v0.12.5. Built: `src/weather_cache.rs` writes
`WeatherData` to `/sdcard/RUSTMIX/FORECAST.TXT` (`# Wave forecast v1`, then
`location`, `timezone`, one `current` and one `day`/`hour` per entry as
`key=value` with `|`-separated integers) and reads it back through
`sd_file`. Reading drops hours before the current local hour and days
before today when the clock is set. `WeatherSnapshot::adopt_cache` keeps
`observed_at` and the state `Stale` until `record_success` makes it
`Ready`; `sleep-live-weather-stale` covers that display. `main.rs` loads
the cache after the first `update_board_snapshot` when the weather is set
up and on and the location matches, and saves it on each successful fetch
(logs a save failure, does not retry). Another location, a broken line,
another version and an empty file give no cache. Tests: a full round trip
(negative tenths, a day and an hour without rain), past hours and days
dropped with a clock and kept without one, the ignore cases, a snapshot
from a cache is `Stale` with its `observed_at` and a fetch makes it
`Ready`. Open: nothing known. Device check: restart with Wi-Fi off and
Home, Weather and the weather sleep screen still show the last forecast
as `Updated … · stale` once it is old.

### D59: Mark the verse of the day on its page

**Why.** `Versículo del día` in the Bible menu opens the chapter at the
verse's page, but nothing on the page says which verse it is: a page of
Psalms holds a dozen.

**Change.**

- `src/bible_state.rs`: the marked verses: the book, the chapter and the
  `VerseRef`'s range (`first..=last`). The menu's `Versículo del día`
  (`open_bible_menu_item` in `src/app/state.rs`) sets it; any other
  `open_at` clears it (`Ir a libro`, `Ir a capítulo`, a translation
  switch, the Bible opened again from Home). Paging through the chapter
  keeps it.
- `render_bible_reading` (`screens/bible.rs`): a 3 px bar in the left
  margin, x 6 to 8, beside every line of the marked verses on the page:
  the line with the number and the lines it wraps to. A range marks every
  verse in it, across a page break too. Nothing else on the page moves.
- **Preview:** `bible-verse-of-day`, a two-verse range whose second verse
  wraps, next to `bible-reading`.

**Tests:** the bar covers exactly the lines of the marked verses (a
wrapped verse, a range across a page break, a range label such as `1-2`
on the card); turning the page keeps the mark and opening a chapter from
the grid clears it; no bar without a mark or in another chapter.

**Status:** done in v0.12.5. Built: `BibleUiState` keeps
`MarkedVerses` (book, chapter, `first..=last`). `Versículo del día` in
`open_bible_menu_item` sets it through `open_at_marked` after the page is
found; every other `open_at` (`Ir a libro`, `Ir a capítulo`, a translation
switch, Home) clears it; paging keeps it. `render_bible_reading` draws a
3 px bar at x 6 to 8 beside every line of the marked verses on the page
(the numbered line and its wraps, every verse in a range, across a page
break). Preview `bible-verse-of-day`: a two-verse range whose second verse
wraps, next to `bible-reading`. Tests: the bar covers the wrapped verse's
lines only; a range including a `1-2` label marks every verse in it; a
range across a page break marks both pages while verse 1 alone does not;
a turn keeps the mark and a grid open clears it; no bar without a mark or
in another chapter. Open: nothing known. Device check: `bible-verse-of-day`
and a real `Versículo del día` on the panel.

### D60: Bible reading counts in Reading Stats

**Why.** Reading Stats counts time and pages only while a book is open
(`note_reading_key` needs a Reader session). Reading the Bible adds
nothing to today's minutes, the week's chart or the streak.

**Change.** `src/app/state.rs`:

- In the Bible reading view, ▲▼ count as reading keys
  (`ReadingClock::on_key`) and a turn that moves counts as a page, as in
  a book (`turn_bible_page`). The other Bible screens do not count.
- `collect_reading_stats` records the Bible's time and pages for the day,
  but no book: the per-book totals and `Books finished` stay the books'.
- `Settings › Reading`'s stats switch applies to the Bible as well.
- No `main.rs` change: the five-minute save and the save before sleep
  already run on every screen. Check that and say so in the Status line.

**Tests:** minutes and pages from a Bible session land on the day, with
the same two-minute cap as a book; a turn at the last page of the last
book counts no page; the Bible adds nothing to the per-book totals; with
stats off nothing is recorded.

**Status:** done in v0.12.5. Built: ▲▼ on `BibleReading` call
`note_reading_key` (the same `ReadingClock` two-minute cap as a book) and
`turn_bible_page` counts a page only when the position moves, so the last
page of the last book counts none. Other Bible screens do not call them.
`collect_reading_stats` records the day's minutes and pages with no book
when the session is Bible (`reading_on_bible`); `record_stats` off
discards them as for a book. `main.rs` needs no change: the five-minute
`save_reading_stats_if_dirty` cadence and the save before sleep both call
`collect_reading_stats` on every screen (`main.rs` around the
`READING_STATS_SAVE_INTERVAL` check and the sleep path). Tests: a Bible
session's minutes (30 + capped 120) and pages land on the day with no
per-book row; an edge turn counts no page; stats off records nothing; menu
keys record nothing. Open: nothing known. Device check: read the Bible a
few pages, sleep, and see today's minutes in Reading Stats.

### D61: Ogg Opus files (library only)

**Why.** Voice notes go as WAV, about 1.9 MB a minute; the mockup records
Opus at 24 kbps, ten times smaller, so uploads are faster and use less
battery (KNOWN_ISSUES, `Voice Notes AI`). The encoder is already in the
firmware for XiaoZhi. What is missing is the file: Ogg pages around the
Opus packets, as RFC 7845 says, which the transcription providers accept
as `.ogg`.

**Change.** A new host-tested `src/ogg_opus.rs`, with no ESP-IDF code:

- **Writer** over any `Write`: the `OpusHead` page first (beginning of
  stream; 1 channel, the pre-skip, input rate 16 000, gain 0, mapping 0),
  then `OpusTags` (the vendor `wave`, no comments), then the audio pages;
  `finish` marks the last page as the end of the stream.
- One audio page per second of audio (50 packets of 20 ms), so a crash
  loses at most a second, and never more than 255 segments. The granule
  is the 48 kHz sample count at the end of the page's last packet
  (960 per 20 ms packet).
- **Reader** over any `Read`: checks each page's capture pattern and CRC,
  joins packets that continue across pages, and returns the head's
  fields and the packets in order. A cut or damaged page ends the stream
  without an error for the pages before it, as the WAV recorder's crash
  recovery does today.
- `duration_ms` from the last page's granule less the pre-skip, read from
  the end of the file, not by reading the whole file.
- The CRC is Ogg's: polynomial `0x04C11DB7`, initial value 0, no
  reflection, no final XOR; `123456789` gives `0x89A1897F`.
- Nothing uses it yet: the main developer wires the encoder into
  recording, the upload and playback after OTA.

**Tests:** the CRC check value; the `OpusHead` (19 bytes) and `OpusTags`
bytes against RFC 7845 sections 5.1 and 5.2; lacing for packets of 0,
254, 255, 256 and 510 bytes; page flags, sequence numbers and granules; a
round trip of a few hundred packets; a file cut inside a page gives the
packets of the pages before it; a changed byte fails that page's CRC; the
duration of a 3:27 file read from its end.

**Status:** done in v0.12.5. Built: `src/ogg_opus.rs` (host-only,
in `lib.rs`). `OggOpusWriter` over any `Write` writes the `OpusHead` page
(BOS, 1 channel, pre-skip 312, input 16 000, gain 0, mapping 0), then
`OpusTags` (vendor `wave`, no comments), then one audio page per 50
20 ms packets (960 samples each at 48 kHz); `finish` marks the last page
EOS. Lacing handles 0, 254, 255, 256 and 510. `read_stream` over any
`Read` checks each page's capture pattern and CRC, joins packets across
pages and returns the head and the packets; a cut or damaged page ends
the stream with the earlier pages kept. `duration_ms` reads the last
page's granule from the file's end less the pre-skip. CRC is Ogg's
(`0x04C11DB7`, init 0, no reflection, no final XOR; `123456789` →
`0x89A1897F`). Nothing uses it yet. Tests: the CRC check value; OpusHead
19 bytes and OpusTags 20 bytes per RFC 7845; the five lacing cases; a
300-packet round trip; one-second pages with the right granule; a cut
file keeps earlier packets; a changed byte drops that page; a 3:27
duration from the end. Open: nothing known. Device check: none yet (the
encoder is wired later).

## Round 12 tasks (done in v0.12.2)

Start from `main` at v0.12.1, where Round 11 is merged; read its result at
the top of this file first. Every screen of the mockup is on the device
now, but four still differ from it: the voice note recording and result
screens, the details at the right of the Home rows, and the verse of the
day sleep screen, which Settings › Sleep screen still lists as `SOON`.
Four tasks, in this order. When they are done, merge `main` into the
branch and open a pull request.

Before you start, put `voice-note-summary.png`, `home.png` and
`sleep-screen.png` (the `screen-previews` artifact of `main`) next to the
mockup's `Voice Notes: resultado`, `Home` and `Settings › Sleep screen`
figures in `mockups/index.html`. The recording screen has no preview yet;
D54 adds one to compare with `Voice Notes: grabando`.

**Main line during this round.** The main developer works on OTA updates
(Phase 8): a second app partition, the download over Wi-Fi and its row in
Settings › System. Leave these files alone:

- `src/main.rs`, except the lines each task names;
- updates and network: `partitions.csv`, `sdkconfig.defaults`,
  `Cargo.toml`, `.github/`, `src/ota*.rs`, `src/network*.rs`,
  `src/radio_burst.rs`, `src/wifi_transfer.rs`, `src/build_info.rs`,
  `src/app/screens/settings.rs`, `src/app/screens/device_info.rs`,
  `src/app/screens/network.rs`;
- XiaoZhi: `src/xiaozhi/`, `src/app/screens/xiaozhi.rs`, `src/audio/`;
- drawing: the files already in `src/app/widgets/` (call them; a new file
  there and its line in `widgets/mod.rs` are fine), `src/app/typography/`,
  `src/app/display_assets.rs`, `scripts/fonts/`.

Put new previews in `src/app/preview.rs` next to the screen's other
previews, not at the end of `preview_states`. Run the doc-comment check of
Round 9 before you hand over. The fonts have no ▲▼●☐▸: draw the shapes
(the recording dot, the to-do boxes). Look at every new shape enlarged
next to the mockup's, as Round 11's result asks.

### D54: Voice note recording screen like the mockup

**Why.** The recording screen is still the diagnostic page from before the
redesign: `RECORD VOICE NOTE` / `ES8311 MICROPHONE TO SD` in the old
header, then rows of PCM bytes, peak, clipped samples and mic gain, and
`UP / DOWN pause` as text. The mockup shows the time large, a level meter
and what the note costs on the card.

**Change.** `render_voice_note_recording` in
`src/app/screens/voice_notes.rs`, after the mockup's
`Voice Notes: grabando` (its CSS: `.timer`, `.meter`, `.chips`,
`.infobox`):

- **Status bar:** `● REC · Voice Notes` with the battery at the right, as
  the XiaoZhi call screen draws `● On call · 02:14` (`screens/xiaozhi.rs`,
  `home::draw_battery`); `PAUSED · Voice Notes` while paused.
- **Title:** `New note · Oct 2, 12:04` centred in Heading, from
  `active_recorded_at` through `ai::split_stamp`. Under it in Detail:
  `The title is generated from the summary` when the note goes to the AI
  on stop, else the file name (`Saved as VOICE012.WAV`). "Goes to the AI"
  is the test `main.rs` makes after `finalize` (`queues_on_save` with the
  providers and keys ready); move it into an `AppState` method that both
  use, which is one named `main.rs` line.
- **Timer:** the elapsed time centred in the sleep clock's numerals
  (`draw_numerals` with `Numerals::Clock`, 126 px; the mockup's is
  120 px). Past an hour (`1:03:27`) use `Numerals::Temperature` when
  `numerals_width` passes 448.
- **Level meter:** as the mockup's `.meter`: 30 bars 10 px wide with 5 px
  gaps, centred in a 130 px band between two 2 px rules, 26 px under the
  timer, 10 to 110 px tall. One bar per second of audio, from the loudest
  chunk in it, the newest at the right, on a log scale (6 dB a step, for
  example `16 - peak.leading_zeros()`) so speech fills the band and
  silence is the shortest bar. `VoiceNotesUiState` keeps the last 30;
  `update_recording_progress` takes the chunk's peak and keeps the note's
  peak itself. `main.rs`: in the capture loop, pass `metrics.peak`
  instead of `session.peak()`.
- **Chips:** `● RECORDING` and `PAUSED`, the active one filled: Detail
  capitals, a 2 px border, as XiaoZhi's `LISTEN ▸ THINK ▸ SPEAK`. Add a
  `widgets/chips.rs` for a chip with an optional drawn dot; D55 uses it.
  XiaoZhi's copy stays until the main line is done with that file.
- **Info box** (`games::draw_info_box`): `WAV · 16 kHz mono · 6.3 MB so
  far`, `SD: 14.2 GB free (≈ 132 h)` and `Wi-Fi off while recording` (the
  radio and the AI queue wait for the recording to stop). The free space
  is `available_storage_bytes` less what the note wrote; sizes have one
  decimal in the binary units of `format_storage_bytes`
  (`voice_note_metadata.rs`); the hours are WAV hours at 32 000 bytes a
  second, grouped with `regional::grouped`. Without the free space, the
  first line only.
- **Hints:** `● stop & process` when the note goes to the AI, else
  `● stop & save`; `BOOT pause`, or `BOOT resume` while paused. ▲▼ keep
  pausing as today, without a hint, as the mockup shows two. For BOOT: a
  new `AppState::apply_voice_recording_boot_short_press` that queues the
  same request as ▲▼ (`PauseRecording` or `ResumeRecording`); `main.rs`:
  one more `|| state.apply_voice_recording_boot_short_press()` in the BOOT
  short-press chain, after `apply_xiaozhi_boot_short_press`. The request
  is then handled where the AI hub's is (`apply_voice_notes_ui_request`
  runs after the chain). Today a short BOOT here is ignored.
- **Other modes** on this route (`Saved`, `Error`, `Idle`) keep today's
  message, in the info box, under the same status bar and title. Playback
  draws on the details screen: check with `git grep` whether this route
  can still show `Playing` before you keep a branch for it.
- **Previews:** `voice-note-recording` (3:27 in, a run of speech-like
  levels, 14.2 GB free), `voice-note-recording-paused` and
  `voice-note-recording-large` (Large UI size).

**Tests:** one bar per second, the loudest chunk wins, the 31st second
drops the oldest; silence gives the shortest bar and a full-scale peak the
tallest; `update_recording_progress` keeps the note's peak; the info box
lines for 3:27 written and 14.2 GB free; a short BOOT on the recording
screen queues a pause, then a resume, and is not claimed on other screens.

**Status:** done on `side-tasks-12`. Built: the recording screen is the
mockup's `Voice Notes: grabando`. The status bar reads `● REC · Voice Notes`
(a drawn dot, the battery at the right; `PAUSED · Voice Notes` while
paused), then `New note · Oct 2, 12:04` centred in Heading from
`active_recorded_at` through `ai::split_stamp` with `The title is generated
from the summary` (when the note goes to the AI) or `Saved as
VOICE012.WAV` under it in Detail — the AI decision is the new
`AppState::voice_note_goes_to_ai`, which `main.rs`'s finalize path now uses
(one named line). The timer is centred in the sleep clock's numerals
(`03:27`; `1:03:27` past an hour falls back to `Numerals::Temperature` when
the clock strike passes 448 px). The level meter is the mockup's: 30 bars
of 10 px with 5 px gaps centred in the 130 px band between two rules, one
bar per second from the loudest chunk in it on a log scale
(`meter_bar_height`, 6 dB a step), the newest at the right;
`VoiceNotesUiState` keeps the last 30 in `level_bars` and the note's own
`note_peak`, fed by `update_recording_progress` — `main.rs` passes
`metrics.peak` now (one named line). The chips (`widgets/chips.rs`: a
bordered label, filled when active, with an optional drawn dot) are `●
RECORDING` and `PAUSED`, the active one filled. The info box (the hub's,
with its dashed frame) reads `WAV · 16 kHz mono · 6.3 MB so far`,
`SD: 14.2 GB free (≈ 132 h)` and `Wi-Fi off while recording` — sizes in the
binary units with one decimal (`size_one_decimal`), the hours as WAV hours
grouped — and the first line only when the free space is unknown. The
hints are `● stop & process` (or `stop & save`) and `BOOT pause`/`resume`;
▲▼ keep pausing without a hint. A short BOOT queues the same
`PauseRecording`/`ResumeRecording` request via the new
`AppState::apply_voice_recording_boot_short_press`, added to `main.rs`'s
BOOT chain after XiaoZhi (one named line); the request is handled where the
AI hub's is. The route cannot show `Playing` (playback lives on the details
screen: `begin_playing` is only reached from there), so `Saved`, `Error`
and `Idle` keep their message in the info box under the same status bar and
title. Previews: `voice-note-recording` (3:27 in, a speech-like run of
levels, 14.2 GB free), `voice-note-recording-paused` and
`voice-note-recording-large`; the chips were checked enlarged next to the
mockup's. Tests: one bar per second with the loudest chunk winning and the
31st second dropping the oldest; silence the shortest bar and full scale
the tallest; the note keeps its peak; the info lines for 3:27 and 14.2 GB
(132 h) and the first-line-only case; a short BOOT queues pause then resume
and is not claimed on other screens. Open: nothing known. Device check:
the three previews and a real recording on the panel.

### D55: Voice note result screen like the mockup

**Why.** The result screen has the mockup's parts, but under the old
`VOICE NOTE / AI RESULT` header, with the title in Heading, the tabs as
body-size boxes and the summary in small body text. The to-dos the summary
prompt asks for (a line with the word for to-dos, then `☐ ` lines; see
`summary_request` in `src/ai_client.rs`) show as plain bullets,
because `prepare_text` turns `☐` into `•`.

**Change.** `render_voice_note_result` in `src/app/screens/voice_notes.rs`
and `src/voice_note_result.rs`, after the mockup's
`Voice Notes: resultado` (its CSS: `.sum`):

- **Status bar:** `Voice Note`, with the time at the right as Home draws
  it (`state.board.time_label`).
- **Title:** the record's title in the Large role, up to two lines (the
  second cut with `…`), then the meta line as today.
- **Tabs:** `SUMMARY`, `TRANSCRIPT` and `AUDIO` as chips from D54's
  `widgets/chips.rs`, the active one filled, from the left margin.
- **Body:** one UI size up from the user's (Compact to Standard, Standard
  to Large; Large stays), near the mockup's 18 px; wrap and scroll with
  the same style.
- **To-dos:** parse the summary once into typed lines in
  `voice_note_result.rs` (bullet, to-do, heading, text) and draw by type.
  A to-do is a 14 × 14 box with a 2 px border, then its text; a bullet is
  `•`, then its text; their wrapped lines start under the text, not under
  the marker. A line that is neither and comes right before a to-do is a
  heading: Heading role, 12 px above it. The transcript and audio tabs
  stay plain text.
- Queued and failed notes keep their status text in the body; the footer
  and the hints stay.
- **Previews:** `voice-note-summary` again, with the mockup's summary in
  the sample record (three bullets, `Pendientes`, three to-dos), and
  `voice-note-summary-large`; check `voice-note-transcript`,
  `voice-note-audio`, `voice-note-queued` and `voice-note-failed` too.

**Tests:** the parser: bullets, to-dos with and without a heading line, a
`☐` inside a sentence stays text, CRLF line ends; the summary frame draws
one box per to-do; a long to-do's second line starts under its text; the
last scroll position shows the last to-do.

**Status:** done on `side-tasks-12`. Built: the result screen is the
mockup's `Voice Notes: resultado`. The status bar reads `Voice Note` with
the clock at the right (`board.time_label`); the title is the record's in
the Large role, up to two lines with the second cut with `…`, then the
meta line as before. The tabs are chips (D54's `widgets/chips.rs`), the
active one filled, from the left margin. The body wraps and scrolls one
UI size up from the user's (`result_body_style`: Compact→Standard,
Standard→Large, Large stays). The summary parses once into typed lines in
`voice_note_result.rs` (`parse_summary`: bullet, to-do, heading, text —
`☐`/`•` at the line start with a space, a clean plain line right before a
to-do is its heading, a marker inside a sentence stays text, CRLF counts
as one ending) and draws by type: a to-do is a 14 × 14 box with a 2 px
border then its text, a bullet is `•`, their wrapped lines start under the
text (`BodyRow::indent`), and a heading is the Heading role with 12 px
above it. The transcript and audio tabs stay plain text; queued and failed
notes keep their status text in the body. The footer and hints stay. The
scroll clamps to the last page counted from the end
(`last_visible_start`), so the heading gaps cannot push the last row off.
Previews: `voice-note-summary` (the mockup's summary: three bullets,
`Pendientes`, three to-dos — the boxes checked enlarged next to the
mockup's), `voice-note-summary-large`, and the transcript, audio, queued
and failed previews re-checked. Tests: the parser (bullets, to-dos with
and without a heading, a `☐` inside a sentence, CRLF); the summary frame
draws exactly one box per to-do (probed per row); a long to-do's second
line starts under its text; the last scroll position shows the last
to-do's box. Open: nothing known. Device check: the six previews and a
summarized note on the panel at both UI sizes.

### D56: Home rows' details like the mockup

**Why.** The mockup's Home shows a short detail at the right of most rows:
`48` photos, `24` books, `Sal 23`, `5-day streak`, `18°`, `2` games and
`2 apps`. The device shows only the books, the streak and the temperature
(`home_meta` in `src/app/screens/home.rs`).

**Change.** `home_meta`:

- **Photos:** the count from the last scan (`state.photos.photos`), empty
  for none. Today the scan runs only when Photos opens; `main.rs`: one
  line at boot, `state.photos.refresh()`, after
  `state.refresh_lua_app_catalog(..)`.
- **Bible:** where reading continues: the book's short name and the
  chapter (`Sal 23`), from `state.bible.usable_place()` and its book in
  `state.bible.books()`; empty without a place. The place is read at boot.
- **Games:** the hub's count (`state.lua_runtime.games().len()`), empty
  for none; the catalog is scanned at boot.
- **AI:** `2 apps`: the AI hub's card count, as a constant next to the
  hub's cards in `screens/ai.rs`, not a literal in Home.
- Tools and Settings stay empty. Keep the detail's style and place; check
  that `1 Cor 13` and `5-day streak` fit at the Large UI size.
- **Previews:** `home` again with the mockup's details (a place in
  Psalms, some photos and games), and `home-large-font` again.

**Tests:** each row's detail from a state that has it, and empty without;
a saved place outside the card's books (`usable_place` is `None`) gives no
Bible detail.

**Status:** done on `side-tasks-12`. Built: `home_meta` shows the mockup's
details. Photos: the count from the last scan (`photos.photos`), empty
before any is found — `main.rs` scans at boot now (one line after
`refresh_lua_app_catalog`). Bible: where the reading continues, the book's
short name and the chapter (`Sal 23`) from `usable_place()` and its book
in `books()`; empty without a place, and a saved place outside the card's
books gives nothing (`usable_place` is `None`). Games: the hub's count
(`lua_runtime.games()`), empty for none. AI: `2 apps` from the new
`AI_HUB_CARDS` constant next to the hub's cards in `screens/ai.rs` (its
`ai_row_count` uses it now), not a literal in Home. Tools and Settings
stay empty; the style and place are unchanged, and `1 Cor 13` and `5-day
streak` fit at the Large UI size (checked in `home-large-font`; the row's
right-aligned value has room beside the labels). Previews: `home` with the
mockup's details (48 photos, a place in Psalms, the SD games) and
`home-large-font`. Tests: each row's detail from a state that has it and
empty without (Photos, Games, AI), and the Bible row with a usable place,
with none, and with a place outside the card's books. Open: nothing known.
Device check: the two previews and Home on the panel with a photo folder,
a Bible place and the sample games.

### D57: Verse of the day sleep screen

**Why.** Settings › Sleep screen lists `Verse of the day`, `From your Bible
on the SD`, with `SOON`: `SleepMode::Verse` cannot be chosen. What it needs
is on `main`: the Bible reader, `VERSES.TXT` read at boot
(`BibleUiState::verse_list`) and `bible::reference_text`, written for this
mode.

**Change.**

- **Mode** (`src/sleep_screen.rs`): Verse can be chosen when the card has
  a verse list (`state.bible.has_verse_list()`); keep `SleepMode` free of
  `AppState` and pass the fact in. It is live (drawn by Wave) and wakes
  once a day. Settings › Sleep screen (`screens/sleep_settings.rs`) shows
  `1 WAKE-UP/DAY` for it, as the mockup, or `NO VERSES` when the card has
  none, and then refuses it as it refuses `SOON` today. A saved `verse`
  mode on a card without the list draws the clock.
- **Text:** a new `AppState` field with today's verse: its text, its
  reference (`VerseRef::label`, `Sal 23:1`) and the translation. One
  method loads it for `local_day()` with `bible::verse_of_the_day` and
  `bible::reference_text` (the verse the Bible menu's `Versículo del día`
  opens), and reads nothing when the day has not changed. Drawing reads
  nothing from the card.
- **Layout** (`screens/sleep_screens.rs`, a new `SleepLayout::Verse`):
  there is no mockup figure, so start from `Reposo: reloj y fecha` and the
  clock layout. The date at the top as its date line; the verse centred in
  the reader's style (`reader_body_style` with the reader's font) at the
  largest `BookFontSize` that fits, from `XLarge` down, else cut with `…`;
  the reference and the translation under it in Heading; the clock
  layout's footer (battery and wake hint).
- **Midnight:** a `verse_redraw_wait(now)` in `sleep_screen.rs` to the next
  local midnight plus the clock's margin, as `clock_redraw_wait`.
  `main.rs`, named lines: `next_sleep_redraw` returns it for
  `SleepLayout::Verse`, and the three `render_sleep_mode` calls (sleep
  entry, redraw and `sleep_preview`) load the verse first.
- **Previews:** `sleep-verse` (Psalm 23:1), `sleep-verse-long` (a verse
  that needs a smaller size) and `sleep-screen-verse` (Settings › Sleep
  screen with Verse chosen), next to the other sleep previews.

**Tests:** the wait to midnight from 23:59:30 and from 00:00:00 (a whole
day); two days in a row give the two verses `verse_of_the_day` gives, and
the same day loads once; Verse is refused without a verse list; the
status labels; without a card or a verse list the clock layout draws; the
long verse stays above its reference.

**Status:** done on `side-tasks-12`.

## Round 11 tasks (done in v0.12.1)

Start from `main` at v0.11.3, where Round 10 is merged; read its result at
the top of this file first. The owner played Race on the device: it plays
well and is fine as a first version, but it does not look like the mockup.
With the sample games on the SD card, the Games hub also never shows
Tetris. Three tasks, in this order. When they are done, merge `main` into
the branch and open a pull request.

Before you start, put `race-play.png` and `race-crash.png` (the
`screen-previews` artifact of `main`) next to the two Race screens of
`mockups/index.html` (section Juegos). Most of the gap is drawing: the game
canvas has only 1 px lines and boxes, so the road reads as black bars and
the cars as empty boxes.

**Main line during this round.** The main developer works on XiaoZhi
voice chat (Phase 7), then OTA updates (Phase 8). Leave these files alone:

- `src/main.rs`: the game timer and the canvas need nothing there;
- XiaoZhi: `src/app/screens/xiaozhi.rs`, `src/audio/`, `src/shared_i2c.rs`,
  `src/photos/worker.rs`; put new previews in `src/app/preview.rs` next to
  the Race ones, not at the end of `preview_states`;
- updates and network: `src/ota*.rs`, `src/wifi_transfer.rs`,
  `src/network*.rs`, `sdkconfig.defaults`, `partitions.csv`, `.github/`;
- voice notes and AI: `src/voice_note*.rs`, `src/voice_notes.rs`,
  `src/ai_*.rs`, `src/xiaozhi/`, `src/app/screens/voice_notes.rs`,
  `src/app/screens/ai.rs`;
- drawing: the files already in `src/app/widgets/` (call them; a new file
  there and its line in `widgets/mod.rs` are fine), `src/app/typography/`,
  `src/app/display_assets.rs`, `scripts/fonts/`.

Run the doc-comment check of Round 9 before you hand over. The fonts have
no ▲▼●▸: draw shapes, and let `draw_list_row` draw the ▸ of a selected row.

### D51: Games hub scrolls

**Why.** The sample SD card has five games and the hub draws four cards.
Its window, `selected.saturating_sub(3) / 4 * 4`, stays on the first four
until the eighth game is selected, so ▼ on Sudoku selects Tetris off
screen.

**Change.** `src/app/screens/games.rs`:

- The cards scroll one at a time and the selected card is always drawn, as
  the Settings lists keep their row in view (`first_visible` in
  `widgets/list_row.rs`). ▼ on the last card goes back to the first and
  the list back to the top; ▲ on the first shows the last at the bottom.
- The status bar keeps the game count; the info box stays under the cards.
- **Preview:** `games-scrolled`, the fifth of five games selected.

**Tests:** for one to nine games and every selection, the window holds the
selected card and does not run past the last game; the hub rendered with
the fifth of five games selected draws its inverted card in the fourth
slot.

**Status:** done on `side-tasks-11`. Built: the Games hub's cards scroll one
at a time (`first_card` in `screens/games.rs`): the window keeps the
selected card in view and never runs past the last game, so ▼ on the last
card wraps to the first with the list back at the top and ▲ on the first
shows the last at the bottom. The status bar keeps the game count and the
info box stays under the cards. Preview: `games-scrolled` (the fifth of
five games selected: the window holds the last four cards and the selected
one is drawn in the fourth slot). Tests: for one to nine games and every
selection the window holds the selected card and stops at the last game
(a short list never scrolls); the hub rendered with the fifth of five
games selected draws its inverted card in the fourth slot and an
unselected card in the first. Open: nothing known. Device check: the
preview and ▼ through five games on the panel.

### D52: Race playing screen like the mockup

**Why.** The playing screen has the mockup's parts but not its look: solid
black bars for the rumble strips, faint 1 px lane lines, empty boxes for
cars, small bars for cones, no labels over the counters and a blank band
under the speed bar.

**Change.** `src/games/race.rs` and the game canvas:

- **Sprites.** One canvas command that draws a named shape at a point, for
  example `DrawCommand::Sprite { x, y, sprite }` with a `GameSprite` enum
  in `games/canvas.rs`. `screens/lua_game.rs` draws it through a new
  `widgets/game_sprites.rs`, from the mockup's SVG symbols (`race-car`,
  `race-traffic`, `race-cone`, `race-burst`), with embedded-graphics
  shapes: `RoundedRectangle`, `Triangle`, `Line` with a stroke width,
  `Arc`. A filled star is a fan of triangles, as `draw_star` in
  `screens/photos.rs`. One command per sprite keeps a frame far below
  `MAX_GAME_DRAW_COMMANDS`.
- **Counters.** Three columns as in the mockup: `DISTANCE`, `BEST` and
  `NITRO` in capitals at the Detail size (as Reading Stats' `TODAY`), the
  values in Heading under them. Nitro is three 14 × 16 slots with a 2 px
  border, one filled per charge, at the right edge. Zen has no nitro: its
  third column is `CARS PASSED` (today Zen shows a charge it cannot spend).
- **Road** as the mockup's SVG (`<svg class="road" viewBox="0 0 448 468">`;
  both screens are 480 × 800, so its numbers carry over): rumble strips of
  16 px black blocks every 32 px, 14 px tall, at the top and the bottom; a
  3 px edge line inside each; lane lines of 30 × 4 px dashes every 52 px.
- **Cars** seen from above, nose to the right: four wheels, a rounded nose
  and windows. Other cars are a 3 px outline with paper inside; the
  player's car is solid with paper windows, with three speed streaks in
  the cell behind it. A cone obstacle is two cones: solid triangles with a
  paper band. Every sprite stays inside its cell, so the cells a step
  dirties cover what changed: scale the 60 × 44 symbols by about 1.2, not
  the mockup's 1.25.
- **Where ▲ goes:** the mockup's dashed rounded outline in the cell ▲ would
  move the car to; none in the top lane.
- **Under the road:** `SPEED 4 AT 3.0 KM` in capitals at the Detail size,
  and a 10 px bar with a 2 px border, filled to the kilometre's progress.
  Then the hub's info box (`draw_info_box` in `screens/games.rs`, through
  a canvas command, so the screen wraps the text with the real font).
  Classic: `One step a second at first, faster every kilometre. A full
  refresh now and then clears ghosting.` Zen: `The road moves only when you
  press. A full refresh now and then clears ghosting.`
- **Zen's bottom bar** says what its keys do: `▲▼ lane`, `● step`,
  `BOOT hold: back`. Today it shows Classic's `nitro` and `pause`.
- A lane change dirties the player's old and new cells and the outline's;
  today a Classic lane change dirties only the counters.
- The Games hub's Race icon becomes the player's car sprite, in paper on
  the selected card.
- `PAUSED` stays as it is.
- **Previews:** `race-play` again, `race-play-large` (Large UI size) and
  `race-zen`.

**Tests:** the fullest road (three obstacles in every column) stays under
`MAX_GAME_DRAW_COMMANDS`; each sprite drawn alone in a cell leaves no ink
outside it; a Classic lane change dirties both of the player's cells; Zen
draws `CARS PASSED`, no nitro slots and its own hints.

**Status:** done on `side-tasks-11`. Built: the playing screen has the
mockup's look. **Sprites:** `DrawCommand::Sprite { x, y, sprite }` with
`GameSprite` in `games/canvas.rs`, drawn by the new `widgets/game_sprites.rs`
from the mockup's symbols — `race-traffic` (a 3 px outline car with paper
inside, four wheels, windshield and door lines), `race-car` (solid with
paper windows and hood stripe), `race-cone` (solid triangle with a paper
band; a cone obstacle draws two), `race-burst` (ink star with a paper star)
and the move hint's dashed rounded outline. One command per sprite; the
sprites stay inside their cell (72 × 53 cars in a 74 × 105 cell), so the
cells a step dirties cover what changed. **Counters:** three columns —
`DISTANCE`, `BEST` and `NITRO` in capitals at the Detail size with the
values in Heading under them; the charges are three 14 × 16 slots with a
2 px border at the right edge, one filled each. Zen's third column is
`CARS PASSED` with the count. **Road:** the mockup's geometry — rumble
strips of 16 px blocks every 32 px, a 3 px edge line inside each, lane
lines of 30 × 4 px dashes every 52 px; the move hint outlines the cell ▲
would move to (none in the top lane), three speed streaks sit in the cell
behind the player. Under the road `SPEED 4 AT 3.0 KM` in Detail capitals,
a 10 px bar with a 2 px border filled to the kilometre's progress, then
the hub's info box through the new `InfoBox` command (Classic's and Zen's
own text). Zen's bottom bar says `lane`, `step`, `hold: back`. A lane
change dirties the player's old and new cells, its streaks' and the hint's
(`RoadSnapshot` carries the player lane); the full refresh comes when the
road step count crosses 30. The Games hub's Race icon is the player's car
sprite, paper on the selected card. `PAUSED` stays as it was. Previews:
`race-play` (with one nitro spent, so the slots mix fills and empties),
`race-play-large` and `race-zen`. Tests: the fullest road (three obstacles
in every column, cones worst-case) stays under `MAX_GAME_DRAW_COMMANDS`;
every sprite drawn alone leaves no ink outside its size; a Classic lane
change dirties both player cells; Zen draws `CARS PASSED`, no nitro slots
and its own hints. Open: nothing known. Device check: the three previews
and a Classic run on the panel at both UI sizes.

### D53: Race crash screen like the mockup

**Why.** The crash screen has the right numbers, but the distance is a
Heading at the left, there is no picture, and the rows are boxes: it reads
as a form, not as a crash.

**Change.** `render_crash` in `src/games/race.rs`, with the sprites of D52:

- Under the status bar, the mockup's picture: a 212 px road (rumble
  strips, edges, one lane line), a car ahead in the top lane, and the
  player's car turned 14° into another car, with the burst (`race-burst`,
  a star with a paper star inside) on the impact. The same picture for
  every crash.
- The distance large and centred, in the Weather screen's display numerals
  (`draw_numerals` with `Numerals::Temperature` in
  `widgets/display_numerals.rs`), through a canvas command.
- `Best 5.1 km · 2.7 km to go` (or `· new record!`) centred under it, in
  Heading; the screen measures centred text, as it does for `TextRight`.
- `TOP SPEED`, `CARS PASSED` and `NITRO USED` in capitals at the Detail
  size, the values in Heading under them, in three columns as the mockup.
- The three rows like every list on the device: full width, a rule under
  each, the selected row inverted with the ▸ marker, the value at the
  right. Draw them with `draw_list_row` (`widgets/list_row.rs`) through a
  canvas command. Their order, actions and hints stay.
- **Previews:** `race-crash` again and `race-crash-large`.

**Tests:** the crash frame holds one numerals command with the distance and
three list-row commands with the first selected; ▼ selects the second row
and the frame keeps its command count.

**Status:** done on `side-tasks-11`. Built: `render_crash` has the mockup's
picture and numbers. Under the status bar the picture: a 212 px road (the
rumble strips of 16 px blocks, the 3 px edges and one lane line), a car
ahead in the top lane, the player's car turned 14° into another car (the
`CrashCar` sprite: the tilt baked in, since the shapes layer has no
rotation) with the `race-burst` star on the impact. The same picture for
every crash. The distance is large and centred in the display numerals
(`Numerals::Temperature` through the new `Numerals` canvas command), the
`Best 5.1 km · 2.7 km to go` line (or `· new record!`) centred under it in
Heading through `TextCenter`, and `TOP SPEED`, `CARS PASSED` and `NITRO
USED` in Detail capitals with the values in Heading in three columns. The
three rows are `draw_list_row` rows through the new `ListRow` canvas
command: full width, a rule under each, the selected row inverted with its
marker and the value at the right; their order, actions and hints stay.
The cursor's row moves dirty only the rows band. Previews: `race-crash`
and `race-crash-large` (both inspected). Tests: the crash frame holds one
numerals command with the distance and three list-row commands with the
first selected; ▼ selects the second row and the frame keeps its command
count. Open: nothing known. Device check: the two previews and a crash on
the panel at both UI sizes.

## Round 10 tasks (done in v0.11.3)

Start from `main` at v0.11.2, where Round 9 is merged; read its result at
the top of this file first. Two tasks that add a racing game, in this order.
When they are done, merge `main` into the branch and open a pull request.

The design is the mockup's two Race screens, in Juegos after Tetris
(`mockups/index.html`). The road runs from right to left so that ▲▼
change lanes, and it moves in steps, as Tetris Classic falls.

**Main line during this round.** The main developer works on OTA updates
(Phase 8). Leave these files alone:

- `src/main.rs`: the game timer already runs any game through
  `AppState::tick_lua_game`, so nothing here needs it;
- updates and network: `src/ota*.rs`, `src/wifi_transfer.rs`,
  `src/network*.rs`, `sdkconfig.defaults`, `partitions.csv`, `.github/`;
- voice notes and AI: `src/voice_note*.rs`, `src/voice_notes.rs`,
  `src/ai_*.rs`, `src/xiaozhi/`, `src/app/screens/voice_notes.rs`,
  `src/app/screens/ai.rs`;
- drawing: `src/app/widgets/`, `src/app/typography/`,
  `src/app/display_assets.rs`, `scripts/fonts/`.

Run the doc-comment check of Round 9 before you hand over.

### D49: Race engine (library only)

**Why.** A game that suits e-paper: one screen change per step, and keys
that map to lanes.

**Change.** `src/games/race.rs`, with no drawing yet:

- A road of 4 lanes and 6 columns. The player's car is in column 1. Cars
  and cones arrive in column 5 and move one column toward the player each
  step. A step is 20 m.
- Each new column leaves at least one lane free, in or next to a lane left
  free by the column before, so a careful driver can always get through.
  Use the seeded generator of `tetris.rs`.
- ▲▼ move the car one lane at once; moving into a car or a cone is a
  crash, and so is a step that brings one into the player's cell.
- Classic: a step every 1000 ms at speed 1. The speed goes up by one each
  kilometre and each level takes 100 ms off, down to 500 ms.
  `tick_interval_ms()` gives the timer the current value.
- Nitro: start with one charge, one more each kilometre, at most three. ●
  spends one and takes three steps at once, checking each; anything in the
  way is a crash.
- Zen: no timer. ▲▼ change lane and then step, ● steps in the same
  lane, and there is no nitro.
- Counters: distance in metres, speed, cars passed and nitro used;
  `is_over()` after a crash.

**Tests:** 10,000 steps from several seeds never close every lane and always
leave one the player can reach; a car arriving in the player's cell
crashes; a lane change into a car crashes; nitro with a car two columns
ahead crashes, and on a clear lane it adds 60 m; the interval after 1, 3
and 6 km; Zen moves only on a key.

**Status:** done on `side-tasks-10`. Built: `src/games/race.rs`, the engine
only (no drawing). The road is four lanes and six columns with the player
fixed in column 1; cars and cones arrive in the last column and move one
column toward the player each step, 20 m a step. Each new column leaves its
free lane in or next to a lane the column before left free (the seeded
xorshift of `tetris.rs`), so the gap a driver follows never jumps. A step
that brings an obstacle into the player's cell is a crash, and so is a lane
change into one; an obstacle that reaches the player's column in another
lane has got past and leaves the road (a car counts as passed), which keeps
the lane beside the player clear to dodge into and makes "a careful driver
can always get through" true. Classic steps every 1000 ms at speed 1, 100
ms less each kilometre (`tick_interval_ms`, down to 500 ms); nitro starts
with one charge, gains one each kilometre (three at most) and spends one for
three checked steps at once; Zen has no timer, ▲▼ change a lane and then
step and ● steps in place. Counters: distance in metres, speed, cars passed
and nitro used. Tests: 10,000 steps from three seeds never close a column
and keep the gap chain touching; a car arriving in the player's cell
crashes; a lane change into a car crashes; nitro into a car two columns
ahead crashes and on a clear lane adds 60 m; the interval is 1000, 900, 700
and 500 ms at 0, 1, 3 and 6 km driven by a careful driver; Zen moves only on
a key. Open: nothing known. Device check: none yet (the engine draws
nothing; D50 puts it on screen).

### D50: Race as an SD game

**Why.** D49 on screen, with a card on Games and a best distance, as the
mockup draws it.

**Change.**

- `race.init()` in `MAIN.LUA` opens a mode list (Classic, Zen), as
  `tetris.init()` does (`lua_runtime/event_bridge.rs`).
  `next_game_tick_ms` (`lua_runtime/mod.rs`) returns the Classic interval
  of the current speed; today it only knows Tetris.
- The mockup's playing screen on `NativeGameCanvas`, at 480 × 800. The
  lanes and edges are drawn once; a step redraws only the cells whose
  content changed (`dirty_regions.rs`), and a full refresh comes every 30
  steps (`refresh_policy.rs`). Other cars are outlines; the player's car is
  solid.
- The mockup's crash screen: the distance, the best and how far it was,
  top speed, cars passed and nitro used, then Race again, Mode and Back to
  Games.
- BOOT pauses Classic (the step clock stops and `PAUSED` shows); hold BOOT
  leaves, as in Tetris.
- Best distances `race_classic` and `race_zen`, in metres, in
  `RECORDS.TXT` (`games/records.rs`), saved when leaving, as Tetris does.
- Games hub: a `race` card with a car icon and `Classic · best 5.1 km`
  over `Zen · best …` (`app/screens/games.rs`).
- `examples/sd-card/RUSTMIX/APPS/RACE/` with `APP.TOM`, `MAIN.LUA` and
  `README.TXT`, like `TETRIS/`; the SD guide's tree and the User Guide's
  Games section.
- **Previews:** `race-start`, `race-play`, `race-crash`, and `games` with
  three cards.

**Tests:** through `AppState::apply` and `tick_lua_game`: Games › Race ›
Classic, steps on the timer, a crash, Race again; the best kept in
`RECORDS.TXT`; the interval shrinks with speed; a step dirties only the
cells that changed.

**Status:** done on `side-tasks-10`. Built: `race.init()` in `MAIN.LUA`
opens the mode list (Classic, Zen) through `lua_runtime/event_bridge.rs`
(`RaceInit`, `RaceStart`, `RaceApp`), as `tetris.init()` does; the press
time seeds the game. `next_game_tick_ms` returns the Classic interval of
the current speed (`tick_interval_ms`), and `tick_game` steps the road.
The mockup's playing screen on `NativeGameCanvas` at 480 × 800: the rumble
strips, edges and lane dashes are drawn once, a step redraws only the
cells whose content changed (the dirty rects of the moved cells plus the
counters), and a full refresh comes every 30 steps (`race.rs`'s dirty
regions). Other cars are
outlines; the player's car is solid with paper windows. The crash screen
shows the distance, the best and how far it was (or `new record!`), top
speed, cars passed and nitro used, then `Race again`, `Mode` (reopens the
mode list) and `Back to Games`. BOOT pauses Classic (the step clock stops
and `PAUSED` shows) and steps Zen; hold BOOT leaves, as in Tetris. Best
distances `race_classic` and `race_zen`, in metres, live in `RECORDS.TXT`
(`games/records.rs`), saved when leaving. The Games hub gains a `race`
card with a car icon and `Classic · best 5.1 km` over `Zen · best …`.
`examples/sd-card/RUSTMIX/APPS/RACE/` with `APP.TOM`, `MAIN.LUA` and
`README.TXT`; the SD guide's tree and the User Guide's Games table follow.
Previews: `race-start`, `race-play`, `race-crash`, and `games` with the
new card (all inspected). Tests: Games › Race › Classic through
`AppState::apply` and `tick_lua_game` (steps on the timer, the interval
shrinks with the speed, a crash, Race again, the best kept in
`RECORDS.TXT`); the crash menu's three rows; a step dirties only the cells
that changed; the race card's best distances; the records round-trip.
Open: nothing known. Device check: the four previews, a Classic run to a
crash and a Zen run on the panel, and `RECORDS.TXT` after leaving.

## Round 9 tasks (done in v0.11.2)

Start from `main` at v0.10.4. Five tasks, in this order; D48 is the
largest, so skip it rather than rush it. When they are done, merge `main`
into the branch and open a pull request (`gh pr create`).

**Main line during this round.** The main developer draws the display
digits and the mockup's weather icons (Weather, the sleep screens and
Home), then sends voice notes for transcription and summary (Phase 7).
Leave these files alone:

- `src/main.rs`, except the lines a task names;
- power and sleep: `src/power*.rs`, `src/sleep_*.rs`,
  `src/app/screens/sleep_*.rs`, `src/app/screens/power*.rs`;
- weather and Home: `src/weather*.rs`, `src/app/screens/weather*.rs`,
  `src/app/screens/home.rs` (D45 may change its two `ScreenRoute::Reader`
  arms, nothing else);
- drawing: `src/app/widgets/icons.rs`, `src/app/widgets/weather_icons.rs`,
  `src/app/widgets/display_numerals.rs`, `src/app/display_assets.rs`,
  `src/app/typography/`, `scripts/fonts/`;
- voice notes and AI: `src/voice_note*.rs`, `src/voice_notes.rs`,
  `src/ai_*.rs`, `src/app/screens/voice_notes.rs`, `src/app/screens/ai.rs`;
  Phase 7 also changed `src/wifi_transfer.rs`, `src/network.rs` and the
  Settings › AI keys row.

Read the Round 8 result first. Before you hand over, run this and fix
every hit: it prints an old doc comment line followed by added code, the
mistake of the last four rounds.

```sh
git diff -U3 main -- src | awk '/^ +\/\/\//{d=$0;n=1;next} n&&/^\+/{print d; print $0} {n=0}'
```

Every option list closes on hold BOOT before its screen does
(`AppState::back`). Percentages are drawn in Body size. The fonts have
ASCII, Latin-1 and `– — ‘ ’ ‚ “ ” „ • … ‹ › € ™ −` only.

### D44: Save the reading place in batches

**Why.** Mockup, "Reglas que seguiría": write progress to the SD card in
batches, every few minutes and before sleep. Today every page turn
rewrites `STATE.TXT`, `POSITS.TXT` and `RECENT.TXT`
(`persist_current_session_best_effort`), so each turn waits for three
safe writes.

**Change.**

- A page turn updates the place in memory (`resume`, positions, recent),
  so Home's card and the Library stay right, and marks it unsaved.
- The files are written when the place has been unsaved for two minutes
  (checked in `AppState::apply` with `event_clock_ms`), when the reading
  view is left (back, Home, another book, a layout rebuild), and before
  sleep: `main.rs` calls a new `ReaderUiState::save_place_if_unsaved` next
  to each `save_reading_stats_if_dirty` call (only those lines).
- A failed write keeps the place unsaved, so the next check tries again.

**Tests:** ten turns inside two minutes write nothing (count writes, as
D36 counts loads); the first check after two minutes writes once; back
from the page writes; the in-memory place moves on every turn; a failed
write is retried.

**Status:** done on `side-tasks-9`. Built: a page turn (and a TOC jump) updates the place in memory (`resume`, positions, recent) and marks it unsaved with its first-unsaved clock, so Home and the Library stay right; the three files and the anchor cache are written by `ReaderUiState::save_place_if_unsaved` when the place has been unsaved two minutes (checked at the head of `AppState::apply_with_clock` with `event_clock_ms`), when the reading view is left (`AppState::back` on the page; another book and a layout rebuild already wrote), and from `main.rs` next to each `save_reading_stats_if_dirty` call (only those lines: the sleep path, the reader-closed path and the five-minute cadence); a failed write keeps the place unsaved and the next check retries; a turn whose page advanced marks the place even when the cache read failed. Tests: ten turns inside two minutes write nothing and the in-memory place moves on every turn (a `place_writes` counter, as D36 counts loads); the first check after two minutes writes once; back from the page writes at once through `AppState::apply`; a failed write is retried. Open: nothing known.

### D45: Home › Library opens the books

**Why.** Left from D35: Home's Library row opens a three-row landing
(Continue Reading, Books, Bookmarks), and the books are two presses
further. The mockup's Library is the first screen, and its RECENT and
BOOKMARKS tabs already cover the other rows.

**Change.**

- Home's Library row opens the Library: RECENT when there is a recent
  book, else ALL. Hold BOOT there returns Home; hold BOOT on a page
  returns to the Library.
- The landing goes, and with it the `Bookmarks` route only it reached (the
  BOOKMARKS tab covers it). Reader Options › Bookmarks keeps its list
  (`ReaderBookmarks`). Home's Continue card still resumes the book.
- `home.rs`: only its two `ScreenRoute::Reader` arms (icon and book count)
  follow the new route.
- **Previews:** `home`, `library` (RECENT), and one with no book read yet.

**Tests:** Home › Library › a book › hold BOOT twice, through
`AppState::apply`, back to Home; the tab that opens with and without a
recent book; Reader Options › Bookmarks opens its list and hold BOOT
returns to Options.

**Status:** done on `side-tasks-9`. Built: Home's Library row opens the books
directly. The landing screen and its three rows are gone, and with them the
`ContinueReading` and `Bookmarks` routes (only the landing reached them;
`render_continue_reading` and its tests go too). The Library opens on RECENT
when the recent list has a book, else ALL (`AppState::enter_library`, from the
Home row's `open_route` and from Options › Go to Library); hold BOOT returns
from the books to Home and from a page to the books (`Library`'s parent is
`Home` now). Reader Options › Bookmarks keeps its list (`ReaderBookmarks`,
hold BOOT back to Options), the BOOKMARKS tab covers the old route, and the
Home card still shows the book to resume: its book sits at the top of RECENT
and opens at the saved place. `home.rs`: only the icon and book-count arms
follow the route. Previews: `home` (the count on the Library row), `library`
(RECENT with the recent book), `library-no-recent` (no book read yet: opens on
ALL, every row `new`), `home-first-boot` (the empty card). Tests: Home ›
Library › a book › hold BOOT twice through `AppState::apply`, back to Home;
the tab that opens with and without a recent book; Reader Options › Bookmarks
opens its list and hold BOOT returns to Options; a recent book still opens at
its saved place. Docs: the user guide's Home table and Library section follow
the new navigation. Open: nothing known. Device check: the previews and a real
book folder with a saved place.

### D46: Reader screens in the new style

**Why.** The Library, Settings and the reading page have the mockup's
look; the screens around them still have rustmix's black header with a
subtitle and a boxed status row: Reader Options, Reading Preferences, the
table of contents, the bookmark list, Opening Book, Settings ›
Reading and Settings › Display.

**Change.**

- The black status bar (`draw_status_bar`, the screen's name on the left,
  a short value on the right such as the book's title or `PREFS.TXT`),
  then rows with the label on the left and the value on the right, as
  Settings › Sleep screen draws them (read `sleep_settings.rs`, do not
  edit it), and option lists through `draw_option_list`.
- The rows scroll when they do not fit, at every UI size.
- Opening Book: the book's title, the stage and a bar, in the same look.
- **Previews:** one per screen at Standard, and Reading Preferences and the
  table of contents at Large.

**Tests:** every row fits above the bar at every UI size, or scrolls with
the selection on screen; a long book title is cut, not drawn under the
right value.

**Status:** done on `side-tasks-9`. Built: Reader Options, Reading
Preferences, the table of contents, the bookmark list, Opening Book, Settings
› Reading and Settings › Display draw in the mockup's look — the black status
bar (the screen's name left, a short value right: the book's title on the
reader screens, `PREFS.TXT` on Settings › Reading, `N saved` on the bookmark
list, `SD FILE` on Display), then `draw_list_row` rows with the
label left and the value right (long titles cut at the value's gutter) and
pickers through `draw_option_list`. Lists scroll through a window
(`list_row::visible_rows` / `first_visible`) that keeps the selection on
screen when the rows do not fit; Opening Book shows the book's title, the
stage and a bar in the row look, with the live message under it. The old
headers, boxed status rows, `draw_row` and the two `draw_setting_row`
helpers are gone. Previews: `reader-options`, `reader-preferences`,
`reader-toc`, `reader-bookmarks`, `reader-loading`, `settings-reading`,
`display`, plus `reader-preferences-large-font` and `reader-toc-large-font`;
all inspected at their sizes. Tests: the rows fit above the bar or scroll
with the selection on screen at every UI size (rendered at Compact, Standard
and Large), the scroll window keeps the selection visible, and a long book
title is cut before its value. Open: nothing known. Device check: the nine
previews and the screens on the panel at Standard and Large.

### D47: Settings › System › Memory

**Why.** Mockup, "Reglas que seguiría": always measure, with the battery
log and free memory in Settings › System. `RuntimeMemorySnapshot` is
logged at worker boundaries but never shown.

**Change.**

- A `Memory` row in Settings › System. The screen shows internal RAM
  free, its largest free block and the lowest free since boot
  (`heap_caps_get_minimum_free_size`), PSRAM free and its lowest, and the
  main task's stack margin.
- `main.rs` captures a snapshot when the status bar refreshes and stores
  it in `AppState` (only those lines). The screen reads the stored one.
- Sizes in KB, or MB above 1 MB, with `regional::grouped`.
- **Preview:** `settings-memory`.

**Tests:** the size labels; the lowest value never goes up across
snapshots; Home › Settings › System › Memory through
`AppState::apply`.

**Status:** done on `side-tasks-9`. Built: a `Memory` row in Settings ›
System (under Device Info) opens a read-only screen in the row style: internal
RAM free, its largest free block, the lowest free since boot, PSRAM free and
its lowest, and the main task's stack margin, in KB or MB above 1 MB with
`regional::grouped` (one decimal in MB). `RuntimeMemorySnapshot` gained the
two `heap_caps_get_minimum_free_size` values (and logs them);
`MemoryUiState` stores the snapshot with remembered lows that never rise
again across snapshots. `main.rs` captures one at each status-bar refresh
(`update_board_snapshot`'s two sites: boot and the live refresh) and stores
it in `AppState`; the screen reads the stored one and the route joins
`uses_live_status`, so the numbers tick while it is open. Previews:
`settings-memory`. Tests: the size labels (KB with grouped digits, MB above
1 MB); the lowest value never goes up across snapshots (and takes lower
ones); Home › Settings › System › Memory through `AppState::apply`, hold BOOT
back to the sub-list. Docs: the guide's System section. Open: nothing known.
Device check: the preview and the screen on the panel with a book open
(PSRAM in use) and after a few minutes of uptime.

### D48: Go to a percent of the book

**Why.** A TXT book has no table of contents, so reaching its middle
takes hundreds of page turns.

**Change.**

- Reader Options › `Go to`: an option list of 0% to 90% in steps of 10,
  with the place in use marked. Choosing one opens the book there, through
  the same path as resuming a saved place.
- TXT: the byte offset at that percent, moved forward to the next line
  start. EPUB: the chapter holding that offset of the flattened text, at
  its page.
- The page label after the jump stays honest (`p. 41+` style, as a
  resumed place shows today).

**Tests:** 50% of a sample TXT opens within a page of the middle and at a
line start; 50% of a sample EPUB opens in the right chapter; the place
saves (with D44's batching).

**Status:** done on `side-tasks-9`. Built: Reader Options › `Go to` opens an
option list of 0% to 90% in steps of 10 (the `ReaderGoTo` screen, hold BOOT
back to Options) with the place in use marked (`IN USE` on its decade) and
the cursor starting there. Choosing one opens the book there through the
same path as resuming a saved place (`request_goto_percent` →
`request_open_book`): TXT at the byte offset at that percent, moved forward
to the next line start (`next_line_start`); EPUB at the chapter holding that
offset of the flattened text, at its page (`show_epub_offset` seeks). A
place ahead of the anchor cache opens at its own offset (the loader now
seeks by the canonical byte offset when the stored page index falls outside
the cached pages), and the label stays honest (`41+` style, with an
estimated page number where the index does not reach). The place moves in
memory at once (`move_place_to`) and saves through the batched writer.
Preview: `reader-go-to`. Tests: 50% of a sample TXT opens at a line start
within a page of the middle and keeps the `N+` label; 50% of a sample EPUB
opens in the third of four chapters; the place saves through the batch
(`STATE.TXT` holds the jump's offset). Open: nothing known. Device check:
the preview, a long TXT with a jump to 50%, and an EPUB with chapter jumps.

## Round 8 tasks (done in v0.10.4)

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

**Status:** done on `side-tasks-8`. Built: a `New alarm` row at the end of the Alarms list while fewer than `MAX_ALARMS` exist (a missing file shows an empty list with the row), opening the editor on a new alarm (07:00, on, weekdays, `Alarm N`); the editor last field chooses with ▲▼ between `Save alarm` and `Delete alarm`, ● runs the choice (Save adds or updates and writes, Delete removes a saved alarm and recomputes the next for the RTC, or only discards a new one); `AlarmEngine::load_or_empty` (the one `main.rs` call changed) so a missing `ALARMS.TXT` is an empty savable list while a file that does not parse still blocks saving. Previews `alarms-new` and `alarms-delete`. Tests: adds up to six and Select on the seventh edits instead; delete and save once; a missing file with a new alarm writes and reads back; a broken file is never overwritten; the next alarm after a delete. Open: nothing known.

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

**Status:** done on `side-tasks-8`. Built: `BibleUiState` keeps the translation list and the verse list, read in `with_root` (a `reads()` counter sees every card read); `bible_menu_items`, the menu keys and the Traducción picker use the kept lists, so drawing and the keys touch nothing; the kept translations are only those whose `index.tsv` reads (and not empty), with their `meta.txt` titles, so choosing one never lands on a broken folder book picker; `translation_count` and `has_verse_list` read the kept lists too. Tests: drawing the menu and the picker and pressing the menu keys add no reads and no loads (counted as D36 counts loads); a translation with a broken index is not listed. Open: nothing known.

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
