# Delegated tasks

Small, self-contained tasks for a second developer or AI working in parallel
with the main line (Phase 3, Photos). Read this file first, then
[architecture.md](architecture.md) and [ROADMAP.md](ROADMAP.md).

## How to work

- **Branch.** Create `side-tasks` from the latest `main` and do every task
  there, one commit per task (`D1: ...`, `D2: ...`). Push often.
- **Pull request.** When done, open a pull request to `main` titled
  `Side tasks`. Do not merge it; the main developer reviews it, resolves any
  conflicts with Phase 3 and merges.
- **Report.** Update the **Status** line of each task below in your branch:
  what changed, what needs a test on the device, and anything left open.
- **Order.** D1, D5, D2, D3, then D4. Each task stands alone, so skip a task
  rather than blocking on it.

### Tools

- Host tests and formatting need stable Rust with rustfmt:
  - run `./scripts/test-host.sh`;
  - run `cargo +stable fmt`;
  - both must pass.
- The firmware build (Xtensa) runs on GitHub Actions:
  `gh workflow run firmware.yml -R ingmiguelfernando/wave-epaper --ref side-tasks`
  takes about 8 minutes and must be green before you hand over.
- `ci.yml` runs on the pull request: host tests, the `screen-previews` artifact
  (one PNG per screen) and `cargo fmt --check`.
- Without local Rust, run `ci.yml` on the branch and apply the "Diff in" lines
  from its log exactly as printed.

### Rules

- English everywhere: UI text, code, comments, commits, docs. The device UI is
  terse (`MOVE  SELECT CHANGE  HOLD BOOT BACK`).
- Follow the existing patterns; the closest reference is named in each task.
  No new dependencies unless a task says so.
- Do not bump the version (Cargo.toml, sdkconfig.defaults, build_info.rs); the
  main developer does that when merging.
- Do not add or change `.py` files.
- Comments only where the code cannot speak for itself, one short line.
- Logic changes come with host tests. UI changes come with a preview: add an
  entry at the end of `preview_states()` in `src/app/preview.rs`, then check
  the PNG in the `screen-previews` artifact.
- Phase 3 is changing these files at the same time. Keep edits to them small
  and local:
  - `src/main.rs`;
  - `src/app/state.rs`;
  - `src/app/router.rs` and `src/app/menu.rs`;
  - `src/app/mod.rs` and `src/app/preview.rs`;
  - `README.md`, `Cargo.toml`, `src/lib.rs`.

  Do not edit these at all:
  - `src/sleep_images.rs`, `src/sleep_mode.rs`;
  - `src/power_settings.rs`, `src/battery_log.rs`;
  - `src/app/screens/power.rs`, `src/app/screens/sleep_card.rs`;
  - `docs/ROADMAP.md`;
  - anything named `photo*`.
- Hardware facts and the event loop are in `architecture.md`. The board cannot
  be tested from CI, so list what to try on the device in the Status line.

### Tips from the D1 review

- **Pull first.** The D1 review was pushed to `side-tasks` (this file only).
  Run `git pull --rebase` before you continue.
- **Choosing the value in use does nothing:** no save, no refresh, no
  repagination. This holds for every list you add.
- **Host tests do not compile `src/main.rs`.** Before you remove or rename
  something, run `git grep -n <name> -- src` and read the `main.rs` hits;
  otherwise only the firmware build finds them.
- **Firmware build without `gh`.** Ask the user to start it: GitHub ›
  Actions › firmware › Run workflow › branch `side-tasks`. Hand a task over
  only when it is green.
- **Code shape.** Prefer an enum to a row index (`PowerSetting` is the
  model), `ALL.get(index)` to `ALL[index % len]`, and one generic helper to
  several copies of the same match arm.
- **D2.** No workflow builds with `rustmix-remote-ble`, so the
  `#[cfg(not(feature = "rustmix-remote-ble"))]` code is what runs today:
  keep it and drop only the attribute. The workflows call only
  `scripts/test-host.sh`; still check the remaining scripts before deleting
  one (this applies to D4 too).
- **D5 draws only.** Phase 3a adds Settings › Sleep screen
  (`src/sleep_screen.rs`, route `SleepScreen`) for the photo options. D5
  does not touch settings, routes or `main.rs`.
- **Fewer conflicts with Phase 3a.** `phase3-photos` will probably reach
  `main` first. It changes `state.rs` (`apply`, `open_route`, `back()`),
  `router.rs`, `menu.rs` (Settings gains "Sleep screen"), `preview.rs`,
  `screens/mod.rs` and `lib.rs`. In those files put new code next to related
  code, and do not move, reorder or reformat existing lines.

