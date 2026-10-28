# Releasing

A release is a tag `vX.Y.Z`. The `firmware` workflow builds it and publishes
the merged image (flash at `0x0`), the app image `-app.bin` (what an update
writes), `SHA256SUMS.txt` and the ELF. A device accepts an update only when
the tag equals the version inside the image, so the three places below must
agree (v0.19.0 shipped with 0.18.1 inside and every update answered `Wrong
image`).

1. **Version in three places**, all `X.Y.Z`:
   - `Cargo.toml` `version`
   - `Cargo.lock` (the `waveshare-epd397-rust-app` entry; `cargo check` rewrites it)
   - `sdkconfig.defaults` `CONFIG_APP_PROJECT_VER`

   A host test (`build_info`) fails when `Cargo.toml` and `sdkconfig.defaults`
   differ, and the release build refuses a tag that does not match the image.
2. **Docs:** a `CHANGELOG.md` entry, `docs/KNOWN_ISSUES.md` first line
   (`Firmware vX.Y.Z`), the `docs/ROADMAP.md` status row, and a section in
   `docs/PHYSICAL_SMOKE_TEST.md` for what the release changes.
3. **Checks:** `./scripts/test-host.sh`, `cargo fmt --check`,
   `cargo check --release` (builds the firmware for the ESP32-S3 in about a
   minute), and the pull request's two CI jobs green.
4. **Merge** the pull request, then on `main`:
   `git tag -a vX.Y.Z -m "..." && git push origin vX.Y.Z`.
5. **Wait for the `firmware` run on the tag** (about 8 minutes) and check the
   release has the four files; the app image's version can be read at bytes
   48 to 79 of `-app.bin`.
6. **Device:** Settings › System › Check for updates, install, and run the
   release's section of the smoke test.

Moving to a partition table other than the current one (two 6 MB OTA slots)
needs one USB flash of the merged image; the README says how.
