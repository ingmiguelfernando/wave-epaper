# Wave known issues

Firmware v0.8.1. This is not a list of promised mockup features. See
[the guide](USER_GUIDE.md) and [release hardware checks](PHYSICAL_SMOKE_TEST.md).

## Percent glyph at Detail size

The `%` sign has been observed looking broken at the small Detail font size
on the sleep card, and in the rain row of the weather sleep layout preview. The [roadmap backlog](ROADMAP.md#backlog) records the
Inter 11–12 px atlas as the investigation target, not a confirmed root cause.
Compare both interface families and all three sizes on the actual panel;
do not infer a battery-reading failure from the glyph. No font fix is included
in D4.

## USB flashing and disappearing serial port

On battery, idle light sleep can make USB serial appear and disappear.
Connect a USB-C **data** cable with USB power and press a key once so the
firmware detects VBUS and stops idle CPU light sleep. If the port still does
not appear, hold BOOT while power-cycling, then release it for download mode.
Use the merged Wave `.bin` at `0x0`, as described in
[README](../README.md#flash-the-board). An ELF requires the ELF-aware
`espflash flash` command; do not flash an ELF at a raw address.

## Photos

Photos go to `/PHOTOS` at the card root, which the Wi-Fi transfer portal
cannot reach (it only serves `/RUSTMIX`): copy them with a computer.
Progressive JPEGs over 1 megapixel are refused with a message; save them as
standard (baseline) JPEGs. A starred photo shows at sleep only after Photos
has prepared it once.

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
bounded intervals. Full MCU deep sleep is not implemented. GPIO38 Power and
GPIO45 RTC-alarm wake must be preserved; the mockup's deep-sleep current is
not a measured firmware result. D5 clock/weather layouts are drawing-only,
not selectable sleep modes or scheduled refreshes.

## EPUB scope

Reader extracts reflowable text with TOC, bookmarks and resume. It is not a
full browser: CSS layout, images, interactive hyperlinks/footnotes,
fixed-layout EPUB, DRM and ZIP64 are not supported. Long books are loaded
chapter by chapter; layout changes can still require repagination.

## Runtime alarm edits do not persist

The Alarms editor saves to the running engine only. Edit `/RUSTMIX/ALARMS.TXT`
and reboot for durable schedules. Calendar personal events are separate and
do not arm RTC alarms; U.S. holiday rows are read-only.

## Inherited diagnostic wording

The unused upstream ELF release helpers are not part of Wave's merged-image
workflow. Their retirement is pending the deletions listed in D9's Status in
[delegated tasks](DELEGATED_TASKS.md#d9-retire-the-upstream-release-helpers).
Use the README build and flashing instructions instead.

Some boot log readiness strings also describe old category counts or an
ELF-only release policy. Use current menus, the guide and the README flashing
instructions, not those legacy strings. Correcting runtime text is outside D4.
