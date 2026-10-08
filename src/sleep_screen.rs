//! Settings › Sleep screen: what shows while Wave sleeps (a picture, the
//! clock or the weather), and the pick made each time it falls asleep.

use std::{fs, path::Path, time::Duration};

use anyhow::{bail, Context, Result};

use crate::{
    photos::{
        cache::{cache_file, read_frame},
        image::PhotoFit,
        PhotoEntry, StarredPhotos,
    },
    rtc::RtcDateTime,
    sleep_images::{SleepImageCatalog, SleepImageSelection},
};

pub const SLEEP_SCREEN_CONFIG_PATH: &str = "/sdcard/RUSTMIX/SLEEPSCREEN.TXT";
/// A global refresh this often clears the ghosting partial refreshes leave.
pub const SLEEP_GLOBAL_REFRESH: Duration = Duration::from_secs(30 * 60);

/// What the screen shows while Wave sleeps.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SleepMode {
    #[default]
    Photo,
    Clock,
    Weather,
    ClockWeather,
    /// Needs the Bible reader (Phase 5); not selectable yet.
    Verse,
}

impl SleepMode {
    pub const ALL: [Self; 5] = [
        Self::Photo,
        Self::Clock,
        Self::Weather,
        Self::ClockWeather,
        Self::Verse,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Photo => "Photo",
            Self::Clock => "Clock & date",
            Self::Weather => "Weather",
            Self::ClockWeather => "Clock + weather",
            Self::Verse => "Verse of the day",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Clock => "clock",
            Self::Weather => "weather",
            Self::ClockWeather => "clock-weather",
            Self::Verse => "verse",
        }
    }

    #[must_use]
    pub const fn is_available(self) -> bool {
        !matches!(self, Self::Verse)
    }

    /// Drawn by Wave rather than a picture, so it can change while asleep.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Clock | Self::Weather | Self::ClockWeather)
    }

    #[must_use]
    pub const fn shows_clock(self) -> bool {
        matches!(self, Self::Clock | Self::ClockWeather)
    }

    #[must_use]
    pub const fn shows_weather(self) -> bool {
        matches!(self, Self::Weather | Self::ClockWeather)
    }
}

/// How often a clock sleep screen shows the new time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ClockRefresh {
    #[default]
    EveryMinute,
    EveryFiveMinutes,
}

impl ClockRefresh {
    pub const ALL: [Self; 2] = [Self::EveryMinute, Self::EveryFiveMinutes];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EveryMinute => "Every minute",
            Self::EveryFiveMinutes => "Every 5 minutes",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::EveryMinute => "1",
            Self::EveryFiveMinutes => "5",
        }
    }

    #[must_use]
    pub const fn minutes(self) -> u8 {
        match self {
            Self::EveryMinute => 1,
            Self::EveryFiveMinutes => 5,
        }
    }
}

/// Time until a clock sleep screen shows a new value: the next minute, or
/// the next multiple of five, plus a margin so the RTC has turned.
#[must_use]
pub fn clock_redraw_wait(now: RtcDateTime, refresh: ClockRefresh) -> Duration {
    let step = u64::from(refresh.minutes()) * 60;
    let into = u64::from(now.minute) * 60 + u64::from(now.second);
    Duration::from_secs(step - into % step) + Duration::from_millis(500)
}

/// Extra battery a day, in tenths of a mAh, for the wake-ups `mode` needs:
/// about 0.009 mAh per clock redraw and 0.15 mAh per weather update.
/// `weather_minutes` is the update interval, `None` when weather is off or
/// manual. `None` for modes that never wake.
#[must_use]
pub fn daily_cost_tenths(
    mode: SleepMode,
    refresh: ClockRefresh,
    weather_minutes: Option<u64>,
) -> Option<u64> {
    let clock = 130 / u64::from(refresh.minutes());
    let weather = weather_minutes.map_or(0, |minutes| 2160 / minutes.max(1));
    match mode {
        SleepMode::Photo | SleepMode::Verse => None,
        SleepMode::Clock => Some(clock),
        SleepMode::Weather => Some(weather),
        SleepMode::ClockWeather => Some(clock + weather),
    }
}

/// `~13 mAh/day`, or `<1 mAh/day` for a few wake-ups.
#[must_use]
pub fn daily_cost_label(tenths: u64) -> String {
    match (tenths + 5) / 10 {
        0 => "<1 mAh/day".into(),
        whole => format!("~{whole} mAh/day"),
    }
}

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

/// One adjustable option row of Settings › Sleep screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepScreenSetting {
    Source,
    Order,
    Fit,
    ClockRefresh,
}

