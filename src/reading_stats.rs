//! Bounded reading history and key-driven time accounting, without runtime wiring.

use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};

use crate::civil_date;

pub const READING_STATS_PATH: &str = "/sdcard/RUSTMIX/READER/STATS.TXT";
const MAX_DAYS: usize = 400;
const MAX_BOOKS: usize = 200;
const GRACE_MS: u64 = 120_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DayStats {
    pub seconds: u32,
    pub pages: u32,
}

impl DayStats {
    fn add(&mut self, seconds: u32, pages: u32) {
        self.seconds = self.seconds.saturating_add(seconds);
        self.pages = self.pages.saturating_add(pages);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BookStats {
    path: String,
    totals: DayStats,
    finished: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReadingStats {
    days: BTreeMap<u32, DayStats>,
    // Oldest touched book first; serialized order preserves eviction across loads.
    books: Vec<BookStats>,
    unsaved: bool,
}

impl ReadingStats {
    /// Valid books become most recently touched, independent of the recorded day.
    /// Invalid book names are ignored; their daily totals still count.
    pub fn record(&mut self, day: u32, seconds: u32, pages: u32, book: Option<&str>) {
        if seconds == 0 && pages == 0 {
            return;
        }
        if self.days.contains_key(&day)
            || self.days.len() < MAX_DAYS
            || self
                .days
                .first_key_value()
                .is_some_and(|(&oldest, _)| day > oldest)
        {
            let totals = self.days.entry(day).or_default();
            let before = *totals;
            totals.add(seconds, pages);
            self.unsaved |= *totals != before;
            self.trim_days();
        }
        if let Some(path) = book.filter(|path| valid_book(path)) {
            let mut entry = self.remove_book(path).unwrap_or_else(|| BookStats {
                path: path.into(),
                totals: DayStats::default(),
                finished: false,
            });
            entry.totals.add(seconds, pages);
            self.push_book(entry);
            self.unsaved = true;
        }
    }

    /// Finishing an already-finished retained path does not alter its recency.
    pub fn mark_finished(&mut self, book: &str) {
        if !valid_book(book)
            || self
                .books
                .iter()
                .any(|entry| entry.path == book && entry.finished)
        {
            return;
        }
        let mut entry = self.remove_book(book).unwrap_or_else(|| BookStats {
            path: book.into(),
            totals: DayStats::default(),
            finished: false,
        });
        entry.finished = true;
        self.push_book(entry);
        self.unsaved = true;
    }

    #[must_use]
    pub fn day(&self, day: u32) -> DayStats {
        self.days.get(&day).copied().unwrap_or_default()
    }

    #[must_use]
    pub fn week(&self, today: u32) -> [u32; 7] {
        std::array::from_fn(|index| {
            today
                .checked_sub(6 - index as u32)
                .map_or(0, |day| self.day(day).seconds / 60)
        })
    }

    #[must_use]
    pub fn streak(&self, today: u32) -> u32 {
        let mut cursor = if self.day(today).seconds >= 300 {
            Some(today)
        } else {
            today.checked_sub(1)
        };
        let mut count = 0;
        while let Some(day) = cursor {
            if self.day(day).seconds < 300 {
                break;
            }
            count += 1;
            cursor = day.checked_sub(1);
        }
        count
    }

    /// Longest run of consecutive retained days with 5 minutes or more.
    #[must_use]
    pub fn best_streak(&self) -> u32 {
        let mut best = 0;
        let mut run = 0;
        let mut run_end: Option<u32> = None;
        for (&day, totals) in &self.days {
            if totals.seconds < 300 {
                run = 0;
                run_end = None;
                continue;
            }
            run = if run_end.is_some_and(|end| end.checked_add(1) == Some(day)) {
                run + 1
            } else {
                1
            };
            run_end = Some(day);
            best = best.max(run);
        }
        best
    }

    /// Sum of the retained days in `[first, last]`, both included.
    #[must_use]
    pub fn total(&self, first: u32, last: u32) -> DayStats {
        let mut sum = DayStats::default();
        if first > last {
            return sum;
        }
        for totals in self.days.range(first..=last).map(|(_, totals)| totals) {
            sum.add(totals.seconds, totals.pages);
        }
        sum
    }

    /// Totals recorded against one book path.
    #[must_use]
    pub fn book(&self, path: &str) -> Option<DayStats> {
        self.books
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.totals)
    }

    /// Finished count covers retained books, not a lifetime counter.
    #[must_use]
    pub fn books_finished(&self) -> usize {
        self.books.iter().filter(|entry| entry.finished).count()
    }

    /// Whether `path` is marked finished.
    #[must_use]
    pub fn is_finished(&self, path: &str) -> bool {
        self.books
            .iter()
            .any(|entry| entry.path == path && entry.finished)
    }

    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.unsaved
    }

    pub fn mark_saved(&mut self) {
        self.unsaved = false;
    }

    /// Unknown records are skipped; duplicate keys use the last complete record.
    pub fn parse(text: &str) -> Result<Self> {
        let mut stats = Self::default();
        for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = line.trim_matches(' ');
            let kind = line.split('|').next().unwrap_or_default();
            if kind != "day" && kind != "book" {
                continue;
            }
            let parsed = (|| -> Result<()> {
                if line.chars().any(char::is_control) {
                    bail!("control character in record");
                }
                let fields: Vec<_> = line.split('|').collect();
                match kind {
                    "day" => {
                        if fields.len() != 4 {
                            bail!("day requires date, seconds and pages");
                        }
                        let day = parse_day(fields[1])?;
                        stats.days.insert(
                            day,
                            DayStats {
                                seconds: parse_count(fields[2])?,
                                pages: parse_count(fields[3])?,
                            },
                        );
                        stats.trim_days();
                    }
                    "book" => {
                        if fields.len() != 5 || !valid_book(fields[1]) {
                            bail!("book requires a valid path, seconds, pages and finished flag");
                        }
                        let finished = match fields[4] {
                            "0" => false,
                            "1" => true,
                            _ => bail!("finished must be 0 or 1"),
                        };
                        let entry = BookStats {
                            path: fields[1].into(),
                            totals: DayStats {
                                seconds: parse_count(fields[2])?,
                                pages: parse_count(fields[3])?,
                            },
                            finished,
                        };
                        stats.remove_book(&entry.path);
                        stats.push_book(entry);
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })();
            parsed.with_context(|| format!("reading stats line {}", index + 1))?;
        }
        Ok(stats)
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        let mut text = String::from("# Wave reading stats v1\n");
        for (&day, totals) in &self.days {
            text.push_str(&format!(
                "day|{}|{}|{}\n",
                day_label(day),
                totals.seconds,
                totals.pages
            ));
        }
        for entry in &self.books {
            text.push_str(&format!(
                "book|{}|{}|{}|{}\n",
                entry.path,
                entry.totals.seconds,
                entry.totals.pages,
                u8::from(entry.finished)
            ));
        }
        text
    }

    /// Falls back to the `.BAK` copy left by a save interrupted between renames.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = crate::sd_file::read_to_string(path)
            .with_context(|| format!("read reading stats {}", path.display()))?;
        Self::parse(&text)
    }

    /// FAT cannot rename onto an existing file, so `crate::sd_file::replace`
    /// moves the old file to `.BAK` until the new one is in place.
    pub fn save_to_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        crate::sd_file::replace(path, &self.serialized())
            .with_context(|| format!("save reading stats {}", path.display()))?;
        self.mark_saved();
        Ok(())
    }

