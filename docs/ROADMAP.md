# Wave roadmap

Status of the firmware and technical specs for the work still to do. How the
current firmware is built is in [architecture.md](architecture.md); small tasks
handed to a second developer are in [DELEGATED_TASKS.md](DELEGATED_TASKS.md).
The UI follows `mockups/index.html`.

Last updated: 2026-10-19, firmware v0.16.0.

## Status

| Step | Version | Commit | State |
|---|---|---|---|
| Baseline: upstream Rustmix Wave v1.2.0 built by GitHub Actions | 1.2.0 | a67ceb5 | Done |
| Phase 0: Home and menus from the mockup, long SD names, rename | 0.1.0 | 72334ad | Done |
| Phase 2a: font generator, Latin-1 glyphs | 0.2.0 | ef529b0 | Done |
| Phase 2b: EPUB watchdog fix, macOS `._` files hidden | 0.2.1 | 23caaa7 | Done |
| Phase 2c: EPUBs of any length, one chapter at a time | 0.3.0 | 55668f2 | Done |
| Phase 2d: pixel-width wrapping, ES/EN hyphenation | 0.4.0 | 7b3b112 | Done |
| Phase 1a: light sleep while the sleep picture shows, auto-sleep | 0.5.0 | f78cb18 | Done |
| Phase 1b: idle light sleep, Wi-Fi bursts, IMU and codec off when idle | 0.6.0 | 038269f | Done |
| Phase 1c: Settings › Power, wake keys, battery log, sleep-picture fixes | 0.7.0 | d27dced | Done, waiting for device test |
| Phase 3a: Photos app, starred photos as sleep screens | 0.8.0 | f36eeb7 | Done, waiting for device test |
| Phase 3b: sleep screen modes (clock, weather) | 0.10.0 | 55c703b | Done, waiting for device test; display numerals and mockup weather icons in 0.10.5 (tested on the device), verse of the day mode in 0.12.2 |
| Phase 4: Weather app and Settings › Weather | 0.9.0 | e9bffc8 | Done, tested on the device |
| Phase 5: Bible and Reading Stats | | | Done (0.19.2); history: Reading Stats in 0.9.7, Bible reader in 0.10.1, verse of the day in 0.10.2; translation switch and hyphenation in 0.10.3; the verse as a sleep screen and the place on Home in 0.12.2 |
| Phase 6: Games (Sudoku, Tetris, Race, Dice) | | | Done (0.19.2); history: Sudoku saves and the Games hub in 0.9.7, Sudoku autosave in 0.10.0, Tetris Classic in 0.10.1, Race in 0.11.3, Race from the mockup and a scrolling hub in 0.12.1 |
| Phase 7: AI (Voice Notes with OpenAI-compatible providers, XiaoZhi) | | | Done (0.19.2) except echo cancellation in XiaoZhi and notes recorded as Opus (both open, see Phase 7); history: AI hub in 0.9.7, Settings › AI and note results in 0.10.1, transcription and summaries from the device with encrypted keys in 0.11.0, XiaoZhi voice chat in 0.12.0, the mockup's recording and result screens in 0.12.2; Opus voice notes open (the Ogg Opus file in 0.12.5) |
| Phase 8: OTA updates | 0.13.0, 0.14.0 | PR #17, PR #19 | Done, tested on the device: updated from 0.14.0 to 0.15.0 over Wi-Fi (the smoke test's update checks are not ticked yet) |
| Delegated tasks D1 to D5: option lists, BLE remote and tilt games removed, guides, sleep layouts | 0.8.1 | PR #1 | Done; D2, D3 and D5 tested on the device, D1 partly (Books and Display checks open) |
| Delegated tasks D6 to D11: sleep layout polish, audio details, old scripts removed, Bible and reading stats data (D6 `%` glyph blocked) | 0.9.1 | PR #2 | Done, waiting for device test |
| Delegated tasks D12 to D17: Tetris engine and Zen SD app, Reading Stats screen, safe SD writes, date module, boot log cleanup | 0.9.2 | PR #3 | Done; D15 settings files tested on the device |
| Shared bottom bar (key caps) on every screen, games included | 0.9.2 | 4574cfc | Done, waiting for device test |
| Delegated tasks D18 to D23: Sudoku three-step entry, game records, Bible navigation screens, JSON reader, OpenAI-compatible and XiaoZhi messages | 0.9.3 | PR #4 | Done, waiting for device test |
| Fixes: Power key menu on a sleeping panel, key retries, 1 s hold, Device Info diagnostics, photos in `/RUSTMIX/PHOTOS` | 0.9.4 | fdb1917 | Done, waiting for device test |
| Fixes: Atkinson photos, no sleep-mode light sleep on USB, weather request buffer, worker memory wait | 0.9.5 | 83d0313 | Done, waiting for device test |
| Fix: Power key read on GPIO1 (`PWR_OUT`), PMIC interrupt as backup | 0.9.6 | e0fc071 | Done, tested on the device |
| Delegated tasks D24 to D28: Sudoku difficulty, timer and saves, Games hub, Settings groups, Reading Stats wiring, AI hub | 0.9.7 | PR #5 | Done, waiting for device test |
| Delegated tasks D29 to D33: Bible data and reading view, Tetris Classic, Settings › AI, Voice Notes results | 0.10.1 | side-tasks-6 | Done, waiting for device test; D33 device processing open |
| Delegated tasks D34 to D38: Reader page and Library from the mockup, Bible translations and hyphenation, hold to repeat, saved alarm edits | 0.10.3 | PR #6 | Done, tested on the device |
| Delegated tasks D39 to D43: EPUB chapter titles and minutes left, Library authors and finished books, Settings › Reading, add and delete alarms, Bible menu without card reads | 0.10.4 | PR #7 | Done, tested on the device |
| Delegated tasks D44 to D48: reading place saved in batches, Library straight from Home, Reader screens in the new style, Settings › System › Memory, Go to a percent | 0.11.2 | PR #8 | Done, tested on the device |
| Delegated tasks D49 to D50: Race engine and Race as an SD game | 0.11.3 | PR #9 | Done, tested on the device; the mockup's look and a scrolling Games hub are Round 11 (D51 to D53) |
| XiaoZhi voice chat: linking, WebSocket call with Opus, device tools (status, volume, Bible), the mockup's call screen | 0.12.0 | 4609d8c | Done, waiting for device test |
| Delegated tasks D51 to D53: the Games hub scrolls, Race's playing and crash screens from the mockup | 0.12.1 | PR #10 | Done, tested on the device |
| Delegated tasks D54 to D57: voice note recording and result screens from the mockup, Home row details, verse of the day sleep screen | 0.12.2 | PR #11 | Done, tested on the device |
| Fix: games could not start for lack of internal RAM (the ended photo thread kept its stack, thumbnails read at boot) | 0.12.3 | 22bc5cd | Done, tested on the device |
| Fix: a new Sudoku overflowed the main task's stack (recursive generator); main task stack 24 KB | 0.12.4 | 47c21ab | Done, tested on the device |
| Delegated tasks D58 to D61: the forecast kept across a restart, the verse of the day marked on its page, Bible reading in Reading Stats, Ogg Opus files | 0.12.5 | PR #12 | Done, tested on the device except one check |
| Delegated tasks D62 to D64: the upload form and parts for Ogg Opus, previews of the cached forecast and the verse mark | | PR #13 | Done, merged without a release (library and previews only) |
| Delegated tasks D65 to D67: books finished this year, Ogg parts streamed from the file, voice note text for Ogg | 0.12.6 | PR #14 | Done, waiting for device test |
| Delegated tasks D68 to D70: `%` at the Detail size, Voice Notes text for both formats, a deep sleep study; D71 the Dice mockup | 0.12.7 | PR #15 | Done; the `%` at the small size tested on the device |
| Delegated tasks D72 to D74: the Dice engine, its screens and previews, the dice rattle sound (libraries and previews only) | | PR #16 | Done, merged without a release; wiring is the main line's |
| Phase 8, slice 1: two OTA app slots, rollback, the app image as a release asset, `ota.rs` decisions | 0.13.0 | PR #17 | Done, tested on the device |
| Phase 8, slice 2: Check for updates in Settings › System, the download, SHA-256 and slot switch, restart | 0.14.0 | | Done, tested on the device |
| Delegated tasks D75 to D77: the Games hub with Race and Dice, Games screens against the mockup, a Dice session for the wiring | | PR #18 | Done, merged without a release; wiring is the main line's |
| Dice wired to the device: keys, shake, rattle, saved options | 0.15.0 | PR #20 | Done, tested on the device |
| Opus uploads: voice notes go to the providers as Ogg Opus 24 kbps (WAV fallback) | 0.16.0 | | Done, waiting for device test |
| Delegated tasks D78 to D80: the whole mockup against the previews, by section (Games leftovers, Home, Photos, Reading, Bible, Weather, AI, sleep screens) | | PR #21 | Done, merged without a release |
| Delegated tasks D81 to D83: Sudoku's bold digits, XiaoZhi's call screen and the AI hub's Wi-Fi mark, the Settings figures | | PR #23 | Done, merged without a release |
| XiaoZhi on your own server (`XIAOZHI.TXT`) | 0.17.0 | PR #24 | Done; the official and own servers tested on the device, the bad-address message open |
| Several Wi-Fi networks, add one from a phone through the device's own Wi-Fi | 0.18.0 | PR #25 | Done, waiting for device test |
| Delegated tasks D84 to D86: ExtraBold strikes, XiaoZhi message face and chips, Library/Reader/Voice Notes audits | 0.18.1 | PR #26 | Done, waiting for device test |
| Minesweeper and Hello Grid removed (firmware and SD samples) | 0.18.2 | PR #27 | Done, merged into 0.19.0 |
| Delegated tasks D87 to D89: Wi-Fi parts tested on the host, Network screens at every size, forget a network (screen only); XiaoZhi bad-file fix; hotspot hints | 0.19.0 | PR #28 | Done, waiting for device test; forgetting a network is wired in 0.19.2 |
| Forget a saved network on the device (Network › Known networks) | 0.19.2 | | Done, waiting for device test |
| Photos: Add photos in the Wi-Fi portal (browser converts any format), PNG on the device | 0.19.0 | PR #28 | Done, tested on the device |

