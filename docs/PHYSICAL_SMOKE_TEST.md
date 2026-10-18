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
	expected release version (currently v0.12.5), not an upstream v1.0.0 marker.
- [ ] Up/Down/Select and hold BOOT work through Home/category navigation;
	Reading Stats opens its screen.
- [ ] Bible: with a translation in `/RUSTMIX/BIBLE`, Home › Bible opens the
	picker first; short BOOT moves to the next section, ● opens a book's
	chapters, short BOOT jumps ten chapters and hold BOOT returns to the
	books. Choose a chapter, page with ▼ (next) and ▲ (back) across a
	chapter boundary, hold BOOT to leave, power-cycle, and Home › Bible
	reopens the same page. With `VERSES.TXT` and the clock set, the menu's
	`Versículo del día` opens the page with today's verse. Without a card it
	shows the Spanish missing view.
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
- [ ] Settings › Sleep screen lists five modes with a cost on the right;
	Select on Verse does nothing (`SOON`). Choosing Clock shows Refresh under
	`CLOCK OPTIONS`; Settings shows `Clock · 1 min`; `SLEEPSCREEN.TXT`
	keeps `mode=` and `clock_refresh=` after a reboot. Short BOOT previews
	the clock.
- [ ] Clock & date on battery: sleep, then watch a minute change. The time
	updates within a second of the minute without a black flash; after 30
	minutes one full refresh. The serial log shows `sleep-redraw
	layout=clock refresh=partial`. Every 5 minutes: changes on 5, 10, 15.
- [ ] Power and (with Any key) BOOT/wheel still wake from a clock sleep and
	restore the screen with one full refresh; an RTC alarm still rings.
- [ ] Weather mode with weather On: sleep, wait for an update (or set 30 min):
	the log shows a `wifi-burst` and `weather-fetch status=completed`, then
	`sleep-redraw layout=weather`. Without Wi-Fi the clock shows until a
	forecast arrives; a forecast older than six hours says `stale`.
- [ ] Overnight in Clock & date, Settings › Power's battery chart drops about
	as the mode's cost says (about 13 mAh a day at 1 min).

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
- [ ] Alarm edits saved in the editor survive a reboot (see Round 7).
	Calendar events do not create RTC alarms.

## Round 7 (D34 to D38)

- [ ] Reader page: the header shows the title, time and battery over a rule;
	the text fills the page down to the bottom bar; the bar shows `p. N · P%`
	(TXT) or `Ch. N · P%` (EPUB), where P grows as you read. Bookmark a page:
	a ribbon appears at the top right without covering the battery. Repeat
	in landscape: the bar and the label sit at the bottom of the panel.
- [ ] Alarms: change an alarm's time in the editor and Save; reboot; the
	new time is still there. `ALARMS.TXT` on the card shows the saved line.
- [ ] Library: short BOOT moves RECENT, ALL, BOOKMARKS, FILES in order; each
	book shows its percent or `new`; Select opens a recent book at its place.
- [ ] Hold ▲ or ▼ on a long list (Settings, Library, Bible books): the cursor
	keeps moving after half a second. On a reading page, one press turns one
	page, with no repeat.
- [ ] Bible: with a second translation on the card, Menu › Traducción lists
	both with the one in use marked; choosing another keeps the place when
	its book is there.
- [ ] Bible: a verse that does not fit a line breaks with a hyphen on a
	Spanish translation; the next page turn does not stall (one chapter read
	per chapter). Versículo del día opens on the page that holds the verse.

## Round 9 (D45 to D48)

- [ ] Home › Library opens the books at once: RECENT when there is a recent
   book, else ALL. Hold BOOT there returns Home; hold BOOT on a reading page
   returns to the books; Reader Options › Bookmarks returns to Options. The
   Home card still shows the book to resume, and RECENT's top row opens it
   at the place the card shows.
- [ ] Reader Options, Reading Preferences, the table of contents, the
   bookmark list, Opening Book, Settings › Reading and Settings › Display
   draw with the black status bar (name left, a short value right) and
   label-left / value-right rows; long titles cut before the value; a list
   that does not fit scrolls with the selection on screen at every UI size
   (Settings › Display changes the size).
- [ ] Settings › System › Memory shows the internal RAM free, its largest
   block, the lowest free since boot, the PSRAM free and its lowest, and the
   main task's stack margin. The numbers move as the device works and the
   lows never rise.
- [ ] Reader Options › Go to on a long TXT book: the list marks the place in
   use; choosing 50% opens near the middle at a line start with the `41+`
   style label, and the place survives a reboot. On an EPUB, 50% opens the
   chapter holding the middle of the flattened text.

