//! Settings › Sleep screen: which pictures show while Wave sleeps, and the
//! pick made each time it falls asleep.

use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    photos::{
        cache::{cache_file, read_frame},
        image::PhotoFit,
        PhotoEntry, StarredPhotos,
    },
    sleep_images::{SleepImageCatalog, SleepImageSelection},
};

pub const SLEEP_SCREEN_CONFIG_PATH: &str = "/sdcard/RUSTMIX/SLEEPSCREEN.TXT";

/// Where sleep pictures come from.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SleepSource {
    /// Photos starred in the Photos app.
    #[default]
    Starred,
    /// BMP pictures in `/RUSTMIX/SLEEP`.
    Folder,
}

impl SleepSource {
    pub const ALL: [Self; 2] = [Self::Starred, Self::Folder];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Starred => "Starred photos",
            Self::Folder => "Sleep folder",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Starred => "starred",
            Self::Folder => "folder",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SleepOrder {
    /// A different random picture each time.
    #[default]
    Shuffle,
    /// The next picture by name.
    InOrder,
}

impl SleepOrder {
    pub const ALL: [Self; 2] = [Self::Shuffle, Self::InOrder];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Shuffle => "Shuffle",
            Self::InOrder => "In order",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Shuffle => "shuffle",
            Self::InOrder => "in-order",
        }
    }
}

/// One adjustable row of Settings › Sleep screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepScreenSetting {
    Source,
    Order,
    Fit,
}

impl SleepScreenSetting {
    pub const ALL: [Self; 3] = [Self::Source, Self::Order, Self::Fit];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Order => "Order",
            Self::Fit => "Fit",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SleepScreenSettings {
    pub source: SleepSource,
    pub order: SleepOrder,
    pub fit: PhotoFit,
}

impl SleepScreenSettings {
    /// Labels of the choices for `setting` and the index of the current one.
    #[must_use]
    pub fn options(self, setting: SleepScreenSetting) -> (Vec<&'static str>, usize) {
        match setting {
            SleepScreenSetting::Source => (
                SleepSource::ALL.map(SleepSource::label).to_vec(),
                index_of(&SleepSource::ALL, self.source),
            ),
            SleepScreenSetting::Order => (
                SleepOrder::ALL.map(SleepOrder::label).to_vec(),
                index_of(&SleepOrder::ALL, self.order),
            ),
            SleepScreenSetting::Fit => (
                PhotoFit::ALL.map(PhotoFit::label).to_vec(),
                index_of(&PhotoFit::ALL, self.fit),
            ),
        }
    }

    pub fn choose(&mut self, setting: SleepScreenSetting, index: usize) {
        match setting {
            SleepScreenSetting::Source => {
                self.source = SleepSource::ALL.get(index).copied().unwrap_or(self.source);
            }
            SleepScreenSetting::Order => {
                self.order = SleepOrder::ALL.get(index).copied().unwrap_or(self.order);
            }
            SleepScreenSetting::Fit => {
                self.fit = PhotoFit::ALL.get(index).copied().unwrap_or(self.fit);
            }
        }
    }

    #[must_use]
    pub const fn value_label(self, setting: SleepScreenSetting) -> &'static str {
        match setting {
            SleepScreenSetting::Source => self.source.label(),
            SleepScreenSetting::Order => self.order.label(),
            SleepScreenSetting::Fit => self.fit.label(),
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("read sleep screen config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut settings = Self::default();
        for (line_number, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .with_context(|| format!("line {} must contain '='", line_number + 1))?;
            let value = value.trim();
            match key.trim() {
                "source" => {
                    let found = SleepSource::ALL.into_iter().find(|x| x.marker() == value);
                    settings.source = found.with_context(|| format!("unknown source {value:?}"))?;
                }
                "order" => {
                    let found = SleepOrder::ALL.into_iter().find(|x| x.marker() == value);
                    settings.order = found.with_context(|| format!("unknown order {value:?}"))?;
                }
                "fit" => {
                    let found = PhotoFit::ALL.into_iter().find(|x| x.marker() == value);
                    settings.fit = found.with_context(|| format!("unknown fit {value:?}"))?;
                }
                other => bail!("unsupported sleep screen key {other:?}"),
            }
        }
        Ok(settings)
    }

    pub fn save_to_path(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.serialized())
            .with_context(|| format!("write sleep screen config {}", path.display()))
    }