impl SleepScreenSetting {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Order => "Order",
            Self::Fit => "Fit",
            Self::ClockRefresh => "Refresh",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SleepScreenSettings {
    pub mode: SleepMode,
    pub source: SleepSource,
    pub order: SleepOrder,
    pub fit: PhotoFit,
    pub clock_refresh: ClockRefresh,
}

impl SleepScreenSettings {
    /// The option rows that apply to the mode in use.
    #[must_use]
    pub const fn option_rows(self) -> &'static [SleepScreenSetting] {
        match self.mode {
            SleepMode::Photo => &[
                SleepScreenSetting::Source,
                SleepScreenSetting::Order,
                SleepScreenSetting::Fit,
            ],
            SleepMode::Clock | SleepMode::ClockWeather => &[SleepScreenSetting::ClockRefresh],
            SleepMode::Weather | SleepMode::Verse => &[],
        }
    }

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
            SleepScreenSetting::ClockRefresh => (
                ClockRefresh::ALL.map(ClockRefresh::label).to_vec(),
                index_of(&ClockRefresh::ALL, self.clock_refresh),
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
            SleepScreenSetting::ClockRefresh => {
                let chosen = ClockRefresh::ALL.get(index).copied();
                self.clock_refresh = chosen.unwrap_or(self.clock_refresh);
            }
        }
    }

    #[must_use]
    pub const fn value_label(self, setting: SleepScreenSetting) -> &'static str {
        match setting {
            SleepScreenSetting::Source => self.source.label(),
            SleepScreenSetting::Order => self.order.label(),
            SleepScreenSetting::Fit => self.fit.label(),
            SleepScreenSetting::ClockRefresh => self.clock_refresh.label(),
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
                "mode" => {
                    let found = SleepMode::ALL.into_iter().find(|x| x.marker() == value);
                    settings.mode = found.with_context(|| format!("unknown mode {value:?}"))?;
                }
                "clock_refresh" => {
                    let found = ClockRefresh::ALL.into_iter().find(|x| x.marker() == value);
                    let found = found.with_context(|| format!("unknown refresh {value:?}"))?;
                    settings.clock_refresh = found;
                }
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
            "# Wave sleep screen\nmode={}\nsource={}\norder={}\nfit={}\nclock_refresh={}\n",
            self.mode.marker(),
            self.source.marker(),
            self.order.marker(),
            self.fit.marker(),
            self.clock_refresh.marker()
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
        choose_sleep_picture, clock_redraw_wait, daily_cost_label, daily_cost_tenths, pick,
        ClockRefresh, SleepMode, SleepOrder, SleepScreenSetting, SleepScreenSettings, SleepSource,
    };
    use crate::{
        photos::{
            image::PhotoFit,
            test_photos::grey_jpeg,
            worker::{prepare, PhotoJob},
            PhotoEntry, StarredPhotos,
        },
        rtc::RtcDateTime,
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
    fn modes_and_clock_refresh_round_trip_and_choose_their_rows() {
        let old_file = SleepScreenSettings::parse("source=folder\n").unwrap();
        assert_eq!(old_file.mode, SleepMode::Photo);
        let mut settings = SleepScreenSettings {
            mode: SleepMode::ClockWeather,
            ..SleepScreenSettings::default()
        };
        settings.choose(SleepScreenSetting::ClockRefresh, 1);
        let restored = SleepScreenSettings::parse(&settings.serialized()).unwrap();
        assert_eq!(restored, settings);
        assert_eq!(restored.clock_refresh, ClockRefresh::EveryFiveMinutes);
        assert_eq!(restored.option_rows(), [SleepScreenSetting::ClockRefresh]);
        assert_eq!(SleepScreenSettings::default().option_rows().len(), 3);
        assert!(SleepScreenSettings::parse("mode=slideshow").is_err());
        assert!(!SleepMode::Verse.is_available());
        assert!(SleepMode::Weather.is_live() && !SleepMode::Photo.is_live());
    }

    #[test]
    fn costs_follow_the_mockup_estimates() {
        let cost = |mode, refresh, weather| daily_cost_tenths(mode, refresh, weather);
        let minute = ClockRefresh::EveryMinute;
        let five = ClockRefresh::EveryFiveMinutes;
        assert_eq!(cost(SleepMode::Photo, minute, Some(120)), None);
        assert_eq!(cost(SleepMode::Clock, minute, None), Some(130));
        assert_eq!(cost(SleepMode::Weather, minute, Some(120)), Some(18));
        assert_eq!(cost(SleepMode::ClockWeather, five, Some(120)), Some(44));
        assert_eq!(daily_cost_label(130), "~13 mAh/day");
        assert_eq!(daily_cost_label(18), "~2 mAh/day");
        assert_eq!(daily_cost_label(3), "<1 mAh/day");
    }

    #[test]
    fn the_clock_redraws_when_the_minute_turns() {
        let wait = |minute, second, refresh| {
            let now = RtcDateTime {
                minute,
                second,
                ..RtcDateTime::default()
            };
            clock_redraw_wait(now, refresh).as_millis()
        };
        let (one, five) = (ClockRefresh::EveryMinute, ClockRefresh::EveryFiveMinutes);
        assert_eq!(wait(41, 20, one), 40_500);
        assert_eq!(wait(41, 0, one), 60_500);
        assert_eq!(wait(42, 30, five), 150_500);
        assert_eq!(wait(45, 0, five), 300_500);
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