Every phase ends with host tests, screen previews, a green firmware build, a
version bump and a test on the device by the owner.

## Phase 3a: Photos (v0.8.0)

Done in v0.8.0; this section stays as the reference for the Photos code.

Goal: browse photos from the SD card and choose the ones shown while the device
sleeps.

### Files

- Photos: `/RUSTMIX/PHOTOS/` since v0.9.4 (`/PHOTOS/` at the SD root before).
  Sub-folders are ignored. Up to 500 files
  are listed, newest file date first.
- Formats: JPEG (`.jpg`, `.jpeg`). Baseline JPEGs of any size; progressive
  JPEGs up to 1 megapixel, because a progressive decode keeps every coefficient
  in memory. Rejected files show the reason, for example "is a progressive
  JPEG; save it as a standard JPEG". BMP and PNG photos are not part of 3a; BMPs
  keep working from the sleep folder.
- EXIF orientation (tag 0x0112) is applied, so phone photos come out upright.
  Values 1, 3, 6 and 8 rotate; mirrored values use the nearest rotation.
- Cache: `/RUSTMIX/CACHE/PHOTOS/<key>.PIC`, one file per photo.
  - `key`: 8 hex digits of FNV-1a 32 over `name|size|mtime`, with the offset
    basis XORed with the pipeline version (2 since v0.9.5), so a new pipeline
    prepares every photo again.
  - Content: magic `WPC1`, version, source width and height after rotation, a
    144 × 216 thumbnail (1 bit, 18 bytes per row), then two 48,000-byte frames
    in native panel layout, "fill" and "whole".
  - About 100 KB per photo. Entries whose photo is gone are deleted during a
    scan.
