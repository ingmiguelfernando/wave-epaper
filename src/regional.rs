//! Regional presentation policy for RTC and sensor values.
//!
//! The PCF85063 stores wall-clock fields without a timezone identifier. The
//! uploaded Waveshare sample uses UTC+08:00 as its RTC basis. Keep that basis
//! explicit and apply a timezone profile only at the presentation boundary.

use anyhow::{bail, Result};

use crate::{calendar::days_in_month, rtc::RtcDateTime};

/// RTC wall-clock basis inherited from the uploaded sample application.
pub const SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES: i16 = 8 * 60;
/// Product-facing default timezone profile.
pub const DEFAULT_TIMEZONE_NAME: &str = "Pacific/Auckland";
const SUPPORTED_TIMEZONES: &str = "Pacific/Auckland, America/New_York, UTC";

/// Temperature unit used by product-facing screens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
        }
    }

    /// Convert tenths of a degree Fahrenheit (the weather provider unit) into
    /// tenths of a degree in this unit, rounded to the nearest tenth.
    #[must_use]
    pub const fn from_fahrenheit_tenths(self, tenths_f: i16) -> i32 {
        let tenths = tenths_f as i32;
        match self {
            Self::Celsius => {
                let scaled = (tenths - 320) * 5;
                if scaled >= 0 {
                    (scaled + 4) / 9
                } else {
                    (scaled - 4) / 9
                }
            }
            Self::Fahrenheit => tenths,
        }
    }
}

/// Supported timezone profiles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimeZoneProfile {
    #[default]
    PacificAuckland,
    AmericaNewYork,
    Utc,
}

/// A daylight-saving boundary: the `sunday`-th Sunday of `month` (5 means the
/// last one) at `hour` local wall-clock time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Transition {
    month: u8,
    sunday: u8,
    hour: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DaylightSaving {
    offset_minutes: i16,
    abbreviation: &'static str,
    start: Transition,
    end: Transition,
}

impl TimeZoneProfile {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "Pacific/Auckland" => Ok(Self::PacificAuckland),
            "America/New_York" => Ok(Self::AmericaNewYork),
            "UTC" => Ok(Self::Utc),
            _ => bail!("unknown timezone {value:?}; supported: {SUPPORTED_TIMEZONES}"),
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PacificAuckland => "Pacific/Auckland",
            Self::AmericaNewYork => "America/New_York",
            Self::Utc => "UTC",
        }
    }

    /// Standard-time offset from UTC and its abbreviation.
    const fn standard(self) -> (i16, &'static str) {
        match self {
            Self::PacificAuckland => (12 * 60, "NZST"),
            Self::AmericaNewYork => (-5 * 60, "EST"),
            Self::Utc => (0, "UTC"),
        }
    }

    const fn daylight_saving(self) -> Option<DaylightSaving> {
        match self {
            Self::PacificAuckland => Some(DaylightSaving {
                offset_minutes: 13 * 60,
                abbreviation: "NZDT",
                start: Transition {
                    month: 9,
                    sunday: 5,
                    hour: 2,
                },
                end: Transition {
                    month: 4,
                    sunday: 1,
                    hour: 3,
                },
            }),
            Self::AmericaNewYork => Some(DaylightSaving {
                offset_minutes: -4 * 60,
                abbreviation: "EDT",
                start: Transition {
                    month: 3,
                    sunday: 2,
                    hour: 2,
                },
                end: Transition {
                    month: 11,
                    sunday: 1,
                    hour: 2,
                },
            }),
            Self::Utc => None,
        }
    }

    /// The daylight-saving rule in force at `utc`, if any.
    fn active_daylight_saving(self, utc: RtcDateTime) -> Option<DaylightSaving> {
        let daylight = self.daylight_saving()?;
        // Clocks spring forward from standard time and fall back from daylight time.
        let start = daylight.start.utc_key(utc.year, self.standard().0);
        let end = daylight.end.utc_key(utc.year, daylight.offset_minutes);
        let now = key_of(utc);
        let active = if start < end {
            now >= start && now < end
        } else {
            // Southern hemisphere: daylight time spans the new year.
            now >= start || now < end
        };
        active.then_some(daylight)
    }

    #[must_use]
    pub fn offset_minutes_for_utc(self, utc: RtcDateTime) -> i16 {
        self.active_daylight_saving(utc)
            .map_or(self.standard().0, |daylight| daylight.offset_minutes)
    }

    #[must_use]
    pub fn abbreviation_for_utc(self, utc: RtcDateTime) -> &'static str {
        self.active_daylight_saving(utc)
            .map_or(self.standard().1, |daylight| daylight.abbreviation)
    }
}

impl Transition {
    /// Sortable UTC key of this transition in `year`, given the offset in force
    /// just before it.
    fn utc_key(self, year: u16, offset_minutes: i16) -> u64 {
        let local = RtcDateTime {
            year,
            month: self.month,
            day: self.day(year),
            weekday: 0,
            hour: self.hour,
            minute: 0,
            second: 0,
        };
        key_of(local.shift_minutes(-i32::from(offset_minutes)))
    }

