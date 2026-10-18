//! Over-the-air update decisions, with no network and no flash access: read
//! the latest GitHub release, compare versions, pick the app image, check the
//! battery and read an image's header before any byte is written to a
//! partition. The device side (download, `esp_ota_*`) calls these.
//!
//! Release assets are named by `.github/workflows/firmware.yml`:
//! `wave-epaper-<tag>-app.bin` (the app image OTA writes),
//! `wave-epaper-<tag>.bin` (the merged image for a USB flash) and
//! `SHA256SUMS.txt`.

use anyhow::{bail, Context, Result};

use crate::json_lite::{parse, JsonValue};

/// Where the newest release is read from.
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/ingmiguelfernando/wave-epaper/releases/latest";
/// An update needs at least this much battery, or USB power.
pub const MIN_BATTERY_PERCENT: u8 = 30;
/// The first byte of every ESP image.
pub const IMAGE_MAGIC: u8 = 0xE9;
/// `chip_id` of the ESP32-S3 in an image header (`esp_chip_id_t`).
pub const CHIP_ID_ESP32S3: u16 = 0x0009;
/// `magic_word` of `esp_app_desc_t`, which follows the first segment header.
pub const APP_DESC_MAGIC: u32 = 0xABCD_5432;
/// Bytes from the start of an image to its `esp_app_desc_t`: the 24-byte
/// image header and the 8-byte header of the first segment.
pub const APP_DESC_OFFSET: usize = 32;
/// `esp_app_desc_t::version` is 32 bytes at offset 16 inside the descriptor.
const APP_DESC_VERSION_OFFSET: usize = APP_DESC_OFFSET + 16;
const APP_DESC_VERSION_BYTES: usize = 32;
/// Enough of the start of an image to read its header and descriptor.
pub const IMAGE_PREFIX_BYTES: usize = APP_DESC_VERSION_OFFSET + APP_DESC_VERSION_BYTES;

/// A `major.minor.patch` version. A leading `v` is accepted.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// `v0.12.7` and `0.12.7` give the same version; anything else is an
    /// error (a pre-release suffix is not accepted: releases are plain).
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.');
        let mut next = |name: &str| -> Result<u32> {
            let part = parts.next().with_context(|| format!("no {name} number"))?;
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                bail!("{name} number {part:?} is not digits");
            }
            part.parse()
                .with_context(|| format!("{name} number {part:?}"))
        };
        let version = Self {
            major: next("major")?,
            minor: next("minor")?,
            patch: next("patch")?,
        };
        if parts.next().is_some() {
            bail!("version {text:?} has more than three numbers");
        }
        Ok(version)
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One file of a release.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
}

/// The parts of a GitHub release the updater reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
    pub assets: Vec<Asset>,
}

/// Read the JSON of `GET /releases/latest`. A draft or a pre-release is an
/// error (the endpoint does not return them, but a mirror might).
pub fn parse_release(json: &str) -> Result<Release> {
    let root = parse(json).map_err(|error| anyhow::anyhow!("release JSON: {error}"))?;
    if root.get("draft").and_then(JsonValue::as_bool) == Some(true)
        || root.get("prerelease").and_then(JsonValue::as_bool) == Some(true)
    {
        bail!("the latest release is a draft or a pre-release");
    }
    let tag = root
        .get("tag_name")
        .and_then(JsonValue::as_str)
        .context("release has no tag_name")?
        .to_string();
    let version = Version::parse(&tag).context("release tag")?;
    let mut assets = Vec::new();
    let mut index = 0;
    while let Some(entry) = root.get("assets").and_then(|list| list.index(index)) {
        index += 1;
        let (Some(name), Some(url)) = (
            entry.get("name").and_then(JsonValue::as_str),
            entry
                .get("browser_download_url")
                .and_then(JsonValue::as_str),
        ) else {
            continue;
        };
        let size = entry
            .get("size")
            .and_then(JsonValue::as_i64)
            .and_then(|size| u64::try_from(size).ok())
            .unwrap_or(0);
        assets.push(Asset {
            name: name.to_string(),
            size,
            url: url.to_string(),
        });
    }
    Ok(Release {
        tag,
        version,
        assets,
    })
}