## Round 10 (D49 to D50)

- [ ] Games › Race opens the mode list (Classic, Zen) with each mode's best
   distance; ● starts the chosen one. The road runs right to left: cars and
   cones arrive in the last column and move one column toward you each step.
- [ ] Classic: a step a second at speed 1, 100 ms less each kilometre (the
   `Speed N at X km` line and its bar show the next one). ▲▼ change a lane,
   ● spends a nitro charge for three steps at once (the charges show as
   blocks), short BOOT pauses (`PAUSED`) and the step clock stops. Zen: the
   road moves only on a key and there is no nitro.
- [ ] A crash shows the distance, the best and how far it was, top speed,
   cars passed and nitro used, then `Race again`, `Mode` and `Back to
   Games`. `Race again` starts a new run; `Mode` reopens the list.
- [ ] Leave the game: `RECORDS.TXT` on the card holds `race_classic` and
   `race_zen` in metres, and the Games hub card shows `Classic · best …`
   over `Zen · best …`.
- [ ] The road redraws only the cars that moved (less ghosting); a full
   refresh every 30 steps clears what is left.

## Round 11 (D51 to D53)

- [ ] Games hub: with the sample card's five games, ▼ moves the cards one
   at a time and the selected one is always drawn; ▼ on the last wraps to
   the first with the list at the top.
- [ ] Race playing screen: the counters read `DISTANCE`, `BEST` and
   `NITRO` with their values under them and the charges as blocks at the
   right edge; the road has the block rumble strips, the edge lines and
   the dashed lane lines; other cars are outlines, the player's car is
   solid with speed streaks behind it and the dashed outline shows where
   ▲ goes. Zen counts `CARS PASSED` and its bar says `step`.
- [ ] `SPEED 4 AT 3.0 KM` and the bar under the road, and the info box
   with the mode's own text. Pause shows `PAUSED` and the road stands.
- [ ] The crash screen: the picture of the impact, the distance large and
   centred, `Best … · … to go`, the three stat columns and the three rows
   like the settings lists; ▼ moves the row, ● runs it.
- [ ] The playing screen and the crash screen at the Large UI size: the
   labels and values fit and nothing runs into the road.

## Round 12 (D54 to D57)

- [ ] Voice note recording: `● REC · Voice Notes` and the battery in the
   status bar, the time large, the meter's newest bar at the right moving
   with your voice (one bar a second; quiet room against speech), and the
   info box's size, free space and hours. Short BOOT pauses (`PAUSED`
   chip and title) and resumes; ● stops, and the screen then says
   `Voice Notes` with only `hold: back`.
- [ ] Record for more than 30 seconds: the meter keeps one bar a second
   and scrolls left, it does not speed up.
- [ ] Voice note result: the summary's bullets, the bold heading and the
   to-dos with drawn boxes, wrapped lines under the text, the text one
   size larger than before; ▲▼ scroll to the last to-do.
- [ ] Home rows: Photos shows how many photos the card holds, the Bible
   row the place read last (`Sal 23`), Games the games count and the AI
   row `2 apps`.
- [ ] Settings › Sleep screen: `Verse of the day` reads `1 WAKE-UP/DAY`
   with `VERSES.TXT` on the card and can be chosen; without it, it reads
   `NO VERSES` and cannot.
- [ ] Sleep with the verse mode: the date and a rule at the top, the
   verse with `Sal 23:1 · RVR1960` right under it, both centred, and the
   footer with the battery and the wake hint. The serial log shows the
   next redraw at local midnight, and the next day shows the next verse.
   A card without the list draws the clock instead.

## Forecast, verse mark and Bible stats (v0.12.5)

- [ ] With a forecast on Home, restart the device with Wi-Fi off: Home and
   Weather show the last forecast, and the Weather footer says `stale`.
   The next update makes it normal again.
- [ ] Settings › Weather: change the place in `WEATHER.TXT`, restart: the
   old forecast is not shown.
- [ ] Bible › menu › `Versículo del día`: a bar stands in the left margin
   beside the verse (both verses of a range, also on the next page); ▲▼
   keep it; `Ir a capítulo` clears it.
- [ ] Read the Bible for a few pages, sleep and wake: Reading Stats has the
   minutes and pages, and the current book is unchanged.
