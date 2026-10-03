# Wave roadmap

Status of the firmware and technical specs for the work still to do. How the
current firmware is built is in [architecture.md](architecture.md); small tasks
handed to a second developer are in [DELEGATED_TASKS.md](DELEGATED_TASKS.md).
The UI follows `mockups/index.html`.

Last updated: 2026-10-03, firmware v0.8.1.

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
| Phase 3b: sleep screen modes (clock, weather) | 0.9.0 | | Planned |
| Phase 4: Weather app and Settings › Weather | | | In progress, branch `phase4-weather` |
| Phase 5: Bible and Reading Stats | | | Planned |
| Phase 6: Games (Sudoku, Tetris) | | | Planned |
| Phase 7: AI (Voice Notes with OpenAI-compatible providers, XiaoZhi) | | | Planned |
| Phase 8: OTA updates | | | Planned |
| Delegated tasks D1 to D5: option lists, BLE remote and tilt games removed, guides, sleep layouts | 0.8.1 | PR #1 | Done, waiting for device test |
| Delegated tasks D6 to D11: `%` glyph, sleep layout polish, small fixes, Bible and reading stats data | | | In progress, branch `side-tasks-2` |

Every phase ends with host tests, screen previews, a green firmware build, a
version bump and a test on the device by the owner.

## Phase 3a: Photos (v0.8.0)

Done in v0.8.0; this section stays as the reference for the Photos code.

Goal: browse photos from the SD card and choose the ones shown while the device
sleeps.

### Files

- Photos: `/PHOTOS/` at the SD root. Sub-folders are ignored. Up to 500 files
  are listed, newest file date first.
- Formats: JPEG (`.jpg`, `.jpeg`). Baseline JPEGs of any size; progressive
  JPEGs up to 1 megapixel, because a progressive decode keeps every coefficient
  in memory. Rejected files show the reason, for example "is a progressive
  JPEG; save it as a standard JPEG". BMP and PNG photos are not part of 3a; BMPs
  keep working from the sleep folder.
- EXIF orientation (tag 0x0112) is applied, so phone photos come out upright.
  Values 1, 3, 6 and 8 rotate; mirrored values use the nearest rotation.
- Cache: `/RUSTMIX/CACHE/PHOTOS/<key>.PIC`, one file per photo.
  - `key`: 8 hex digits of FNV-1a 32 over `name|size|mtime`.
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
- RGB to grey (77/150/29 weights), area-average resampling to each target,
  Floyd–Steinberg dithering. This is the same dithering as `sleep_images.rs`;
  share the code.
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

## Phase 3b: sleep screen modes (v0.9.0)

Settings › Sleep screen gains a mode list, each with its estimated cost from
the mockup:

| Mode | What it shows | Cost estimate | Refresh |
|---|---|---|---|
| Photo | Starred photo (3a) | No wake-ups | At sleep entry |
| Clock & date | Large time, date, optional weather line, battery | about 13 mAh/day at 1 min | Every 1 or 5 min (setting) |
| Weather | Place, updated time, now, three days | about 2 mAh/day | After each weather update (default 2 h) |
| Clock + weather | Clock and weather line | about 5 mAh/day | Clock 5 min, weather 2 h |
| Verse of the day | Verse from the Bible on the SD | 1 wake-up per day | Disabled until Phase 5 |

- Layouts come from task D5 (`src/app/screens/sleep_screens.rs`); delegated
  task D7 brings them closer to the mockup:
  - clock: the rule about 40 px under the date, the weather icon and summary
    centered as one group, the details right under the summary;
  - weather: a rule above the three days;
  - the rain row in Body size, since the `%` glyph is broken at Detail size.
- **Clock.** While asleep the loop already wakes at least every 60 s. When a
  minute is due:
  1. power the panel (ALDO3) and initialize it;
  2. draw the screen and do a partial refresh;
  3. put the panel back to deep sleep.

  Do a global refresh every 30 minutes against ghosting. Check the real cost
  with the battery log.
- **Weather.** Needs weather turned on (Phase 4 setting; until then
  `WEATHER.TXT` present).
  - While asleep, schedule Wi-Fi bursts at the weather interval and redraw
    after each successful update.
  - Data older than 6 h is labeled "stale".
- RTC alarms and wake keys keep working in every mode.
- Acceptance: the battery log over a night roughly matches each mode's
  estimate, with no visible ghosting after hours on the clock.

## Phase 4: Weather app

- **Settings › Weather** (mockup):
  - Weather service ON or OFF.
  - Update every 30 min, 1 h, 2 h (default), 6 h, or Manual.
  - Location (from `WEATHER.TXT`; editing it from the Wi-Fi portal comes
    later).
  - Units: °C or °F, km/h or mph.
  - Show on Home: yes or no.
  - Provider: Open-Meteo, no key.
- **Settings file.** New `WEATHER.TXT` keys: `enabled=yes|no`,
  `refresh_minutes=0|30|60|120|360` (0 = manual), `show_on_home=yes|no`,
  `units=metric|imperial`. Old files keep working.
- **OFF** means no weather requests at all; NTP bursts continue. Weather is
  hidden on Home and on sleep screens.
- **Weather screen.**
  - Now: icon, temperature, condition, feels like, humidity, wind.
  - Next 6 hours: time, icon, temperature, rain %.
  - Four days: day, icon, condition, high and low, rain %.
  - Footer line: `Updated 13:30 · next 15:30 · Open-Meteo`.
  - Keys: ▲▼ scroll, ● refresh now, BOOT short press switches °C and °F.
