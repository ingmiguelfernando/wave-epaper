//! SD-card weather configuration: the location comes from the owner, the
//! update and display choices from Settings › Weather.
//!
//! Weather retrieval is intentionally separate from Wi-Fi credentials. The
//! Open-Meteo provider does not require an API key.

use std::{collections::BTreeMap, fs, path::Path, time::Duration};

use anyhow::{bail, Context, Result};

use crate::{
    app::widgets::option_list::option_labels,
    regional::{TemperatureUnit, TimeZoneProfile, DEFAULT_TIMEZONE_NAME},
};

/// Weather configuration file; Settings › Weather rewrites it.
pub const WEATHER_CONFIG_PATH: &str = "/sdcard/RUSTMIX/WEATHER.TXT";
/// Provider selected for the first weather milestone.
pub const DEFAULT_WEATHER_PROVIDER: &str = "open-meteo";
/// Product-facing fallback location label.
pub const DEFAULT_LOCATION_LABEL: &str = "Configured location";
/// Each update keeps Wi-Fi on for about 4 s, so two hours is about 2 mAh a day.
pub const DEFAULT_REFRESH_MINUTES: u64 = 120;
/// Lower refresh bound protects the free provider and the e-paper workflow.
pub const MIN_REFRESH_MINUTES: u64 = 15;
/// Upper refresh bound keeps stale data visible without excessive polling.
pub const MAX_REFRESH_MINUTES: u64 = 360;
/// Intervals offered by Settings › Weather; 0 updates only on request.
pub const REFRESH_CHOICES: [u64; 5] = [30, 60, 120, 360, 0];
const REFRESH_LABELS: [&str; 5] = ["30 minutes", "1 hour", "2 hours", "6 hours", "Manual"];
const KEYS: [&str; 9] = [
    "provider",
    "location",
    "latitude",
    "longitude",
    "timezone",
    "refresh_minutes",
    "enabled",
    "show_on_home",
    "units",
];

/// Temperature and wind units shown by the weather screens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WeatherUnits {
    #[default]
    Metric,
    Imperial,
}

impl WeatherUnits {
    pub const ALL: [Self; 2] = [Self::Metric, Self::Imperial];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Metric => "°C · km/h",
            Self::Imperial => "°F · mph",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Metric => "metric",
            Self::Imperial => "imperial",
        }
    }

    #[must_use]
    pub const fn temperature_unit(self) -> TemperatureUnit {
        match self {
            Self::Metric => TemperatureUnit::Celsius,
            Self::Imperial => TemperatureUnit::Fahrenheit,
        }
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Metric => Self::Imperial,
            Self::Imperial => Self::Metric,
        }
    }
}

/// Rows of Settings › Weather that open an option list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherSetting {
    Service,
    Refresh,
    Units,
    ShowOnHome,
}

impl WeatherSetting {
    pub const ALL: [Self; 4] = [Self::Service, Self::Refresh, Self::Units, Self::ShowOnHome];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Service => "Weather service",
            Self::Refresh => "Update every",
            Self::Units => "Units",
            Self::ShowOnHome => "Show on Home",
        }
    }
}

/// Validated weather configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct WeatherConfig {
    pub provider: String,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    /// Minutes between automatic updates; 0 means on request only.
    pub refresh_minutes: u64,
    /// Off means no weather requests at all, and no weather on Home.
    pub enabled: bool,
    pub show_on_home: bool,
    pub units: WeatherUnits,
}