- Starred photos: `/RUSTMIX/STARRED.TXT`, one file name per line.
- Sleep screen settings: `/RUSTMIX/SLEEPSCREEN.TXT`, `key=value` lines:
  - `source`: `starred` (default) or `folder` (the existing `/RUSTMIX/SLEEP/`);
  - `order`: `shuffle` (default) or `in-order`;
  - `fit`: `fill` (crop, default) or `whole` (whole photo, white bars).

### Decoding

- Crate: `jpeg-decoder` with `default-features = false` (no rayon threads).
- Decode at the smallest DCT scale (1/1, 1/2, 1/4, 1/8) that still covers the
  target size. A 12 MP photo decodes at 1/4, about 1008 × 756 RGB, roughly
  3.5 MB of PSRAM at the peak. A scaled decode over 4 MB is refused.
- RGB to grey (77/150/29 weights), area-average resampling to each target, a
  1st to 99th percentile contrast stretch, then Atkinson dithering in
  serpentine order (`dither.rs`, shared with `sleep_images.rs`). Floyd–Steinberg
  until v0.9.4 looked grainy on the panel.
- Decoding runs on a background worker thread (`photos/worker.rs`, 48 KB
  stack) pinned to core 1, so the main loop on core 0 keeps reading the keys.
  It receives "photo ready" messages and redraws the visible page. Light sleep
  waits while the worker is busy, so it never stops in the middle of an SD
  write. A 12 MP photo takes about 2 to 5 s the first time.