impl Release {
    /// The app image for this release: `wave-epaper-<tag>-app.bin`, over
    /// HTTPS from github.com, with a size.
    pub fn app_image(&self) -> Result<&Asset> {
        let wanted = format!("wave-epaper-{}-app.bin", self.tag);
        let asset = self
            .assets
            .iter()
            .find(|asset| asset.name == wanted)
            .with_context(|| format!("release {} has no {wanted}", self.tag))?;
        if !asset.url.starts_with("https://") {
            bail!("{wanted} is not served over HTTPS");
        }
        if asset.size == 0 {
            bail!("{wanted} has no size");
        }
        Ok(asset)
    }

    /// The checksum list of this release, `SHA256SUMS.txt`.
    pub fn checksums(&self) -> Result<&Asset> {
        self.assets
            .iter()
            .find(|asset| asset.name == "SHA256SUMS.txt" && asset.url.starts_with("https://"))
            .with_context(|| format!("release {} has no SHA256SUMS.txt", self.tag))
    }
}

/// What checking found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateCheck {
    /// The installed version is the latest, or newer (a development build).
    UpToDate,
    Available(Version),
}

/// Compare the installed version with the latest release.
#[must_use]
pub fn check(installed: Version, latest: Version) -> UpdateCheck {
    if latest > installed {
        UpdateCheck::Available(latest)
    } else {
        UpdateCheck::UpToDate
    }
}

/// Why an update does not start now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// Battery below the minimum and no USB power.
    LowBattery { percent: u8 },
    /// The battery level is unknown and there is no USB power.
    BatteryUnknown,
}

impl Refusal {
    /// One line for the screen.
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::LowBattery { percent } => format!(
                "Battery {percent}%: charge to {MIN_BATTERY_PERCENT}% or connect USB to update."
            ),
            Self::BatteryUnknown => "Battery level unknown: connect USB to update.".to_string(),
        }
    }
}

/// An update may start with USB power, or with a known battery level of at
/// least `MIN_BATTERY_PERCENT`.
pub fn may_start(battery_percent: Option<u8>, on_usb: bool) -> Result<(), Refusal> {
    if on_usb {
        return Ok(());
    }
    match battery_percent {
        Some(percent) if percent >= MIN_BATTERY_PERCENT => Ok(()),
        Some(percent) => Err(Refusal::LowBattery { percent }),
        None => Err(Refusal::BatteryUnknown),
    }
}

/// What the first bytes of an image say about it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageInfo {
    /// `esp_app_desc_t::version`, the `CONFIG_APP_PROJECT_VER` text.
    pub version: String,
}

/// Check the start of an app image before it is written: the magic byte, the
/// ESP32-S3 chip id, the app descriptor and its version text. `expected` is
/// the version the release says the image is; a mismatch is refused, so a
/// wrong or older asset is never flashed.
pub fn check_image_prefix(prefix: &[u8], expected: Version) -> Result<ImageInfo> {
    if prefix.len() < IMAGE_PREFIX_BYTES {
        bail!(
            "image starts with {} bytes, need {IMAGE_PREFIX_BYTES}",
            prefix.len()
        );
    }
    if prefix[0] != IMAGE_MAGIC {
        bail!("not an ESP image: first byte is {:#04x}", prefix[0]);
    }
    let chip = u16::from_le_bytes([prefix[12], prefix[13]]);
    if chip != CHIP_ID_ESP32S3 {
        bail!("image is for chip id {chip:#06x}, not the ESP32-S3");
    }
    let magic = u32::from_le_bytes([
        prefix[APP_DESC_OFFSET],
        prefix[APP_DESC_OFFSET + 1],
        prefix[APP_DESC_OFFSET + 2],
        prefix[APP_DESC_OFFSET + 3],
    ]);
    if magic != APP_DESC_MAGIC {
        bail!("image has no app descriptor (magic {magic:#010x})");
    }
    let raw = &prefix[APP_DESC_VERSION_OFFSET..APP_DESC_VERSION_OFFSET + APP_DESC_VERSION_BYTES];
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    let text = core::str::from_utf8(&raw[..end]).context("image version is not text")?;
    let found = Version::parse(text).context("image version")?;
    if found != expected {
        bail!("image is version {found}, the release says {expected}");
    }
    Ok(ImageInfo {
        version: text.to_string(),
    })
}

