# Wave architecture

How the firmware is put together as of v0.8.0. Plans and specs for upcoming
work are in [ROADMAP.md](ROADMAP.md).

## Hardware

| Part | Details |
|---|---|
| Module | ESP32-S3-WROOM-1-N16R8: 16 MB flash, 8 MB octal PSRAM |
| Display | 3.97" 800 × 480 1-bit e-paper on SPI; BUSY on GPIO3; panel rail is AXP2101 ALDO3 |
| Keys | BOOT on GPIO0; wheel Up GPIO4, Select GPIO5, Down GPIO6 (all active low); Power key on the AXP2101 |
| PMIC | AXP2101 on I2C; IRQ on GPIO38 (only Power-key short and long press interrupts are enabled) |
| RTC | PCF85063; alarm interrupt on GPIO45 (active low) |
| Sensors | SHTC3 temperature and humidity, QMI8658 IMU |
| Audio | ES8311 codec on I2S0 (speaker and microphone) |
| Storage | microSD on SDMMC 4-bit, FAT with long names, mounted at `/sdcard` |

## Code layout

- `src/main.rs` (module `firmware`, device only) boots the board, loads the SD
  settings and runs the single event loop. It owns every ESP-IDF handle: panel
  SPI, I2C, Wi-Fi, audio.
- `src/lib.rs` and the modules below it build on a PC too, so they carry the
  host tests.
- `src/app/` is the UI:
  - `state.rs`: `AppState` holds every screen's state and turns button events
    into state changes. Hardware requests go back to `main.rs` as flags or
    request enums.
  - `router.rs`: `ScreenRoute`, the screen tree (`parent()` is where hold-BOOT
    goes).
  - `menu.rs`: Home and category entries.
  - `screens/`: one renderer per route.
  - `widgets/`: header, status row, list row, option list, footer, icons.
  - `typography/`: bitmap fonts and `UiTextStyle` (`text_width`, `wrap`, `fit`).
  - `preview.rs`: host-only PNG previews of the screens (CI artifact
    `screen-previews`).
- Feature modules: `reader.rs`, `epub.rs`, `hyphenation.rs` (books);
  `weather.rs`; `network.rs`, `radio_burst.rs` (Wi-Fi); `alarm.rs`, `rtc*.rs`;
  `audio/`, `voice_notes.rs`; `calendar.rs`, `dictionary.rs`,
  `unit_converter.rs`; `lua_runtime/`, `games/` (SD apps); `wifi_transfer.rs`
  (file portal); `sleep_images.rs`, `sleep_mode.rs`, `sleep_screen.rs`;
  `photos/` and `dither.rs` (gallery, JPEG decoding, cache files, worker);
  `power_settings.rs`, `battery_log.rs`.
- The inherited `rustmix-remote-ble` feature (BLE page turner) is scheduled for
  removal.

## Display pipeline

- `FrameBuffer`: native 800 × 480, 1 bit per pixel, bit set = white, MSB first,
  48,000 bytes.
- The UI is portrait 480 × 800. `OrientedFrameBuffer` maps logical (x, y) to
  native (y, 479 − x). The Reader can switch to landscape.
- `render_current_screen(frame, state)` clears the frame and draws the active
  route. `PanelRefreshCoordinator` chooses partial or global refreshes (global
  after a run of partials, after waking, or on request). Refreshes block until
  BUSY clears.
- Common layout: header at the top, status row below it, content from y ≈ 150,
  footer rule at y 746 with its text baseline at 782.

## Event loop

`main.rs` runs one loop. Each pass, roughly in order:

1. RTC alarm poll.
2. Power key (PMIC IRQ) and auto-sleep; wake from sleep mode.
3. Weather refresh and Wi-Fi bursts (awake only).
4. Reader ticks, photo worker results and deletes, live status refresh.
5. BOOT button (short press: contextual action, hold: back), then the wheel.
6. Battery sample every 15 minutes.
7. Idle light sleep.

Rules that keep it stable:

- Buttons are polled, and `poll` blocks until the key is released.
- FreeRTOS runs at 100 Hz, and `std::thread::sleep` busy-waits below 10 ms. Use
  `FreeRtos::delay_ms` for waits and `watchdog::Pacer` inside long CPU-bound
  work.
- Heavy jobs (weather HTTPS, EPUB loading, Lua loading) run on named worker
  threads (`runtime_worker.rs`). The panel SPI stays on the main task.
- The photo worker (`photos/worker.rs`) runs alongside the loop, pinned to
  core 1 at priority 1, so a JPEG decode never delays the keys. Light sleep
  waits until it is idle, and a photo is deleted only then.

## Power