- Nothing is decoded at sleep entry; the sleep screen only reads cached frames.

### UI

- **Home › Photos** (first entry, the SOON badge goes away).
  - Grid of 3 × 2 thumbnails with the file name under each.
  - The selected thumbnail has a thick frame; starred ones carry a star badge
    (drawn shape, the fonts have no ★).
  - Status row: `48 photos`, `12 starred`, page `1/8`. Info line:
    `IMG_0412.jpg · 4032×3024 · Sep 28, 2026`.
  - Keys: ▲▼ previous or next photo, crossing pages; ● opens the viewer; BOOT
    short press stars or unstars; hold BOOT goes back.
  - Thumbnails not ready yet show a placeholder box; the page redraws as they
    arrive. The visible page is prepared first, then the rest.
  - Empty folder: a message explains where to put photos.
- **Viewer.**
  - Full-screen photo using the Fit setting, with a top overlay
    `1 / 48 · IMG_0412.jpg` and a star when starred.
  - ▲▼ previous or next photo (global refresh); BOOT short press stars;
    ● opens the actions.
- **Actions.**
  - Add to sleep set, or Remove from sleep set.
  - Use only this photo: unstar the others and star this one.
  - Delete photo: asks again (● deletes, hold BOOT cancels), then removes the
    file, its cache entry and its star once the worker is idle.
  - The size and date are on the gallery's info line rather than in an action.
- **Settings › Sleep screen** (new, under Settings): Source, Order and Fit as
  option lists (`widgets/option_list.rs`). BOOT short press previews the next
  sleep picture until a key is pressed; the preview does not move the folder's
  order.

### Sleep entry

1. With source `starred`, pick a starred photo that has a cache entry: shuffle
   without repeating the last one, or the next one in order. Load its 48 KB
   frame for the chosen fit.
2. If there is none, use the sleep folder as today.
3. If that also fails, show the sleep card with the reason, for example "No
   starred photos yet. Star photos in Photos."

### Acceptance

- Host tests:
  - EXIF orientation parser;
  - cache file round trip and stale-entry cleanup;
  - star list parse and save;
  - sleep selection (shuffle, in order, skips uncached photos);
  - resampling sizes;
  - decoding small JPEGs built by the tests (`jpeg-encoder`, a dev
    dependency), including rotated and progressive ones. Image files are not
    committed.