    fn day(self, year: u16) -> u8 {
        if self.sunday == 5 {
            last_sunday_of_month(year, self.month)
        } else {
            nth_sunday_of_month(year, self.month, self.sunday)
        }
    }
}

/// Regional presentation settings owned by UI state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionalPreferences {
    pub rtc_storage_utc_offset_minutes: i16,
    pub timezone: TimeZoneProfile,
    pub temperature_unit: TemperatureUnit,
}

impl Default for RegionalPreferences {
    fn default() -> Self {
        Self {
            rtc_storage_utc_offset_minutes: SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES,
            timezone: TimeZoneProfile::default(),
            temperature_unit: TemperatureUnit::default(),
        }
    }
}

impl RegionalPreferences {
    /// Apply a validated boot-time timezone profile from removable storage.
    pub fn with_timezone_name(mut self, timezone: &str) -> Result<Self> {
        self.timezone = TimeZoneProfile::parse(timezone)?;
        Ok(self)
    }

    #[must_use]
    pub const fn timezone_name(self) -> &'static str {
        self.timezone.name()
    }

    /// Convert the stored RTC wall clock into UTC.
    #[must_use]
    pub fn rtc_to_utc(self, rtc: RtcDateTime) -> RtcDateTime {
        rtc.shift_minutes(-i32::from(self.rtc_storage_utc_offset_minutes))
    }

    /// Convert stored RTC fields into the selected timezone with automatic DST.
    #[must_use]
    pub fn localize_rtc(self, rtc: RtcDateTime) -> RtcDateTime {
        let utc = self.rtc_to_utc(rtc);
        utc.shift_minutes(i32::from(self.timezone.offset_minutes_for_utc(utc)))
    }

    /// Convert a local schedule value into the retained RTC storage basis.
    /// In the repeated fall-back hour, prefer the earlier (daylight-time)
    /// candidate so alarms stay deterministic without storing a DST flag.
    #[must_use]
    pub fn local_to_rtc(self, local: RtcDateTime) -> RtcDateTime {
        let standard = local.shift_minutes(-i32::from(self.timezone.standard().0));
        let utc = match self.timezone.daylight_saving() {
            Some(daylight) => {
                let candidate = local.shift_minutes(-i32::from(daylight.offset_minutes));
                if self.timezone.offset_minutes_for_utc(candidate) == daylight.offset_minutes {
                    candidate
                } else {
                    standard
                }
            }
            None => standard,
        };
        utc.shift_minutes(i32::from(self.rtc_storage_utc_offset_minutes))
    }

    /// Render the selected timezone, using standard time when the date is unknown.
    #[must_use]
    pub fn timezone_label_for_rtc(self, rtc: Option<RtcDateTime>) -> String {
        let (offset, abbreviation) = match rtc {
            Some(rtc) => {
                let utc = self.rtc_to_utc(rtc);
                (
                    self.timezone.offset_minutes_for_utc(utc),
                    self.timezone.abbreviation_for_utc(utc),
                )
            }
            None => self.timezone.standard(),
        };
        format!("{abbreviation} {}", format_utc_offset(offset))
    }

    /// Compatibility label for startup logs before an RTC snapshot exists.
    #[must_use]
    pub fn timezone_label(self) -> String {
        self.timezone_label_for_rtc(None)
    }

    #[must_use]
    pub fn rtc_storage_label(self) -> String {
        format_utc_offset(self.rtc_storage_utc_offset_minutes)
    }
}

/// Render an offset using the user-facing `UTC+HH:MM` form.
#[must_use]
pub fn format_utc_offset(minutes: i16) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let magnitude = i32::from(minutes).abs();
    format!("UTC{sign}{:02}:{:02}", magnitude / 60, magnitude % 60)
}

fn nth_sunday_of_month(year: u16, month: u8, nth: u8) -> u8 {
    let first_weekday = weekday_for_date(year, month, 1);
    let first_sunday = if first_weekday == 0 {
        1
    } else {
        8 - first_weekday
    };
    first_sunday + 7 * (nth - 1)
}

fn last_sunday_of_month(year: u16, month: u8) -> u8 {
    let last_day = days_in_month(year, month);
    last_day - weekday_for_date(year, month, last_day)
}

/// Sunday is zero, matching the RTC weekday convention used by this firmware.
fn weekday_for_date(year: u16, month: u8, day: u8) -> u8 {
    let table = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = i32::from(year);
    if month < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + table[usize::from(month - 1)] + i32::from(day))
        .rem_euclid(7) as u8
}

fn key_of(value: RtcDateTime) -> u64 {
    u64::from(value.year) * 10_000_000_000
        + u64::from(value.month) * 100_000_000
        + u64::from(value.day) * 1_000_000
        + u64::from(value.hour) * 10_000
        + u64::from(value.minute) * 100
        + u64::from(value.second)
}

