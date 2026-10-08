# Wave known issues

Firmware v0.9.7. This is not a list of promised mockup features. See
[the guide](USER_GUIDE.md) and [release hardware checks](PHYSICAL_SMOKE_TEST.md).

## Percent glyph at Detail size

In Inter at the Standard size, the Detail strike (12 px) draws `%` without its
slash. Inter Compact and Large and every Atkinson size are fine. Task D6 found
no `fonts.toml` setting that fixes it without changing other glyphs, so
screens show percentages in Body size instead (sleep card battery, weather
rain chances). Do not infer a battery-reading failure from the glyph.

## USB flashing and disappearing serial port

On battery, idle light sleep can make USB serial appear and disappear.
Connect a USB-C **data** cable with USB power and press a key once so the
firmware detects VBUS and stops idle CPU light sleep. Since v0.9.5 sleep mode
also stays out of light sleep while USB power is present, so the console stays
connected while the sleep picture shows. If the port still does not appear,
hold BOOT while power-cycling, then release it for download mode.
Use the merged Wave `.bin` at `0x0`, as described in
[README](../README.md#flash-the-board). An ELF requires the ELF-aware
`espflash flash` command; do not flash an ELF at a raw address.

## Photos

Photos go to `/RUSTMIX/PHOTOS` (the root `/PHOTOS` of v0.8.0 to v0.9.3 is no
longer read). Progressive JPEGs over 1 megapixel are refused with a message;
save them as standard (baseline) JPEGs. A starred photo shows at sleep only
after Photos has prepared it once.

## Weather provider reliability

Open-Meteo can fail with transport, TLS, timeout or HTTP errors. Bounded
retries/backoff retain the last good result in memory; a cold boot without a
successful request shows "No forecast yet" with the error under it. Once a
forecast exists, the second Weather page (Down) shows the last error. Wi-Fi
is normally off between bursts and weather is paused during sleep-image mode;
neither is a continuous-connection guarantee. The forecast is not saved to the
SD card, so a reboot starts without one.

## MCU deep sleep

The panel deep-sleeps with its rail off, but the CPU uses light sleep in
bounded intervals. Full MCU deep sleep is not implemented. Power-key wake on
GPIO1 (also high during an RTC alarm) and GPIO38, and RTC-alarm wake on
GPIO45 must be preserved; the mockup's deep-sleep current is not a measured
firmware result. D5 clock/weather layouts are drawing-only, not selectable
sleep modes or scheduled refreshes.

## EPUB scope

Reader extracts reflowable text with TOC, bookmarks and resume. It is not a
full browser: CSS layout, images, interactive hyperlinks/footnotes,
fixed-layout EPUB, DRM and ZIP64 are not supported. Long books are loaded
chapter by chapter; layout changes can still require repagination.

## Runtime alarm edits do not persist

The Alarms editor saves to the running engine only. Edit `/RUSTMIX/ALARMS.TXT`
and reboot for durable schedules. Calendar personal events are separate and
do not arm RTC alarms; U.S. holiday rows are read-only.