- **Request.** Add `hourly=temperature_2m,weather_code,precipitation_probability`
  and `forecast_hours=12`. Keep the bounded hand-written JSON parser in
  `weather.rs`.
- **Icons.** Map the WMO code with day and night variants, as in the mockup
  legend. Current icons are 26 px ASCII art in `widgets/icons.rs`.
- **Cost.** About 4 s of Wi-Fi per update, so 2 h is about 2 mAh/day.

## Phase 5: Bible and Reading Stats

The data layers are delegated tasks D10 (`bible.rs`) and D11
(`reading_stats.rs`); this phase adds the screens and wires them up.

### Bible

- **Data.** `/RUSTMIX/BIBLE/<CODE>/`, for example Reina-Valera 1909 (public
  domain); the owner supplies the text.
  - `BOOKS.TXT`: one line per book, `number|name|short name|chapters`.
  - One UTF-8 file per book, `NN.TXT`, with lines
    `chapter:verse<TAB>text`.
  - Only this format is documented; conversion happens on a computer.
- **Navigation.** Home › Bible → Old or New Testament (BOOT short press) →
  book → chapter grid → reading view.
  - The reading view uses the Reader typography and pagination, with small
    verse numbers.
  - The last position is remembered, and Home shows it (for example `Sal 23`).
- **Verse of the day.** Picked from `/RUSTMIX/BIBLE/VERSES.TXT` (one reference
  per line) by date. Used by the sleep screen in Phase 3b.

### Reading Stats

- Reading time counts while a Reader page is open and a key was pressed in the
  last 2 minutes.
- Stored in `/RUSTMIX/READER/STATS.TXT`, one line per day
  (`date,seconds,pages`) plus per-book totals. Writes are batched every few
  minutes and before sleep.
- **Screen.** Today, a bar chart of this week (like the battery chart), the
  streak (days with at least 5 minutes) and books finished.
- Home shows `5-day streak`, and the Continue reading card shows
  `12% · 25 min today`.

## Phase 6: Games

- **Sudoku** (`src/games/sudoku.rs`, SD app `SUDOKU`).
  - Add the three-step entry from the mockup: row, then cell, then number.
  - Also a timer, auto-save, best time per difficulty and a three-level
    difficulty choice.
- **Tetris** (new native engine).
  - Board: 10 × 20.
  - Modes: Zen (no gravity; pieces move only when a key is pressed) and Classic
    (slow gravity, at least 1 s per step).
  - Keys: ▲▼ move, ● rotate, BOOT drop.
  - Show the next piece, a dotted ghost piece, score, lines and level, and save
    the best score.
  - Refresh only the changed cells (`games/dirty_regions.rs`), with a full
    refresh every few moves.
- **Games hub.** One row per game with a short description and the best score.

## Phase 7: AI

- **Voice Notes v2.**
  1. Record.
  2. Transcribe with an OpenAI-compatible `POST /audio/transcriptions`, for
     example Groq `whisper-large-v3-turbo`.
  3. Summarize with `POST /chat/completions`, for example OpenRouter.
  4. Take the title from the summary.

  Notes queue while offline, and each shows its state: queued, transcribing, or
  summary ready.
  - Recording stays WAV at first. Opus at 24 kbps needs an encoder (an ESP-IDF
    Opus component); that decision is open.
- **Settings › AI.** Two providers, each with a base URL, model and key, plus
  language, summary style and "process when online".
- **API keys.** Typed in the Wi-Fi portal and stored encrypted in NVS. Never
  written to the SD card, never shown again.
- **XiaoZhi** voice chat (WebSocket and Opus streaming, device tools such as
  reading a Bible verse) is the largest item. Look at reusing the
  xiaozhi-esp32 implementation as an ESP-IDF component.

## Phase 8: OTA updates

- **Partitions.** A custom `partitions.csv` with `nvs`, `otadata`, `phy_init`,
  `ota_0` and `ota_1` (about 6 MB each). Moving to it needs one USB flash, and
  the README must say so.
- **Release assets.** `firmware.yml` also publishes the app-only image used by
  OTA.
- **Update source.** Settings › System › Check for updates reads the latest
  GitHub release over HTTPS (certificate bundle) and downloads the app image.
  Uploading the image through the Wi-Fi portal also works.
- **Safety.**
  - `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE`: a new image marks itself valid
    only after reaching Home.
  - Require at least 30% battery or USB power.
  - Show a progress screen.

## Backlog

- **Hold ▲▼ to repeat** (mockup): first repeat after 500 ms, then every
  200 ms. `buttons.rs` has to report the press before the release, and the
  loop has to merge repeats while a refresh runs.
- **Deeper sleep.** The mockup's 8 µA needs deep sleep in sleep mode. Deep
  sleep can only wake from RTC GPIOs (0 to 21), so BOOT and the wheel could wake
  it but the Power key (GPIO38) and the RTC alarm (GPIO45) could not. Needs
  research before any change.
- The `%` sign looks broken at the Detail font size (seen on the sleep card);
  delegated as task D6.
- **Settings regrouping per the mockup.** Display, Reading, Sleep screen,
  Weather, AI, Wi-Fi & transfer, Clock & alarms, Power, System. Motion and
  Environment move under System as diagnostics.
- **Settings › Reading.** Hyphenation, margins, stats.