- Previews: grid with placeholders and thumbnails, viewer, actions,
  Settings › Sleep screen.
- On the device:
  - 12 MP phone photos show upright;
  - thumbnails are built once;
  - starred photos rotate at each sleep;
  - entering sleep stays as fast as with BMPs.

## Phase 3b: sleep screen modes

Settings › Sleep screen gains a mode list, each with its estimated cost from
the mockup:

| Mode | What it shows | Cost estimate | Refresh |
|---|---|---|---|
| Photo | Starred photo (3a) | No wake-ups | At sleep entry |
| Clock & date | Large time, date, optional weather line, battery | about 13 mAh/day at 1 min | Every 1 or 5 min (setting) |
| Weather | Place, updated time, now, three days | about 2 mAh/day | After each weather update (default 2 h) |
| Clock + weather | Clock and weather line | about 5 mAh/day | Clock 5 min, weather 2 h |
| Verse of the day | Verse from the Bible on the SD | 1 wake-up per day | At local midnight (0.12.2) |

- Layouts come from tasks D5 and D7 (`src/app/screens/sleep_screens.rs`),
  already close to the mockup: the clock's weather icon and summary centered
  as one group, a rule above the three days, rain chances in Body size.
- **Built in v0.10.0.** `SleepMode` and `ClockRefresh` in `sleep_screen.rs`
  (keys `mode=` and `clock_refresh=` in `SLEEPSCREEN.TXT`);
  `render_sleep_mode` draws the mode from `AppState`. In a live mode the
  panel sleeps with `Epaper397::sleep_keeping_ram`: deep sleep mode 1 with
  ALDO3 on, so the controller keeps both RAM planes and the hardware reset in
  `show_partial_fullscreen` wakes it for a partial update. The loop redraws
  on the minute (`clock_redraw_wait`, from the RTC seconds) and does a
  global refresh when 30 minutes have passed since the last one
  (`SLEEP_GLOBAL_REFRESH`). Light sleep while asleep lasts until the next
  redraw or weather burst, and not while the radio is on.
- **Clock.** While asleep the loop already wakes at least every 60 s. When a
  minute is due:
  1. wake the panel controller (its rail stays on);
  2. draw the screen and do a partial refresh;
  3. put the controller back to deep sleep.

  Do a global refresh every 30 minutes against ghosting. Check the real cost
  with the battery log.
- **Weather.** Needs the weather service on (`AppState::weather_enabled`,
  Settings › Weather).
  - While asleep, schedule Wi-Fi bursts at the weather interval and redraw
    after each successful update.
  - Data older than 6 h is labeled "stale".
- RTC alarms and wake keys keep working in every mode.
- Acceptance: the battery log over a night roughly matches each mode's
  estimate, with no visible ghosting after hours on the clock.

## Phase 4: Weather app (v0.9.0)

Done in v0.9.0; this section stays as the reference for the Weather code.

- **Settings › Weather** (`screens/weather_settings.rs`), option lists like
  Settings › Sleep screen:
  - Weather service On or Off.
  - Update every 30 min, 1 h, 2 h (default), 6 h, or Manual.
  - Units: °C · km/h or °F · mph.
  - Show on Home: Yes or No.
  - Location and provider lines, then a note with the battery cost of the
    chosen interval. The location stays in `WEATHER.TXT`; editing it from the
    Wi-Fi portal comes later.
- **Settings file.** `WEATHER.TXT` keys `enabled=yes|no`,
  `refresh_minutes=0|15-360` (0 = manual, default 120), `show_on_home=yes|no`,
  `units=metric|imperial`. Old files keep working. `main.rs` saves the whole
  file when Settings or BOOT changes a value (`sync_weather_config`).
- **OFF** means no weather requests at all (and no pending retry); NTP bursts
  continue. Weather is hidden on Home. Manual means no automatic update, not
  even at boot.
