//! Last good forecast on the SD card, so a restart can show it before the
//! next fetch. Text as the other settings files: `# Wave forecast v1`, then
//! one `key=value` per line.

use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    rtc::RtcDateTime,
    sd_file,
    weather::{CurrentConditions, DailyForecast, HourlyForecast, WeatherData},
};

/// SD location of the forecast cache written after each successful fetch.
pub const FORECAST_CACHE_PATH: &str = "/sdcard/RUSTMIX/FORECAST.TXT";
const HEADER: &str = "# Wave forecast v1";

/// Serialize `data` with the location it was fetched for.
#[must_use]
pub fn serialize(location: &str, data: &WeatherData) -> String {
    let mut text = format!(
        "{HEADER}\nlocation={location}\ntimezone={}\n",
        data.provider_timezone
    );
    let current = &data.current;
    text.push_str(&format!(
        "current={}|{}|{}|{}|{}|{}|{}\n",
        current.observed_at,
        current.weather_code,
        current.temperature_tenths_f,
        current.apparent_temperature_tenths_f,
        current.humidity_percent,
        current.wind_speed_tenths_mph,
        flag(current.is_day)
    ));
    for day in &data.forecast {
        text.push_str(&format!(
            "day={}|{}|{}|{}|{}\n",
            day.date,
            day.weather_code,
            day.high_tenths_f,
            day.low_tenths_f,
            optional_u8(day.precipitation_probability_percent)
        ));
    }
    for hour in &data.hourly {
        text.push_str(&format!(
            "hour={}|{}|{}|{}|{}\n",
            hour.time,
            hour.weather_code,
            hour.temperature_tenths_f,
            optional_u8(hour.precipitation_probability_percent),
            flag(hour.is_day)
        ));
    }
    text
}

/// Parse a cache file. `None` for another version, an empty file, a broken
/// line, or a location other than `location`.
#[must_use]
pub fn parse(text: &str, location: &str) -> Option<WeatherData> {
    let mut lines = text.lines();
    if lines.next()?.trim() != HEADER {
        return None;
    }
    let mut stored_location = None;
    let mut timezone = None;
    let mut current = None;
    let mut forecast = Vec::new();
    let mut hourly = Vec::new();
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=')?;
        match key.trim() {
            "location" => stored_location = Some(value.trim().to_string()),
            "timezone" => timezone = Some(value.trim().to_string()),
            "current" => current = Some(parse_current(value)?),
            "day" => forecast.push(parse_day(value)?),
            "hour" => hourly.push(parse_hour(value)?),
            _ => {}
        }
    }
    if stored_location.as_deref()? != location {
        return None;
    }
    Some(WeatherData {
        provider_timezone: timezone?,
        current: current?,
        forecast,
        hourly,
    })
}

/// Write the cache with `sd_file::replace`.
pub fn save_to_path(path: impl AsRef<Path>, location: &str, data: &WeatherData) -> Result<()> {
    let path = path.as_ref();
    let text = serialize(location, data);
    sd_file::replace(path, &text)
        .with_context(|| format!("write forecast cache {}", path.display()))
}

/// Read the cache for `location`, dropping hours before the current local
/// hour and days before today when `now` is set. A missing, empty, broken,
/// wrong-version or wrong-location file is `None`, never an error.
#[must_use]
pub fn load_from_path(
    path: impl AsRef<Path>,
    location: &str,
    now: Option<RtcDateTime>,
) -> Option<WeatherData> {
    let text = sd_file::read_to_string(path.as_ref()).ok()?;
    let mut data = parse(&text, location)?;
    if let Some(now) = now {
        drop_passed(&mut data, now);
    }
    Some(data)
}

fn drop_passed(data: &mut WeatherData, now: RtcDateTime) {
    let today = format!("{:04}-{:02}-{:02}", now.year, now.month, now.day);
    let hour_prefix = format!("{today}T{:02}", now.hour);
    data.forecast
        .retain(|day| day.date.as_str() >= today.as_str());
    data.hourly.retain(|hour| {
        hour.time
            .get(..13)
            .is_some_and(|prefix| prefix >= hour_prefix.as_str())
    });
}

fn parse_current(value: &str) -> Option<CurrentConditions> {
    let fields: Vec<_> = value.split('|').collect();
    if fields.len() != 7 || fields[0].is_empty() {
        return None;
    }
    Some(CurrentConditions {
        observed_at: fields[0].into(),
        weather_code: fields[1].parse().ok()?,
        temperature_tenths_f: fields[2].parse().ok()?,
        apparent_temperature_tenths_f: fields[3].parse().ok()?,
        humidity_percent: fields[4].parse().ok()?,
        wind_speed_tenths_mph: fields[5].parse().ok()?,
        is_day: parse_flag(fields[6])?,
    })
}

fn parse_day(value: &str) -> Option<DailyForecast> {
    let fields: Vec<_> = value.split('|').collect();
    if fields.len() != 5 || fields[0].is_empty() {
        return None;
    }
    Some(DailyForecast {
        date: fields[0].into(),
        weather_code: fields[1].parse().ok()?,
        high_tenths_f: fields[2].parse().ok()?,
        low_tenths_f: fields[3].parse().ok()?,
        precipitation_probability_percent: parse_optional_u8(fields[4])?,
    })
}