    fn trim_days(&mut self) {
        while self.days.len() > MAX_DAYS {
            self.days.pop_first();
        }
    }

    fn remove_book(&mut self, path: &str) -> Option<BookStats> {
        self.books
            .iter()
            .position(|entry| entry.path == path)
            .map(|index| self.books.remove(index))
    }

    fn push_book(&mut self, entry: BookStats) {
        self.books.push(entry);
        if self.books.len() > MAX_BOOKS {
            self.books.remove(0);
        }
    }
}

fn valid_book(path: &str) -> bool {
    !path.trim().is_empty()
        && !path
            .chars()
            .any(|ch| ch.is_control() || "|<>:\"?*".contains(ch))
}

fn parse_count(text: &str) -> Result<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("invalid unsigned count");
    }
    text.parse().context("count exceeds u32")
}

/// Parse a Gregorian Unix date; expanded years support the entire u32 day range.
pub fn parse_day(text: &str) -> Result<u32> {
    let fields: Vec<_> = text.split('-').collect();
    if fields.len() != 3 || fields[0].len() < 4 || fields[1].len() != 2 || fields[2].len() != 2 {
        bail!("date must be YYYY-MM-DD");
    }
    let year = i64::from(parse_count(fields[0])?);
    let month = parse_count(fields[1])?;
    let day = parse_count(fields[2])?;
    if !(1..=12).contains(&month) {
        bail!("invalid month");
    }
    if day == 0 || day > u32::from(civil_date::days_in_month(year, month as u8)) {
        bail!("invalid day of month");
    }
    // Both fields are in calendar range after the checks above.
    let days = civil_date::days_from_civil(year, month as u8, day as u8);
    u32::try_from(days).context("date outside u32 Unix days")
}

