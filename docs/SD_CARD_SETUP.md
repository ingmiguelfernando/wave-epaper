# SD-card setup

Use a FAT-formatted SD card. Rustmix Wave mounts it at `/sdcard` and expects the following product tree:

```text
/RUSTMIX/
  WIFI.TXT
  WEATHER.TXT
  ALARMS.TXT
  DISPLAY.TXT
  POWER.TXT
  BATTERY.TXT
  BOOKS/
  READER/
    CACHE/
  VOICE/
  SLEEP/
    *.BMP
  APPS/
    HGRID/
    SUDOKU/
    MINES/
    TILTMAZE/
    M2048/
    SOKOBAN/
    DICT/
      INDEX.TXT
      DATA/*.JSN
    CALENDAR/
      EVENTS.TXT
      US2026.TXT
```

## Install bundled examples

```bash
./scripts/install-sd-examples.sh /Volumes/YOUR_SD_CARD
```

Existing paths are preserved by default. Use `--force` only when deliberately replacing bundled example files:

```bash
./scripts/install-sd-examples.sh --force /Volumes/YOUR_SD_CARD
```

The generic installer preserves an existing Dictionary and Calendar tree. Use the dedicated installers for intentional complete-pack replacement.

## Wi-Fi

Copy or edit `/RUSTMIX/WIFI.TXT`:

```text
ssid=YOUR_NETWORK
password=YOUR_PASSWORD
timezone=Pacific/Auckland
ntp_server=pool.ntp.org
```

Supported time zones: `Pacific/Auckland` (default), `America/New_York` and `UTC`.
Do not commit real credentials.

## Weather

Optional `/RUSTMIX/WEATHER.TXT` example:

```text
provider=open-meteo
location=Auckland
latitude=-36.8485
longitude=174.7633
timezone=Pacific/Auckland
refresh_minutes=30
```

## Alarms

Optional `/RUSTMIX/ALARMS.TXT` example:

```text
snooze_minutes=10
alarm=Workday,07:30,weekdays,on,recurring
alarm=Weekend,09:00,weekends,off,recurring
alarm=Appointment,16:45,2026-06-10,on,once
```

Calendar personal events remain separate from alarms.

## Display preferences

`/RUSTMIX/DISPLAY.TXT` supports:

```text
font_family=inter|atkinson-hyperlegible
font_size=compact|standard|large
```

## Power

Settings › Power writes `/RUSTMIX/POWER.TXT`:

```text
auto_sleep=off|5m|10m|15m|30m|1h
wake_keys=any|power
```

`wake_keys=any` lets BOOT and the wheel wake the device; `power` keeps the
Power key as the only wake key. The device also keeps a week of battery levels
in `/RUSTMIX/BATTERY.TXT` for the Power screen chart.

## Sleep images

Put pictures in `/RUSTMIX/SLEEP` (the folder name must be exactly `SLEEP`,
inside `RUSTMIX`). Each one must be an uncompressed Windows BMP:

```text
480 × 800 (portrait) or 800 × 480 (landscape)
1, 4, 8, 24 or 32-bit color
```

Grey and color pictures are dithered to black and white. Files whose names
start with `.` are skipped, such as the `._NAME` copies macOS leaves on FAT
cards. When no picture can be used, the sleep screen shows the reason, for
example `SLEEP.BMP is 1024 × 768; it must be 480 × 800 or 800 × 480`.

Install bundled samples:

```bash
./scripts/install-sleep-images.sh /Volumes/YOUR_SD_CARD
```

## Reader books and state

Copy TXT, EPUB, or FAT-friendly `.EPU` books into:

```text
/RUSTMIX/BOOKS
```

The device creates Reader state automatically:

```text
/RUSTMIX/READER/STATE.TXT
/RUSTMIX/READER/POSITS.TXT
/RUSTMIX/READER/RECENT.TXT
/RUSTMIX/READER/MARKS.TXT
/RUSTMIX/READER/PREFS.TXT
/RUSTMIX/READER/CACHE/<8HEX>.CCH
```

Reader writes use `.TMP` and `.BAK` siblings for recovery.

## Voice Notes

The device creates:

```text
/RUSTMIX/VOICE/VOICE###.WAV
/RUSTMIX/VOICE/INDEX.TXT
/RUSTMIX/VOICE/META.TXT
/RUSTMIX/VOICE/SETTINGS.TXT
```

Do not hand-edit sidecars while the device is active.

## Complete Dictionary pack

Install from a local `rustmix-x4-firmware` checkout:

```bash
./scripts/install-dictionary-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

Verify representative lookups:

```bash
./scripts/verify-dictionary-x4-pack.sh /Volumes/YOUR_SD_CARD
```

## U.S.-only Calendar pack

Install from a local X4 checkout:

```bash
./scripts/install-calendar-x4-pack.sh \
  --force \
  --x4-repo /Users/piyushdaiya/Documents/projects/rustmix-x4-firmware \
  /Volumes/YOUR_SD_CARD
```

The installer includes `EVENTS.TXT` and `US2026.TXT`, and explicitly excludes `HINDU26.TXT`.

Calendar personal-event writes use:

```text
EVENTS.TMP -> EVENTS.TXT
EVENTS.BAK retained for rollback
```