## D1: Option lists instead of cycling values

**Why.** Pressing Select on a setting steps to the next value, so reaching a
value can take several presses and refreshes. Settings › Power already opens a
list; do the same everywhere else.

**Where values cycle today:**

- **Settings › Display**, rows "UI font" and "UI size". These go through
  `AppState::apply_display` in `src/app/state.rs`, which calls
  `DisplayPreferences::cycle_font_family` and `cycle_font_size`
  (`src/app/display.rs`). The screen is `src/app/screens/display.rs`.
- **Reader › Options › Reading preferences**, all six `ReadingPreference` rows
  in `src/reader.rs`:
  - theme, orientation, book font size, book font, paragraph alignment, show
    progress;
  - each cycles with `.next()` in `ReaderUiState`;
  - state handling is the `ScreenRoute::ReaderPreferences` arm in `state.rs`;
  - the screen is `render_preferences` in `src/app/screens/reader.rs`.

**Change:**

1. Select opens a list of every value for the highlighted row.
2. ▲▼ move through the list, wrapping around.
3. Select applies the value and closes the list.
4. Holding BOOT closes the list without changing anything. Handle this at the
   top of `AppState::back()`, like the `ScreenRoute::Power` case.

Other details:

- Draw the list with `widgets::option_list::draw_option_list` (it marks the
  value in use and scrolls long lists). Put the row name above it, and use the
  footer `MOVE  SELECT CHOOSE  HOLD BOOT CANCEL`.
- Give each enum an `ALL` array and a `label()` if it lacks them (`UiFontFamily`,
  `UiFontSize`, `ReadingTheme`, `ReaderOrientation`, `BookFontSize`,
  `BookFont`, `ParagraphAlignment`). Show progress is On/Off.
- Reference implementation: `PowerUiState`, `AppState::apply_power` and
  `screens/power.rs` (`draw_picker`).
- Keep what happens after a change: Display preferences are saved to
  `DISPLAY.TXT` by `main.rs` (unchanged). Reader preference changes still go
  through `finish_preferences_edit`, so a layout change repaginates the book
  as it does today.
- Remove the cycle functions once nothing calls them.

**Done when:**

- No setting cycles any more.
- Tests cover open, move, apply, cancel and back for both screens.
- Previews `display-picker` and `reader-preferences-picker` exist.
- Existing reader tests still pass.

**Status:** done on `side-tasks` (2026-10-03). Settings › Display and Reader
preferences now open option lists: SELECT opens the list at the value in use,
MOVE wraps, SELECT applies and closes, HOLD BOOT cancels. Both pickers draw
with `widgets::option_list::draw_option_list` and the footer
`MOVE  SELECT CHOOSE  HOLD BOOT CANCEL`. The value enums got `ALL` arrays;
`DisplayPreferences::cycle_font_family/size`, `activate_selected_preference`
and the `ReaderUiState` value-cycling functions are gone (tests that called
them now use the picker API). Tests cover open, move, apply, cancel and back
for both screens; previews `display-picker` and `reader-preferences-picker`
are in the `screen-previews` artifact. Host tests (378) and
`cargo +stable fmt` pass locally. Needs on-device test: both pickers on the
panel (open, scroll, apply, cancel), Display changes still reach
`DISPLAY.TXT`, and layout-sensitive Reader choices still repaginate. Open:
the Xtensa firmware build still has to run green on this branch
(`gh` was not logged in on the machine used for this work).

**D1 follow-up status (2026-10-03):** implemented locally on `side-tasks`,
not committed or pushed. Choosing any current Reader value closes the picker
without saving, requesting ghost clearing, or rebuilding the book. Added the
shared `option_labels` helper and typed `DisplaySetting::{Font, Size}` API;
all D1 option choices use checked `.get()` lookups, including On/Off, and
invalid indices leave preferences unchanged. Removed unused Reader preference
enum `next`/`previous` methods after checking tracked call sites, including
`main.rs`; compatibility tests now check `ALL` order and persisted markers.
Host tests: 386 passed; stable formatting check and `git diff --check` passed.
No changes to D5 files, previews, module registration, or firmware runtime.
Still needs: a firmware build for this uncommitted follow-up and panel smoke
tests; the previously reviewed firmware build does not cover these changes.