impl WeatherConfig {
    /// Load and validate the read-only boot configuration.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("unable to read {}", path.display()))?;
        Self::parse(&contents)
            .with_context(|| format!("invalid weather configuration in {}", path.display()))
    }

    /// Parse the intentionally small `key=value` configuration format.
    pub fn parse(contents: &str) -> Result<Self> {
        let mut values = BTreeMap::<String, String>::new();
        for (line_number, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("line {} must use key=value", line_number + 1))?;
            let key = key.trim();
            let value = value.trim();
            if !KEYS.contains(&key) {
                bail!("line {} uses unsupported key {key:?}", line_number + 1);
            }
            if values.insert(key.into(), value.into()).is_some() {
                bail!("line {} repeats key {key:?}", line_number + 1);
            }
        }

        let provider = values
            .get("provider")
            .cloned()
            .unwrap_or_else(|| DEFAULT_WEATHER_PROVIDER.into());
        if provider != DEFAULT_WEATHER_PROVIDER {
            bail!("provider must be open-meteo in this milestone");
        }

        let location = values
            .get("location")
            .cloned()
            .unwrap_or_else(|| DEFAULT_LOCATION_LABEL.into());
        if location.is_empty() || location.len() > 40 {
            bail!("location must contain 1 to 40 UTF-8 bytes");
        }

        let latitude = required(&values, "latitude")?
            .parse::<f64>()
            .context("latitude must be a decimal number")?;
        if !(-90.0..=90.0).contains(&latitude) {
            bail!("latitude must be between -90 and 90 degrees");
        }

        let longitude = required(&values, "longitude")?
            .parse::<f64>()
            .context("longitude must be a decimal number")?;
        if !(-180.0..=180.0).contains(&longitude) {
            bail!("longitude must be between -180 and 180 degrees");
        }

        let timezone = values
            .get("timezone")
            .cloned()
            .unwrap_or_else(|| DEFAULT_TIMEZONE_NAME.into());
        TimeZoneProfile::parse(&timezone)?;

        let refresh_minutes = match values.get("refresh_minutes") {
            Some(value) => value
                .parse::<u64>()
                .context("refresh_minutes must be an integer")?,
            None => DEFAULT_REFRESH_MINUTES,
        };
        if refresh_minutes != 0
            && !(MIN_REFRESH_MINUTES..=MAX_REFRESH_MINUTES).contains(&refresh_minutes)
        {
            bail!("refresh_minutes must be 0 or {MIN_REFRESH_MINUTES}-{MAX_REFRESH_MINUTES}");
        }
        let units = match values.get("units").map(String::as_str) {
            None | Some("metric") => WeatherUnits::Metric,
            Some("imperial") => WeatherUnits::Imperial,
            Some(other) => bail!("units must be metric or imperial, not {other:?}"),
        };

        Ok(Self {
            provider,
            location,
            latitude,
            longitude,
            timezone,
            refresh_minutes,
            enabled: yes_no(&values, "enabled")?,
            show_on_home: yes_no(&values, "show_on_home")?,
            units,
        })
    }

    /// Time between automatic updates; `None` when off or manual.
    #[must_use]
    pub fn refresh_interval(&self) -> Option<Duration> {
        (self.enabled && self.refresh_minutes > 0)
            .then(|| Duration::from_secs(self.refresh_minutes * 60))
    }

    /// Labels of the choices for `setting` and the index of the one in use,
    /// or `usize::MAX` for an interval from an older file that is not offered.
    #[must_use]
    pub fn options(&self, setting: WeatherSetting) -> (Vec<&'static str>, usize) {
        match setting {
            WeatherSetting::Service => option_labels(&[true, false], self.enabled, on_off),
            WeatherSetting::Refresh => {
                let current = REFRESH_CHOICES
                    .iter()
                    .position(|&minutes| minutes == self.refresh_minutes);
                (REFRESH_LABELS.to_vec(), current.unwrap_or(usize::MAX))
            }
            WeatherSetting::Units => {
                option_labels(&WeatherUnits::ALL, self.units, WeatherUnits::label)
            }
            WeatherSetting::ShowOnHome => {
                option_labels(&[true, false], self.show_on_home, yes_no_label)
            }
        }
    }

    /// Apply a valid option-list choice to `setting`; other indices do nothing.
    pub fn choose(&mut self, setting: WeatherSetting, index: usize) {
        match setting {
            WeatherSetting::Service => {
                if let Some(&enabled) = [true, false].get(index) {
                    self.enabled = enabled;
                }
            }
            WeatherSetting::Refresh => {
                if let Some(&minutes) = REFRESH_CHOICES.get(index) {
                    self.refresh_minutes = minutes;
                }
            }
            WeatherSetting::Units => {
                if let Some(&units) = WeatherUnits::ALL.get(index) {
                    self.units = units;
                }
            }
            WeatherSetting::ShowOnHome => {
                if let Some(&show) = [true, false].get(index) {
                    self.show_on_home = show;
                }
            }
        }
    }

    #[must_use]
    pub fn value_label(&self, setting: WeatherSetting) -> String {
        match setting {
            WeatherSetting::Service => on_off(self.enabled).into(),
            WeatherSetting::Refresh => refresh_label(self.refresh_minutes),
            WeatherSetting::Units => self.units.label().into(),
            WeatherSetting::ShowOnHome => yes_no_label(self.show_on_home).into(),
        }
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        format!(
            "# Wave weather; Settings > Weather saves this file.\n\
             provider={}\nlocation={}\nlatitude={}\nlongitude={}\ntimezone={}\n\
             enabled={}\nrefresh_minutes={}\nunits={}\nshow_on_home={}\n",
            self.provider,
            self.location,
            self.latitude,
            self.longitude,
            self.timezone,
            yes_no_marker(self.enabled),
            self.refresh_minutes,
            self.units.marker(),
            yes_no_marker(self.show_on_home)
        )
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.serialized())
            .with_context(|| format!("write weather configuration {}", path.display()))
    }

    /// Construct one bounded Open-Meteo request URL. The provider returns
    /// Fahrenheit and mph values directly so rendering does not mix units.
    /// Hourly data starts at the current hour.
    #[must_use]
    pub fn forecast_url(&self) -> String {
        format!(
            "https://api.open-meteo.com/v1/forecast?latitude={:.4}&longitude={:.4}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,is_day&hourly=temperature_2m,weather_code,precipitation_probability,is_day&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&temperature_unit=fahrenheit&wind_speed_unit=mph&timezone={}&forecast_days=5&forecast_hours=12",
            self.latitude, self.longitude, self.timezone
        )
    }
}