#[must_use]
pub fn day_label(epoch_day: u32) -> String {
    let (year, month, day) = civil_date::civil_from_days(i64::from(epoch_day));
    format!("{year:04}-{month:02}-{day:02}")
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReadingClock {
    last_key: Option<u64>,
    cursor: u64,
    pending_ms: u64,
}

impl ReadingClock {
    /// Regressing timestamps are ignored; expired eligible time is kept before restart.
    pub fn on_key(&mut self, now_ms: u64) {
        if self.last_key.is_some() && now_ms < self.cursor {
            return;
        }
        self.accrue(now_ms);
        self.last_key = Some(now_ms);
        self.cursor = now_ms;
    }

    /// Fractions and any seconds beyond u32::MAX remain available for the next call.
    pub fn take_seconds(&mut self, now_ms: u64) -> u32 {
        self.accrue(now_ms);
        let seconds = (self.pending_ms / 1_000).min(u64::from(u32::MAX)) as u32;
        self.pending_ms -= u64::from(seconds) * 1_000;
        seconds
    }

    fn accrue(&mut self, now_ms: u64) {
        let Some(key) = self.last_key else { return };
        if now_ms < self.cursor {
            return;
        }
        let end = now_ms.min(key.saturating_add(GRACE_MS));
        self.pending_ms = self
            .pending_ms
            .saturating_add(end.saturating_sub(self.cursor));
        self.cursor = now_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs, io,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "wave-reading-stats-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn stats_path(&self) -> PathBuf {
            self.0.join("STATS.TXT")
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn default_is_empty_clean_and_uses_the_required_path() {
        assert_eq!(READING_STATS_PATH, "/sdcard/RUSTMIX/READER/STATS.TXT");
        let stats = ReadingStats::default();
        assert!(!stats.has_unsaved());
        assert_eq!(stats.day(0), DayStats::default());
        assert_eq!(stats.week(0), [0; 7]);
        assert_eq!(stats.streak(0), 0);
        assert_eq!(stats.books_finished(), 0);
        assert_eq!(stats.serialized(), "# Wave reading stats v1\n");
    }

    #[test]
    fn dates_match_unix_and_existing_rtc_helpers_in_their_valid_range() {
        assert_eq!(parse_day("1970-01-01").unwrap(), 0);
        assert_eq!(parse_day("2000-01-01").unwrap(), 10_957);
        assert_eq!(parse_day("2026-10-03").unwrap(), 20_729);
        for label in ["2000-02-29", "2026-10-03", "2099-12-31"] {
            let day = parse_day(label).unwrap();
            let rtc = crate::ntp::utc_from_unix_seconds(u64::from(day) * 86_400);
            assert_eq!(rtc.epoch_minutes() / 1_440 + 10_957, day);
            assert_eq!(
                rtc.date_time().split_whitespace().next().unwrap(),
                day_label(day)
            );
        }
    }

    #[test]
    fn dates_roundtrip_at_leap_centuries_and_the_full_day_range() {
        for label in [
            "1972-02-29",
            "2000-02-29",
            "2100-03-01",
            "2400-02-29",
            "9999-12-31",
            "10000-01-01",
        ] {
            assert_eq!(day_label(parse_day(label).unwrap()), label);
        }
        for day in [0, 1, 365, 10_957, 20_729, u32::MAX - 1, u32::MAX] {
            assert_eq!(parse_day(&day_label(day)).unwrap(), day);
        }
        for day in 10_900..11_100 {
            assert_eq!(parse_day(&day_label(day)).unwrap(), day);
        }
    }

    #[test]
    fn dates_reject_bad_shapes_ranges_leap_days_and_utf8_without_panics() {
        for label in [
            "",
            "2026-1-01",
            "2026-01-1",
            "026-01-01",
            "2026/01/01",
            "2026-01-01-extra",
            "2026-00-01",
            "2026-13-01",
            "2026-01-00",
            "2026-01-32",
            "2026-04-31",
            "2026-02-29",
            "2100-02-29",
            "1900-02-29",
            "1969-12-31",
            "+2026-01-01",
            "2026-+1-01",
            "２０２６-01-01",
            "éééé-01-01",
            "2026-01-01\n",
            "4294967295-12-31",
            "4294967296-01-01",
            "99999999999999999999-01-01",
        ] {
            assert!(parse_day(label).is_err(), "{label:?}");
        }
    }

    #[test]
    fn records_accumulate_days_and_books_independently() {
        let mut stats = ReadingStats::default();
        stats.record(2, 120, 3, Some("/books/Café.txt"));
        stats.record(1, 60, 1, Some("/books/Café.txt"));
        stats.record(2, 30, 2, None);
        assert_eq!(
            stats.day(1),
            DayStats {
                seconds: 60,
                pages: 1
            }
        );
        assert_eq!(
            stats.day(2),
            DayStats {
                seconds: 150,
                pages: 5
            }
        );
        assert_eq!(
            stats.books[0].totals,
            DayStats {
                seconds: 180,
                pages: 4
            }
        );
        assert!(stats.has_unsaved());
        stats.mark_saved();
        stats.record(3, 0, 0, Some("zero"));
        assert!(!stats.has_unsaved());
        assert_eq!(stats.books.len(), 1);
        stats.record(3, 0, 1, None);
        assert!(stats.has_unsaved());
    }

    #[test]
    fn counters_saturate_without_overflow_in_days_and_books() {
        let mut stats = ReadingStats::default();
        stats.record(u32::MAX, u32::MAX, u32::MAX, Some("book"));
        stats.record(u32::MAX, 1, 1, Some("book"));
        assert_eq!(
            stats.day(u32::MAX),
            DayStats {
                seconds: u32::MAX,
                pages: u32::MAX
            }
        );
        assert_eq!(stats.books[0].totals, stats.day(u32::MAX));
        assert_eq!(stats.week(u32::MAX)[6], u32::MAX / 60);
        assert_eq!(stats.streak(u32::MAX), 1);
        assert_eq!(
            ReadingStats::parse(&stats.serialized())
                .unwrap()
                .serialized(),
            stats.serialized()
        );
    }

    #[test]
    fn week_is_oldest_first_minutes_and_zeros_missing_days() {
        let mut stats = ReadingStats::default();
        for day in 0..10 {
            stats.record(day, day * 60 + 59, 0, None);
        }
        assert_eq!(stats.week(8), [2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(stats.week(12), [6, 7, 8, 9, 0, 0, 0]);
    }

    #[test]
    fn week_does_not_repeat_epoch_day_for_underflow_buckets() {
        let mut stats = ReadingStats::default();
        stats.record(0, 600, 0, None);
        stats.record(1, 120, 0, None);
        assert_eq!(stats.week(0), [0, 0, 0, 0, 0, 0, 10]);
        assert_eq!(stats.week(1), [0, 0, 0, 0, 0, 10, 2]);
    }

    #[test]
    fn streak_preserves_yesterday_in_the_morning_and_extends_at_300() {
        let mut stats = ReadingStats::default();
        for day in 4..7 {
            stats.record(day, 300, 0, None);
        }
        assert_eq!(stats.streak(6), 3);
        assert_eq!(stats.streak(7), 3);
        stats.record(7, 299, 0, None);
        assert_eq!(stats.streak(7), 3);
        stats.record(7, 1, 0, None);
        assert_eq!(stats.streak(7), 4);
        assert_eq!(stats.streak(8), 4);
        assert_eq!(stats.streak(9), 0);
        stats.record(9, 300, 0, None);
        assert_eq!(stats.streak(9), 1);
    }

    #[test]
    fn streak_handles_epoch_and_ignores_future_and_page_only_records() {
        let mut stats = ReadingStats::default();
        stats.record(0, 299, u32::MAX, None);
        stats.record(1, 300, 0, None);
        assert_eq!(stats.streak(0), 0);
        assert_eq!(stats.streak(1), 1);
        stats.record(0, 1, 0, None);
        assert_eq!(stats.streak(0), 1);
        assert_eq!(stats.streak(1), 2);
        assert_eq!(stats.streak(2), 2);
    }

    #[test]
    fn best_streak_finds_the_longest_run_over_retained_days() {
        let mut stats = ReadingStats::default();
        assert_eq!(stats.best_streak(), 0);
        for day in [1, 2, 3, 10, 11, 12, 13, 20] {
            stats.record(day, 300, 0, None);
        }
        stats.record(14, 299, 0, None);
        assert_eq!(stats.best_streak(), 4);
        stats.record(14, 1, 0, None);
        assert_eq!(stats.best_streak(), 5);
        stats.record(21, 299, 0, None);
        assert_eq!(stats.best_streak(), 5);
    }

    #[test]
    fn best_streak_ignores_short_days_and_keeps_disjoint_runs_apart() {
        let mut stats = ReadingStats::default();
        for day in [0, 1, 2, 4, 5] {
            stats.record(day, 300, 0, None);
        }
        stats.record(3, 299, 0, None);
        assert_eq!(stats.best_streak(), 3);
        stats.record(3, 1, 0, None);
        assert_eq!(stats.best_streak(), 6);
    }

    #[test]
    fn total_sums_both_inclusive_days_and_ignores_the_rest() {
        let mut stats = ReadingStats::default();
        for day in 0..5 {
            stats.record(day, 60, 1, None);
        }
        assert_eq!(
            stats.total(1, 3),
            DayStats {
                seconds: 180,
                pages: 3
            }
        );
        assert_eq!(stats.total(3, 3), stats.day(3));
        assert_eq!(stats.total(3, 1), DayStats::default());
        assert_eq!(stats.total(9, 12), DayStats::default());
        assert_eq!(
            stats.total(0, 4),
            DayStats {
                seconds: 300,
                pages: 5
            }
        );
    }

    #[test]
    fn total_adds_days_without_overflow_for_heavy_but_real_readers() {
        let mut stats = ReadingStats::default();
        stats.record(10, 28_800, 600, None);
        stats.record(11, 28_800, 600, None);
        assert_eq!(
            stats.total(10, 11),
            DayStats {
                seconds: 57_600,
                pages: 1200
            }
        );
    }

    #[test]
    fn book_totals_match_records_and_unknown_paths_return_none() {
        let mut stats = ReadingStats::default();
        assert_eq!(stats.book("/a/One.txt"), None);
        stats.record(1, 60, 2, Some("/a/One.txt"));
        stats.record(2, 120, 3, Some("/a/One.txt"));
        stats.record(2, 30, 1, Some("/a/Two.txt"));
        assert_eq!(
            stats.book("/a/One.txt"),
            Some(DayStats {
                seconds: 180,
                pages: 5
            })
        );
        assert_eq!(
            stats.book("/a/Two.txt"),
            Some(DayStats {
                seconds: 30,
                pages: 1
            })
        );
        assert_eq!(stats.book("/a/one.txt"), None);
        assert_eq!(stats.book(""), None);
    }

    #[test]
    fn finishing_is_idempotent_by_exact_path_and_preserves_totals() {
        let mut stats = ReadingStats::default();
        stats.record(0, 80, 2, Some("/a/Café.txt"));
        stats.mark_finished("/a/Café.txt");
        stats.mark_finished("/b/Café.txt");
        assert_eq!(stats.books_finished(), 2);
        assert_eq!(stats.books[0].totals.seconds, 80);
        stats.mark_saved();
        let text = stats.serialized();
        stats.mark_finished("/a/Café.txt");
        assert_eq!(stats.serialized(), text);
        assert!(!stats.has_unsaved());
        stats.record(0, 20, 1, Some("/a/Café.txt"));
        assert_eq!(stats.books_finished(), 2);
        assert_eq!(stats.books.last().unwrap().totals.seconds, 100);
    }

    #[test]
    fn unsafe_names_cannot_inject_file_records_but_daily_totals_count() {
        for path in [
            "",
            " ",
            "book|part",
            "book\nbook|fake|1|1|1",
            "book\rname",
            "book\tname",
            "book\0name",
            "book\u{7f}name",
            "book\u{85}name",
            "book?",
            "book*",
            "book:",
            "book<",
            "book>",
            "book\"",
        ] {
            let mut stats = ReadingStats::default();
            stats.mark_finished(path);
            assert!(!stats.has_unsaved());
            stats.record(0, 60, 1, Some(path));
            assert!(stats.books.is_empty(), "{path:?}");
            assert_eq!(stats.day(0).seconds, 60);
            assert!(ReadingStats::parse(&stats.serialized()).is_ok());
        }
    }

    #[test]
    fn serialization_roundtrip_is_stable_utf8_and_clean() {
        let mut stats = ReadingStats::default();
        stats.record(20_729, 1_520, 34, Some("/sdcard/BOOKS/El corazón.txt"));
        stats.mark_finished("/sdcard/BOOKS/El corazón.txt");
        stats.record(0, 1, 2, Some(" leading and trailing spaces "));
        let text = stats.serialized();
        assert!(text.contains("day|2026-10-03|1520|34\n"));
        let restored = ReadingStats::parse(&text).unwrap();
        assert!(!restored.has_unsaved());
        assert_eq!(restored.serialized(), text);
        assert_eq!(restored.books_finished(), 1);
        assert_eq!(restored.days, stats.days);
        assert_eq!(restored.books, stats.books);
    }

    #[test]
    fn parser_skips_unknown_blank_comment_bom_and_accepts_crlf() {
        let stats = ReadingStats::parse("\u{feff}# Wave reading stats v1\r\n\r\nfuture|anything\r\n# comment\n day|1970-01-01|60|1 \r\nbook|Café.txt|60|1|1\r\n").unwrap();
        assert_eq!(stats.day(0).seconds, 60);
        assert_eq!(stats.books_finished(), 1);
        assert!(!stats.has_unsaved());
        assert_eq!(ReadingStats::parse("").unwrap(), ReadingStats::default());
    }

    #[test]
    fn known_malformed_records_report_physical_line_numbers() {
        for line in [
            "day",
            "day|1970-01-01|1",
            "day|1970-01-01|1|2|3",
            "day|2026-02-29|1|2",
            "day|1970-01-01||2",
            "day|1970-01-01|-1|2",
            "day|1970-01-01|+1|2",
            "day|1970-01-01|4294967296|2",
            "day|1970-01-01|1|é",
            "day|1970-01-01|1|2\0",
            "day|1970-01-01|1|2\t",
            "book",
            "book||0|0|0",
            "book|name|0|0",
            "book|name|0|0|2",
            "book|name|0|0|01",
            "book|name|0|0|",
            "book|name|x|0|0",
            "book|name|0|-1|0",
            "book|name|0|4294967296|0",
            "book|name|extra|0|0|0",
            "book|name\rpart|0|0|0",
            "book|bad?|0|0|0",
        ] {
            let error = ReadingStats::parse(&format!("# comment\nunknown\n\n{line}")).unwrap_err();
            assert!(
                format!("{error:#}").contains("line 4"),
                "{line:?}: {error:#}"
            );
        }
    }

    #[test]
    fn duplicate_records_replace_not_add_and_move_books_to_latest_position() {
        let stats = ReadingStats::parse("day|1970-01-01|60|1\nday|1970-01-01|120|2\nbook|a|60|1|1\nbook|b|0|0|1\nbook|a|120|2|1\n").unwrap();
        assert_eq!(stats.day(0).seconds, 120);
        assert_eq!(stats.books_finished(), 2);
        assert_eq!(stats.books[0].path, "b");
        assert_eq!(stats.books[1].totals.seconds, 120);
        let restored = ReadingStats::parse(&stats.serialized()).unwrap();
        assert_eq!(restored, stats);
        assert!(ReadingStats::parse("book|a|60|1|1\nbook|a|bad|0|1").is_err());
    }

    #[test]
    fn record_trims_to_latest_400_chronological_days_even_out_of_order() {
        let mut stats = ReadingStats::default();
        for day in (0..450).rev() {
            stats.record(day, 300, 1, None);
        }
        assert_eq!(stats.days.len(), 400);
        assert_eq!(stats.days.first_key_value().unwrap().0, &50);
        assert_eq!(stats.days.last_key_value().unwrap().0, &449);
        assert_eq!(stats.streak(449), 400);
        stats.mark_saved();
        stats.record(0, 60, 1, None);
        assert!(!stats.has_unsaved());
        let text = stats.serialized();
        assert_eq!(ReadingStats::parse(&text).unwrap().serialized(), text);
        let labels: Vec<_> = text
            .lines()
            .skip(1)
            .map(|line| parse_day(line.split('|').nth(1).unwrap()).unwrap())
            .collect();
        assert!(labels.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn parser_trims_unsorted_days_and_books_without_dirtying() {
        let mut text = String::new();
        for day in (0..430).rev() {
            text.push_str(&format!("day|{}|60|1\n", day_label(day)));
            text.push_str(&format!("book|{day}|60|1|1\n"));
        }
        let stats = ReadingStats::parse(&text).unwrap();
        assert_eq!(stats.days.len(), 400);
        assert_eq!(stats.days.first_key_value().unwrap().0, &30);
        assert_eq!(stats.books.len(), 200);
        assert_eq!(stats.books.first().unwrap().path, "199");
        assert_eq!(stats.books.last().unwrap().path, "0");
        assert!(!stats.has_unsaved());
        assert_eq!(ReadingStats::parse(&stats.serialized()).unwrap(), stats);
    }

    #[test]
    fn books_use_touch_order_not_date_or_name_and_preserve_policy_after_load() {
        let mut stats = ReadingStats::default();
        for index in 0..200 {
            stats.record(100, 1, 1, Some(&format!("book-{index}")));
        }
        stats.record(0, 1, 1, Some("book-0"));
        stats.mark_finished("book-1");
        let mut restored = ReadingStats::parse(&stats.serialized()).unwrap();
        for state in [&mut stats, &mut restored] {
            state.mark_finished("new");
            assert!(!state.books.iter().any(|book| book.path == "book-2"));
            assert!(state.books.iter().any(|book| book.path == "book-0"));
            assert_eq!(state.books_finished(), 2);
            assert_eq!(state.books.len(), 200);
        }
        assert_eq!(stats.serialized(), restored.serialized());
    }

    #[test]
    fn finished_count_is_bounded_and_idempotent_does_not_refresh_recency() {
        let mut stats = ReadingStats::default();
        for index in 0..200 {
            stats.mark_finished(&format!("book-{index}"));
        }
        stats.mark_finished("book-0");
        stats.mark_finished("new");
        assert_eq!(stats.books_finished(), 200);
        assert!(!stats.books.iter().any(|book| book.path == "book-0"));
    }

    #[test]
    fn filesystem_save_load_replace_and_tmp_cleanup() {
        let root = TempRoot::new();
        let path = root.stats_path();
        let mut stats = ReadingStats::default();
        stats.record(20_729, 60, 1, Some("Café.txt"));
        stats.save_to_path(&path).unwrap();
        assert!(!stats.has_unsaved());
        assert!(!path.with_extension("TMP").exists());
        assert!(!path.with_extension("BAK").exists());
        assert_eq!(ReadingStats::load_from_path(&path).unwrap(), stats);
        stats.mark_finished("Café.txt");
        stats.save_to_path(&path).unwrap();
        assert!(!stats.has_unsaved());
        assert!(!path.with_extension("TMP").exists());
        assert!(!path.with_extension("BAK").exists());
        assert_eq!(
            ReadingStats::load_from_path(path).unwrap().books_finished(),
            1
        );
    }

    #[test]
    fn replacing_goes_through_a_backup_that_load_falls_back_to() {
        let root = TempRoot::new();
        let path = root.stats_path();
        let mut stats = ReadingStats::default();
        stats.record(0, 60, 1, None);
        stats.save_to_path(&path).unwrap();
        stats.record(1, 60, 1, None);
        stats.save_to_path(&path).unwrap();
        assert!(!path.with_extension("BAK").exists());
        // A save interrupted between its two renames leaves only the backup.
        fs::rename(&path, path.with_extension("BAK")).unwrap();
        assert_eq!(ReadingStats::load_from_path(&path).unwrap(), stats);
    }

    #[test]
    fn filesystem_load_reports_missing_invalid_utf8_and_malformed_files() {
        let root = TempRoot::new();
        let path = root.stats_path();
        let error = ReadingStats::load_from_path(&path).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        fs::write(&path, [0xff]).unwrap();
        assert!(ReadingStats::load_from_path(&path).is_err());
        fs::write(&path, "# header\nday|bad|0|0").unwrap();
        assert!(
            format!("{:#}", ReadingStats::load_from_path(&path).unwrap_err()).contains("line 2")
        );
    }

    #[test]
    fn temporary_creation_failure_preserves_original_and_dirty_state() {
        let root = TempRoot::new();
        let path = root.stats_path();
        fs::write(&path, "original").unwrap();
        fs::create_dir(path.with_extension("TMP")).unwrap();
        let mut stats = ReadingStats::default();
        stats.record(0, 60, 1, None);
        assert!(stats.save_to_path(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert!(stats.has_unsaved());
        assert!(stats
            .save_to_path(root.0.join("missing/STATS.TXT"))
            .is_err());
        assert!(stats.has_unsaved());
    }

    #[test]
    fn stale_tmp_file_is_discarded_and_the_save_succeeds() {
        let root = TempRoot::new();
        let path = root.stats_path();
        fs::write(&path, "original").unwrap();
        fs::write(path.with_extension("TMP"), "interrupted save").unwrap();
        let mut stats = ReadingStats::default();
        stats.record(0, 60, 1, None);
        stats.save_to_path(&path).unwrap();
        assert!(!stats.has_unsaved());
        assert!(!path.with_extension("TMP").exists());
        assert!(!path.with_extension("BAK").exists());
        assert_eq!(ReadingStats::load_from_path(&path).unwrap(), stats);
    }

    #[test]
    fn failed_save_does_not_make_a_clean_instance_dirty() {
        let root = TempRoot::new();
        let mut stats = ReadingStats::parse("day|1970-01-01|60|1").unwrap();
        assert!(stats
            .save_to_path(root.0.join("missing/STATS.TXT"))
            .is_err());
        assert!(!stats.has_unsaved());
        assert_eq!(stats.day(0).seconds, 60);
    }

    #[test]
    fn saturated_daily_noop_without_book_stays_clean() {
        let mut stats = ReadingStats::default();
        stats.record(0, u32::MAX, u32::MAX, None);
        stats.mark_saved();
        stats.record(0, 1, 1, None);
        assert!(!stats.has_unsaved());
    }

    #[test]
    fn rename_failure_keeps_destination_and_removes_temporary_file() {
        let root = TempRoot::new();
        let path = root.stats_path();
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), "original").unwrap();
        let mut stats = ReadingStats::default();
        stats.mark_finished("book");
        assert!(stats.save_to_path(&path).is_err());
        assert!(stats.has_unsaved());
        assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "original");
        assert!(!path.with_extension("TMP").exists());
    }

    #[test]
    fn tmp_destination_is_rejected_without_overwriting_it() {
        let root = TempRoot::new();
        let path = root.0.join("STATS.TMP");
        fs::write(&path, "original").unwrap();
        let mut stats = ReadingStats::default();
        stats.mark_finished("book");
        assert!(stats.save_to_path(&path).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
        assert!(stats.has_unsaved());
    }

    #[cfg(unix)]
    #[test]
    fn stale_tmp_symlink_is_discarded_without_writing_through_it() {
        use std::os::unix::fs::symlink;
        let root = TempRoot::new();
        let path = root.stats_path();
        fs::write(&path, "original").unwrap();
        symlink(&path, path.with_extension("TMP")).unwrap();
        let mut stats = ReadingStats::default();
        stats.mark_finished("book");
        stats.save_to_path(&path).unwrap();
        // The stale link is removed before any write, so it cannot redirect it.
        assert!(!stats.has_unsaved());
        assert!(!path.with_extension("TMP").exists());
        assert!(!path.with_extension("BAK").exists());
        assert_eq!(ReadingStats::load_from_path(&path).unwrap(), stats);
    }

    #[test]
    fn clock_counts_nothing_before_first_key() {
        let mut clock = ReadingClock::default();
        assert_eq!(clock.take_seconds(10_000), 0);
        assert_eq!(clock.take_seconds(u64::MAX), 0);
        clock.on_key(100);
        assert_eq!(clock.take_seconds(1_100), 1);
    }

    #[test]
    fn clock_short_gaps_extend_the_window_without_double_counting() {
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        assert_eq!(clock.take_seconds(60_000), 60);
        clock.on_key(90_000);
        assert_eq!(clock.take_seconds(150_000), 90);
        assert_eq!(clock.take_seconds(150_000), 0);
        assert_eq!(clock.take_seconds(300_000), 60);
        assert_eq!(clock.take_seconds(400_000), 0);
    }

    #[test]
    fn clock_long_gap_key_keeps_old_eligible_time_before_restarting() {
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        assert_eq!(clock.take_seconds(30_000), 30);
        clock.on_key(300_000);
        assert_eq!(clock.take_seconds(301_000), 91);
        assert_eq!(clock.take_seconds(600_000), 119);
        assert_eq!(clock.take_seconds(600_000), 0);
    }

    #[test]
    fn clock_exact_grace_boundary_and_key_at_boundary() {
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        assert_eq!(clock.take_seconds(119_999), 119);
        assert_eq!(clock.take_seconds(120_000), 1);
        assert_eq!(clock.take_seconds(120_001), 0);
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        clock.on_key(120_000);
        assert_eq!(clock.take_seconds(240_000), 240);
    }

    #[test]
    fn clock_retains_fractions_across_takes_keys_and_inactive_gaps() {
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        assert_eq!(clock.take_seconds(400), 0);
        clock.on_key(500);
        assert_eq!(clock.take_seconds(900), 0);
        assert_eq!(clock.take_seconds(1_100), 1);
        assert_eq!(clock.take_seconds(200_000), 119);
        clock.on_key(300_000);
        assert_eq!(clock.take_seconds(300_499), 0);
        assert_eq!(clock.take_seconds(300_500), 1);
    }

    #[test]
    fn clock_ignores_regressions_and_repeated_events_without_restarting_time() {
        let mut clock = ReadingClock::default();
        clock.on_key(1_000);
        clock.on_key(1_000);
        assert_eq!(clock.take_seconds(2_500), 1);
        assert_eq!(clock.take_seconds(500), 0);
        clock.on_key(500);
        assert_eq!(clock.take_seconds(3_000), 1);
        assert_eq!(clock.take_seconds(3_000), 0);
        assert_eq!(clock.take_seconds(200_000), 118);
        clock.on_key(50_000);
        assert_eq!(clock.take_seconds(300_000), 0);
    }

    #[test]
    fn clock_huge_elapsed_and_u64_boundary_are_safe() {
        let mut clock = ReadingClock::default();
        clock.on_key(0);
        assert_eq!(clock.take_seconds(u64::MAX), 120);
        assert_eq!(clock.take_seconds(u64::MAX), 0);
        clock.on_key(u64::MAX - 5_000);
        assert_eq!(clock.take_seconds(u64::MAX), 0);
        let mut clock = ReadingClock::default();
        clock.on_key(u64::MAX - 1_500);
        assert_eq!(clock.take_seconds(u64::MAX), 1);
        assert_eq!(clock.pending_ms, 500);
    }

    #[test]
    fn clock_pending_seconds_exceeding_u32_are_drained_without_loss() {
        let mut clock = ReadingClock {
            pending_ms: (u64::from(u32::MAX) + 2) * 1_000 + 999,
            ..ReadingClock::default()
        };
        assert_eq!(clock.take_seconds(0), u32::MAX);
        assert_eq!(clock.take_seconds(0), 2);
        assert_eq!(clock.take_seconds(0), 0);
        assert_eq!(clock.pending_ms, 999);
    }
}
