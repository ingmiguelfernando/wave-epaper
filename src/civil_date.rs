//! Shared proleptic-Gregorian calendar arithmetic (Howard Hinnant's algorithm).
//!
//! Days are counted since 1970-01-01 and may be negative for earlier dates.
//! `days_from_civil` and `civil_from_days` are total and panic-free: a month
//! or day of month outside a calendar month rolls over arithmetically (month
//! 13 is January of the next year, day 0 the last day of the previous month)
//! and the arithmetic saturates instead of overflowing. Callers that must
//! reject invalid dates (`rtc::validate_datetime`,
//! `reading_stats::parse_day`) check the fields first and keep their own
//! error messages.

/// Short weekday names starting on Sunday.
pub const WEEKDAY_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// Short month names from January to December.
pub const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Weekday names starting on Sunday.
pub const WEEKDAY_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Month names from January to December.
pub const MONTH_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Days since 1970-01-01 for a proleptic-Gregorian date (Howard Hinnant's
/// algorithm). Out-of-range months and days roll over arithmetically, so the
/// function is total; results are exact while the day count fits in `i64`.
#[must_use]
pub fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
    let month = i64::from(month);
    let year = year.saturating_sub(i64::from(month <= 2));
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era.saturating_mul(146_097)
        .saturating_add(day_of_era)
        .saturating_sub(719_468)
}

/// Inverse of [`days_from_civil`]: the `(year, month, day)` of a day count.
/// The offset is applied with saturating arithmetic, so no input panics.
#[must_use]
pub fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let z = days.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month as u8, day as u8)
}

/// Weekday of a day count, 0 is Sunday: 1970-01-01 is 4 (Thursday).
#[must_use]
pub fn weekday(days: i64) -> u8 {
    ((days.rem_euclid(7) + 4) % 7) as u8
}

/// Days in a Gregorian calendar month; 0 for a month outside 1..=12, as the
/// firmware's `rtc` validation helpers have always reported it.
#[must_use]
pub fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Proleptic-Gregorian leap year rule: divisible by 4, except centuries that
/// are not divisible by 400.
#[must_use]
pub fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, days_from_civil, days_in_month, weekday};

    #[test]
    fn epoch_is_thursday_1970_01_01() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(weekday(0), 4);
        assert_eq!(weekday(-1), 3);
    }

    #[test]
    fn accepts_the_2000_leap_day() {
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_from_civil(2000, 2, 29), 11_016);
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(weekday(11_016), 2);
    }

    #[test]
    fn saturday_2026_10_03_is_weekday_six() {
        assert_eq!(days_from_civil(2026, 10, 3), 20_729);
        assert_eq!(civil_from_days(20_729), (2026, 10, 3));
        assert_eq!(weekday(20_729), 6);
    }

    #[test]
    fn last_day_of_every_month_in_leap_and_common_years() {
        for (year, february) in [(2024, 29), (2026, 28)] {
            let last_days = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            for (index, last) in last_days.iter().enumerate() {
                let month = u8::try_from(index + 1).unwrap();
                assert_eq!(days_in_month(year, month), *last, "{year}-{month:02}");
                assert_eq!(
                    civil_from_days(days_from_civil(year, month, *last)),
                    (year, month, *last),
                    "{year}-{month:02}"
                );
                // The day after a month's last day is the first of the next one.
                let (next_year, next_month, next_day) =
                    civil_from_days(days_from_civil(year, month, *last) + 1);
                assert_eq!(next_day, 1);
                assert_eq!(
                    (next_year, next_month),
                    if month == 12 {
                        (year + 1, 1)
                    } else {
                        (year, month + 1)
                    },
                    "{year}-{month:02}"
                );
            }
        }
    }

    #[test]
    fn round_trips_sample_dates_across_centuries() {
        for (year, month, day) in [
            (1600, 2, 29), // leap century
            (1600, 12, 31),
            (1900, 2, 28), // common century
            (1900, 3, 1),
            (1970, 1, 1),
            (1970, 1, 2),
            (2000, 1, 1),
            (2026, 10, 3),
            (2400, 2, 29),
            (2400, 7, 4),
        ] {
            let days = days_from_civil(year, month, day);
            assert_eq!(
                civil_from_days(days),
                (year, month, day),
                "{year:04}-{month:02}-{day:02}"
            );
        }
    }

    #[test]
    fn weekday_advances_by_one_each_day() {
        // Two years from 2024-01-01 cross the 2024 leap day; the short range
        // around zero crosses the epoch with negative day counts.
        for days in days_from_civil(2024, 1, 1)..days_from_civil(2026, 1, 1) {
            assert_eq!(weekday(days + 1), (weekday(days) + 1) % 7, "day {days}");
        }
        for days in -3..3 {
            assert_eq!(weekday(days + 1), (weekday(days) + 1) % 7, "day {days}");
        }
    }
}