/// `2 hours`, `30 minutes` or `Manual`.
#[must_use]
pub fn refresh_label(minutes: u64) -> String {
    match minutes {
        0 => "Manual".into(),
        60 => "1 hour".into(),
        hours if hours % 60 == 0 => format!("{} hours", hours / 60),
        _ => format!("{minutes} minutes"),
    }
}

const fn on_off(value: bool) -> &'static str {
    if value {
        "On"
    } else {
        "Off"
    }
}

const fn yes_no_label(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

const fn yes_no_marker(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

/// A `yes`/`no` key that defaults to yes.
fn yes_no(values: &BTreeMap<String, String>, key: &str) -> Result<bool> {
    match values.get(key).map(String::as_str) {
        None | Some("yes") => Ok(true),
        Some("no") => Ok(false),
        Some(other) => bail!("{key} must be yes or no, not {other:?}"),
    }
}

fn required<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing required key {key:?}"))
}

/// The SD card guide's example, for tests and previews.
#[cfg(test)]
pub(crate) const SAMPLE_CONFIG: &str =
    "location=Auckland\nlatitude=-36.8485\nlongitude=174.7633\ntimezone=Pacific/Auckland\n";

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        WeatherConfig, WeatherSetting, WeatherUnits, DEFAULT_REFRESH_MINUTES,
        DEFAULT_WEATHER_PROVIDER,
    };

    #[test]
    fn parses_open_meteo_configuration_with_safe_defaults() {
        let config = WeatherConfig::parse(
            "location=Jersey City, NJ\nlatitude=40.7178\nlongitude=-74.0431\ntimezone=America/New_York\n",
        )
        .unwrap();
        assert_eq!(config.provider, DEFAULT_WEATHER_PROVIDER);
        assert_eq!(config.refresh_minutes, DEFAULT_REFRESH_MINUTES);
        assert!(config.enabled && config.show_on_home);
        assert_eq!(config.units, WeatherUnits::Metric);
        let url = config.forecast_url();
        assert!(url.contains("temperature_unit=fahrenheit"));
        assert!(url.contains("forecast_days=5") && url.contains("forecast_hours=12"));
        assert!(url.contains("hourly=temperature_2m,weather_code,precipitation_probability"));
    }

    #[test]
    fn settings_keys_parse_round_trip_and_drive_the_schedule() {
        let mut config = WeatherConfig::parse(
            "latitude=40.4168\nlongitude=-3.7038\nlocation=Madrid\nenabled=no\nrefresh_minutes=0\nunits=imperial\nshow_on_home=no\n",
        )
        .unwrap();
        assert!(!config.enabled && !config.show_on_home);
        assert_eq!(config.units, WeatherUnits::Imperial);
        assert_eq!(config.refresh_interval(), None);
        assert_eq!(WeatherConfig::parse(&config.serialized()).unwrap(), config);

        config.choose(WeatherSetting::Service, 0);
        assert_eq!(config.refresh_interval(), None);
        config.choose(WeatherSetting::Refresh, 2);
        assert_eq!(config.refresh_interval(), Some(Duration::from_secs(7200)));
        assert_eq!(config.value_label(WeatherSetting::Refresh), "2 hours");
        config.choose(WeatherSetting::Units, 9);
        assert_eq!(config.units, WeatherUnits::Imperial);
        assert!(WeatherConfig::parse("latitude=0\nlongitude=0\nenabled=maybe\n").is_err());
        assert!(WeatherConfig::parse("latitude=0\nlongitude=0\nunits=kelvin\n").is_err());
    }

    #[test]
    fn option_lists_mark_the_value_in_use() {
        let mut config = WeatherConfig::parse("latitude=0\nlongitude=0\n").unwrap();
        let (labels, current) = config.options(WeatherSetting::Refresh);
        assert_eq!(labels[current], "2 hours");
        let (labels, current) = config.options(WeatherSetting::Units);
        assert_eq!(labels[current], "°C · km/h");
        assert_eq!(config.options(WeatherSetting::ShowOnHome).0, ["Yes", "No"]);
        config.refresh_minutes = 45;
        assert_eq!(config.options(WeatherSetting::Refresh).1, usize::MAX);
        assert_eq!(config.value_label(WeatherSetting::Refresh), "45 minutes");
    }

    #[test]
    fn accepts_utc_and_explicit_refresh_interval() {
        let config = WeatherConfig::parse(
            "provider=open-meteo\nlatitude=0\nlongitude=0\ntimezone=UTC\nrefresh_minutes=60\n",
        )
        .unwrap();
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.refresh_minutes, 60);
    }

    #[test]
    fn defaults_to_auckland_timezone() {
        let parsed = WeatherConfig::parse("latitude=-36.8485\nlongitude=174.7633\n");
        let config = parsed.unwrap();
        assert_eq!(config.timezone, "Pacific/Auckland");
        assert!(config.forecast_url().contains("timezone=Pacific/Auckland"));
    }

    #[test]
    fn rejects_unknown_provider_invalid_coordinates_and_aggressive_refresh() {
        assert!(WeatherConfig::parse("provider=other\nlatitude=0\nlongitude=0\n").is_err());
        assert!(WeatherConfig::parse("latitude=91\nlongitude=0\n").is_err());
        assert!(WeatherConfig::parse("latitude=0\nlongitude=-181\n").is_err());
        assert!(WeatherConfig::parse("latitude=0\nlongitude=0\nrefresh_minutes=5\n").is_err());
    }
}
