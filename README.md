# Wave e-Paper

Custom firmware for the [Waveshare ESP32-S3-ePaper-3.97](https://docs.waveshare.com/ESP32-S3-ePaper-3.97)
board (800 × 480 e-paper, buttons only), written in Rust on top of ESP-IDF.

This project is a fork of [Rustmix Wave](https://github.com/aimindseye/rustmix-wave) (MIT),
reworked phase by phase as listed below.

## Roadmap

- [x] Baseline: upstream v1.2.0 (Wi-Fi) built by GitHub Actions
- [x] Phase 0: long SD file names, project rename (code cleanup moved to the backlog)
- [ ] Phase 1: power (light sleep, auto-sleep, button interrupts, short Wi-Fi bursts)
- [x] Phase 2: typography (Latin-1 fonts, large EPUBs, pixel-width wrapping, ES/EN hyphenation)
- [ ] Phase 3: Photos and configurable sleep screens
- [ ] Phase 4: Weather app
- [ ] Phase 5: Bible reader and Reading Stats
- [ ] Phase 6: Games (Sudoku, Tetris)
- [ ] Phase 7: AI (Voice Notes with OpenAI-compatible providers, XiaoZhi)
- [ ] Phase 8: OTA updates

### Backlog

Unscheduled improvements:

- Reading Preferences: pick a font size or font from a list instead of cycling through the values.
- Remove inherited code Wave no longer uses (tilt games, BLE remote).

## Get the firmware

Every push to `main` builds the firmware with GitHub Actions; tags named `v*` also publish a release.

- **Releases**: download `wave-epaper-<version>.bin` from the
  [Releases](https://github.com/ingmiguelfernando/wave-epaper/releases) page.
- **Latest build**: open [Actions › firmware](https://github.com/ingmiguelfernando/wave-epaper/actions/workflows/firmware.yml),
  pick the latest green run and download the artifact (a zip that contains the `.bin`).

Each build also includes the matching `.elf` (for decoding crash backtraces) and `SHA256SUMS.txt`.

## Flash the board

You need a desktop Chrome or Edge browser and a USB-C data cable.

1. Open the Espressif web flasher: <https://espressif.github.io/esptool-js/>
2. Connect the board, click **Connect** and select its serial port.
   If the port does not show up, hold **BOOT**, power-cycle the board, then release **BOOT**.
3. Set **Flash Address** to `0x0`, choose `wave-epaper-<version>.bin` and click **Program**.
4. When it finishes, power-cycle the board.

The image contains the bootloader, partition table and application, so it always goes to `0x0`.
Flashing resets settings stored in internal flash; files on the SD card are not touched.

Command-line alternative:

```sh
esptool.py --chip esp32s3 write_flash 0x0 wave-epaper-<version>.bin
```

## Build locally (optional)

1. Install Rust, then the Xtensa toolchain: `espup install --targets esp32s3 --std`
2. Install the helpers: `cargo install ldproxy espflash`
3. Build: `cargo build --release` (ESP-IDF v5.5.1 is downloaded on the first build)
4. Create the image:

```sh
espflash save-image --chip esp32s3 --merge --skip-padding --flash-size 16mb \
  target/xtensa-esp32s3-espidf/release/waveshare-epd397-rust-app wave-epaper.bin
```

Host tests run on stable Rust: `./scripts/test-host.sh`

## Conventions

- The device UI is in English; books in Spanish and other Latin-script languages must render correctly.
- All code, comments, commit messages and documentation are written in English.

## Upstream documentation

The original Rustmix Wave guides are kept in [docs/](docs/) for reference
(architecture, board pins, SD card layout, user guide).

## License

MIT, see [LICENSE](LICENSE). Original work © 2026 Piyush Daiya.