- **Weather screen** (`screens/weather.rs`).
  - Now: 5× icon, seven-segment temperature, condition, today's high and low.
  - Chips: feels like, humidity, wind (whole km/h or mph), today's rain %.
  - Next hours: six columns, every second hour from the next one.
  - Next days: four rows with day, icon, condition, high / low and rain %.
  - Footer line: `Updated 13:30 · next 15:30 · Open-Meteo`, the next time
    being the observed time plus the interval.
  - Down opens the second page: twelve hourly rows, place and time zone, last
    error. Up returns. Select updates now; BOOT short press switches units.
- **Request.** `current` adds `is_day`; `hourly=temperature_2m,weather_code,
  precipitation_probability,is_day` with `forecast_hours=12`;
  `forecast_days=5`. The hand-written parser in `weather.rs` reads at most 48
  hours and checks that the arrays have the same length.
- **Icons.** `weather_icon_at(code, is_day)` adds the moon and the
  partly-cloudy night icons; `weather_icon(code)` stays for the sleep layouts.
- **Cost.** About 4 s of Wi-Fi per update, so 2 h is about 2 mAh/day.

## Phase 5: Bible and Reading Stats

The data layers are done (v0.9.1, tasks D10 and D11): `bible.rs` and
`reading_stats.rs`. The Reading Stats screen (v0.9.2, D14) and the Bible
book and chapter pickers (v0.9.3, D20) are drawn, and Reading Stats is wired
since v0.9.7 (D27). This phase adds the Bible reading view, its routes and
its wiring.

### Bible

- **Data.** `/RUSTMIX/BIBLE/<ABBR>/` as written by the reference project's
  `referencias/folloup-waveshare/scripts/bible_json_to_sd.py`: `meta.txt`,
  `index.tsv`, one `<USFM>.txt` per book with headings, paragraph marks and
  verses, and a `<USFM>.idx` of chapter byte offsets. Decided on 2026-10-08
  (task D29): the headings are what the mockup's reading view shows
  ("Jehová es mi pastor"), the index makes chapter reads cheap, and the
  owner already converts with that script. The `BOOKS.TXT` reader of D10
  goes. The text never goes into this repo.
- **Language.** The Bible app is the one part of the UI in Spanish (owner's
  decision, 2026-10-08): every screen from Home › Bible on. Home's row stays
  `Bible`.