**Review (main developer, 2026-10-03):** good work; merge after the fix
below. `ci.yml` (378 host tests, fmt, no warnings) and the firmware build
are green on 1773deb, which closes the open item above. Nothing calls the
removed functions, `main.rs` included, and the previews match the Power
picker.

- **Fix before merge: choosing the value in use must do nothing.**
  - The list opens on the value in use, so pressing Select at once is the
    natural "keep it".
  - `ReaderUiState::choose_preference` treats it as a change. Theme and
    show progress are saved again, and the theme flashes the panel to clear
    ghosting. Orientation, font size, font and alignment reopen the book
    through `ReaderLoading` and repaginate it: seconds of waiting and
    battery on a large EPUB, for nothing.
  - Return early at the top of `choose_preference`:

    ```rust
    if index == self.preference_options().1 {
        self.preferences_picker = None;
        return false;
    }
    ```

  - Add a test: choosing the orientation in use returns `false`, closes the
    list and leaves the preferences unchanged.
  - Display needs nothing: `main.rs` only saves `DISPLAY.TXT` when
    `state.display` changed.
- **Optional polish**, in the same follow-up commit if it is cheap:
  - Five arms of `preference_options` and both branches of
    `DisplayPreferences::options` repeat one pattern. One helper in
    `widgets/option_list.rs` replaces them:

    ```rust
    pub fn option_labels<T: Copy + PartialEq>(
        all: &[T],
        current: T,
        label: fn(T) -> &'static str,
    ) -> (Vec<&'static str>, usize)
    ```

    Call it as
    `option_labels(&BookFont::ALL, self.preferences.book_font, BookFont::label)`.
  - `DisplayPreferences::options` and `choose` take `action: usize`, where 0
    means font and anything else size. A `DisplaySetting { Font, Size }`
    enum like `PowerSetting`, mapped once from the row index in
    `apply_display`, makes every call read clearly.
  - `ALL[index % len]` hides a wrong index by wrapping. Use
    `if let Some(&value) = ALL.get(index)`, as `PowerSettings::choose` does.
- Commit the fix as `D1 fix: ...`; do not amend or force-push the D1 commit.

**Follow-up review results (2026-10-03):** regression tests exercise SELECT
on all six current preferences with an actual open TXT Reader session,
including orientation, and verify unchanged session/page/cache state, no
`ReaderLoading` route, no ghost-clearing request, no persistence event, and
an untouched preferences-file sentinel. Every supported current value and
invalid index is also covered, as are helper fallbacks and Display current
choices. Ordinary UI redraw still closes the picker; no extra preference
refresh is requested. No host failures or editor errors in the changed files.
The main developer review above is retained unchanged.

## D2: Remove the BLE remote build

**Why.** The inherited `rustmix-remote-ble` feature (a BLE page turner for a
Wear OS watch) is not used by Wave and complicates `main.rs`.

**Remove:**

- `src/rustmix_remote/` and `pub mod rustmix_remote;` in `src/lib.rs`.
- The `rustmix-remote-ble` feature in `Cargo.toml`, and its optional dependency
  if nothing else uses it. Let cargo update `Cargo.lock`.
- In `src/main.rs`:
  - every `#[cfg(feature = "rustmix-remote-ble")]` item: the import, the event
    queue, the BLE service start, the queue drain in the loop, and the early
    return in `light_sleep_until_wake`;
  - `#[cfg(not(feature = "rustmix-remote-ble"))]` code becomes unconditional.
- Files: `sdkconfig.defaults.rustmix-remote-ble`, `README_RUSTMIX_REMOTE_R1.md`,
  `README_RUSTMIX_REMOTE_BLE_R1.md`, `docs/rustmix-remote/`.
- Scripts: `build_release_ble_v1_2_0.sh`, `build_rustmix_remote_ble_r1.sh`,
  `validate_rustmix_remote_ble_r1.sh`, `validate_rustmix_remote_rrbp.sh`,
  `release_v1_2_0_ble.sh`, `prepare_dual_release_v1_2_0_docs.sh`.
  - First `grep -r` the name in `.github/` and `scripts/`.
  - Keep any script a workflow still calls, and say so in the Status line.

**Done when:**

- `grep -rni "rustmix.remote\|rrbp" --exclude-dir=target --exclude-dir=.git .`
  finds nothing outside `CHANGELOG.md`.
- Host tests and fmt pass.
- The firmware build is green.