- **Awake and idle.** About 300 ms after the last press, if nothing is running,
  the CPU light-sleeps until one of these happens:
  - a key is pressed, the PMIC IRQ fires, or the RTC alarm line drops;
  - the next timed job falls due (panel power-down after 60 s, auto-sleep, live
    status refresh, alarm poll, next Wi-Fi burst).

  Each light sleep lasts at most 60 s. It is skipped on USB power so flashing
  and the serial console work (VBUS is checked every 5 s).
- **Wi-Fi bursts.** Wi-Fi is off except for short bursts: weather refresh, NTP
  every 12 h, manual refresh, file transfer.
  - A burst gives up after 20 s without a connection and ends after 90 s.
  - Failures back off from 15 min up to 4 h.
- **Sleep mode.** Holding the Power key, or the auto-sleep delay from Settings ›
  Power, enters sleep mode:
  - The sleep picture is drawn: a starred photo or a picture from
    `/RUSTMIX/SLEEP/`, as set in Settings › Sleep screen. Only cached photo
    frames are used, so nothing is decoded here. If no picture can be used, a
    card says why.
  - The panel enters deep sleep with ALDO3 off.
  - Wi-Fi and the IMU are off, and the codec is suspended.
  - The CPU light-sleeps in 60 s steps.

  The Power key always wakes the device; BOOT and the wheel wake it too when
  Settings › Power › Wake keys is "Any key".
- **Peripherals.** The IMU runs only on the Motion screens and IMU games. The
  ES8311 codec is suspended when nothing plays.
- **Battery log.** One sample (RTC minute and percent) every 15 min, kept for 7
  days.

## Memory

- Internal SRAM holds stacks (main task 16 KB), Wi-Fi buffers and DMA.
- Allocations of 16 KB or more go to PSRAM (`CONFIG_SPIRAM_MALLOC_ALWAYSINTERNAL=16384`).
  That covers frame buffers, page caches, EPUB chapters and decoded pictures.
- Font atlases are `static` data in flash.

## SD card files

| Path | Module | Contents |
|---|---|---|
| `/RUSTMIX/WIFI.TXT` | `network_config.rs` | SSID, password, time zone, NTP server |
| `/RUSTMIX/WEATHER.TXT` | `weather_config.rs` | Open-Meteo location, refresh minutes |
| `/RUSTMIX/ALARMS.TXT` | `alarm.rs` | Alarms and snooze minutes |
| `/RUSTMIX/DISPLAY.TXT` | `app/display.rs` | UI font and size |
| `/RUSTMIX/POWER.TXT` | `power_settings.rs` | Auto-sleep delay, wake keys |
| `/RUSTMIX/BATTERY.TXT` | `battery_log.rs` | Battery history, `minute,percent` per line |
| `/RUSTMIX/BOOKS/` | `reader.rs` | TXT and EPUB books |
| `/RUSTMIX/READER/` | `reader.rs` | Positions, recent books, bookmarks, preferences, page cache |
| `/RUSTMIX/SLEEP/` | `sleep_images.rs` | Sleep pictures (BMP) |
| `/RUSTMIX/SLEEPSCREEN.TXT` | `sleep_screen.rs` | Sleep picture source, order and fit |
| `/PHOTOS/` | `photos/` | JPEG photos for Photos and the sleep screen |
| `/RUSTMIX/STARRED.TXT` | `photos/mod.rs` | Starred photo names |
| `/RUSTMIX/CACHE/PHOTOS/` | `photos/cache.rs` | Per photo: thumbnail and two screen frames (`.PIC`) |
| `/RUSTMIX/VOICE/` | `voice_notes.rs` | Voice notes (WAV) and their settings |
| `/RUSTMIX/APPS/` | `lua_runtime/` | SD apps and games, Dictionary and Calendar packs |

## Build and CI

- `rust-toolchain.toml` selects the Xtensa `esp` toolchain for the firmware.
  Host tests use stable Rust: `./scripts/test-host.sh`.
- `.github/workflows/ci.yml` (push to `main`, pull requests, manual) runs the
  host tests, uploads `screen-previews`, then runs `cargo fmt --check`.
- `.github/workflows/firmware.yml` (push to `main`, manual) builds with ESP-IDF
  v5.5.1 and uploads `wave-epaper-<sha>.bin` (merged image, flashed at 0x0), the
  `.elf` and `SHA256SUMS.txt`.
- `.github/workflows/fonts.yml` regenerates the font atlases from
  `scripts/fonts/fonts.toml`.
- The version lives in `Cargo.toml` and `Cargo.lock`, `CONFIG_APP_PROJECT_VER`
  in `sdkconfig.defaults`, and `UI_SHELL_MILESTONE` in `src/build_info.rs` (with
  its test).