- **Navigation.** Home › Bible opens the last place; the book picker, then
  the chapter grid, then the reading view on first use.
  - The book picker follows the mockup "Bible: elegir libro": eight sections
    (Pentateuco, Libros históricos, Poesía y sabiduría, Profetas mayores,
    Profetas menores, Evangelios y Hechos, Cartas de Pablo, Cartas generales
    y Apocalipsis), with a thumb index of each section's first book (`GÉN JOS
    JOB ISA OSE MAT ROM HEB`) instead of English codes. BOOT jumps to the
    next section, ▲▼ moves through its books, ● opens the chapters.
  - The reading view uses the Reader typography and pagination, with small
    verse numbers; short BOOT jumps to the chapter grid.
  - The last position is remembered, and Home shows it (for example `Sal 23`).
- **Verse of the day.** Picked from `/RUSTMIX/BIBLE/VERSES.TXT` (one reference
  per line) by date. Used by the sleep screen in Phase 3b; the Bible menu
  opens its page with a bar in the left margin beside the verse (D59).

### Reading Stats

Done in v0.9.7 (D27); kept as the reference.

- Reading time counts while a Reader page or the Bible's reading view is
  open (the Bible adds to the day only, no book; D60): each key press adds
  the time since the previous one, at most 2 minutes. Only page turns that move
  count as pages. The day is the local date (`AppState::local_day`).
- Stored in `/RUSTMIX/READER/STATS.TXT`, one line per day
  (`date,seconds,pages`) plus per-book totals. Writes are batched: every five
  minutes with unsaved data, when the Reader closes and before sleep.
- **Screen.** Today, a bar chart of this week (like the battery chart), the
  streak (days with at least 5 minutes) and books finished (D14). Without a
  set clock it says how to set one.
  - "Books finished" counts the books finished this year (D65, v0.12.6):
    the finish day is a `done|<path>|<date>` line after the books, which
    older firmware skips.
- Home shows `5-day streak`, and the Continue reading card shows
  `12% · 25 min today`.

## Phase 6: Games

- **Sudoku** (`src/games/sudoku.rs`, SD app `SUDOKU`).
  - The three-step entry from the mockup (row, then cell, then number)
    shipped in v0.9.3 (D18).
  - The start list, puzzles generated by difficulty, the play timer, the
    save and best times per difficulty shipped in v0.9.7 (D24).
  - The player's digits as large as the givens, in a lighter face (Literata),
    since v0.10.1.
- **Tetris** (native engine). Zen ships in v0.9.2 (D12, D13), and the best
  score survives in `/RUSTMIX/GAMES/RECORDS.TXT` since v0.9.3 (D19). This
  phase adds Classic gravity, which needs a timer in `main.rs`.
  - Board: 10 × 20.
  - Modes: Zen (no gravity; pieces move only when a key is pressed) and Classic
    (slow gravity, at least 1 s per step).
  - Keys: ▲▼ move, ● rotate, BOOT drop.
  - Show the next piece, a dotted ghost piece, score, lines and level, and save
    the best score.
  - Refresh only the changed cells (`games/dirty_regions.rs`), with a full
    refresh every few moves.
- **Games hub.** Done in v0.9.7 (D25): one card per SD game with its state.
- **Dice** (planned, owner's request 2026-10-18): one to three dice that
  roll when the device is shaken or ● is pressed, with a rattle from the
  speaker; the roll is two or three partial refreshes, then the result.
  The IMU runs only while Dice is open (D3 kept motion out of games until
  now). The mockup is D71; the engine, screens and rattle are Round 17 (D72 to
  D74), the hub card Round 18 (D75), the session D77; wired to the device
  in 0.15.0: keys, shake (a detector of its own, so Motion Events' thresholds
  stay put), the rattle on a thread of its own, saved options.
- **Race** (native engine, SD app `RACE`): the mockup's two Race screens.
  Four lanes on a road that runs right to left and moves in steps; ▲▼
  change lanes, ● spends a nitro, BOOT pauses. Classic steps every second
  and faster each kilometre; Zen steps on a key. Done in v0.11.3 (D49,
  D50); the mockup's sprites, counters and crash picture in v0.12.1 (D52,
  D53).

## Phase 7: AI

- **Voice Notes v2.** Done in 0.11.0 (`ai_jobs.rs`, `ai_worker.rs`,
  `ai_keys.rs`): a queued note goes to `POST /audio/transcriptions` in WAV
  parts of at most 10 minutes, the transcript to `POST /chat/completions`,
  and the title comes from the summary. One note at a time on a worker
  thread on core 1; Wi-Fi stays on for it; recording, playback, the portal
  and sleep stop it and queue it again; network errors back off from 2
  minutes to an hour.
  - Opus uploads: done in 0.16.0. Each ten-minute part is encoded to Ogg
    Opus at 24 kbps on the AI worker (`src/opus_encode.rs`, the encoder
    XiaoZhi brought in 0.12.0) just before it is sent, about ten times
    smaller than the WAV, with the WAV as the fallback when the encoder
    fails or the provider refuses Ogg. Not done, on purpose: recording
    straight to Opus and playing Opus notes. The card keeps WAV, which
    survives a power cut and plays as is; a 32 GB card holds hundreds of
    hours, so the saving is small, while the change touches the catalog,
    the playback path, deletion, export and the AI queue (all keyed on the
    WAV name), and live encoding competes with the screen refresh for the
    main loop. Reconsider only if the card fills up in practice or the
    encoder's `took-ms` on the device shows room to spare.
- **Settings › AI.** Two providers, each with a base URL, model and key, plus
  language, summary style and "process when online". Done in 0.10.1; the
  keys row in 0.11.0.
- **API keys.** Typed in the Wi-Fi portal and stored in the default NVS
  partition, encrypted with the HMAC scheme (key in eFuse block 5, burned
  on the first boot of 0.11.0). Never written to the SD card, never shown
  again. One key serves both providers on the same host.
- **XiaoZhi** voice chat. Done in 0.12.0: xiaozhi-esp32 needs ESP-IDF 6.0.1
  or later (Wave uses 5.5.1), so its protocol is ported to Rust
  (`src/xiaozhi/`, see [architecture](architecture.md#xiaozhi-call)): the
  OTA check-in and linking with a code at xiaozhi.me, the WebSocket call in
  auto mode with Opus both ways, and MCP device tools (status, volume, a
  Bible passage from the SD card).
  - A server of your own is done (0.17.0). Not planned, by the owner's
    decision: text in scripts the fonts lack (Chinese). Echo cancellation, so
    one can speak over XiaoZhi, is open and needs the speaker's reference
    signal on the device.
- **Delegated groundwork (v0.9.3).** `json_lite.rs` (D21), `ai_client.rs`
  (D22, OpenAI-compatible requests and responses) and `xiaozhi/protocol.rs`
  (D23, messages and binary frames) are libraries without network access;
  this phase adds HTTPS, WebSocket and the screens.

## Phase 8: OTA updates

- **Partitions.** Done in 0.13.0: `partitions.csv` with `nvs` (same place and
  size as before, so the AI keys survive), `phy_init`, `otadata`, `ota_0` and
  `ota_1` (6 MB each). Moving to it needs one USB flash; the README says so.
- **Release assets.** Done in 0.13.0: `firmware.yml` also publishes
  `wave-epaper-<tag>-app.bin`, the app-only image used by OTA.
- **Rollback.** Done in 0.13.0: `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE`; the
  running image marks itself valid after 30 s of uptime (`ota.rs`,
  `main.rs`), so a reset before then returns to the previous image.
- **Decisions in `src/ota.rs`** (host-tested, no network): the latest
  release's JSON, version comparison, the app asset, the battery gate and
  the check of an image's header and version before it is written.
- **Update source.** Settings › System › Check for updates reads the latest
  GitHub release over HTTPS (certificate bundle) and downloads the app image.
  Uploading the image through the Wi-Fi portal also works. The screens are
  drawn in the mockup (`Ajustes y reposo`: System, update available, up to
  date, downloading, installed, refused); slice 2 builds them once the owner
  approves them. Built in 0.14.0 (the mockup's "Free slot" column and release
  date became the release tag and the size).
- **Safety.**
  - `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE`: a new image marks itself valid
    only after reaching Home.
  - Require at least 30% battery or USB power.
  - Show a progress screen.

## Backlog

- **Deeper sleep.** The mockup's 8 µA needs deep sleep in sleep mode. Deep
  sleep can only wake from RTC GPIOs (0 to 21). BOOT and the wheel qualify,
  and so does the Power key through GPIO1 (`PWR_OUT`), which an RTC alarm
  also drives high. The study is `DEEP_SLEEP.md` (D70): the RTC alarm
  (GPIO45) and the PMIC interrupt (GPIO38) are not RTC GPIOs, so the alarm
  needs a timer or a board rewire. The owner ruled out the rewire (2026-10-28),
  so only the timer path is left, and it is not scheduled.
- **Not planned (owner, 2026-10-28):** an automatic update check (Check for
  updates stays manual) and text in scripts the fonts lack.
- **Settings regrouping per the mockup.** Display, Reading, Sleep screen,
  Weather, AI, Wi-Fi & transfer, Clock & alarms, Power, System. Motion and
  Environment move under System as diagnostics. Reading is done (0.10.4).