**Status:** implemented on `side-tasks` (2026-10-03), pending firmware
validation. Removed the library module
registration, the optional feature and direct `enumset` dependency, and all
remote runtime imports, queue setup/drain, BLE startup and light-sleep bypass
from `main.rs`. The former non-feature Wi-Fi path is unchanged and now
unconditional. Cargo updated `Cargo.lock` through stable host tests/checks;
`enumset` remains transitively required by the ESP service/HAL dependencies.
Removed the architecture removal note, the Wi-Fi release's BLE recommendation,
and the Wi-Fi builder's obsolete config-backup restore.

All tracked references, including `main.rs`, workflow/script callers and
filename links, were enumerated before deletion. Removed all 24 candidates:
the five live remote module files, three duplicate assistant scaffold files,
two remote READMEs, optional SDK defaults, four remote docs, BLE release notes,
two patch overlays and six task-specified scripts. The reference scan passes
with necessary scan
exceptions are `CHANGELOG.md` history and this task specification, whose
original text is retained unchanged.

Retained `scripts/build_release_wifi_v1_2_0.sh` and
`scripts/release_v1_2_0_wifi.sh` (the latter calls the former), along with the
current build/validation/flash/package helpers. No workflow calls a BLE script;
CI calls `scripts/test-host.sh`, and firmware builds directly with Cargo.
Tests: 389 host tests passed; `cargo +stable fmt --all -- --check`, native
`cargo +stable check --all-targets --all-features`, locked ESP-target Cargo
metadata, and `git diff --check` passed. No version bump, new dependency,
Python edit, protected-file change or unrelated task work.
Firmware validation is pending: only stable Rust is installed locally, and
remote CI cannot validate these uncommitted/unpushed changes. On-device needs:
boot with and without Wi-Fi config, TXT/EPUB page turns using wheel keys,
weather/NTP/file-transfer bursts, battery idle light-sleep, Power/wheel wake
and RTC alarm wake. D2 is not ready for handover until deletions, final scans,
an Xtensa firmware build and device checks are complete.

## D3: Remove the IMU tilt games

**Why.** Wave keeps button games only (Phase 6 adds Sudoku and Tetris). The
IMU should only run on the Motion diagnostic screens.

**Remove:**

- The three games:
  - Motion 2048: `src/games/motion_2048.rs`, `examples/sd-card/RUSTMIX/APPS/M2048/`;
  - Sokoban Tilt: `src/games/sokoban_tilt.rs`, `APPS/SOKOBAN/`;
  - Tilt Maze: `src/games/tilt_maze.rs`, `APPS/TILTMAZE/`.
- Their install scripts (`install-motion-2048-sd-sample.sh`,
  `install-sokoban-tilt-sd-sample.sh`, `install-tilt-maze-sd-sample.sh`) and
  their lines in `install-sd-examples.sh`.
- The Lua `imu` input path: apps with `input = ["imu"]`,
  `AppState::lua_game_needs_imu_events()`, and the tilt events in
  `lua_runtime/event_bridge.rs`. Remove whatever else becomes unused.
- In `main.rs`, `imu_sampling` should then depend only on
  `ScreenRoute::MotionEvents`. Keep that edit minimal.
- Mentions in `docs/USER_GUIDE.md` and the tree in `docs/SD_CARD_SETUP.md`.

**Keep:**

- Settings › Motion and its event diagnostics (`imu.rs`, `imu_events.rs`,
  `screens/motion.rs`).
- Sudoku, Minesweeper and Hello Grid.

**Done when:**

- `grep -rni "tilt\|motion_2048\|sokoban" --exclude-dir=target --exclude-dir=.git .`
  only finds history in `CHANGELOG.md`.
- The Games list still opens Sudoku, Minesweeper and Hello Grid (add a test).
- Host tests, fmt and the firmware build pass.

**Status:** D3 implemented on `side-tasks` (2026-10-03), pending firmware
validation. Removed
the three game registrations and native bridges, Lua motion dispatch and
IMU input capability. `main.rs` samples events only on `MotionEvents`;
Motion/Details power and diagnostics remain unchanged. Removed game entries
from the generic installer, User Guide and SD tree. Added a Games-route test
that scans the real sample manifests and opens Hello Grid, Minesweeper and
Sudoku through the worker/native bootstrap, exercises SELECT/BOOT and back,
and a manifest test rejecting sensor input.