fn parse_hour(value: &str) -> Option<HourlyForecast> {
    let fields: Vec<_> = value.split('|').collect();
    if fields.len() != 5 || fields[0].is_empty() {
        return None;
    }
    Some(HourlyForecast {
        time: fields[0].into(),
        weather_code: fields[1].parse().ok()?,
        temperature_tenths_f: fields[2].parse().ok()?,
        precipitation_probability_percent: parse_optional_u8(fields[3])?,
        is_day: parse_flag(fields[4])?,
    })
}

fn parse_flag(text: &str) -> Option<bool> {
    match text {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn parse_optional_u8(text: &str) -> Option<Option<u8>> {
    if text.is_empty() {
        Some(None)
    } else {
        text.parse().ok().map(Some)
    }
}

fn flag(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn optional_u8(value: Option<u8>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{drop_passed, load_from_path, parse, save_to_path, serialize, FORECAST_CACHE_PATH};
    use crate::{
        rtc::RtcDateTime,
        weather::{CurrentConditions, DailyForecast, HourlyForecast, WeatherData},
    };

    const LOCATION: &str = "Jersey City, NJ";

    fn sample_data() -> WeatherData {
        WeatherData {
            provider_timezone: "America/New_York".into(),
            current: CurrentConditions {
                observed_at: "2026-10-02T13:30".into(),
                weather_code: 2,
                temperature_tenths_f: -12,
                apparent_temperature_tenths_f: -30,
                humidity_percent: 61,
                wind_speed_tenths_mph: 75,
                is_day: true,
            },
            forecast: vec![
                DailyForecast {
                    date: "2026-10-02".into(),
                    weather_code: 2,
                    high_tenths_f: 698,
                    low_tenths_f: -40,
                    precipitation_probability_percent: Some(10),
                },
                DailyForecast {
                    date: "2026-10-03".into(),
                    weather_code: 0,
                    high_tenths_f: 734,
                    low_tenths_f: 536,
                    precipitation_probability_percent: None,
                },
            ],
            hourly: vec![
                HourlyForecast {
                    time: "2026-10-02T13:00".into(),
                    weather_code: 2,
                    temperature_tenths_f: -12,
                    precipitation_probability_percent: Some(10),
                    is_day: true,
                },
                HourlyForecast {
                    time: "2026-10-02T14:00".into(),
                    weather_code: 2,
                    temperature_tenths_f: 649,
                    precipitation_probability_percent: None,
                    is_day: true,
                },
                HourlyForecast {
                    time: "2026-10-02T15:00".into(),
                    weather_code: 3,
                    temperature_tenths_f: 651,
                    precipitation_probability_percent: Some(20),
                    is_day: true,
                },
            ],
        }
    }

    fn at(month: u8, day: u8, hour: u8) -> RtcDateTime {
        RtcDateTime {
            year: 2026,
            month,
            day,
            hour,
            minute: 30,
            second: 0,
            weekday: 5,
        }
    }

    #[test]
    fn path_is_the_card_forecast_file() {
        assert_eq!(FORECAST_CACHE_PATH, "/sdcard/RUSTMIX/FORECAST.TXT");
    }

    #[test]
    fn round_trips_a_full_payload_including_negatives_and_missing_rain() {
        let data = sample_data();
        let text = serialize(LOCATION, &data);
        let restored = parse(&text, LOCATION).unwrap();
        assert_eq!(restored, data);
    }

    #[test]
    fn drops_past_hours_and_days_with_a_clock_and_keeps_them_without_one() {
        let data = sample_data();
        let text = serialize(LOCATION, &data);
        let kept = parse(&text, LOCATION).unwrap();
        let mut dropped = kept.clone();
        // 15:30: the 13:00 and 14:00 hours are past; 15:00 is the current hour.
        drop_passed(&mut dropped, at(10, 2, 15));
        assert_eq!(dropped.hourly.len(), 1);
        assert_eq!(dropped.hourly[0].time, "2026-10-02T15:00");
        assert_eq!(dropped.forecast.len(), 2);

        let mut day_only = kept.clone();
        drop_passed(&mut day_only, at(10, 3, 0));
        assert_eq!(day_only.forecast.len(), 1);
        assert_eq!(day_only.forecast[0].date, "2026-10-03");
        assert!(day_only.hourly.is_empty());

        let path = std::env::temp_dir().join(format!("wave-forecast-keep-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        save_to_path(&path, LOCATION, &data).unwrap();
        let without_clock = load_from_path(&path, LOCATION, None).unwrap();
        assert_eq!(without_clock, data);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ignores_other_location_version_broken_and_empty() {
        let data = sample_data();
        let text = serialize(LOCATION, &data);
        assert!(parse(&text, "Madrid").is_none());
        assert!(parse("# Wave forecast v2\nlocation=x\n", LOCATION).is_none());
        assert!(parse("", LOCATION).is_none());
        assert!(parse("\n", LOCATION).is_none());
        let broken = text.replace("day=2026-10-02|", "day=2026-10-02");
        assert!(parse(&broken, LOCATION).is_none());
        let path =
            std::env::temp_dir().join(format!("wave-forecast-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(load_from_path(&path, LOCATION, None).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_and_load_round_trip_on_disk() {
        let data = sample_data();
        let path = std::env::temp_dir().join(format!("wave-forecast-round-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        save_to_path(&path, LOCATION, &data).unwrap();
        let restored = load_from_path(&path, LOCATION, Some(at(10, 2, 13))).unwrap();
        assert_eq!(restored, data);
        let _ = std::fs::remove_file(&path);
    }
}
