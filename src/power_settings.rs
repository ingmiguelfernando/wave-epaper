//! Power preferences kept on the SD card: the auto-sleep delay and which keys
//! wake the device from sleep.

use std::{path::Path, time::Duration};

use anyhow::{bail, Context, Result};

pub const POWER_CONFIG_PATH: &str = "/sdcard/RUSTMIX/POWER.TXT";

/// Idle time before the device goes to sleep on its own.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoSleep {
    Off,
    Minutes5,
    #[default]
    Minutes10,
    Minutes15,
    Minutes30,
    Hour1,
}

impl AutoSleep {
    pub const ALL: [Self; 6] = [
        Self::Off,
        Self::Minutes5,
        Self::Minutes10,
        Self::Minutes15,
        Self::Minutes30,
        Self::Hour1,
    ];

    #[must_use]
    pub const fn delay(self) -> Option<Duration> {
        let minutes = match self {
            Self::Off => return None,
            Self::Minutes5 => 5,
            Self::Minutes10 => 10,
            Self::Minutes15 => 15,
            Self::Minutes30 => 30,
            Self::Hour1 => 60,
        };
        Some(Duration::from_secs(minutes * 60))
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Minutes5 => "5 min",
            Self::Minutes10 => "10 min",
            Self::Minutes15 => "15 min",
            Self::Minutes30 => "30 min",
            Self::Hour1 => "1 hour",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minutes5 => "5m",
            Self::Minutes10 => "10m",
            Self::Minutes15 => "15m",
            Self::Minutes30 => "30m",
            Self::Hour1 => "1h",
        }
    }
}

/// Keys that wake the device from sleep.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WakeKeys {
    #[default]
    AnyKey,
    PowerKey,
}

impl WakeKeys {
    pub const ALL: [Self; 2] = [Self::AnyKey, Self::PowerKey];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AnyKey => "Any key",
            Self::PowerKey => "Power key only",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::AnyKey => "any",
            Self::PowerKey => "power",
        }
    }

    /// Line shown on the sleep card.
    #[must_use]
    pub const fn wake_hint(self) -> &'static str {
        match self {
            Self::AnyKey => "Press any key to wake",
            Self::PowerKey => "Press the power key to wake",
        }
    }
}

/// One adjustable row of Settings › Power.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerSetting {
    AutoSleep,
    WakeKeys,
}

impl PowerSetting {
    pub const ALL: [Self; 2] = [Self::AutoSleep, Self::WakeKeys];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AutoSleep => "Auto-sleep",
            Self::WakeKeys => "Wake keys",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PowerPreferences {
    pub auto_sleep: AutoSleep,
    pub wake_keys: WakeKeys,
}

impl PowerPreferences {
    /// Labels of the choices for `setting` and the index of the current one.
    #[must_use]
    pub fn options(self, setting: PowerSetting) -> (Vec<&'static str>, usize) {
        match setting {
            PowerSetting::AutoSleep => (
                AutoSleep::ALL.map(AutoSleep::label).to_vec(),
                AutoSleep::ALL
                    .iter()
                    .position(|option| *option == self.auto_sleep)
                    .unwrap_or(0),
            ),
            PowerSetting::WakeKeys => (
                WakeKeys::ALL.map(WakeKeys::label).to_vec(),
                WakeKeys::ALL
                    .iter()
                    .position(|option| *option == self.wake_keys)
                    .unwrap_or(0),
            ),
        }
    }

    pub fn choose(&mut self, setting: PowerSetting, index: usize) {
        match setting {
            PowerSetting::AutoSleep => {
                if let Some(option) = AutoSleep::ALL.get(index) {
                    self.auto_sleep = *option;
                }
            }
            PowerSetting::WakeKeys => {
                if let Some(option) = WakeKeys::ALL.get(index) {
                    self.wake_keys = *option;
                }
            }
        }
    }

    #[must_use]
    pub const fn value_label(self, setting: PowerSetting) -> &'static str {
        match setting {
            PowerSetting::AutoSleep => self.auto_sleep.label(),
            PowerSetting::WakeKeys => self.wake_keys.label(),
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = crate::sd_file::read_to_string(path)
            .with_context(|| format!("read power config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut preferences = Self::default();
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
                "auto_sleep" => {
                    preferences.auto_sleep = AutoSleep::ALL
                        .into_iter()
                        .find(|option| option.marker() == value)
                        .with_context(|| format!("unknown auto_sleep value {value:?}"))?;
                }
                "wake_keys" => {
                    preferences.wake_keys = WakeKeys::ALL
                        .into_iter()
                        .find(|option| option.marker() == value)
                        .with_context(|| format!("unknown wake_keys value {value:?}"))?;
                }
                other => bail!("unsupported power config key {other:?}"),
            }
        }
        Ok(preferences)
    }

    pub fn save_to_path(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        crate::sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write power config {}", path.display()))
    }

    #[must_use]
    pub fn serialized(self) -> String {
        format!(
            "# Wave power settings\nauto_sleep={}\nwake_keys={}\n",
            self.auto_sleep.marker(),
            self.wake_keys.marker()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AutoSleep, PowerPreferences, PowerSetting, WakeKeys};

    #[test]
    fn defaults_to_ten_minutes_and_any_key() {
        let preferences = PowerPreferences::default();
        assert_eq!(preferences.auto_sleep, AutoSleep::Minutes10);
        assert_eq!(preferences.wake_keys, WakeKeys::AnyKey);
        assert_eq!(
            preferences.auto_sleep.delay(),
            Some(Duration::from_secs(600))
        );
        assert_eq!(AutoSleep::Off.delay(), None);
    }

    #[test]
    fn round_trips_through_the_sd_file_format() {
        let preferences = PowerPreferences {
            auto_sleep: AutoSleep::Hour1,
            wake_keys: WakeKeys::PowerKey,
        };
        let text = preferences.serialized();
        assert!(text.contains("auto_sleep=1h"));
        assert_eq!(PowerPreferences::parse(&text).unwrap(), preferences);
    }

    #[test]
    fn choosing_an_option_updates_the_matching_setting() {
        let mut preferences = PowerPreferences::default();
        let (labels, current) = preferences.options(PowerSetting::AutoSleep);
        assert_eq!(labels[current], "10 min");
        preferences.choose(PowerSetting::AutoSleep, 0);
        preferences.choose(PowerSetting::WakeKeys, 1);
        preferences.choose(PowerSetting::WakeKeys, 9);
        assert_eq!(preferences.value_label(PowerSetting::AutoSleep), "Off");
        assert_eq!(preferences.wake_keys, WakeKeys::PowerKey);
    }

    #[test]
    fn rejects_unknown_keys_and_values() {
        assert!(PowerPreferences::parse("auto_sleep=2h").is_err());
        assert!(PowerPreferences::parse("brightness=5").is_err());
        assert_eq!(
            PowerPreferences::parse("# comment\n\nwake_keys = power\n")
                .unwrap()
                .wake_keys,
            WakeKeys::PowerKey
        );
    }
}