Checks: `./scripts/test-host.sh` passed (378 tests),
`cargo +stable fmt --all -- --check`, `git diff --check` and
`bash -n scripts/install-sd-examples.sh` passed; editor error checks found no
errors in changed files. Reviewed runtime references including `main.rs`,
manifest/catalog schemas, workflow/script callers and the final diff.
No protected files, dependencies, Python files or versions changed.

Removed all 12 obsolete files:
`src/games/{motion_2048,sokoban_tilt,tilt_maze}.rs`,
`examples/sd-card/RUSTMIX/APPS/{M2048,SOKOBAN,TILTMAZE}/{APP.TOM,MAIN.LUA}`,
and `scripts/install-{motion-2048,sokoban-tilt,tilt-maze}-sd-sample.sh`.
Reference scans preserve the diagnostic exceptions below.

Remaining reference exceptions beyond CHANGELOG and this task document:
diagnostic tilt events in `src/imu_events.rs`, `src/app/screens/motion.rs`,
`src/app/state.rs`, `src/main.rs` and the User Guide's Motion event list;
the manifest rejection test deliberately names the removed input capability.
README's removal note, upstream `RELEASE_NOTES-v1.0.0.md`, the architecture's
old IMU-games statement and protected ROADMAP text are left for the planned
documentation follow-up. Protected ROADMAP text remains unchanged.
Firmware validation is pending. Device needs: open and use
all three remaining samples, SELECT/short BOOT/hold BOOT, Motion/Details live
readings, MotionEvents tilt/shake/rotate/level and threshold/reset controls,
and IMU off outside diagnostics including Games. Handover requires a green
Xtensa firmware build; physical checks remain with the owner.

## D4: Documentation refresh

Do this last so it describes the code after D1 to D3.

**Write or update:**

- **`docs/USER_GUIDE.md`.** Rewrite it for Wave as it is now.
  - Cover keys, Home, Library and the Reader, Settings (Display, Power, Network,
    Clock, Alarms, Audio), sleep and wake, and the SD card layout (link
    `SD_CARD_SETUP.md`).
  - Use the `screen-previews` artifact and the code as the source of truth.
  - Keep it practical: what a key does on each screen.
- **`CHANGELOG.md`.** Replace the upstream content with Wave's history, newest
  first. Take the versions and commits from the Status table in `ROADMAP.md`
  and `git log`. Keep one line linking to upstream Rustmix Wave for older
  history.
- **`docs/KNOWN_ISSUES.md`.** Only real, current issues: the `%` sign at the
  Detail font size, flashing needs USB power or BOOT held (see README), and
  anything you find.
- **`docs/PHYSICAL_SMOKE_TEST.md`.** A checklist for each release:
  - boot to Home;
  - open a book and turn pages;
  - Settings › Power lists;
  - hold Power to sleep, then wake with Power and with a wheel key;
  - the sleep card when `/RUSTMIX/SLEEP` is empty;
  - weather refresh (Wi-Fi burst);
  - an RTC alarm while asleep;
  - flashing over USB.
- **Upstream leftovers.** Remove `RELEASE_NOTES-v1.0.0.md`, `docs/releases/`,
  and upstream-only release docs and scripts (`docs/RELEASE.md`,
  `*_v1_2_0*.sh`, `package-release.sh`, `release*.sh`).
  - Only remove what no workflow or remaining script uses; check with
    `grep -r <name> .github scripts`.
  - Update the "Documentation" section of `README.md` to match. Leave the
    rest of the README alone.

**Done when:** links between docs work, and nothing removed is still referenced.

**Status:** not started.

## D5: Sleep screen layouts (drawing only)

**Why.** Phase 3b adds clock and weather sleep screens. The main developer
wires up when they show and how they refresh; this task only draws them.
Mockup: "Reposo: reloj y fecha" and "Reposo: clima" in `mockups/index.html`.

**Add:**