    #[must_use]
    pub fn serialized(self) -> String {
        format!(
            "# Wave sleep screen\nsource={}\norder={}\nfit={}\n",
            self.source.marker(),
            self.order.marker(),
            self.fit.marker()
        )
    }
}

fn index_of<T: PartialEq>(all: &[T], value: T) -> usize {
    all.iter().position(|item| *item == value).unwrap_or(0)
}

/// Pick and load the picture for this sleep. Starred photos need a cache file
/// from the Photos app; without one the sleep folder is used, and failing
/// that the sleep card explains what to do.
pub fn choose_sleep_picture(
    settings: SleepScreenSettings,
    starred: &StarredPhotos,
    (photos, cache): (&Path, &Path),
    folder: &mut SleepImageCatalog,
    previous: Option<&str>,
    random_word: u32,
) -> SleepImageSelection {
    let in_order = settings.order == SleepOrder::InOrder;
    if settings.source == SleepSource::Starred {
        let ready: Vec<&str> = starred
            .names()
            .filter(|name| {
                PhotoEntry::read(photos, name)
                    .is_ok_and(|photo| cache_file(cache, photo.key()).exists())
            })
            .collect();
        let last = ready.iter().position(|name| Some(*name) == previous);
        if let Some(index) = pick(ready.len(), last, in_order, random_word) {
            let name = ready[index];
            let frame = PhotoEntry::read(photos, name)
                .and_then(|photo| read_frame(&cache_file(cache, photo.key()), settings.fit));
            if let Ok(frame) = frame {
                return SleepImageSelection::picture(name.to_string(), frame);
            }
        }
    }
    let mut selection = folder.select(random_word, in_order);
    if settings.source == SleepSource::Starred && selection.fallback {
        selection.note = Some(if starred.is_empty() {
            "No starred photos yet. Star photos in Photos.".into()
        } else {
            "Starred photos are not ready yet. Open Photos to prepare them.".into()
        });
    }
    selection
}

/// The picture after `previous`, or a random one other than it.
fn pick(count: usize, previous: Option<usize>, in_order: bool, random_word: u32) -> Option<usize> {
    let random = random_word as usize;
    match (count, previous) {
        (0, _) => None,
        _ if in_order => Some(previous.map_or(0, |index| (index + 1) % count)),
        (2.., Some(previous)) => {
            let slot = random % (count - 1);
            Some(if slot >= previous { slot + 1 } else { slot })
        }
        _ => Some(random % count),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        choose_sleep_picture, pick, SleepOrder, SleepScreenSetting, SleepScreenSettings,
        SleepSource,
    };
    use crate::{
        photos::{
            image::PhotoFit,
            test_photos::grey_jpeg,
            worker::{prepare, PhotoJob},
            PhotoEntry, StarredPhotos,
        },
        sleep_images::SleepImageCatalog,
    };

    #[test]
    fn settings_round_trip_and_offer_option_lists() {
        let mut settings = SleepScreenSettings::default();
        assert_eq!(settings.source, SleepSource::Starred);
        let (labels, current) = settings.options(SleepScreenSetting::Fit);
        assert_eq!(labels[current], "Fill (crop)");
        settings.choose(SleepScreenSetting::Order, 1);
        settings.choose(SleepScreenSetting::Fit, 1);
        let restored = SleepScreenSettings::parse(&settings.serialized()).unwrap();
        assert_eq!(restored, settings);
        assert_eq!(restored.order, SleepOrder::InOrder);
        assert_eq!(restored.fit, PhotoFit::Whole);
        assert!(SleepScreenSettings::parse("fit=stretch").is_err());
    }

    #[test]
    fn picks_in_order_or_avoid_the_last_picture() {
        assert_eq!(pick(0, None, false, 7), None);
        assert_eq!(pick(3, Some(2), true, 7), Some(0));
        assert_eq!(pick(3, None, true, 7), Some(0));
        for random in 0..10 {
            assert_ne!(pick(3, Some(1), false, random), Some(1));
        }
        assert_eq!(pick(1, Some(0), false, 5), Some(0));
    }

    #[test]
    fn starred_photos_with_a_cache_file_come_first() {
        let root = std::env::temp_dir().join(format!("wave-sleep-pick-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (photos, cache) = (root.join("PHOTOS"), root.join("CACHE"));
        fs::create_dir_all(&photos).unwrap();
        fs::write(photos.join("A.jpg"), grey_jpeg(96, 64, None)).unwrap();
        let mut folder = SleepImageCatalog::new(root.join("SLEEP"));
        let mut starred = StarredPhotos::default();
        let settings = SleepScreenSettings::default();
        let dirs = (photos.as_path(), cache.as_path());

        let empty = choose_sleep_picture(settings, &starred, dirs, &mut folder, None, 1);
        assert_eq!(
            empty.note.as_deref(),
            Some("No starred photos yet. Star photos in Photos.")
        );

        starred.toggle("A.jpg");
        let waiting = choose_sleep_picture(settings, &starred, dirs, &mut folder, None, 1);
        let note = waiting.note.unwrap();
        assert!(note.starts_with("Starred photos are not ready"), "{note}");

        let key = PhotoEntry::read(&photos, "A.jpg").unwrap().key();
        let job = PhotoJob {
            name: "A.jpg".into(),
            key,
        };
        let _ = prepare(&photos, &cache, &job);
        let chosen = choose_sleep_picture(settings, &starred, dirs, &mut folder, None, 1);
        assert_eq!(chosen.file_name, "A.jpg");
        assert!(!chosen.fallback);
        let _ = fs::remove_dir_all(root);
    }
}
