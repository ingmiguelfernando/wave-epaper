//! SD photo gallery: the photo list, prepared cache files and the starred set
//! that feeds the sleep screen.

pub mod cache;
pub mod exif;
pub mod image;
pub mod ui;
pub mod worker;

use std::{collections::BTreeSet, fs, io, path::Path, time::UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::ntp::utc_from_unix_seconds;

pub const PHOTOS_DIRECTORY: &str = "/sdcard/PHOTOS";
pub const PHOTO_CACHE_DIRECTORY: &str = "/sdcard/RUSTMIX/CACHE/PHOTOS";
pub const STARRED_PATH: &str = "/sdcard/RUSTMIX/STARRED.TXT";
/// Most photos listed; the newest are kept.
pub const MAX_PHOTOS: usize = 500;
/// Directory entries examined in one scan.
const MAX_SCANNED_ENTRIES: usize = 2_000;
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// One JPEG in the photos folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotoEntry {
    pub name: String,
    pub bytes: u64,
    /// File date in seconds since 1970, as the card stores it.
    pub modified: u64,
}

impl PhotoEntry {
    /// Names this version of the file, so a replaced photo is prepared again.
    #[must_use]
    pub fn key(&self) -> u32 {
        let mut hash = 0x811C_9DC5_u32;
        for byte in format!("{}|{}|{}", self.name, self.bytes, self.modified).bytes() {
            hash = (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193);
        }
        hash
    }

    /// For example `Sep 28, 2026`.
    #[must_use]
    pub fn date_label(&self) -> String {
        let date = utc_from_unix_seconds(self.modified);
        let month = MONTHS[usize::from(date.month.clamp(1, 12)) - 1];
        format!("{month} {}, {}", date.day, date.year)
    }

    /// For example `2.8 MB` or `640 KB`.
    #[must_use]
    pub fn size_label(&self) -> String {
        if self.bytes >= 1_000_000 {
            let tenths = self.bytes / 100_000;
            format!("{}.{} MB", tenths / 10, tenths % 10)
        } else {
            format!("{} KB", self.bytes.div_ceil(1_000))
        }
    }

    /// Read the size and date of `name` in `directory`.
    pub fn read(directory: &Path, name: &str) -> io::Result<Self> {
        let metadata = fs::metadata(directory.join(name))?;
        Ok(Self {
            name: name.to_string(),
            bytes: metadata.len(),
            modified: modified_seconds(&metadata),
        })
    }
}

/// JPEG files, skipping the hidden `._` copies macOS leaves on FAT cards.
#[must_use]
pub fn is_photo_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.starts_with('.') && (lower.ends_with(".jpg") || lower.ends_with(".jpeg"))
}

/// Photos in `directory`, newest first, at most `MAX_PHOTOS`.
pub fn scan_photos(directory: &Path) -> io::Result<Vec<PhotoEntry>> {
    let mut photos = Vec::new();
    for entry in fs::read_dir(directory)?.take(MAX_SCANNED_ENTRIES) {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_photo_name(&name) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_file() {
            photos.push(PhotoEntry {
                name,
                bytes: metadata.len(),
                modified: modified_seconds(&metadata),
            });
        }
    }
    photos.sort_by(|a, b| (b.modified, &a.name).cmp(&(a.modified, &b.name)));
    photos.truncate(MAX_PHOTOS);
    Ok(photos)
}

fn modified_seconds(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |age| age.as_secs())
}

/// Photo names marked for the sleep screen, kept in `STARRED.TXT`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StarredPhotos {
    names: BTreeSet<String>,
}

impl StarredPhotos {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let names = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect();
        Self { names }
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        let mut text = String::from("# Photos shown while Wave sleeps\n");
        for name in &self.names {
            text.push_str(name);
            text.push('\n');
        }
        text
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = crate::sd_file::read_to_string(path)
            .with_context(|| format!("read starred photos {}", path.display()))?;
        Ok(Self::parse(&text))
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        crate::sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write starred photos {}", path.display()))
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Star or unstar `name`; returns whether it is starred now.
    pub fn toggle(&mut self, name: &str) -> bool {
        if self.names.remove(name) {
            false
        } else {
            self.names.insert(name.to_string());
            true
        }
    }

