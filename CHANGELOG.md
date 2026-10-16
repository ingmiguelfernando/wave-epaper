# Wave changelog

Newest first. Versions and milestone commits follow the
[roadmap status table](docs/ROADMAP.md#status) and Git history.

## v0.12.2 — Voice note screens and the verse of the day

- **Recording a voice note** looks like the mockup: `● REC · Voice
  Notes`, the time large, a level meter with one bar a second, the
  `RECORDING` and `PAUSED` chips, and a box with the size so far, the free
  space and the hours it still holds. A short BOOT pauses and resumes.
- **The result screen:** the title large, the tabs as chips and the
  summary one size larger, with its to-dos as boxes under their heading.
- **Home rows** show the photo count, the Bible place (`Sal 23`), the game
  count and `2 apps`, as the mockup.
- **Verse of the day sleep screen:** Settings › Sleep screen can choose it
  when the card has `VERSES.TXT`. It shows the date, the day's verse in the
  reader's font and its reference, and changes at midnight with one
  wake-up a day.

## v0.12.1 — Race from the mockup

- **Race looks like the mockup.** Cars seen from above (outlines for the
  traffic, solid for yours, with speed streaks), pairs of cones, block
  rumble strips, the dashed outline where ▲ would take you, and
  `DISTANCE`, `BEST` and `NITRO` over the road with the charges as
  blocks. Zen counts `CARS PASSED`. Under the road, `SPEED 4 AT 3.0 KM`
  with its bar.
- **Crash screen:** a picture of the impact, the distance in large
  numerals, the stats in three columns and the rows like every list.
- **Games hub scrolls:** with five games or more, the selected card is
  always on screen (before, Tetris stayed hidden below the fourth card).
- Info boxes (AI hub, Games hub, Race, XiaoZhi) have the mockup's dashed
  frame.

## v0.12.0 — XiaoZhi

- **XiaoZhi voice chat** in Home › AI. Opening it calls XiaoZhi from
  xiaozhi.me over Wi-Fi. The first call shows a six-digit code: add the
  device at xiaozhi.me with it and the call goes on; the hub card then
  says `LINKED`. Speak, and the screen shows LISTEN › THINK › SPEAK, your
  question and the reply as text. ● interrupts, ▲▼ set the volume, a
  short BOOT turns the microphone off or on, and hold BOOT hangs up.
- **Device tools.** XiaoZhi can read the device status, set the volume and
  read a Bible passage from the SD card in the reader's translation.
- The call is the mockup's: the face with sound waves while XiaoZhi
  speaks, `On call · 02:14` and the battery in the status bar, the
  messages, the tool line and the volume bar. The AI hub's card has the
  mockup's face too.
- Under the hood: a port of xiaozhi-esp32's protocol to Rust, with ESP-IDF's
  WebSocket client and Espressif's Opus codec (16 kHz, 60 ms frames) as
  managed components; the call runs on the second core.

## v0.11.3 — Race

- **Race** in Games: a four-lane road that runs right to left in steps.
  ▲▼ change lanes to pass cars and cones; Classic steps once a second
  and faster each kilometre, with up to three nitro charges (●, three
  steps at once); Zen steps only on a key. Short BOOT pauses Classic. A
  crash shows the distance, the best, top speed, cars passed and nitro
  used, then Race again, Mode or Back to Games. Each mode keeps its best
  distance in `RECORDS.TXT`, and the Games hub card shows both.
- Install the `RACE` folder from `examples/sd-card/RUSTMIX/APPS` with the
  other games.

## v0.11.2 — Reader screens, Library first, Memory, Go to

- **Library from Home.** Home's Library row opens the books: RECENT when a
  book was read, else ALL. The three-row landing is gone; hold BOOT on a
  page returns to the books.
- **Reader screens** (Options, Reading Preferences, contents, bookmarks,
  Opening Book) and Settings › Reading and Display have the black status
  bar and the label-and-value rows of the other settings; long lists
  scroll. Reader Options shows bookmark and contents counts.
- **Fewer card writes while reading.** A page turn keeps the place in
  memory; it is written after two minutes, when you leave the page and
  before sleep.
- **Go to** in Reader Options opens the book at 0% to 90%.
- **Settings › System › Memory:** free RAM, the largest block, the lowest
  since boot, PSRAM and the main task's stack margin.
- Rows without a description centre their title on the value's line.

## v0.11.1 — Voice note volume

- **Volume while a note plays.** ▲▼ turn the speaker up or down in steps
  of 5% while a note plays, and the note's screen shows the volume. Before,
  the volume was only in Settings › System › Audio.
- **Mockup:** a Race game for e-paper, playing and crash screens; it is
  the next delegated round.

## v0.11.0 — Voice notes to the AI providers

- **Transcripts and summaries from the device.** With both providers in
  `AI.TXT` and their keys saved, a note goes to the transcription provider
  and its transcript to the summary provider. The list and the AI hub show
  each note's state, the result screen fills in, and the note takes the
  summary's title. `process=online` queues each new recording; otherwise
  choose Transcribe and summarize in the note's details.
- **Queue.** One note at a time, oldest first, about 8 seconds after the
  last key press, with Wi-Fi on until it is done. Notes longer than 10
  minutes go in parts. A lost connection or a busy provider tries again
  later; a refused request shows the provider's reason and offers Retry.
  Summarize again makes a new summary from the saved transcript.
- **API keys** are typed in the Wi-Fi portal's new AI keys section and kept
  in encrypted flash, never on the SD card. The first boot of this version
  burns a random key into the chip's eFuse block 5 for that encryption;
  this is permanent and does not change USB flashing.
- AI.TXT and the note list reload when the transfer portal stops.

## v0.10.5 — Display numerals and weather icons

- **Sleep clock and temperatures** are drawn in Inter ExtraBold, as in the
  mockup, instead of seven-segment digits. Digits keep equal widths, so
  the clock does not shift as the minutes change, and a negative
  temperature shows a true minus sign.
- **Weather icons** on the Weather screen and the sleep screens are drawn
  from the mockup's symbols at any size, so the large ones are smooth
  instead of stepped. Clear and partly cloudy nights show the moon on the
  sleep screens too.

## v0.10.4 — Chapter titles, Settings › Reading, new alarms

- **Reader page.** An EPUB chapter opens with its title and a rule, and the
  title is no longer repeated in the text. The bar adds the minutes left in
  the chapter at your own pace (`Ch. 8 · 12% · 9 min left`) once twenty
  pages are recorded this week. The clock, battery and page label are drawn
  in a size whose `%` is whole. Books paginate again on their first open.
- **Library.** Rows show the author of an EPUB, a check for a finished
  book (reaching the last page finishes it) and the percent of EPUBs too.
  Home's Continue card shows an EPUB's percent.
- **Settings › Reading**: book font, size, alignment, margins (narrow,
  normal, wide), hyphenation (Auto ES/EN or off), progress and reading
  stats (off records nothing), without opening a book.
- **Alarms.** A New alarm row adds an alarm and the editor can delete one;
  both are saved to `ALARMS.TXT`, which the first alarm creates.
- **Bible.** The reading menu no longer reads the card on every key, and a
  translation folder with a broken index is left out.
- **Fix.** Hold BOOT in an option list of Settings › Reading or Settings ›
  AI closes the list before leaving the screen.

## v0.10.3 — Reader page, Library, Bible translations, hold to repeat

- **Reader page** as in the mockup: a thin header with the title, time and
  battery; the text fills the page down to the shared bottom bar, which
  shows `p. 41 · 12%` (TXT) or `Ch. 8 · 12%` (EPUB), the place in the book.
  A bookmarked page hangs a ribbon at the top right. Landscape too. Books
  paginate again on their first open, since a page now holds more lines.
- **Library**: the status bar with the book count, tab chips RECENT, ALL,
  BOOKMARKS and FILES (short BOOT moves between them), and book rows with
  the title, the format, a bar with the percent read or `new`. FILES shows
  sizes. ● opens a book at its place. Long lists scroll.
- **Bible**: Menu › Traducción switches between the translations on the
  card and keeps the place when the book is there. Verses break long words
  with a hyphen in the translation's language. A chapter is read from the
  card once, not on every page turn.
- **Hold ▲ or ▼** to repeat on lists, grids and option lists: after half a
  second, then five times a second. Reading pages and games take one press
  at a time.
- **Alarms**: a saved edit is written to `/RUSTMIX/ALARMS.TXT` and survives
  a reboot.

## v0.10.2 — Bible navigation and verse of the day

- **Bible.** Hold BOOT in the chapter grid goes back to the book list; it
  kept showing the chapters. Short BOOT now does what the bars say: the next
  section in the book list, ten chapters on in the grid.
- **Verse of the day.** With `/RUSTMIX/BIBLE/VERSES.TXT` and the clock set,
  the reading menu offers `Versículo del día`, which opens today's verse at
  its page.
- **Voice Notes.** On a note's result, ● opens the note's actions (play,
  rename, export, delete).

## v0.10.1 — Bible reader, Tetris Classic and AI settings

- **Bible**, in Spanish. Home › Bible opens where you stopped, or the book
  picker the first time: eight sections with a thumb index (`GÉN JOS JOB ISA
  OSE MAT ROM HEB`), then the chapter grid. ▼ and ▲ turn pages across
  chapters and books, short BOOT opens the chapter grid and ● a menu (`Ir a
  libro`, `Ir a capítulo`). The place is saved when you leave and before
  sleep. The text is the folloup script's layout in `/RUSTMIX/BIBLE/<ABBR>/`
  ([SD card setup](docs/SD_CARD_SETUP.md)).
- **Tetris** opens on a mode list: Zen (no gravity) or Classic (one row a
  second), each with its own best; the Games card shows both. Copy the new
  `APPS/TETRIS/MAIN.LUA`; an old card still starts Zen. Classic pauses while
  the screen is powered down.
- **Settings › AI** shows the transcription and summary providers of
  `/RUSTMIX/AI.TXT` (see `AI.TXT.example`); Language, Style and Process
  change on the device. Settings reads `Ready` or `Not set up`.
- **Voice Notes** read each note's AI record (`VOICE###.AI`): the list and
  the AI hub show `SUMMARY`, `TRANSCRIBING…`, `QUEUED` or `FAILED`, and a
  summarized note opens on its summary, transcript and audio tabs. The device
  does not send notes for processing yet.
- Sudoku draws your digits in Literata, about the size of the given ones.

## v0.10.0 — Clock and weather sleep screens

- **Sleep screen modes.** Settings › Sleep screen lists five modes, each with
  what it shows and its estimated battery cost a day: Photo (as before),
  Clock & date, Weather, Clock + weather, and Verse of the day (not ready
  yet, marked `SOON`). Below the modes are the options of the one in use:
  Source, Order and Fit for Photo; Refresh (every minute or every 5 minutes)
  for the clock modes. A short BOOT press previews the chosen mode.
- **Live while asleep.** The clock changes on the minute without a flash; a
  full refresh every 30 minutes clears ghosting. The panel keeps its memory
  (its rail stays on) in these modes; Photo still turns the rail off. The
  weather modes turn Wi-Fi on for each weather update and redraw after it;
  a forecast older than six hours is marked `stale`. Until the first update
  the Weather mode shows the clock.
- **Sudoku saves itself** each time you place or erase a number, not only
  when you leave with hold BOOT, so a restart or a flat battery no longer
  loses the game. Solving deletes the save and records the best time at once.
  A game left open behind an alarm is now closed and saved when a game opens
  again; before, reopening dropped it.
- Settings shows the sleep mode: `Photo · 2 starred`, `Clock · 1 min`,
  `Weather`, `Clock + weather`.

## v0.9.7 — Hubs, Sudoku saves and reading stats

- **Sudoku** opens on a start list: continue the saved game, or a new Easy,
  Medium or Hard puzzle made on the device (a different one each time). The
  title bar shows the difficulty and the play time. Hold BOOT saves the game
  to `/RUSTMIX/GAMES/SUDOKU.TXT`; solving it deletes the save and keeps the
  best time per difficulty (`Solved in 12:41 · best 11:02`). Copy the new
  `APPS/SUDOKU/MAIN.LUA` from the samples; an old card's puzzle still works
  and shows as `SD puzzle`.
- **Games** is a hub with one card per SD game: Sudoku's saved game and best
  time, Tetris' best score. Hold BOOT in a game returns to the hub.
- **Settings** is one page of seven groups, each with its current value;
  Clock & alarms and System open short lists.
- **Reading Stats** opens from Home and records time and pages while a book
  page is open, by local date, saved at most every five minutes, when the
  Reader closes and before sleep. Home shows the streak, and the Continue
  card the minutes read today. Without a set clock the screen says so.
- **AI** is a hub with XiaoZhi (SOON), Voice Notes and the three newest
  notes; a short BOOT starts a recording right away.

## v0.9.6 — Power key read on its own line

- The Power key is now read on GPIO1, the board's `PWR_OUT` line, which goes
  high while the key is held. Until now only the PMIC's key interrupt was
  used, and on the test board it reported no presses, so Power did nothing.
  A tap opens the maintenance menu, holding it for a second starts sleep, and
  a press wakes the device.
- The PMIC key interrupt stays as a backup until GPIO1 shows its first press.
- A BOOT press made while Power is held is ignored, so Power never acts as
  Back.
- The serial log shows `power-key-gpio down=true` and `down=false` for each
  press, and the source of every Power event: `gpio1`, `axp2101-pek` or
  `auto-sleep`.

## v0.9.5 — Crisper photos, sleep on USB, memory for games

- Photos, thumbnails and sleep BMPs use Atkinson dithering: whites and
  blacks stay clean instead of filling with dots. Every photo is prepared
  again once after the update.
- With USB power, sleep mode no longer puts the CPU in light sleep, so the USB
  serial console stays connected while the sleep picture shows.
- Weather requests use a 1 KB transmit buffer, so the long request line no
  longer logs `HTTP_HEADER: Buffer length is small to fit all the headers`.
- Opening a game waits up to 3 s for free internal memory while a photo is
  still being prepared; if memory stays short, the message says to try
  again. TLS buffers moved to PSRAM.

## v0.9.4 — Power key and Photos folder fixes

- A short Power press after a minute without keys opened the menu on the
  powered-down panel, so nothing appeared. The panel now wakes first.
- One PMIC read error turned the Power key off until the next reboot, and
  auto-sleep with it. The key setup is now retried after 1 s, doubling up to
  30 s; auto-sleep also runs meanwhile when Wake keys is Any key.
- Holding Power for one second starts sleep: the PMIC long-press time (1 to
  2.5 s) is now set at boot, and the log shows the previous value.
- Device Info shows the Power key status and its last error (page 3), and
  why the device last restarted and the real milestone (page 1).
- Photos are read from `/RUSTMIX/PHOTOS`, which the Wi-Fi transfer reaches
  and the SD installer creates. `/PHOTOS` at the card root is no longer read.

## v0.9.3 — Sudoku three-step entry and delegated tasks (pull request #4)

- D18 (`5dd70ff`): Sudoku enters a value in three steps, as in the mockup:
  pick a row, then a cell, then a number or erase. At merge, the screen got
  the mockup's title bar, row frame, inverted cell with a number preview and
  boxed number strip; the old row outline was invisible.
- D19 (`5a57d3f`): the Tetris best score survives closing the game and
  rebooting, in `/RUSTMIX/GAMES/RECORDS.TXT`, written once on exit. At merge,
  the first save creates the `GAMES` folder.
- D20 (`c3e25e5`): Bible book and chapter pickers, drawn in previews only;
  Phase 5 adds the routes and the reading view.
- D21 (`7d67ad8`): `json_lite.rs`, a small JSON reader without floats.
- D22 (`1e2fd22`): OpenAI-compatible transcription and summary requests and
  responses. At merge, the transcript is escaped once instead of twice.
- D23 (`bd7b52c`): XiaoZhi WebSocket messages and binary audio frames.

## v0.9.2 — Tetris Zen, shared bottom bar and delegated tasks (pull request #3)

- Every screen ends with the same bottom bar: key caps and short labels as in
  the mockup, replacing the old uppercase footer text. Labels drop to the
  Detail size when they would not fit. SD games get the bar too, with hints
  for the game's mode; Sudoku and Minesweeper no longer draw their own
  footer, and Sudoku's hints now refresh when the mode changes.
- D12 (`132a16f`): `games/tetris.rs`, the Tetris rules: 10 × 20 board, 7-bag,
  wall kicks, line scores and levels. At merge, Rotate was made clockwise as
  specified.
- D13 (`53877be`, `7809768`, `cf9b1e2`): Tetris Zen as an SD app
  (`APPS/TETRIS`): no gravity, ▲▼ move, ● rotate, BOOT drops. Next piece,
  ghost, score, lines and best, drawn like the mockup with bounded partial
  refreshes and a full refresh every 20 pieces. At merge, the SD installer
  learned to copy the app.
- D14 (`fbbad98`): the Reading Stats screen (tiles, minutes per day, 12-week
  heatmap, year totals, current book). Drawn in previews only; Phase 5 adds
  the route.
- D15 (`075e512`): `sd_file.rs` writes settings files through `.TMP` and
  `.BAK`, so a power cut cannot leave them empty. At merge, a `.BAK` left as
  the only copy is restored before the next save.
- D16 (`4624eba`): `civil_date.rs`, one module for calendar dates.
- D17 (`248826b`): 88 stale `rustmix-wave=*-ready` boot log lines removed.

## v0.9.1 — Delegated tasks (pull request #2)

- D6 (`b1fa110`): the `%` glyph at Inter Standard Detail cannot be fixed from
  `fonts.toml` alone; findings recorded, fonts unchanged.
- D7 (`7b22583`): clock and weather sleep layouts closer to the mockup
  (centered groups, rule above the days, rain in Body size). Still not
  selectable.
- D8 (`e03d6e9`): Audio details describe the bidirectional I2S link and the
  Voice Notes input.
- D9 (`dec9ee4`): remove the unused upstream ELF build, flash and release
  scripts.
- D10 (`3d2aa1e`): `bible.rs`, the Bible text library: book list, streamed
  chapter reads, translations and verse of the day. Not wired yet.
- D11 (`e7900ce`): `reading_stats.rs`, daily and per-book reading totals,
  week chart, streak and a key-driven reading clock. Not wired yet. At merge,
  saves were made FAT-safe with a `.BAK` swap.

## v0.9.0 — Weather (Phase 4)

Milestone `e9bffc8`.

- Weather follows the mockup: big icon and temperature, today's high and
  low, feels like, humidity, wind and rain chips, the next hours, four days
  and an `Updated · next · Open-Meteo` line. Clear and partly cloudy nights
  show a moon.
- Down opens twelve hourly rows with the place, time zone and last error;
  Select updates now; a short BOOT press switches °C/°F (km/h or mph).
- Settings › Weather: service On/Off, update interval (30 min to 6 h, or
  Manual), units and Show on Home, saved in `WEATHER.TXT`. Off makes no
  weather requests; Manual makes none on its own.
- Home hides the weather when the service is off or Show on Home is No.
- The forecast request adds hourly data, `is_day` and five days.

## v0.8.1 — Delegated tasks (pull request #1)

- D1 (`1773deb`, follow-up `27bd2bf`): Display and Reader preference option
	lists; selecting the value already in use closes without unnecessary save,
	ghost clearing or repagination.
- D5 (`dc5baec`): add drawing-only clock and weather sleep layouts, large
	digits, scaled icons, tests and previews. No runtime sleep-mode selection.
- D2 (`2e30703`): remove the unused BLE remote build and its optional runtime
	path, docs and helpers; keep Wi-Fi bursts.
- D3 (`cd9dd3f`): remove IMU tilt games and Lua motion input; retain Motion
	diagnostics and button-driven Hello Grid, Minesweeper and Sudoku.
- D4 (`1c6495b`): refresh the practical guide, known issues, release smoke
	checklist and documentation index; remove obsolete upstream release files.

## v0.8.0 — Photos (Phase 3a)

Milestone `f36eeb7`; release commit `4f3de3a`.

- Home › Photos: JPEG gallery from `/PHOTOS` with thumbnails, a full-screen
	viewer and actions (sleep set, use only this photo, delete).
- Photos are decoded once in the background on the second core, upright from
	EXIF, dithered and cached in `/RUSTMIX/CACHE/PHOTOS/`.
- Settings › Sleep screen: source (starred photos or the sleep folder), order
	and fit, with a BOOT preview; sleep shows starred photos.
- Holding BOOT to close an option list now redraws the screen.

## v0.7.0 — Power settings (Phase 1c)

Milestone `d27dced`; follow-ups `fe0b9da`, `8eda12a`, `72e9554`.

- Settings › Power: auto-sleep and wake-key lists, battery history and sleep
	diagnostics; persist power preferences and sample battery every 15 minutes.
- Accept portrait/landscape sleep BMPs, show actionable fallback-card reasons,
	and improve card text. Owner device validation remains pending in the roadmap.

## v0.6.0 — Idle power and radio bursts (Phase 1b)

Milestone `038269f`.

- Light-sleep between presses on battery; keep awake on USB for flashing.
- Use short Wi-Fi bursts for weather/NTP; turn off IMU and codec when idle.

## v0.5.0 — Sleep mode (Phase 1a)

Milestone `f78cb18`.

- Light-sleep while the sleep image is displayed; auto-sleep after ten idle
	minutes; preserve wake keys and RTC alarm handling.

## v0.4.0 — Reader layout (Phase 2d)

Milestone `7b3b112`.

- Wrap by measured pixel width, add Spanish/English hyphenation, and open books
	in one tick. Keep logical reading anchors across layout changes.

## v0.3.0 — Long EPUBs (Phase 2c)

Milestone `55668f2`.

- Load EPUB text one chapter at a time instead of retaining the whole book.

## v0.2.1 — Reader reliability (Phase 2b)

Milestone `23caaa7`.

- Fix EPUB watchdog resets and hide macOS `._` files from the library.

## v0.2.0 — Font generation (Phase 2a)

Milestone `ef529b0`; typography follow-ups include `1bab141`, `54fb828`,
`553d41e` and `6d8d2a5`.

- Generate bitmap font atlases with Latin-1 and punctuation coverage; tune
	rendering and centralize font configuration.

## v0.1.0 — Wave shell (Phase 0)

Milestone `72334ad`; follow-up `0066e71`.

- Add the Wave Home/menu layout and SOON placeholders; support long SD names,
	New Zealand time and metric units; rename the fork.

## Baseline — upstream v1.2.0

Milestone `a67ceb5`: build the inherited Wi-Fi firmware with GitHub Actions.
Wave version numbering starts at v0.1.0 after this baseline.

Older history: [upstream Rustmix Wave](https://github.com/aimindseye/rustmix-wave).