#[cfg(test)]
mod tests {
    use super::{
        format_utc_offset, RegionalPreferences, TemperatureUnit, TimeZoneProfile,
        SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES,
    };
    use crate::rtc::RtcDateTime;

    fn utc(month: u8, day: u8, hour: u8) -> RtcDateTime {
        RtcDateTime {
            year: 2026,
            month,
            day,
            weekday: 0,
            hour,
            minute: 0,
            second: 0,
        }
    }

    fn new_york() -> RegionalPreferences {
        RegionalPreferences::default()
            .with_timezone_name("America/New_York")
            .unwrap()
    }

    #[test]
    fn defaults_to_auckland_and_celsius() {
        let preferences = RegionalPreferences::default();
        assert_eq!(preferences.timezone_name(), "Pacific/Auckland");
        assert_eq!(preferences.temperature_unit, TemperatureUnit::Celsius);
        assert_eq!(preferences.timezone_label(), "NZST UTC+12:00");
    }

    #[test]
    fn converts_local_alarm_schedule_back_into_rtc_storage_basis() {
        let local = RtcDateTime {
            year: 2026,
            month: 6,
            day: 3,
            weekday: 3,
            hour: 7,
            minute: 30,
            second: 0,
        };
        let stored = new_york().local_to_rtc(local);
        assert_eq!(stored.date_time(), "2026-06-03  19:30:00");
        assert_eq!(new_york().localize_rtc(stored), local);

        let auckland = RegionalPreferences::default();
        let stored = auckland.local_to_rtc(local);
        assert_eq!(stored.date_time(), "2026-06-03  03:30:00");
        assert_eq!(auckland.localize_rtc(stored), local);
    }

    #[test]
    fn records_uploaded_sample_rtc_storage_basis_explicitly() {
        assert_eq!(SAMPLE_RTC_STORAGE_UTC_OFFSET_MINUTES, 480);
    }

    #[test]
    fn new_york_profile_applies_automatic_dst_transitions() {
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(1, 4, 12)),
            -300
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(6, 4, 12)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(3, 8, 6)),
            -300
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(3, 8, 7)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(11, 1, 5)),
            -240
        );
        assert_eq!(
            TimeZoneProfile::AmericaNewYork.offset_minutes_for_utc(utc(11, 1, 6)),
            -300
        );
    }

    #[test]
    fn auckland_profile_applies_southern_hemisphere_dst() {
        let auckland = TimeZoneProfile::PacificAuckland;
        // Daylight time ends 2026-04-05 03:00 NZDT, which is 04-04 14:00 UTC.
        assert_eq!(auckland.offset_minutes_for_utc(utc(4, 4, 13)), 13 * 60);
        assert_eq!(auckland.offset_minutes_for_utc(utc(4, 4, 14)), 12 * 60);
        // Daylight time starts 2026-09-27 02:00 NZST, which is 09-26 14:00 UTC.
        assert_eq!(auckland.offset_minutes_for_utc(utc(9, 26, 13)), 12 * 60);
        assert_eq!(auckland.offset_minutes_for_utc(utc(9, 26, 14)), 13 * 60);
        assert_eq!(auckland.abbreviation_for_utc(utc(1, 15, 12)), "NZDT");
        assert_eq!(auckland.abbreviation_for_utc(utc(6, 15, 12)), "NZST");
    }

    #[test]
    fn localizes_sample_storage_clock_into_local_time() {
        let sample_wall_clock = RtcDateTime {
            year: 2026,
            month: 6,
            day: 4,
            weekday: 4,
            hour: 1,
            minute: 25,
            second: 30,
        };
        assert_eq!(
            new_york().localize_rtc(sample_wall_clock).date_time(),
            "2026-06-03  13:25:30"
        );
        assert_eq!(
            RegionalPreferences::default()
                .localize_rtc(sample_wall_clock)
                .date_time(),
            "2026-06-04  05:25:30"
        );
    }

    #[test]
    fn accepts_utc_profile_and_formats_offsets() {
        let preferences = RegionalPreferences::default()
            .with_timezone_name("UTC")
            .unwrap();
        assert_eq!(preferences.timezone_name(), "UTC");
        assert!(RegionalPreferences::default()
            .with_timezone_name("Mars/Olympus")
            .is_err());
        assert_eq!(format_utc_offset(480), "UTC+08:00");
        assert_eq!(format_utc_offset(-240), "UTC-04:00");
        assert_eq!(format_utc_offset(330), "UTC+05:30");
    }

    #[test]
    fn converts_provider_fahrenheit_to_rounded_tenths() {
        assert_eq!(TemperatureUnit::Celsius.from_fahrenheit_tenths(784), 258);
        assert_eq!(TemperatureUnit::Celsius.from_fahrenheit_tenths(140), -100);
        assert_eq!(TemperatureUnit::Fahrenheit.from_fahrenheit_tenths(-5), -5);
    }
}