    /// Keep `name` as the only starred photo.
    pub fn only(&mut self, name: &str) {
        self.names.clear();
        self.names.insert(name.to_string());
    }

    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Starred names in alphabetical order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

#[cfg(test)]
pub(crate) mod test_photos {
    use jpeg_encoder::{ColorType, Encoder};

    use super::exif::test_exif;

    /// A grey JPEG, dark on its left half, with an optional EXIF orientation.
    pub fn grey_jpeg(width: u16, height: u16, orientation: Option<u16>) -> Vec<u8> {
        let pixels = half_dark(width, height, 1);
        encode(
            &pixels,
            (width, height),
            ColorType::Luma,
            orientation,
            false,
        )
    }

    pub fn color_jpeg(width: u16, height: u16) -> Vec<u8> {
        let pixels = half_dark(width, height, 3);
        encode(&pixels, (width, height), ColorType::Rgb, None, false)
    }

    pub fn progressive_jpeg(width: u16, height: u16) -> Vec<u8> {
        let pixels = half_dark(width, height, 1);
        encode(&pixels, (width, height), ColorType::Luma, None, true)
    }

    pub fn jpeg_from_grey(pixels: &[u8], width: u16, height: u16) -> Vec<u8> {
        encode(pixels, (width, height), ColorType::Luma, None, false)
    }

    fn half_dark(width: u16, height: u16, channels: usize) -> Vec<u8> {
        let width = usize::from(width);
        let mut pixels = Vec::new();
        for _ in 0..height {
            for x in 0..width {
                let level = if x < width / 2 { 30 } else { 220 };
                pixels.extend(std::iter::repeat(level).take(channels));
            }
        }
        pixels
    }

    fn encode(
        pixels: &[u8],
        (width, height): (u16, u16),
        color: ColorType,
        orientation: Option<u16>,
        progressive: bool,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, 90);
        encoder.set_progressive(progressive);
        if let Some(value) = orientation {
            let mut segment = b"Exif\0\0".to_vec();
            segment.extend(test_exif(value, true));
            encoder.add_app_segment(1, segment).unwrap();
        }
        encoder.encode(pixels, width, height, color).unwrap();
        bytes
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{is_photo_name, scan_photos, PhotoEntry, StarredPhotos};

    fn entry(name: &str, bytes: u64, modified: u64) -> PhotoEntry {
        PhotoEntry {
            name: name.into(),
            bytes,
            modified,
        }
    }

    #[test]
    fn keys_change_when_the_file_changes() {
        let photo = entry("IMG_0412.jpg", 2_800_000, 1_790_000_000);
        let replaced = entry("IMG_0412.jpg", 2_800_001, 1_790_000_000);
        assert_eq!(photo.key(), photo.clone().key());
        assert_ne!(photo.key(), replaced.key());
        assert_eq!(photo.size_label(), "2.8 MB");
        assert_eq!(entry("a.jpg", 640_000, 0).size_label(), "640 KB");
        let dated = entry("a.jpg", 0, 1_759_017_600);
        assert_eq!(dated.date_label(), "Sep 28, 2025");
    }

    #[test]
    fn lists_jpegs_newest_first_and_skips_hidden_files() {
        assert!(is_photo_name("IMG_1.JPG") && is_photo_name("beach.jpeg"));
        assert!(!is_photo_name("._IMG_1.JPG") && !is_photo_name("notes.txt"));
        let root = std::env::temp_dir().join(format!("wave-photos-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for name in ["B.jpg", "A.JPG", "._A.JPG", "C.png"] {
            fs::write(root.join(name), b"x").unwrap();
        }
        let names: Vec<String> = scan_photos(&root)
            .unwrap()
            .into_iter()
            .map(|photo| photo.name)
            .collect();
        // Same file date: alphabetical.
        assert_eq!(names, ["A.JPG", "B.jpg"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn starred_names_round_trip_and_toggle() {
        let mut starred = StarredPhotos::parse("# comment\nB.jpg\n\nA.jpg\n");
        assert_eq!(starred.len(), 2);
        assert!(!starred.toggle("A.jpg"));
        assert!(starred.toggle("C.jpg"));
        let restored = StarredPhotos::parse(&starred.serialized());
        assert_eq!(restored.names().collect::<Vec<_>>(), ["B.jpg", "C.jpg"]);
        starred.only("Z.jpg");
        assert_eq!(starred.names().collect::<Vec<_>>(), ["Z.jpg"]);
    }
}
