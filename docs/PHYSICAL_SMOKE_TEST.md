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
	expected release version (currently v0.9.7), not an upstream v1.0.0 marker.
- [ ] Up/Down/Select and hold BOOT work through Home/category navigation;
	Bible and XiaoZhi remain SOON. Reading Stats opens its screen.
- [ ] Reading Stats: read a page or two, leave the Reader, reopen the screen —
	today's minutes and pages grow; the Home row shows `N-day streak` from
	the second day above five minutes; the Continue card shows
	`· M min today`. With the clock unset, reading records nothing
	(`STATS.TXT` untouched). `STATS.TXT` appears only after a save (five
	minutes, Reader close or sleep), never per page.
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
- [ ] Settings is one page of seven groups, each row showing a live value:
	`Inter · Standard`, `Photo · N starred`, `On · 2 h`/`Manual`/`Off`/
	`Not set up`, `Off`/`Not set up`/the network name, `No alarms`/
	`N alarms`, the battery percent and the version (also on the status
	bar). Clock & alarms opens a two-row list (Clock, Alarms); System
	opens a four-row list (Device Info, Audio, Environment, Motion); hold
	BOOT from every sub-screen walks back to Settings and then Home.
- [ ] Review Home, lists, book text and footers at both families/all UI sizes.

## Photos and sleep screen

- [ ] Copy a few phone JPEGs (one 12 MP, one portrait) and one progressive
	JPEG into `/RUSTMIX/PHOTOS`. Open Photos: thumbnails appear one by one while the
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
- [ ] Leave a screen alone for over a minute (the panel powers down), then
	press Power briefly: the menu still appears. Device Info page 3 shows
	`Power key: Ready`.
- [ ] With the serial console, tap Power: the log shows
	`power-key-gpio down=true`, `down=false`, then
	`power-key event=short-press source=gpio1`, and no `boot-button` line.
- [ ] Hold Power from a Reader page for about a second, release and wait for
	the guard: sleep picture appears, no immediate false wake, network/portal
	stop.
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

- [ ] Install current examples; Home › Games lists one card per game —
	Hello Grid, Sudoku, Minesweeper and Tetris — without the removed
        sensor-controlled samples. The status bar shows `Games` and `4`.
        Each card shows an icon (also on the selected, black card), its name
        and its state: Sudoku `Medium · in progress N/81` (or `No game in
        progress`) plus its best time; Tetris `Zen · best N` with thousands
        separators. Beating a best time updates the card after the next hub
        refresh. The info box shows both of its lines.
- [ ] Hello Grid draws its static canvas; hold BOOT returns to the hub.
- [ ] Sudoku start list: `New · Easy/Medium/Hard`, plus `Continue · …`
        after leaving an unfinished game. Two `New · Medium` games in a row
        are different puzzles; `New · Hard` appears within a second or two.
        The title bar shows `Sudoku · Medium` and the time on the right; the
        time grows between presses (at most a minute per press). Hold BOOT
        mid-game, reopen: `Continue` restores the board and the time, also
        after a reboot. Place a number and press the reset key without
        leaving the game: after the reboot `Continue` keeps that number.
        Solving says `Solved in … · new best`; the hub card
        shows the best time and `No game in progress`.
- [ ] Sudoku three-step entry: ▲▼ row (given rows skipped), Select confirms;
        ▲▼ cell (givens skipped), Select confirms; ▲▼ number or erase (×),
        Select places. The row frame and the inverted cell are easy to see;
        short BOOT goes back one step; the step chips and the bottom bar
        follow each step; a conflict keeps the number step and hold BOOT
        returns to the hub.
- [ ] Minesweeper: movement/axis toggle, Reveal/Flag action choice, cancel,
	first-reveal safety and hold BOOT exit. The bottom bar follows the mode.
- [ ] Tetris Zen: Up/Down move, Select rotates clockwise, short BOOT drops;
        the bottom bar reads move, rotate, drop; no ghosting builds up (full
        refresh every 20 pieces); game over, Select restarts, hold BOOT exits.
        Best: beat the saved score, leave, reopen (Best shows it), reboot
        (still there); a lower score writes nothing (one log save).
- [ ] Move the board during games: no sensor-driven movement; IMU stays off
	outside Motion diagnostics (use runtime logs/power evidence).
- [ ] Motion and Motion details show live readings; Motion Events reports
	TILT/SHAKE/ROTATE/LEVEL and threshold/debounce/reset controls work. Leaving
	diagnostics stops IMU sampling/enables power-down.
- [ ] Environment and details show live temperature/humidity; Clock and the
	three Device Info pages open/back correctly.
- [ ] Audio test chime, stop, volume and mute work; codec suspends when idle.

## Other implemented SD tools

- [ ] Home › AI hub: XiaoZhi card with SOON, Voice Notes card, `RECENT
	NOTES` with the three newest (newest first, `Oct 2 · 12:04 · 18 min`
	stamps), the SD-card info box and the `move / open / BOOT new note` bar.
	Select opens the placeholder, the list or the chosen note's details;
	short BOOT starts a recording directly (the recording screen counts up
	without another press). With no notes the hub still renders, and the
	status bar says `Online` or `Offline`.
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
