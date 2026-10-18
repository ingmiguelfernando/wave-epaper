# Deep sleep study

A design study for replacing sleep mode's light sleep with MCU deep sleep.
No code here. Sleep mode today keeps the CPU light-sleeping in 60 s steps
(`docs/architecture.md` › Power); the mockup's 8 µA needs deep sleep
(`docs/ROADMAP.md` › Backlog › Deeper sleep). Deep sleep restarts the chip
on wake, so every claim below names its source: a file and line in this
repo, a documentation page, or a measurement the owner still has to take.

Sources used: this repo (`docs/BOARD_CONTRACT.md`, `docs/architecture.md`,
`src/main.rs` and the modules it names), and the ESP-IDF v5.5
documentation (built for ESP32-S3):

- [Sleep Modes](https://docs.espressif.com/projects/esp-idf/en/v5.5/esp32s3/api-reference/system/sleep_modes.html)
- [ULP Coprocessor](https://docs.espressif.com/projects/esp-idf/en/v5.5/esp32s3/api-reference/system/ulp.html)

---

## 1. Wake sources and which pins can use them

### 1.1 What ESP32-S3 deep sleep can wake on

From ESP-IDF v5.5 › Sleep Modes › Wakeup Sources:

| Source | Mechanism | Pins | Notes |
|---|---|---|---|
| `ext0` | RTC IO, one pin, one level | RTC GPIOs **0–21** only | Keeps the RTC peripherals domain on |
| `ext1` | RTC controller, any of many pins, all same level (ANY_HIGH or ANY_LOW) | RTC GPIOs **0–21** only | Can work with RTC peripherals powered down (pad hold) |
| Timer | RTC timer | none | Microsecond request; resolution follows `RTC_SLOW_CLK` |
| ULP | ULP coprocessor program | RTC GPIOs (via ULP) | Program lives in RTC SLOW memory; not used by Wave today |
| GPIO | Digital GPIO, per-pin level | **any** IO | **Light sleep only** — not available in deep sleep |
| UART | RX edges | UART RX | Light sleep only |
| Touch | Touch pads | touch pins | Light sleep and deep sleep; Wave has no touch pads |

`esp_sleep_is_valid_wakeup_gpio()` accepts any RTC IO input pin
(sleep_modes.html › API Reference). On ESP32-S3 the RTC GPIOs are
**GPIO 0 through GPIO 21** (`esp_sleep_enable_ext0_wakeup` notes).

### 1.2 Wave's inputs and whether they reach an RTC GPIO

| Input | Pin | Named in | RTC GPIO (0–21)? | Deep-sleep wake |
|---|---|---|---|---|
| BOOT | GPIO0 | `docs/BOARD_CONTRACT.md:23`, `docs/architecture.md:12`; log `rustmix-wave=boot-button-back` `src/main.rs:384` | **yes** | `ext0`/`ext1` (active low: `BOOT` is active-low) |
| Wheel Up | GPIO4 | `docs/architecture.md:12` | **yes** | `ext1` ANY_LOW |
| Wheel Select | GPIO5 | `docs/architecture.md:12` | **yes** | `ext1` ANY_LOW |
| Wheel Down | GPIO6 | `docs/architecture.md:12` | **yes** | `ext1` ANY_LOW |
| Power key (`PWR_OUT`) | GPIO1 | `src/power_key.rs:16`, `docs/BOARD_CONTRACT.md:24`, `docs/architecture.md:12` | **yes** | `ext0`/`ext1` (high while held) |
| RTC alarm (PCF85063) | GPIO45 | `src/rtc_alarm_interrupt.rs:9`, `docs/BOARD_CONTRACT.md:48` | **no** (45 > 21) | **cannot** use `ext0`/`ext1` |
| PMIC IRQ (AXP2101) | GPIO38 | `src/main.rs:2455`, `docs/architecture.md:13` | **no** (38 > 21) | **cannot** use `ext0`/`ext1` |
| Panel BUSY | GPIO3 | `docs/architecture.md:11`, `docs/BOARD_CONTRACT.md:16` | yes | not a wake key; the panel rail is off in sleep |

Today's light sleep uses `gpio_wakeup_enable` on any IO
(`src/main.rs:2885-2908`, `light_sleep_until_wake`), so GPIO38 and GPIO45
already wake light sleep. That is the **main hardware gap**: deep sleep
cannot watch the RTC alarm line or the PMIC IRQ the same way.

**Workarounds the owner must choose among** (each needs a board check):

1. **Timer wake to the next alarm.** Read the PCF85063 before sleeping
   (`src/rtc*.rs`), arm `esp_sleep_enable_timer_wakeup` for the seconds
   until the next alarm (or the next clock minute / weather slot). GPIO45
   then only matters while awake. Risk: an alarm the firmware has not
   programmed yet, or a snooze written after the timer was set.
2. **ULP polling of a level.** ULP cannot read GPIO45 as an RTC IO, so
   this does not help unless the board routes the alarm to an RTC GPIO.
3. **Board change.** Wire the PCF85063 `INT` (and, if wanted, the PMIC IRQ)
   to an RTC GPIO in 0–21. That is the only way `ext0`/`ext1` can see the
   alarm. GPIO45 is outside the RTC IO block
   (sleep_modes.html › `esp_sleep_enable_ext0_wakeup`).

GPIO1 (`PWR_OUT`) is already the Power-key path and is an RTC GPIO, so the
"always wakes" rule of `docs/architecture.md:148` can survive deep sleep
without a board change.

### 1.3 Pull-ups, levels and hold

`ext0`/`ext1` need the pin's pull state set before `esp_deep_sleep_start`
(`rtc_gpio_pullup_en` / `rtc_gpio_pulldown_en`). The keys are active low
except `PWR_OUT` (high while held). With `ext1`, every pin shares one level
mode (ANY_LOW or ANY_HIGH) on ESP32-S3 (sleep_modes.html › ext1 note), so
Power (high) cannot share an `ext1` mask with BOOT and the wheel (low).
Use `ext0` on GPIO1 for the Power key and `ext1` ANY_LOW for BOOT + wheel,
or invert one side in hardware.

After wake, the pad stays in RTC IO until `rtc_gpio_deinit`
(sleep_modes.html › ext0/ext1 warnings). The key poll in `src/main.rs`
must re-init GPIO0/1/4/5/6 as digital GPIOs before `poll` runs.

---

## 2. What a wake loses and what survives

From ESP-IDF v5.5 › Sleep Modes › Overview and Power-down Options:

### Lost on deep-sleep wake (cold start of the app)

| Thing | Why |
|---|---|
| Internal SRAM (except RTC) | CPUs and most RAM are powered off |
| PSRAM (8 MB) | Same; frame buffers, page caches, decoded photos, EPUB chapters are gone |
| Wi-Fi / Bluetooth | Wireless is powered down; connections are not kept |
| Digital peripherals (SPI, I2C, I2S, SDMMC, …) | Clocked from APB, powered off |
| Panel **driver** state (SSD1677 registers, SPI) | The controller is re-initialised on every boot |
| FreeRTOS tasks, heap, stacks | Application restarts from the entry point |
| `esp_timer` / system time base as RAM state | Chip restarts; wall time must come from the RTC (PCF85063) or NVS |

### Survives

| Thing | Why | Source |
|---|---|---|
| RTC controller, ULP, RTC FAST/SLOW memory | Only powered domains in deep sleep | sleep_modes.html › Overview |
| RTC SLOW data marked `RTC_DATA_ATTR` | Kept on if that memory is powered | sleep_modes.html › Power-down of RTC Peripherals |
| The e-paper **image** | Electrophoretic panel holds pixels with the rail off | `docs/architecture.md:144` (panel deep sleep with ALDO3 off) |
| SD card files | FAT is on the card, not in RAM | `docs/architecture.md` › SD card files |
| NVS (Wi-Fi calibration, AI keys, Xiaozhi id) | In flash | `docs/architecture.md:112`, `src/main.rs:614` |
| Flash image (firmware) | App restarts into the same image | — |

### Firmware state kept only in RAM today

`AppState` (`src/app/state.rs:167`) is rebuilt on every boot from SD and
NVS. The fields that are **not** written to the card before a deep sleep
would need a home. The table lists the volatile fields that a wake-to-sleep
path would care about, and the file that already owns the durable twin
where one exists. (UI cursors and one-shot flags can be re-defaulted.)

| Field (`src/app/state.rs`) | Already on disk? | File |
|---|---|---|
| `reader.resume`, `reader.session`, `reader.positions`, `reader.recent`, `reader.bookmarks` | yes (place is saved in batches) | `/RUSTMIX/READER/` (`docs/architecture.md:179`) |
| `reader.preferences` | yes | `/RUSTMIX/READER/` |
| `display` | yes | `/RUSTMIX/DISPLAY.TXT` (`app/display.rs`) |
| `power` (`PowerPreferences`) | yes | `/RUSTMIX/POWER.TXT` (`power_settings.rs`) |
| `alarms` | yes | `/RUSTMIX/ALARMS.TXT` (`alarm.rs`) |
| `weather_config` | yes | `/RUSTMIX/WEATHER.TXT` |
| `weather` (forecast snapshot) | yes since v0.12.5 | `/RUSTMIX/FORECAST.TXT` (`weather_cache.rs`) |
| `verse_of_day` | no | would go in a small cache under `/RUSTMIX/` (or reload on wake) |
| `battery_log` | yes | `/RUSTMIX/BATTERY.TXT` |
| `voice_notes` catalog / settings | yes | `/RUSTMIX/VOICE/` |
| `home_selected`, `category_selected`, `router` (current route) | **no** | `SleepModeState` already stores `restore_route` in RAM only (`src/sleep_mode.rs:100`); a deep-sleep path must put it in RTC memory or a `/RUSTMIX/SLEEP.TXT` sidecar |
| `light_sleep` share, `partial_refreshes`, `select_presses`, `memory` | **no** | diagnostics; re-default is fine |
| `board` (PMIC %, RTC sample, IMU) | **no** | re-read from hardware on wake |
| `network`, `ai_busy_note`, `wifi_transfer`, `xiaozhi` | **no** | not started at sleep entry |

**RTC memory (`RTC_DATA_ATTR`)** is the right home for: `restore_route`,
a wake cause flag, and the last sleep-entry minute (so a wake can tell a
planned deep sleep from a reset). ESP-IDF keeps that memory across deep
sleep if the slow memory domain is left on (sleep_modes.html ›
Power-down of RTC Peripherals and Memories).

---

## 3. What boot does before the first screen

`src/main.rs` logs one `rustmix-wave=…` marker per step. Order at cold
boot (from `src/main.rs:148` onward), with what a **wake to the sleep
screen** could skip:

| Step | Marker | Could a sleep-wake skip it? | Time estimate | How estimated |
|---|---|---|---|---|
| App entry | `epd397-rust-app-start` (`:148`) | no (restart) | ~0 | — |
| Product shell | `product-ui-shell-start` (`:150`) | no | ~0 | — |
| SD mount (4-bit FAT) | `sdmmc-mount` (`:182`) | **maybe** — keep mounted is impossible; must remount | 50–200 ms | SDMMC init + FAT mount is tens of ms; **owner must measure** |
| Display config | `display-config` (`:198`) | yes (already known) | ~1 ms | one small file |
| Power config | `power-config` (`:217`) | yes if parked in RTC | ~1 ms | one small file |
| Wi-Fi config | `wifi-config` (`:235`) | yes (not needed for sleep screen) | ~1 ms | one small file |
| Weather config | `weather-config` (`:251`) | yes | ~1 ms | one small file |
| Alarm config | `alarm-config` (`:276`) | yes if next alarm in RTC | ~1 ms | one small file |
| Audio init (ES8311 + I2S) | `audio-init` (`:304`) … `audio-subsystem-ready` | **yes** | 10–50 ms | I2C probe + I2S setup; **owner must measure** |
| BOOT button | `boot-button-back` (`:384`) | no (need keys) | ~0 | — |
| RTC alarm int | `rtc-alarm-int` (`:391`) | partial | ~0 | — |
| Power key GPIO | `power-key-gpio` (`:399`) | no | ~0 | — |
| Reset reason | `reset-reason` (`:416`) | no | ~0 | — |
| Battery log / starred / sleep-screen / reading-stats / AI / games / voice cleanup / voice settings / reader persistence | `:422`–`:484` | **yes** for a pure sleep-screen wake | 1–20 ms each | several small FAT files; **owner must measure** |
| Regional profile | `regional-profile` (`:527`) | yes if in RTC | ~0 | — |
| Board services (RTC, SHTC3, QMI8658, power) | `sample-board-services-init` (`:536`) | **partial** — power (AXP2101) needed for battery; sensors not | 5–30 ms | I2C probes; **owner must measure** |
| PMIC power-key poll | `power-key status=ready` (`:554`) | no | ~0 | — |
| Weather cache | `weather-cache` (`:577`) | yes if parked in RTC / card | 1–5 ms | one file |
| Panel bring-up + global refresh | `panel-refresh plan=global-base reason=initial-boot` (`:598`), `epd397-rust-display-ready` (`:600`) | **no full refresh** if the image is already on the panel — but a cold panel init is required after ALDO3 was off | **1–3 s** typical SSD1677 global refresh; **owner must measure** | panel data sheet / scope |
| NVS | `nvs status=ready` (`:614`) | no (needed for keys) | 10–50 ms | flash NVS open; **owner must measure** |
| AI keys | `ai-keys` (`:633`) | yes | ~0 | from NVS |
| Wi-Fi radio start | `wifi-radio` (`:640`) | **yes** — never start Wi-Fi on a sleep-screen wake | 50–150 ms | Wi-Fi init; **owner must measure** |
| IMU thresholds / voice catalog | `:674`–`:677` | yes | 1–10 ms | — |
| First frame draw + refresh | (end of boot, `:598` first refresh) | draw yes, refresh maybe | 50–200 ms draw | host preview path is much faster; device needs a scope |

**Rough cold-boot budget the owner should confirm on the device** (sum of
the "must" rows plus panel): about **1.5–4 s**, dominated by the panel
global refresh. A wake-to-sleep path that skips Wi-Fi, audio, sensors,
and most SD files and **does not** re-run a global refresh should land in
the **200–800 ms** range — still to be measured.

`main.rs` also logs `reset-reason last-restart=…` (`:416`). After a deep
sleep the reset reason is deep-sleep wake; that is how a fast path can
tell "wake to sleep screen" from "power-on".

---

## 4. How the sleep modes wake the chip today

### Alarms

- Hardware: PCF85063 `INT` on **GPIO45**, active low
  (`docs/BOARD_CONTRACT.md:48`, `src/rtc_alarm_interrupt.rs:9`).
- The loop polls the line and the RTC flag
  (`src/main.rs:1187`–`:1287`, marker `rtc-alarm-int`, `alarm-triggered`).
- Light sleep: `SLEEP_WAKE_GPIOS` includes `RTC_ALARM_INTERRUPT_GPIO`
  (`src/main.rs:2457`), so a falling alarm line ends light sleep
  (`light_sleep_until_wake` `:2885`).
- **Deep sleep:** GPIO45 is **not** an RTC GPIO. The alarm cannot use
  `ext0`/`ext1`. Plan option 1 in §1.2 (timer to the next alarm) is the
  default; option 3 (board rewire) is the robust one.

### Weather sleep screens' updates

- Clock + weather refreshes the weather line every 2 h and the clock every
  1 or 5 min (mockup "Settings › Sleep screen"; `docs/ROADMAP.md` Phase
  sleep table).
- Today those are timed jobs in the light-sleep budget
  (`light_sleep_budget` `src/sleep_mode.rs:20`, used from `src/main.rs:2389`).
- Deep sleep: the **RTC timer** wakes the chip for the next slot (minute
  tick or weather interval). On wake the fast path redraws only the
  weather/clock region and goes back to deep sleep. No ext0/ext1 involved.

### Clock sleep screen's minute

- Same as above: `esp_sleep_enable_timer_wakeup` for the remainder of the
  minute (or 5 minutes, per setting), then a partial redraw of the time.
- Resolution of the RTC slow clock sets the jitter (sleep_modes.html ›
  Timer). The owner should measure how far the minute tick drifts over a
  night.

### Power key

- GPIO1 `PWR_OUT` is high while held (`src/power_key.rs:16`).
- Light sleep enables `gpio_wakeup_enable` on GPIO1 high
  (`src/main.rs:2889-2898`).
- Deep sleep: `ext0` on GPIO1, level = 1, or `ext1` if it can be the only
  pin in its mask. This is the one wake that must never be lost
  (`docs/architecture.md:148`).

---

## 5. Plan in small steps

Each step is testable on the device alone. Do not skip the measurements.

### Step 1 — Measure the baseline (no code)

On a battery, with the current firmware:

1. Log time from reset to `epd397-rust-display-ready` (`src/main.rs:600`).
2. Log time from sleep-mode entry (`sleep-mode-enter`, `:1565`) to the
   first light sleep.
3. Measure sleep-mode current (mockup ~8 µA target; today "tens of mA" per
   `mockups/index.html` battery table).
4. Scope GPIO1, GPIO45, and the panel rail (ALDO3) across a sleep cycle.

**Pass:** numbers written into this file's §3 table.

### Step 2 — Identify wake sources on a bench harness

With a test build that only configures sleep (still light sleep at first):

1. Prove `ext0` on GPIO1 (high) wakes a **deep sleep** test stub.
2. Prove `ext1` ANY_LOW on GPIO0 + GPIO4 + GPIO5 + GPIO6 wakes deep sleep.
3. Prove GPIO45 **cannot** be used with `esp_sleep_is_valid_wakeup_gpio` /
   `ext0` (expect `ESP_ERR_INVALID_ARG`).

**Pass:** screenshots or serial lines for each; the GPIO45 failure is the
evidence for the alarm strategy.

### Step 3 — RTC-memory hand-off

Store `restore_route`, wake cause, and next-alarm deadline in
`RTC_DATA_ATTR` before `esp_deep_sleep_start`. On boot, if the reset
reason is deep-sleep wake and the hand-off magic is set, jump to the
sleep-screen renderer.

**Pass:** serial shows a short boot path (no `wifi-radio`, no
`audio-init`) and the sleep picture is already on the panel.

### Step 4 — Timer wake for the clock minute

Arm the RTC timer for the next minute boundary; on timer wake, redraw the
clock (partial), re-arm, deep sleep again.

**Pass:** the clock advances every minute overnight; current drops toward
the mockup figure. Measure the drop.

### Step 5 — Timer wake for weather slots

Same as step 4 for the 2 h weather slot: wake, `weather-cache` update if
due (needs Wi-Fi — a short burst, then deep sleep again), redraw the
weather line.

**Pass:** the sleep weather screen shows a fresh `updated` time after a
slot; the burst's cost is visible in the battery log
(`/RUSTMIX/BATTERY.TXT`).

### Step 6 — Alarm wake via timer

Before deep sleep, read the next alarm from the RTC (or `ALARMS.TXT`).
Arm the timer for that deadline. On timer wake, run the existing alarm
path (`src/main.rs:1187` onward).

**Pass:** an alarm set for +2 minutes fires from deep sleep. Risk: a
snooze or a new alarm written by another path after the timer was armed —
re-read the RTC on every wake before re-arming.

### Step 7 — Full sleep-mode integration

Replace the 60 s light-sleep loop (`src/main.rs:2346`–`:2456`) with:
draw sleep image → save durable state → park `restore_route` in RTC →
configure `ext0`/`ext1`/timer → `esp_deep_sleep_start`.

**Pass:** Power key always wakes; BOOT and wheel wake when
`WakeKeys::AnyKey` (`src/power_settings.rs`); battery log shows deep
sleep current.

### Step 8 — Board change (only if the alarm must wake without a timer)

If step 6's timer is not acceptable, the owner rewires PCF85063 `INT` to
an RTC GPIO (0–21) and this firmware switches the alarm to `ext0`/`ext1`.
No firmware change beyond the pin constant
(`src/rtc_alarm_interrupt.rs:9`) and the wake mask
(`src/main.rs:2457`).

---

## 6. Risks and what to measure

| Risk | Why it matters | Measure / mitigate |
|---|---|---|
| **GPIO45 is not an RTC GPIO** | Alarm cannot use `ext0`/`ext1` | Step 2.3; use timer (step 6) or rewire (step 8) |
| **GPIO38 is not an RTC GPIO** | PMIC IRQ cannot deep-sleep-wake | Power key is already on GPIO1; do not rely on GPIO38 |
| **`ext1` one level for all pins** | Power is high, keys are low | Split: `ext0` on GPIO1, `ext1` ANY_LOW for the rest |
| **Panel needs a full refresh after reset** | Deep sleep powers the panel rail off (`ALDO3`) in photo mode; a wake may not be able to partial-refresh | Step 1 scope of ALDO3; clock/weather modes keep the rail on (`docs/architecture.md:144`) — those may partial-refresh after wake |
| **Wake pad left in RTC IO** | Keys stop working as digital GPIO | `rtc_gpio_deinit` on each wake pad before `poll` |
| **PSRAM lost** | Page caches / photos gone; wake must not need them | Sleep path uses only the cached sleep image (`docs/architecture.md:142`) |
| **SD mount cost** | Every wake remounts FAT | Measure step 3; if too slow, keep the sleep path free of SD after the image is drawn |
| **Timer drift** | Clock minute and alarms slip | Measure overnight; PCF85063 is the source of truth for wall time |
| **Wi-Fi burst during weather slot** | Deep sleep is broken by a 20–90 s burst | Acceptable; log it in the battery log. Do not leave Wi-Fi up |
| **USB / serial** | Light sleep is skipped on USB today (`docs/architecture.md:131`) | Keep that rule: no deep sleep on VBUS |
| **Reset reason ambiguity** | A brownout looks like a wake | `reset-reason` (`src/main.rs:416`) plus an RTC-memory magic |

---

## 7. Open questions for the owner

1. Is the timer-to-next-alarm acceptable for alarms, or must the board
   move PCF85063 `INT` onto an RTC GPIO?
2. In clock + weather modes, can the panel rail stay on (as today) so a
   wake can partial-refresh, or must those modes also drop ALDO3 for the
   last µA?
3. What current does the mockup's 8 µA assume for the PMIC, SD and
   sensors — and which of those rails does Wave actually cut in sleep
   today?
4. Is a 200–800 ms wake-to-sleep-screen acceptable, or should a separate
   "warm" path in RTC memory redraw the clock without a full boot?

---

## References

- ESP-IDF v5.5, Sleep Modes (ESP32-S3):
  <https://docs.espressif.com/projects/esp-idf/en/v5.5/esp32s3/api-reference/system/sleep_modes.html>
- ESP-IDF v5.5, ULP Coprocessor (ESP32-S3):
  <https://docs.espressif.com/projects/esp-idf/en/v5.5/esp32s3/api-reference/system/ulp.html>
- This repo: `docs/architecture.md`, `docs/BOARD_CONTRACT.md`,
  `docs/ROADMAP.md` › Backlog › Deeper sleep,
  `mockups/index.html` › "Batería y memoria"
- Code: `src/main.rs` (boot markers, `light_sleep_until_wake`,
  `SLEEP_WAKE_GPIOS`), `src/power_key.rs`, `src/rtc_alarm_interrupt.rs`,
  `src/sleep_mode.rs`, `src/app/state.rs`