- [ ] (v0.12.6) Turn to the last page of a book: Reading Stats' `books
   finished` beside this year goes up by one. `STATS.TXT` has a
   `done|<path>|<date>` line for it, and its `book|` line still ends in `1`.

## Update over Wi-Fi (v0.14.0)

Needs two releases: flash v0.14.0 by USB (the v0.13.0 flash notes below
apply), then publish a tag `v0.14.1` (the tag must equal `Cargo.toml`'s
version, which sets the version inside the image).

- [ ] Settings › System › Check for updates with Wi-Fi off in the settings of
   the portal: `No Wi-Fi` and the installed version still 0.14.0.
- [ ] With Wi-Fi set and v0.14.0 the latest release: `You have the latest
   version` and the time; Wi-Fi goes off after the check (Network shows Off).
- [ ] With v0.14.1 published: the screen offers 0.14.1 with its size; with the
   battery under 30% and no USB, `Download and install` shows `Battery NN%`
   and nothing downloads.
- [ ] On USB power, `Download and install`: the percentage climbs, the screen
   redraws about every 10%, then `Installed and verified`; the boot log has
   `rustmix-wave=ota status=installed`.
- [ ] Hold BOOT during the download: `Download stopped`; Device Info still
   says 0.14.0 and a restart boots it as before.
- [ ] `Restart now`: the device starts 0.14.1 (Device Info), the log shows
   `status=image-confirmed result=0` about 30 s later, and Check for updates
   says `You have the latest version`.
- [ ] Rollback: publish a release whose image crashes at start (or reset the
   board three times in the first 30 s of a new image): the bootloader
   returns to the previous version.

## Two app slots (v0.13.0), flashed once by USB

- [ ] Flash the merged image at `0x0` from v0.12.x without erasing the flash.
   The boot log shows `rustmix-wave=ota status=image-confirmed result=0`
   about 30 s after boot (result 0 is OK).
- [ ] Settings still there (display, Wi-Fi, weather) and the AI keys row still
   says set (the NVS partition kept its place); the clock sleep screen and
   alarms work as before.
- [ ] Device Info shows the version 0.13.0.
- [ ] Reset the board before 30 s have passed: it boots the same image again
   (there is no older one to return to yet).

## `%` at the small size (v0.12.7)

- [ ] Sleep card (no `SLEEP.BMP`): `Battery 78%` shows a whole `%` with its
   slash and two open rings, in the small size.
- [ ] Weather sleep screen: the rain chances under each day read `80%`.
- [ ] Library rows and the Reader's page label show `12%`; Reading Stats'
   `+15% vs last` and the current-book line too.
- [ ] A note's result screen says `SAVED AUDIO`; the notes list says `SD
   AUDIO RECORDINGS`.

## Games after Photos (v0.12.3) and a new Sudoku (v0.12.4)

- [ ] Right after boot, Home › Games opens a game.
- [ ] Open Photos with new photos on the card and wait until their
   thumbnails appear, go back to Home, then open a game: it starts, with
   no `not enough free memory` message. If one shows, note the numbers on
   Settings › System › Memory.
- [ ] Sudoku: start a new Easy, Medium and Hard game. Each starts without
   a restart; then note the main task stack margin on Settings › System ›
   Memory.

## Voice note volume (v0.11.1)

- [ ] Play a note: ▲ makes it louder and ▼ quieter while it plays, the
	Volume line follows, ● stops, and Settings › System › Audio shows the
	same volume. When the note ends, ▲▼ move through the actions again.

## Voice notes to the AI providers (v0.11.0)

- [ ] First boot: the serial log shows `rustmix-wave=nvs status=ready` and
	then a Wi-Fi burst that connects; Settings › AI › API keys reads
	`Not set`, not `Unavailable`. A second boot shows the same.
- [ ] Portal › AI keys: save a transcription and a summary key; the page
	says both are set and never shows them. After the portal stops, API keys
	reads `Set`. Remove one: it reads which key is missing.
- [ ] With `process=online`, record a 20-second note: about 8 seconds after
	the last key press the list shows `TRANSCRIBING…`, then `SUMMARIZING…`,
	then `SUMMARY`, and the note takes the summary's title. Its result shows
	the summary, the transcript and the providers.
- [ ] A wrong key gives `FAILED`; the SUMMARY tab shows the provider's
	reason, and Retry after fixing the key finishes the note.
- [ ] Record while a note is sent: the sent note goes back to `QUEUED` and
	finishes after the recording is saved. Power-key sleep does the same.
- [ ] A note longer than 10 minutes goes in two parts and the transcript
	holds both.
- [ ] Summarize again on a summarized note changes only the summary.

## Display numerals and weather icons (v0.10.5)

- [ ] Clock sleep mode: the bold digits change every minute without a
	shadow of the previous time worth worrying about, and the 30-minute
	full refresh clears what is left. The time stays centred as digits
	change (11:11, 18:48).
- [ ] Weather and the weather sleep mode: the large icon and the
	temperature are smooth, a below-zero temperature shows its minus sign,
	and a clear night shows the moon.

## Round 8 (D39 to D43)

- [ ] An EPUB chapter opens with its title and its rule, and the title is
	not repeated in the text; the footer shows minutes left after 20
	recorded pages and drops them below. Old bookmarks still open.
- [ ] Library rows show the author under the title, a drawn check on a
	finished book and the percent on an in-progress one (EPUB too, after it
	is opened once). Home's Continue card shows an EPUB's percent.
- [ ] Settings › Reading: wide margins push the text in, Hyphenation Off
	wraps whole words, stats Off records no time and no pages. Hold BOOT in
	an option list closes the list, not the screen.
- [ ] Alarms: without `ALARMS.TXT`, New alarm creates the file; the editor
	deletes one and the next alarm moves; deleting the last alarm leaves
	the RTC idle; a broken ALARMS.TXT is never overwritten.
- [ ] Bible: two translations, Menu › Traducción switches and keeps the
	place; the menu draws with the card idle.

## Remaining games and diagnostics (D3)

- [ ] Install current examples; Home › Games lists one card per game —
	Hello Grid, Sudoku, Minesweeper and Tetris — without the removed
        sensor-controlled samples. The status bar shows `Games` and `4`.
        Each card shows an icon (also on the selected, black card), its name
        and its state: Sudoku `Medium · in progress N/81` (or `No game in
        progress`) plus its best time; Tetris `Zen · best N` with thousands
        separators. Beating a best time updates the card after the next hub
        refresh. The info box shows both of its lines.
- [ ] Tetris: Games › Tetris opens the mode list `Zen · no best yet` and
        `Classic · no best yet`. Choose Classic: the piece falls one row a second
        with no key pressed, the title reads `Tetris · Classic`, and leaving the game
        saves the Classic best (check the card after reboot).
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

- [ ] Settings › AI: with `/RUSTMIX/AI.TXT` copied from the example, the AI row
        reads `Ready`; without it, `Not set up`. Choose Language: the list opens
        on the value in use, choosing another saves it, and it is still there
        after a reboot.
- [ ] Home › AI hub: XiaoZhi card with `NOT LINKED` (or `LINKED`), Voice
	Notes card, `RECENT NOTES` with the three newest (newest first, `Oct 2 ·
	12:04 · 18 min` stamps), the SD-card info box and the `move / open / BOOT
	new note` bar. Select opens a XiaoZhi call, the list or the chosen note's
	details; short BOOT starts a recording directly (the recording screen
	counts up without another press). With no notes the hub still renders,
	and the status bar says `Online` or `Offline`.
- [ ] XiaoZhi, first call: `Connecting…`, then `Link this device` with a
	six-digit code. Add the device at xiaozhi.me with the code; the call goes
	on without a key press, and after hanging up the hub card says `LINKED`
	(also after a reboot). Hold BOOT on the code screen cancels at once.
- [ ] XiaoZhi call: ask a question; the chips go LISTEN › THINK › SPEAK, the
	question and the reply show as text, the reply is heard clearly and the
	call listens again by itself. ● during a reply stops it; ▲▼ change the
	volume (the bar and the sound); short BOOT shows `MIC OFF` and XiaoZhi
	hears nothing until the next short BOOT. Ask "read me Psalm 23" with a
	Bible on the card: the reply reads it and `Device tool:
	bible.read(PSA 23) from your SD` shows. Hold BOOT hangs up; the Audio
	screen works afterwards (test chime) and Wi-Fi switches off after the
	burst. The status bar shows `On call · mm:ss` and the battery.
- [ ] XiaoZhi without Wi-Fi set up says `Wi-Fi is not set up`; with the
	router off it says `Wi-Fi did not connect`. A five-minute call does not
	sleep the panel or the device, and the serial log shows no task watchdog.
- [ ] AI › Voice Notes: record, pause/resume, save, playback, friendly-title
	edit/cancel, delete confirmation and LAN export; WAV survives reboot.
- [ ] AI result: a note with a `VOICE###.AI` record shows its state label on
    the hub and in the list; its result opens with the summary title, tabs
    switch on BOOT, Up/Down scroll, Select opens the note's actions. A note without a
    record opens its details. Put a hand-made record on the card to check.
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
