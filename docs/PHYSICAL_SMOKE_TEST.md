# Wave release physical smoke test

Run on the Waveshare board for each release and after cross-cutting runtime
changes. CI proves neither panel quality nor power/wake behavior. Use the
[current guide](USER_GUIDE.md), [SD setup](SD_CARD_SETUP.md) and
[known issues](KNOWN_ISSUES.md), not future mockups.

Record: date, firmware version/commit, firmware Actions run ID, board, SD card,
battery level, USB/battery power, tester, failures and serial log/photo evidence.
Unchecked boxes mean **not tested**, not passed. Back up SD settings and books
before tests; never remove the card while writes are active.

## Build, USB flash and boot

- [ ] Host tests, stable format check and `git diff --check` pass for the exact
	source being released; firmware Actions build is green for that commit.
- [ ] Download the matching merged `.bin`, `.elf` and checksum file from the
	firmware artifact; verify the image checksum.
- [ ] Follow [README flashing](../README.md#flash-the-board): USB-C data cable,
	USB power, press a key after plugging in; merged `.bin` at `0x0`.
- [ ] If normal serial discovery fails, hold BOOT during a power cycle,
	release it, flash and power-cycle again. Record which path was needed.
- [ ] Boot reaches Home without reset/panic loops; Device Info shows the
	expected release version (currently v0.9.2), not an upstream v1.0.0 marker.
- [ ] Up/Down/Select and hold BOOT work through Home/category navigation;
	Bible, Reading Stats and XiaoZhi remain SOON.
- [ ] Every screen ends with the same bottom bar of key caps, and its labels
	match what the keys do. At Large size in both font families nothing is
	cut off at the right edge.
- [ ] Boot without Wi-Fi configuration still reaches usable offline Home.

## Books and Display (D1)

- [ ] Open a TXT and a long EPUB/`.EPU`; turn pages both ways, cross an EPUB
	chapter, and cancel an Opening Book operation with hold BOOT.
- [ ] Latin-1 accents, Spanish/English wrapping/hyphenation and long SD names
	are readable; macOS `._` files do not appear as books.
- [ ] Bookmark add/remove, bookmark jump, EPUB TOC and Continue Reading work;
	reboot restores the saved position.
- [ ] Reader's six preference lists open at `IN USE`, wrap with Up/Down,
	apply with Select and cancel with hold BOOT before leaving the screen.
- [ ] Select every current Reader choice: list closes without a loading screen,
	extra full ghost refresh or preference-file write. Change a layout value and
	verify repagination retains the reading anchor.
- [ ] Settings › Display lists both fonts and all three sizes; open/move/apply/
	cancel work, `DISPLAY.TXT` changes only when needed and survives reboot.
- [ ] Review Home, lists, book text and footers at both families/all UI sizes.

## Photos and sleep screen

- [ ] Copy a few phone JPEGs (one 12 MP, one portrait) and one progressive
	JPEG into `/PHOTOS`. Open Photos: thumbnails appear one by one while the
	keys stay responsive; the progressive file shows its reason.
- [ ] Photos are upright in the viewer; Up/Down change photo with a full
	refresh; hold BOOT returns to the gallery without a ghost of the photo.
- [ ] Short BOOT stars and unstars in the gallery and the viewer; the starred
	count changes and `STARRED.TXT` survives a reboot.
- [ ] Viewer actions: Use only this photo works; Delete photo asks again, hold
	BOOT cancels, Select deletes the file and its star.
- [ ] Settings › Sleep screen: change Source, Order and Fit; `SLEEPSCREEN.TXT`
	survives a reboot. Short BOOT previews the next sleep picture; any key
	returns.
- [ ] With two or more starred photos, sleep and wake several times: a starred
	photo shows each time, without immediate repeats on Shuffle.
- [ ] With no starred photo, sleep falls back to `/RUSTMIX/SLEEP/`, or to the
	sleep card that explains why.

## Power lists, sleep and wake

Perform battery-only checks unplugged; USB intentionally suppresses idle CPU
light sleep. Restore the owner's original preferences afterward.

- [ ] Settings › Power opens Auto-sleep and Wake keys lists at the applied
	value; Up/Down wraps, Select applies and hold BOOT cancels.
- [ ] Exercise Off and a timed auto-sleep choice; confirm `POWER.TXT` and
	persistence after reboot. Restore the intended delay (default 10 min).
- [ ] Short Power opens maintenance, Cancel returns without sleep, and
	Clear ghosting now performs a full refresh and restores the prior route.
- [ ] Hold Power from a Reader page, release and wait for the guard: sleep
	picture appears, no immediate false wake, network/portal stop.
- [ ] Wake with Power and confirm previous route/page is restored.
- [ ] With Any key, repeat sleep/wake separately using Up, Down, Select and
	BOOT. The wake press must not also change page or activate an action.
- [ ] With Power key only, wheel/BOOT do not wake; Power still does.
- [ ] With `/RUSTMIX/SLEEP/` **empty**, enter sleep: fallback card shows a
	useful reason, battery and the matching wake hint; Power/wheel wake work.
- [ ] Repeat with a missing folder and an invalid-size BMP; reason is useful.
	Restore valid portrait and landscape BMPs; hidden files are skipped and
	multiple images do not immediately repeat.
- [ ] Inspect the `%` glyph at Detail size on the card; record the known
	defect for both fonts/sizes, without treating it as battery telemetry failure.
- [ ] Battery chart gains samples after 15 minutes; last-sleep report and
	light-sleep share update. Check idle light sleep on battery vs awake USB.

## Weather

- [ ] With valid `WIFI.TXT`/`WEATHER.TXT`, Weather shows now, the next hours
	and four days; the bottom line reads `Updated HH:MM · next HH:MM ·
	Open-Meteo`. After dark, clear hours show a moon.
- [ ] Select updates now with a Wi-Fi burst; the radio returns to idle after.
- [ ] Down opens twelve hours, Up returns; hold BOOT returns to Home.
- [ ] Short BOOT switches °C/°F on both pages; Home, Clock and Environment
	follow, and the choice survives a reboot (`units=` in `WEATHER.TXT`).
- [ ] Settings › Weather: change Update every, Units and Show on Home; the
	file is rewritten and the values survive a reboot. Show on Home No hides
	the strip temperature and the row value.
- [ ] Service Off: Weather says it is off, Home shows no weather, and the
	serial log shows no `weather-fetch` for an interval. Service On updates
	straight away.
- [ ] Manual: no automatic `weather-fetch` after boot or an interval; Select
	on Weather still updates.
- [ ] Failed/offline update keeps the shell responsive and the last good
	forecast; the second page shows the last error.

## Settings files (D15)

- [ ] Change Display, Power and Weather settings and star a photo, then
	reboot: every value survives, and `RUSTMIX/` holds no `.TMP` or `.BAK`
	files afterwards.
- [ ] Pull the power right after a settings change (repeat a few times):
	the next boot keeps either the old or the new value, never defaults.
- [ ] The serial log at boot has no `rustmix-wave=*-ready` design markers,
	only results such as SD mounted or codec found (D17).

## Network and alarm while asleep (D2)

- [ ] Inspect NTP/Clock status.
- [ ] Start LAN transfer explicitly, authenticate with its displayed code,
	upload/download a test file; stop with Select and separately with hold BOOT.
	Protected paths remain protected; sleep shuts the portal down.
- [ ] Arm a near-future RTC alarm with verified clock time, enter sleep on
	battery, and wait: alarm wakes to Alarms and sounds. Test Snooze and Dismiss.
- [ ] Repeat the asleep alarm with Power key only; the key policy must not
	block RTC wake. Confirm alarm UI is not hidden by the maintenance menu.
- [ ] Runtime alarm edits work, but reboot reloads `ALARMS.TXT`; durable edits
	are made in that file. Calendar events do not create RTC alarms.

## Remaining games and diagnostics (D3)

- [ ] Install current examples; Games › SD Games lists Hello Grid, Sudoku,
	Minesweeper and Tetris without the removed sensor-controlled samples.
- [ ] Hello Grid draws its static canvas; hold BOOT returns to the catalog.
- [ ] Sudoku: Up/Down movement, short BOOT H/V toggle, Select edit, candidate
	choice, commit, short BOOT cancel and hold BOOT exit. The bottom bar
	switches between move/edit and number/save/cancel right away.
- [ ] Minesweeper: movement/axis toggle, Reveal/Flag action choice, cancel,
	first-reveal safety and hold BOOT exit. The bottom bar follows the mode.
- [ ] Tetris Zen: Up/Down move, Select rotates clockwise, short BOOT drops;
	the bottom bar reads move, rotate, drop; no ghosting builds up (full
	refresh every 20 pieces); game over, Select restarts, hold BOOT exits.
- [ ] Move the board during games: no sensor-driven movement; IMU stays off
	outside Motion diagnostics (use runtime logs/power evidence).
- [ ] Motion and Motion details show live readings; Motion Events reports
	TILT/SHAKE/ROTATE/LEVEL and threshold/debounce/reset controls work. Leaving
	diagnostics stops IMU sampling/enables power-down.
- [ ] Environment and details show live temperature/humidity; Clock and the
	three Device Info pages open/back correctly.
- [ ] Audio test chime, stop, volume and mute work; codec suspends when idle.

## Other implemented SD tools

- [ ] AI › Voice Notes: record, pause/resume, save, playback, friendly-title
	edit/cancel, delete confirmation and LAN export; WAV survives reboot.
- [ ] Dictionary with a full pack: exact lookup, prefix fallback and `*`
	cycling; keyboard short BOOT switches H/V without moving the selected key.
- [ ] Tools › Calendar: day/month, agenda, personal create/edit/delete;
	holidays stay read-only, SAVE persists and hold BOOT cancels unsaved edits.
- [ ] File Browser folder/text preview/back and Unit Converter fields work.

## D5 drawing evidence only

- [ ] Review `sleep-clock`, `sleep-clock-weather`, `sleep-weather` and font/
	size variants in `screen-previews`: frame, digits, negative temperatures,
	icons and wake/battery footer are readable. A hardware render harness may
	check panel clarity separately; ordinary firmware cannot select these modes.

Do not mark clock/weather sleep refresh scheduling, Tetris Classic or other
planned features as passed. Record failures/open checks in
[delegated task status](DELEGATED_TASKS.md) and the release handover.