/// How long a freshly started image must run before it marks itself valid.
/// A crash or reset before then makes the bootloader return to the previous
/// image (`CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE`).
pub const VALIDATE_AFTER_MS: u64 = 30_000;

/// Whether the running image should mark itself valid now: it has been up
/// `uptime_ms`, or it is about to sleep (a sleeping device must not be left
/// pending, or a flat battery would count as a crash).
#[must_use]
pub const fn should_mark_valid(uptime_ms: u64, about_to_sleep: bool) -> bool {
    uptime_ms >= VALIDATE_AFTER_MS || about_to_sleep
}

#[cfg(test)]
mod tests {
    use super::{
        check, check_image_prefix, may_start, parse_release, should_mark_valid, UpdateCheck,
        Version, APP_DESC_MAGIC, APP_DESC_OFFSET, CHIP_ID_ESP32S3, IMAGE_PREFIX_BYTES,
        MIN_BATTERY_PERCENT, VALIDATE_AFTER_MS,
    };

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_parse_with_or_without_v_and_order_by_number() {
        assert_eq!(v("v0.12.7"), v("0.12.7"));
        assert_eq!(v("1.2.3").to_string(), "1.2.3");
        assert!(v("0.12.10") > v("0.12.9"), "numbers, not text");
        assert!(v("0.13.0") > v("0.12.99"));
        assert!(v("1.0.0") > v("0.99.99"));
        for bad in [
            "",
            "v",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1..3",
            "v1.2.3-rc1",
            "-1.2.3",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad:?} parsed");
        }
    }

    const RELEASE: &str = r#"{
      "tag_name": "v0.13.0", "draft": false, "prerelease": false,
      "body": "Notes with \"quotes\" and é",
      "assets": [
        {"name": "wave-epaper-v0.13.0.bin", "size": 1500000,
         "browser_download_url": "https://github.com/o/r/releases/download/v0.13.0/wave-epaper-v0.13.0.bin"},
        {"name": "wave-epaper-v0.13.0-app.bin", "size": 1000000,
         "browser_download_url": "https://github.com/o/r/releases/download/v0.13.0/wave-epaper-v0.13.0-app.bin"},
        {"name": "SHA256SUMS.txt", "size": 300,
         "browser_download_url": "https://github.com/o/r/releases/download/v0.13.0/SHA256SUMS.txt"},
        {"name": "broken"}
      ]
    }"#;

    #[test]
    fn a_release_gives_its_version_and_the_app_image() {
        let release = parse_release(RELEASE).unwrap();
        assert_eq!(release.version, v("0.13.0"));
        let app = release.app_image().unwrap();
        assert_eq!(app.name, "wave-epaper-v0.13.0-app.bin");
        assert_eq!(app.size, 1_000_000);
        assert_eq!(release.checksums().unwrap().name, "SHA256SUMS.txt");
        assert_eq!(release.assets.len(), 3, "an asset without a url is skipped");
    }

    #[test]
    fn the_merged_image_is_never_taken_for_the_app_image() {
        let text = RELEASE.replace("-app.bin", "-merged.bin");
        let release = parse_release(&text).unwrap();
        assert!(release.app_image().is_err());
    }

    #[test]
    fn bad_releases_are_refused() {
        assert!(parse_release("not json").is_err());
        assert!(parse_release(r#"{"assets": []}"#).is_err(), "no tag");
        assert!(parse_release(r#"{"tag_name": "latest", "assets": []}"#).is_err());
        assert!(parse_release(r#"{"tag_name": "v1.0.0", "draft": true, "assets": []}"#).is_err());
        assert!(
            parse_release(r#"{"tag_name": "v1.0.0", "prerelease": true, "assets": []}"#).is_err()
        );
        // The image over plain HTTP, or with no size, is refused.
        let http = RELEASE.replacen(
            "https://github.com/o/r/releases/download/v0.13.0/wave-epaper-v0.13.0-app.bin",
            "http://example.com/app.bin",
            1,
        );
        assert!(parse_release(&http).unwrap().app_image().is_err());
        let empty = RELEASE.replace("\"size\": 1000000", "\"size\": 0");
        assert!(parse_release(&empty).unwrap().app_image().is_err());
        let none = r#"{"tag_name": "v1.0.0", "assets": []}"#;
        assert!(parse_release(none).unwrap().checksums().is_err());
    }

    #[test]
    fn only_a_newer_release_is_an_update() {
        assert_eq!(
            check(v("0.12.7"), v("0.13.0")),
            UpdateCheck::Available(v("0.13.0"))
        );
        assert_eq!(check(v("0.13.0"), v("0.13.0")), UpdateCheck::UpToDate);
        assert_eq!(check(v("0.14.0"), v("0.13.0")), UpdateCheck::UpToDate);
    }

    #[test]
    fn the_battery_gate_needs_usb_or_thirty_percent() {
        assert!(may_start(Some(MIN_BATTERY_PERCENT), false).is_ok());
        assert!(may_start(Some(100), false).is_ok());
        assert!(may_start(Some(5), true).is_ok(), "USB power is enough");
        assert!(may_start(None, true).is_ok());
        let low = may_start(Some(MIN_BATTERY_PERCENT - 1), false).unwrap_err();
        assert!(low.message().contains("29%"), "{}", low.message());
        let unknown = may_start(None, false).unwrap_err();
        assert!(unknown.message().contains("USB"));
    }

    /// An image prefix with the header and descriptor a real build has.
    fn prefix(version: &str) -> Vec<u8> {
        let mut bytes = vec![0_u8; IMAGE_PREFIX_BYTES];
        bytes[0] = 0xE9;
        bytes[12..14].copy_from_slice(&CHIP_ID_ESP32S3.to_le_bytes());
        bytes[APP_DESC_OFFSET..APP_DESC_OFFSET + 4].copy_from_slice(&APP_DESC_MAGIC.to_le_bytes());
        bytes[APP_DESC_OFFSET + 16..APP_DESC_OFFSET + 16 + version.len()]
            .copy_from_slice(version.as_bytes());
        bytes
    }

    #[test]
    fn an_image_of_the_released_version_passes() {
        let info = check_image_prefix(&prefix("0.13.0"), v("0.13.0")).unwrap();
        assert_eq!(info.version, "0.13.0");
    }

    #[test]
    fn a_wrong_image_is_refused_before_it_is_written() {
        let good = prefix("0.13.0");
        assert!(check_image_prefix(&good[..IMAGE_PREFIX_BYTES - 1], v("0.13.0")).is_err());
        let mut not_esp = good.clone();
        not_esp[0] = 0x00;
        assert!(check_image_prefix(&not_esp, v("0.13.0")).is_err());
        let mut other_chip = good.clone();
        other_chip[12] = 0x02; // an ESP32-S2 image
        assert!(check_image_prefix(&other_chip, v("0.13.0")).is_err());
        let mut no_desc = good.clone();
        no_desc[APP_DESC_OFFSET] = 0;
        assert!(check_image_prefix(&no_desc, v("0.13.0")).is_err());
        assert!(
            check_image_prefix(&good, v("0.13.1")).is_err(),
            "version differs"
        );
        assert!(
            check_image_prefix(&prefix("0.12.7"), v("0.13.0")).is_err(),
            "an older image under a newer tag"
        );
        assert!(check_image_prefix(&prefix("garbage"), v("0.13.0")).is_err());
        // A version field with no end is read as the whole 32 bytes and fails.
        let mut unterminated = good;
        for byte in &mut unterminated[APP_DESC_OFFSET + 16..APP_DESC_OFFSET + 48] {
            *byte = b'1';
        }
        assert!(check_image_prefix(&unterminated, v("0.13.0")).is_err());
    }

    #[test]
    fn an_image_marks_itself_valid_after_thirty_seconds_or_before_sleep() {
        assert!(!should_mark_valid(0, false));
        assert!(!should_mark_valid(VALIDATE_AFTER_MS - 1, false));
        assert!(should_mark_valid(VALIDATE_AFTER_MS, false));
        assert!(should_mark_valid(1_000, true));
    }
}
