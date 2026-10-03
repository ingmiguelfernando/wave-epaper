# Wave changelog

Newest first. Versions and milestone commits follow the
[roadmap status table](docs/ROADMAP.md#status) and Git history; delegated work
does not bump the firmware version. Photos v0.8.0 is on a separate branch,
not a shipped entry here.

## Unreleased — side-tasks (based on v0.7.0)

- D4: refresh the practical guide, known issues, release smoke checklist and
	documentation index; audit obsolete upstream release files for removal.
- D3 (`cd9dd3f`): remove IMU tilt games and Lua motion input; retain Motion
	diagnostics and button-driven Hello Grid, Minesweeper and Sudoku.
- D2 (`2e30703`): remove the unused BLE remote build and its optional runtime
	path, docs and helpers; keep Wi-Fi bursts.
- D5 (`dc5baec`): add drawing-only clock and weather sleep layouts, large
	digits, scaled icons, tests and previews. No runtime sleep-mode selection.
- D1 (`1773deb`, follow-up `27bd2bf`): Display and Reader preference option
	lists; selecting the value already in use closes without unnecessary save,
	ghost clearing or repagination.

## v0.7.0 — Power settings (Phase 1c)

Milestone `d27dced`; follow-ups `fe0b9da`, `8eda12a`, `72e9554`.

- Settings › Power: auto-sleep and wake-key lists, battery history and sleep
	diagnostics; persist power preferences and sample battery every 15 minutes.
- Accept portrait/landscape sleep BMPs, show actionable fallback-card reasons,
	and improve card text. Owner device validation remains pending in the roadmap.

## v0.6.0 — Idle power and radio bursts (Phase 1b)

Milestone `038269f`.

- Light-sleep between presses on battery; keep awake on USB for flashing.
- Use short Wi-Fi bursts for weather/NTP; turn off IMU and codec when idle.

## v0.5.0 — Sleep mode (Phase 1a)

Milestone `f78cb18`.

- Light-sleep while the sleep image is displayed; auto-sleep after ten idle
	minutes; preserve wake keys and RTC alarm handling.

## v0.4.0 — Reader layout (Phase 2d)

Milestone `7b3b112`.

- Wrap by measured pixel width, add Spanish/English hyphenation, and open books
	in one tick. Keep logical reading anchors across layout changes.

## v0.3.0 — Long EPUBs (Phase 2c)

Milestone `55668f2`.

- Load EPUB text one chapter at a time instead of retaining the whole book.

## v0.2.1 — Reader reliability (Phase 2b)

Milestone `23caaa7`.

- Fix EPUB watchdog resets and hide macOS `._` files from the library.

## v0.2.0 — Font generation (Phase 2a)

Milestone `ef529b0`; typography follow-ups include `1bab141`, `54fb828`,
`553d41e` and `6d8d2a5`.

- Generate bitmap font atlases with Latin-1 and punctuation coverage; tune
	rendering and centralize font configuration.

## v0.1.0 — Wave shell (Phase 0)

Milestone `72334ad`; follow-up `0066e71`.

- Add the Wave Home/menu layout and SOON placeholders; support long SD names,
	New Zealand time and metric units; rename the fork.

## Baseline — upstream v1.2.0

Milestone `a67ceb5`: build the inherited Wi-Fi firmware with GitHub Actions.
Wave version numbering starts at v0.1.0 after this baseline.

Older history: [upstream Rustmix Wave](https://github.com/aimindseye/rustmix-wave).
