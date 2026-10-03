# Wave user guide

This guide describes firmware v0.9.1: what is implemented, not the future
mockup. Current menus and key handlers are authoritative. For visual
references, download the `screen-previews` artifact from a green
[host CI run](https://github.com/ingmiguelfernando/wave-epaper/actions/workflows/ci.yml).
Its `sleep-clock` and `sleep-weather` images are layouts for a later release,
**not modes you can choose yet**.

## Controls

The wheel has Up, Select and Down keys; there is no touch input.

| Key | Action |
|---|---|
| Up / Down | Move a highlighted row; in a book, previous / next page |
| Select | Open the highlighted row or run its action |
| BOOT short | Contextual action only: Calendar agenda, keyboard axis, game axis/cancel, star a photo, preview the sleep picture, or switch °C/°F on Weather |
| Hold BOOT (at least 900 ms, then release) | Back one level; cancel an open option list before leaving its screen |
| Power short | Open the display-maintenance menu: Clear ghosting now or Cancel |
| Hold Power | Enter sleep-image mode; after releasing and waiting for the wake guard, Power wakes it |

Wheel actions are processed after release; holding a wheel key does not repeat.
Follow the footer for screen-specific actions. A short BOOT press is not a
general Back key. On editors, holding BOOT cancels unsaved text; settings
already applied with Select remain applied.

## Home

Home shows time, battery, Wi-Fi burst state, date, weather and a Continue
Reading summary. Settings › Weather can hide the weather from Home. The
summary card is informational: use **Library › Continue Reading** to resume.
Up/Down wraps through the nine application rows; Select opens the highlighted
one.

| Home row | Current behavior |
|---|---|
| Photos | Gallery of the JPEG photos in `/PHOTOS`; see [Photos](#photos) |
| Library | Continue Reading, Books, Bookmarks |
| Bible | SOON placeholder |
| Reading Stats | SOON placeholder |
| Weather | Now, next hours and next days; see [Weather](#weather) |
| Games | SD Games catalog |
| AI | XiaoZhi (SOON) and implemented Voice Notes |
| Tools | File Browser, Dictionary, Unit Converter, Calendar |
| Settings | Alarms, Audio, Clock, Display, Device Info, Environment, Motion, Network, Power, Sleep screen, Weather |

Hold BOOT to return from a category or placeholder. On Home, Back does nothing.
Tetris, AI transcription, OTA updates and the clock or weather sleep screens
are not implemented yet.

## Photos

Copy JPEG photos into `/PHOTOS` at the root of the SD card, next to
`RUSTMIX` (details in [SD-card setup](SD_CARD_SETUP.md#photos)).

- **Gallery.** Six thumbnails per page, newest first. Up/Down moves through
  the photos across pages; Select opens the viewer; short BOOT stars or
  unstars the highlighted photo; hold BOOT returns Home. The status row shows
  the photo count, the starred count and the page; the line under the grid
  shows the name, size and date.
- **Preparation.** Each photo is prepared once while Photos is open, a few
  seconds for a 12 MP photo, and its thumbnail appears when ready. The keys
  keep working meanwhile. A photo that cannot be used says why, for example a
  progressive JPEG over 1 megapixel.
- **Viewer.** The photo fills the screen as set by Settings › Sleep screen ›
  Fit. Up/Down shows the previous or next photo; short BOOT stars; Select
  opens the actions; hold BOOT returns to the gallery.
- **Actions.** Add to (or Remove from) sleep set; Use only this photo, which
  unstars the others; Delete photo, which asks again: Select deletes the file,
  hold BOOT cancels.

Starred photos form the sleep set: with Settings › Sleep screen › Source on
**Starred photos**, one of them shows each time the device sleeps.

## Library and Reader

Put `.TXT`, `.EPUB` or `.EPU` books in `/RUSTMIX/BOOKS/` on the SD card.
See [SD-card setup](SD_CARD_SETUP.md) for formats and generated state files.
Long file names are supported; hidden macOS `._` files are ignored.

1. Open **Home › Library › Books**.
2. Up/Down selects a row. Select on **Change tab** cycles Recent, Books,
	 Files and Bookmarks; Select on a book or bookmark opens it.
3. During Opening Book, wait for the first page or hold BOOT to cancel.
	 EPUB text is loaded chapter by chapter, rather than loading an entire
	 long book into memory.
4. In the reading view, Up goes back a page, Down goes forward, Select opens
	 Reader Options, and hold BOOT returns to the book list.

**Continue Reading** resumes the saved book position (or returns to the list
when nothing is saved). **Bookmarks** opens saved anchors. Positions, recent
books, bookmarks and preferences persist in `/RUSTMIX/READER/`; page numbers
can change when typography changes, while anchors retain the reading position.

### Reader Options

Up/Down chooses an action; Select runs it. Hold BOOT returns to the page.
Actions add/remove the current bookmark, show bookmarks, open the EPUB Table
of Contents, open Reading Preferences, clear ghosting, go to Library or go Home.
TXT has no chapter TOC. In bookmarks or TOC, Select jumps to the highlighted
entry and hold BOOT returns to Options.

### Reading Preferences

Up/Down chooses a setting; Select opens its option list at the value in use.
Within the list, Up/Down wraps, Select applies and closes, and hold BOOT
cancels without changing the value. `IN USE` identifies the applied choice.
Selecting that same choice closes the list without saving, clearing ghosting
or repaginating the book.

| Setting | Choices |
|---|---|
| Reading Theme | Classic, High Contrast |
| Orientation | Portrait, Landscape (book pages only) |
| Book font size | Small, Medium, Large, XLarge |
| Book font | Inter, Atkinson, Serif, Literata |
| Paragraph alignment | Justified, Left, Center, Right |
| Show progress | On, Off |

Layout changes may reopen/repaginate the book; wait for loading. Theme and
progress changes do not rebuild the layout. These are Reader preferences,
independent of the interface font in Settings › Display. Hold BOOT again
after closing a picker to leave Reading Preferences.

## Settings

Up/Down chooses one of the ten rows, Select opens it, hold BOOT returns Home.

### Display

Choose **UI font** (Inter / Atkinson Hyperlegible) or **UI size** (Compact /
Standard / Large). Select opens the list, Up/Down moves, Select applies,
hold BOOT cancels. Choices persist in `/RUSTMIX/DISPLAY.TXT`; a missing file
uses Inter / Standard. This changes interface text, not book typography.

### Power

The overview shows battery percent, voltage, USB/charging state, a 24-hour
chart, the last sleep report and the light-sleep share. Samples are taken
every 15 minutes; up to seven days are kept in `/RUSTMIX/BATTERY.TXT`.

Up/Down chooses **Auto-sleep** or **Wake keys**. Select opens the list;
Up/Down moves, Select applies, hold BOOT cancels.

- Auto-sleep: Off, 5 min, 10 min (default), 15 min, 30 min, 1 hour.
- Wake keys: Any key (default), or Power key only.
- Choices persist in `/RUSTMIX/POWER.TXT`.

### Sleep screen

Up/Down chooses **Source**, **Order** or **Fit**; Select opens the list,
Up/Down moves, Select applies, hold BOOT cancels. Choices persist in
`/RUSTMIX/SLEEPSCREEN.TXT`.

- Source: Starred photos (default) or Sleep folder (`/RUSTMIX/SLEEP/`).
- Order: Shuffle (default) or In order.
- Fit: Fill (crop, default) or Whole photo. It also applies to the Photos
  viewer.

A short BOOT press shows the picture the next sleep would use; any key
returns to the settings.

### Network and Wi-Fi transfer

Configure `/RUSTMIX/WIFI.TXT` on a computer and reboot to apply changes.
Network shows Wi-Fi state, SSID, IPv4, RSSI and NTP state. **Provisioning
details** is read-only, not an on-device credential editor.

Wi-Fi normally runs in short bursts for weather and time sync, then turns off.
An idle/off Wi-Fi indicator does not itself mean provisioning failed.

1. Choose **Start Wi-Fi Transfer** with Up/Down and Select.
2. Wait for the portal URL and six-digit session code.
3. From a browser on the same LAN, open that URL and enter the code.
4. Use the portal for permitted upload, download, rename, mkdir and delete
	 operations inside `/RUSTMIX`; internal configuration/state paths are protected.
5. Select **Stop and return**, or hold BOOT, to stop the portal and return.

The portal is temporary, plain HTTP and LAN-only, not an Internet service.
It also stops on sleep, Wi-Fi loss or inactivity. Do not expect it to run
automatically at boot or while the device sleeps.

### Clock

Shows localized RTC time/date and board status. Select opens RTC details;
hold BOOT returns one level. This is a read-only screen, not a clock editor.
Time zone and NTP server come from `WIFI.TXT`; supported zones and examples
are in [SD-card setup](SD_CARD_SETUP.md).

### Alarms

Persistent schedules and snooze minutes come from `/RUSTMIX/ALARMS.TXT`.
Calendar events do not create alarms.

- List: Up/Down chooses a schedule; Select opens the runtime editor.
- Editor: Up/Down changes hour, minute, enable state, recurrence or schedule;
	Select advances fields. On **Save runtime edit**, Select applies it to the
	running session. For changes that survive reboot, edit `ALARMS.TXT`.
- Hold BOOT leaves the screen; short BOOT is not an alarm-editor Back action.
- Active alarm: Up/Down chooses Snooze or Dismiss; Select runs it.

An RTC alarm can wake sleep-image mode and open Alarms. Check clock accuracy,
the RTC-armed indicator and audible output before relying on an alarm.

### Audio

Up/Down chooses Play test chime, Stop playback, Increase volume, Decrease
volume, Mute/Unmute or Audio details; Select runs it. Hold BOOT returns.
The codec is suspended when no audio is needed. Details are diagnostic: they
show the bidirectional I2S link and the microphone input that Voice Notes
records through.

### Device and sensor diagnostics

- **Device Info:** Select advances Firmware → Board services → Runtime
	services. Hold BOOT walks back through those pages.
- **Environment:** temperature/humidity; Select opens sensor details.
- **Motion:** live accelerometer/gyroscope; Select opens Motion Events.
	Up/Down selects threshold/debounce/reset/details controls; Select changes
	the selected control or resets counters. Events are TILT, SHAKE, ROTATE and
	LEVEL. Hold BOOT returns one level.

The IMU is enabled only on Motion diagnostics. There is no motion-game or
Lua sensor-input path. Threshold controls are diagnostic, not game settings.

## Weather

Open Home › Weather. The first page shows the current conditions, today's
high and low, feels-like temperature, humidity, wind and chance of rain, every
second hour for the next twelve hours, and the next four days. The bottom line
says when the forecast was updated and when the next update is due.

- Down opens the second page: each of the next twelve hours, the place and
  time zone, and the last error if an update failed. Up returns.
- Select updates now, using a Wi-Fi burst.
- A short BOOT press switches between °C with km/h and °F with mph. The
  choice is saved in `WEATHER.TXT` and also applies to Home, Clock and
  Environment.
- Hold BOOT returns to Home.

Clear and partly cloudy nights show a moon. A failed update keeps the last
good forecast and the bottom line says so. No weather updates run in
sleep-image mode, and the forecast is kept in memory only, not on the SD card.
Without `/RUSTMIX/WEATHER.TXT` the screen explains how to add it; see
[SD-card setup](SD_CARD_SETUP.md#weather).

### Settings › Weather

| Setting | Choices |
|---|---|
| Weather service | On or Off. Off makes no weather requests and hides weather on Home |
| Update every | 30 minutes, 1 hour, 2 hours (default), 6 hours, or Manual (only when you press Select on Weather) |
| Units | °C · km/h or °F · mph |
| Show on Home | Yes or No |

Select opens a list, Up/Down moves, Select applies and hold BOOT cancels.
Turning the service on updates straight away. Choices are saved by rewriting
`WEATHER.TXT`; the location and coordinates are edited in that file. Each
update keeps Wi-Fi on for a few seconds, about 0.1–0.2 mAh, so every 2 hours
costs about 2 mAh a day.

## Games

Install the bundled SD samples, then open **Home › Games › SD Games**.
Up/Down chooses an app, Select opens it, hold BOOT returns to the catalog.
The supplied games are Hello Grid, Sudoku, Minesweeper and Tetris; there are no
IMU-controlled games or BLE remote page-turner in this build.

| Sample | Controls |
|---|---|
| Hello Grid | Static canvas demonstration; wheel/Select have no game action; hold BOOT exits |
| Sudoku | Up/Down moves along the active H/V axis; short BOOT switches axis. Select enters edit mode; Up/Down chooses a candidate; Select commits; short BOOT cancels editing |
| Minesweeper | Up/Down moves along the active axis; short BOOT switches axis. Select enters action mode; Up/Down chooses Reveal/Flag; Select applies; short BOOT cancels action mode |
| Tetris Zen | Up moves left, Down right, Select rotates, short BOOT drops and locks. No gravity: the piece moves only on a press. Game over shows the score; Select starts a new game |

These are button-driven samples. The Sudoku three-step redesign and Tetris
Classic gravity belong to Phase 6. A missing/invalid SD app reports an error;
hold BOOT returns to the catalog.

## AI › Voice Notes

XiaoZhi remains a SOON placeholder. Voice Notes records local PCM16 mono
16 kHz WAV; it does not transcribe or summarize recordings.

- List: Up/Down chooses Record new note, microphone gain or a saved note;
	Select starts recording, cycles gain or opens details.
- Recording: Up/Down toggles pause/resume; Select stops and saves; hold BOOT
	cancels. The screen reports duration, peak and clipping.
- Saved note: Up/Down chooses Play/Stop, Edit friendly title, Export/download,
	Delete or Return; Select runs the action. Delete asks for confirmation.
- Title keyboard: Up/Down moves in the active axis; short BOOT toggles
	NAV H / NAV V; Select activates a key, SAVE or CANCEL. Hold BOOT cancels.

WAV names stay `VOICE###.WAV`; friendly titles are sidecar metadata in
`/RUSTMIX/VOICE/`. Export uses the temporary LAN portal. Do not edit sidecars
or remove the SD card during recording/playback/writes.

## Tools

| Tool | Practical controls |
|---|---|
| File Browser | Up/Down selects; Select enters a folder or opens/closes a bounded text preview; hold BOOT goes to the parent, then Tools. Read-only; binary preview is unavailable |
| Dictionary | Up/Down moves on the keyboard; short BOOT switches NAV H/V; Select activates letters, DEL, CLR, GO or `*`. GO uses exact lookup with prefix fallback; `*` performs prefix lookup/result cycling. Requires the Dictionary SD pack |
| Unit Converter | Up increases and Down decreases the active value; Select advances Category, From unit, Value, To unit, Step size; hold BOOT returns |
| Calendar | Up/Down changes selected day or month; Select toggles Day/Month navigation; short BOOT opens the daily agenda |

In Calendar agenda, Up/Down selects an event, Select opens details, and short
BOOT creates a personal event. Personal details offer edit/delete actions;
U.S. holidays are read-only. In the event keyboard, short BOOT switches NAV
H/V, Select activates keys, FIELD changes title/detail, SAVE writes and CANCEL
or hold BOOT cancels. Calendar writes `EVENTS.TXT` with temporary/backup files.
Hold BOOT returns through agenda/month view to Tools.

## Sleep, wake and ghosting

A short Power press opens the maintenance menu. Up/Down selects **Clear
ghosting now** or **Cancel**, Select runs it, and hold BOOT cancels. Clearing
ghosting performs a full refresh and returns to the underlying screen.

Holding Power, or reaching the Auto-sleep delay, draws the sleep picture set
in Settings › Sleep screen: a starred photo once Photos has prepared it, or a
BMP from `/RUSTMIX/SLEEP/`, which is also the fallback. Shuffle avoids showing
the same picture twice in a row. If no picture can be used, a sleep card shows
the reason, battery and wake hint. BMP requirements are in
[SD-card setup](SD_CARD_SETUP.md#sleep-images).

Sleep mode retains the previous route, stops Wi-Fi/transfer/weather activity,
turns off the IMU, suspends idle audio, and deep-sleeps the panel with its
rail off. The CPU uses **light sleep**, not full MCU deep sleep.

Release the entry key and wait for the wake guard. Power always wakes;
BOOT/Up/Select/Down also wake with **Wake keys: Any key**. With **Power key
only**, wheel/BOOT presses do not wake. A wake press restores the previous
screen rather than activating its highlighted action. An RTC alarm is a
separate wake source.

The clock, clock-with-weather and weather images in `screen-previews` are
layouts for a later release; they cannot be selected yet.

While awake but idle on battery, the CPU also light-sleeps between work;
the panel can power down without changing the displayed page. USB power
disables idle CPU light sleep for flashing/serial use: after connecting,
press a key once. See [README flashing instructions](../README.md#flash-the-board)
and [known issues](KNOWN_ISSUES.md).

## Related documents

- [SD-card setup](SD_CARD_SETUP.md): folders, configuration and installers.
- [Physical smoke test](PHYSICAL_SMOKE_TEST.md): owner checks before a release.
- [Known issues](KNOWN_ISSUES.md): current limitations and workarounds.
- [Architecture](architecture.md): hardware, event loop and power policy.
- [Wave history](../CHANGELOG.md) and [roadmap](ROADMAP.md): shipped vs planned work.