- `src/app/screens/sleep_screens.rs`, registered in `src/app/screens/mod.rs`,
  with exactly this API. Field examples are in the comments.

  ```rust
  pub struct SleepClock<'a> {
      pub time: &'a str,                        // "13:42"
      pub date: &'a str,                        // "Friday, October 2"
      pub weather: Option<SleepWeatherLine<'a>>,
      pub battery_percent: Option<u8>,
      pub wake_hint: &'a str,                   // "Press any key to wake"
  }

  pub struct SleepWeatherLine<'a> {
      pub weather_code: u16,                    // WMO code
      pub summary: &'a str,                     // "18° · Partly cloudy"
      pub details: &'a str,                     // "H 21° · L 11° · Rain 10%"
  }

  pub struct SleepWeather<'a> {
      pub place: &'a str,                       // "Madrid"
      pub updated: &'a str,                     // "Fri, Oct 2 · updated 13:30"
      pub weather_code: u16,
      pub temperature: &'a str,                 // "18°"
      pub condition: &'a str,                   // "Partly cloudy"
      pub details: &'a str,                     // "H 21° · L 11° · Wind 12 km/h · Rain 10%"
      pub days: &'a [SleepWeatherDay<'a>],      // up to 3
      pub battery_percent: Option<u8>,
      pub wake_hint: &'a str,
  }

  pub struct SleepWeatherDay<'a> {
      pub name: &'a str,                        // "Sat"
      pub weather_code: u16,
      pub range: &'a str,                       // "23° / 12°"
      pub rain: &'a str,                        // "0%"
  }

  pub fn render_sleep_clock(
      display: &mut OrientedFrameBuffer<'_>,
      preferences: DisplayPreferences,
      clock: &SleepClock<'_>,
  ) -> Result<(), Infallible>;

  pub fn render_sleep_weather(
      display: &mut OrientedFrameBuffer<'_>,
      preferences: DisplayPreferences,
      weather: &SleepWeather<'_>,
  ) -> Result<(), Infallible>;
  ```

- Frame-level wrappers in `src/app/mod.rs` next to `render_sleep_card`
  (`render_sleep_clock` and `render_sleep_weather` taking `&mut FrameBuffer`).
  They clear the frame and draw in portrait.
- `src/app/widgets/big_digits.rs`: large numerals drawn with embedded-graphics
  primitives, because the fonts stop at about 30 px.
  - Seven-segment style with slightly rounded or beveled segments, stroke
    about 1/8 of the height.
  - Supports `0-9`, `:`, `°` and `-`.
  - API: `draw_big_text(display, text, center_x, top, height)` returning the
    drawn width, plus `big_text_width(text, height)`.
  - The clock uses about 150 px digits, the weather temperature about 110 px.
- `Icon::draw_scaled(display, top_left, scale, color)` in
  `src/app/widgets/icons.rs` (nearest neighbour), for weather icons at 2× to 5×.

**Layout** (portrait 480 × 800, black on white):

- Both screens have the same 3 px frame as the sleep card (inset 16 px).
- Both have a bottom row inside the frame: the wake hint on the left, battery
  % on the right.
- **Clock screen**, top to bottom:
  1. the time, centered, around y 180–330;
  2. the date in the Large style, centered;
  3. a horizontal rule;
  4. if `weather` is set: the 2× icon with the summary (Large), and the details
     (Body) below.
- **Weather screen**, top to bottom:
  1. place (Detail) and updated line (Body), top left;
  2. a big icon (about 5×) beside the big temperature, with the condition
     under it;
  3. the details centered;
  4. three day columns (name, 2× icon, range, rain).

**Tests and previews:**

- Previews `sleep-clock`, `sleep-clock-weather` and `sleep-weather`. Add them
  in `render_screen_previews` next to `sleep-card`, the same way.
- One render test per layout that checks a few pixels (frame, a digit
  segment).
- A test of `big_text_width`.

**Done when:** the previews look like the mockup, all text fits at every
Display size (Compact, Standard, Large), and the tests pass.

**Status:** implemented locally on `side-tasks` (2026-10-03), not committed
or pushed. Added the exact drawing API and portrait frame wrappers, rounded
seven-segment digits (`0-9`, `:`, `°`, `-`), allocation-free width measurement,
and transparent nearest-neighbour icon scaling (zero scale is a no-op).
Both layouts use the inset-16, 3 px frame and measured, bounded text; the wake
hint and battery percent have separate footer budgets. Long temperatures shrink
to fit beside the 5× icon; forecasts show at most three 2× icon columns.
Host tests: 399 passed (13 D5 tests added), including frame/digit pixels,
glyph widths, icon scaling, all six size/family combinations and extreme text.
Stable full formatting check, native library check, `git diff --check` and
editor diagnostics pass. Required previews and 18 typography variants are in
`/tmp/wave-d5-previews`; required images and Compact/Large family samples were
visually inspected without overlaps. No settings, routes, runtime, dependencies,
versions or prohibited files changed; D1 review remains unchanged.
Still needs: Xtensa firmware build and on-device checks of frame/digit clarity,
both font families at all sizes, negative temperatures and footer readability.
Sleep-mode selection, refresh scheduling and wake behavior remain Phase 3b work.
