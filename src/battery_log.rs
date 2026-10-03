//! Battery level history for Settings › Power, kept on the SD card.
//!
//! One sample every 15 minutes, timestamped in RTC minutes since 2000, covers
//! the last week. The loop saves it in batches to spare SD writes.

use std::path::Path;

use anyhow::{Context, Result};

pub const BATTERY_LOG_PATH: &str = "/sdcard/RUSTMIX/BATTERY.TXT";
/// Minimum spacing between samples.
pub const SAMPLE_MINUTES: u32 = 15;
/// Unsaved samples that are worth one SD write.
pub const SAVE_AFTER_SAMPLES: usize = 8;
const KEEP_MINUTES: u32 = 7 * 24 * 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatterySample {
    pub minute: u32,
    pub percent: u8,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatteryLog {
    samples: Vec<BatterySample>,
    unsaved: usize,
}

impl BatteryLog {
    /// Read `minute,percent` lines, skipping any that do not parse.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut log = Self::default();
        for line in text.lines() {
            let Some((minute, percent)) = line.trim().split_once(',') else {
                continue;
            };
            if let (Ok(minute), Ok(percent)) = (minute.parse(), percent.parse()) {
                log.record(minute, percent);
            }
        }
        log.unsaved = 0;
        log
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = crate::sd_file::read_to_string(path)
            .with_context(|| format!("read battery log {}", path.display()))?;
        Ok(Self::parse(&text))
    }

    pub fn save_to_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        crate::sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write battery log {}", path.display()))?;
        self.unsaved = 0;
        Ok(())
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        self.samples
            .iter()
            .map(|sample| format!("{},{}\n", sample.minute, sample.percent))
            .collect()
    }

    /// Store a sample unless the last one is under `SAMPLE_MINUTES` old. A
    /// clock that moved backwards drops the samples it now precedes.
    pub fn record(&mut self, minute: u32, percent: u8) -> bool {
        self.samples.retain(|sample| sample.minute <= minute);
        if let Some(last) = self.samples.last() {
            if minute - last.minute < SAMPLE_MINUTES {
                return false;
            }
        }
        self.samples.push(BatterySample {
            minute,
            percent: percent.min(100),
        });
        let oldest = minute.saturating_sub(KEEP_MINUTES);
        self.samples.retain(|sample| sample.minute >= oldest);
        self.unsaved += 1;
        true
    }

    #[must_use]
    pub fn needs_save(&self) -> bool {
        self.unsaved >= SAVE_AFTER_SAMPLES
    }

    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.unsaved > 0
    }

    /// Battery level at the end of each of the last 24 hours, oldest first.
    #[must_use]
    pub fn last_day(&self, now_minute: u32) -> [Option<u8>; 24] {
        let mut hours = [None; 24];
        for sample in &self.samples {
            let age = now_minute.saturating_sub(sample.minute);
            if sample.minute <= now_minute && age < 24 * 60 {
                hours[23 - (age / 60) as usize] = Some(sample.percent);
            }
        }
        hours
    }
}

#[cfg(test)]
mod tests {
    use super::{BatteryLog, SAMPLE_MINUTES, SAVE_AFTER_SAMPLES};

    #[test]
    fn keeps_samples_spaced_and_saves_in_batches() {
        let mut log = BatteryLog::default();
        assert!(log.record(1_000, 90));
        assert!(!log.record(1_000 + SAMPLE_MINUTES - 1, 89));
        for step in 1..SAVE_AFTER_SAMPLES as u32 {
            assert!(!log.needs_save());
            assert!(log.record(1_000 + step * SAMPLE_MINUTES, 90 - step as u8));
        }
        assert!(log.needs_save());
        let restored = BatteryLog::parse(&log.serialized());
        assert!(!restored.has_unsaved());
        assert_eq!(restored.serialized(), log.serialized());
    }

    #[test]
    fn last_day_puts_the_latest_sample_of_each_hour_in_its_bucket() {
        let mut log = BatteryLog::default();
        let now = 100_000;
        log.record(now - 25 * 60, 99);
        log.record(now - 23 * 60 - 30, 80);
        log.record(now - 23 * 60 - 10, 79);
        log.record(now, 60);
        let hours = log.last_day(now);
        assert_eq!(hours[0], Some(79));
        assert_eq!(hours[1], None);
        assert_eq!(hours[23], Some(60));
    }

    #[test]
    fn clock_corrections_drop_samples_from_the_future() {
        let mut log = BatteryLog::default();
        log.record(5_000, 80);
        log.record(2_000, 85);
        assert_eq!(log.serialized(), "2000,85\n");
        let cleaned = BatteryLog::parse("x\n10,y\n30,70\n");
        assert_eq!(cleaned.serialized(), "30,70\n");
    }
}
