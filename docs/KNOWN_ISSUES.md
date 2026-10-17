# Wave known issues

Firmware v0.12.5. This is not a list of promised mockup features. See
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
retries/backoff retain the last good result, and since v0.12.5 it is saved
to `/RUSTMIX/FORECAST.TXT`, so a restart shows it as `stale` until the next
update succeeds. A first boot without a successful request shows "No
forecast yet" with the error under it. Once a
forecast exists, the second Weather page (Down) shows the last error. Wi-Fi
is normally off between bursts and weather is paused during sleep, except in
the Weather and Clock + weather sleep modes; neither is a continuous-connection
guarantee. A forecast saved for another place is ignored.

## MCU deep sleep

The panel deep-sleeps with its rail off (in the clock and weather sleep
modes the rail stays on so the panel keeps its memory), but the CPU uses
light sleep in bounded intervals. Full MCU deep sleep is not implemented.
Power-key wake on GPIO1 (also high during an RTC alarm) and GPIO38, and
RTC-alarm wake on GPIO45 must be preserved; the mockup's deep-sleep current
is not a measured firmware result. The sleep modes' battery figures are
estimates until the battery log confirms them on the device.

## Voice Notes AI

- Notes go as WAV (about 1.9 MB a minute), so a long note takes a while to
  upload and uses battery; the mockup's Opus recording is not done yet
  (the Ogg Opus file format is, in v0.12.5, but nothing records or sends it).
- Keys typed in the portal cross the LAN as plain HTTP, like the files the
  portal moves. Type them on a home network you trust.
- Recording, playback, the transfer portal or sleep stops the note being
  sent; it goes again later from its first part.
- The first start of 0.11.0 burns eFuse key block 5 for NVS encryption,
  and the Wi-Fi settings saved by older firmware are dropped (Wi-Fi
  rebuilds them). A chip whose block 5 already holds another key cannot
  open NVS: Wi-Fi then runs without saved calibration and the portal says
  key storage is unavailable.

## XiaoZhi

- The call is half duplex, as xiaozhi-esp32 runs without echo
  cancellation: the microphone is off while XiaoZhi speaks, so speaking over
  it does nothing; ● interrupts.
- The screen shows Latin-1 text only. A conversation in Chinese or with
  emoji keeps its audio, but those characters are left out of the text.
- Only the official server (`api.tenclass.net`, linked at xiaozhi.me) is
  supported; a self-hosted server cannot be set yet.
- The server sends 24 kHz speech; the device decodes it at its 16 kHz
  speaker rate.
- Speech still in the speaker's buffer when a reply ends (under 0.1 s) is
  cut when the microphone opens again.
- Not yet tested on the device; see the physical smoke test.

## EPUB scope

Reader extracts reflowable text with TOC, bookmarks and resume. It is not a
full browser: CSS layout, images, interactive hyperlinks/footnotes,
fixed-layout EPUB, DRM and ZIP64 are not supported. Long books are loaded
chapter by chapter; layout changes can still require repagination.

