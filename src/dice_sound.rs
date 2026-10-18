//! Dice rattle: short decaying noise clicks and a final knock as PCM.
//!
//! The firmware holds no sound files and plays signed-16-bit stereo PCM at
//! 16 kHz (`src/audio/tone.rs`). A rattle is a run of band-limited noise
//! clicks whose gaps grow and whose amplitudes fall, then a low decaying
//! thump where the dice land. Everything is fixed-point: no float
//! trigonometry and no large tables.
//!
//! Timing lives in the constants below. To change how long the rattle runs,
//! edit `RATTLE_MS` and `KNOCK_MS` (and the silent tail if needed); to
//! change how many clicks one die makes, edit `CLICKS_BASE` and
//! `CLICKS_PER_DIE`. `duration_ms` always reports the true length.

use crate::audio::{
    tone::{PCM_BYTES_PER_FRAME, PCM_CHUNK_FRAMES},
    AUDIO_SAMPLE_RATE_HZ,
};

/// Peak sample, under half of full scale (the speaker is small; the chime
/// uses the same 12 000).
const PEAK: i32 = 12_000;
/// Clicks for one die.
const CLICKS_BASE: usize = 12;
/// Extra clicks per die beyond the first.
const CLICKS_PER_DIE: usize = 3;
/// Longest rattle we schedule; more dice only add clicks inside it.
const RATTLE_MS: u32 = 520;
/// The knock where the dice land.
const KNOCK_MS: u32 = 80;
/// Quiet tail so the last 20 ms is silent.
const TAIL_MS: u32 = 20;
/// Length of one click's noise burst.
const CLICK_MS: u32 = 4;
/// Frames of one playback chunk the main line will pull.
pub const RATTLE_CHUNK_FRAMES: usize = PCM_CHUNK_FRAMES;
/// xorshift keeps zero fixed, so a zero seed starts from this constant.
const ZERO_SEED: u32 = 0x9E37_79B9;

/// How long a rattle for `dice` dice plays, in milliseconds. The roll's
/// three animation frames fit inside it.
#[must_use]
pub fn duration_ms(dice: usize) -> u32 {
    let _ = dice;
    RATTLE_MS + KNOCK_MS + TAIL_MS
}

/// Frames of a rattle for `dice` dice.
#[must_use]
pub fn duration_frames(dice: usize) -> u32 {
    AUDIO_SAMPLE_RATE_HZ.saturating_mul(duration_ms(dice)) / 1_000
}

/// A seeded dice rattle. Pull PCM with `next_chunk` until it returns 0.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rattle {
    rng: u32,
    /// Start frame of each click, in order.
    click_starts: [u32; 18],
    click_count: usize,
    click_frames: u32,
    knock_start: u32,
    knock_frames: u32,
    total_frames: u32,
    frame: u32,
    /// Simple noise state for the current click (not used during the knock).
    noise: u32,
}

impl Rattle {
    /// A rattle for `dice` dice whose timing lines up with `frames` of the
    /// roll animation (the sound lasts at least that long). `dice` is
    /// clamped to 1..=3 and `frames` to at least 1.
    #[must_use]
    pub fn new(seed: u32, dice: usize, frames: u8) -> Self {
        let dice = dice.clamp(1, 3);
        let animation_frames = u32::from(frames.max(1));
        let click_count = CLICKS_BASE + CLICKS_PER_DIE * (dice - 1);
        let click_frames = frames_for_ms(CLICK_MS);
        let rattle_frames = frames_for_ms(RATTLE_MS).max(animation_frames);
        let mut click_starts = [0_u32; 18];
        // Growing gaps: click i starts later by a gap proportional to i + 1,
        // so the rattle thins out as the dice settle.
        let payload = rattle_frames.saturating_sub(click_count as u32 * click_frames);
        let weight_sum = (click_count * (click_count + 1) / 2) as u32;
        let mut cursor = 0_u32;
        for index in 0..click_count {
            click_starts[index] = cursor;
            let weight = (index as u32) + 1;
            cursor += click_frames + payload * weight / weight_sum.max(1);
        }
        Self {
            rng: if seed == 0 { ZERO_SEED } else { seed },
            click_starts,
            click_count,
            click_frames,
            knock_start: rattle_frames,
            knock_frames: frames_for_ms(KNOCK_MS),
            total_frames: rattle_frames + frames_for_ms(KNOCK_MS + TAIL_MS),
            frame: 0,
            noise: if seed == 0 { ZERO_SEED } else { seed },
        }
    }

    /// Fill `out` with signed-16-bit little-endian stereo PCM. Returns the
    /// bytes written, or 0 when the rattle is finished. A buffer that is not
    /// a whole number of frames is truncated to whole frames (the remainder
    /// is left untouched).
    pub fn next_chunk(&mut self, out: &mut [u8]) -> usize {
        let usable = out.len() - out.len() % PCM_BYTES_PER_FRAME;
        if usable == 0 || self.frame >= self.total_frames {
            return 0;
        }
        let frames = (usable / PCM_BYTES_PER_FRAME).min(RATTLE_CHUNK_FRAMES) as u32;
        let frames = frames.min(self.total_frames - self.frame);
        let mut written = 0;
        for _ in 0..frames {
            let sample = self.next_sample();
            let bytes = sample.to_le_bytes();
            let slot = &mut out[written..written + PCM_BYTES_PER_FRAME];
            slot[0] = bytes[0];
            slot[1] = bytes[1];
            slot[2] = bytes[0];
            slot[3] = bytes[1];
            written += PCM_BYTES_PER_FRAME;
        }
        written
    }

    /// One stereo sample at the current frame.
    fn next_sample(&mut self) -> i16 {
        let frame = self.frame;
        self.frame = self.frame.saturating_add(1);
        if frame >= self.knock_start + self.knock_frames {
            return 0;
        }
        if frame >= self.knock_start {
            return self.knock_sample(frame - self.knock_start);
        }
        // Inside a click: band-limited noise with an exponential decay.
        for index in 0..self.click_count {
            let start = self.click_starts[index];
            if frame >= start && frame < start + self.click_frames {
                let age = frame - start;
                let peak = self.click_peak(index);
                return self.noise_sample(age, peak);
            }
        }
        0
    }

    /// Peak of click `index`, falling across the rattle.
    fn click_peak(&self, index: usize) -> i32 {
        let count = self.click_count.max(1) as i32;
        PEAK * (count - index as i32) / count
    }

    /// One band-limited noise sample: xorshift noise through a one-pole
    /// low-pass, times an exponential decay over `age` frames.
    fn noise_sample(&mut self, age: u32, peak: i32) -> i16 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        let raw = ((self.noise & 0xFFFF) as i32) - 32_768;
        // Simple low-pass: keep a running smoothed value in the high bits.
        let smooth = (raw + ((self.noise >> 16) as i32 - 32_768)) / 2;
        // Decay: multiply by 0.98^age in fixed point (0.98 ≈ 1600/1638).
        let mut decay = 16_384_i32;
        for _ in 0..age {
            decay = decay * 1_600 / 1_638;
        }
        (smooth * peak / 32_768 * decay / 16_384) as i16
    }

    /// Low decaying sine-like thump (triangle wave) for the landing knock.
    fn knock_sample(&self, age: u32) -> i16 {
        // ~120 Hz triangle: phase step of 120 Hz at 16 kHz.
        let phase = age.wrapping_mul(phase_step(120));
        let ramp = ((phase >> 16) & 0xFFFF) as i32;
        let triangle = if ramp < 32_768 {
            ramp * 2 - 32_768
        } else {
            (65_535 - ramp) * 2 - 32_768
        };
        let mut decay = 16_384_i32;
        for _ in 0..age {
            decay = decay * 1_600 / 1_638;
        }
        (triangle * PEAK / 32_768 * decay / 16_384) as i16
    }
}

#[must_use]
const fn frames_for_ms(duration_ms: u32) -> u32 {
    AUDIO_SAMPLE_RATE_HZ.saturating_mul(duration_ms) / 1_000
}

#[must_use]
const fn phase_step(frequency_hz: u32) -> u32 {
    (((frequency_hz as u64) << 32) / AUDIO_SAMPLE_RATE_HZ as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(rattle: &mut Rattle) -> Vec<u8> {
        let total = duration_frames(3) as usize * PCM_BYTES_PER_FRAME;
        let mut all = Vec::with_capacity(total);
        let mut buffer = [0_u8; RATTLE_CHUNK_FRAMES * PCM_BYTES_PER_FRAME];
        loop {
            let written = rattle.next_chunk(&mut buffer);
            if written == 0 {
                break;
            }
            all.extend_from_slice(&buffer[..written]);
        }
        all
    }

    fn samples(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect()
    }

    /// Rough peak count: local maxima above `threshold`, ignoring neighbours.
    fn peak_count(samples: &[i16], threshold: i32) -> usize {
        let mut count = 0;
        let mut index = 1;
        while index + 1 < samples.len() {
            let value = i32::from(samples[index]).abs();
            let before = i32::from(samples[index - 1]).abs();
            let after = i32::from(samples[index + 1]).abs();
            if value >= threshold && value > before && value >= after {
                count += 1;
                index += 40;
            } else {
                index += 1;
            }
        }
        count
    }

    #[test]
    fn the_same_seed_gives_the_same_bytes() {
        let left = render(&mut Rattle::new(42, 2, 3));
        let right = render(&mut Rattle::new(42, 2, 3));
        assert_eq!(left, right);
    }

    #[test]
    fn a_different_seed_gives_different_bytes() {
        let left = render(&mut Rattle::new(42, 2, 3));
        let right = render(&mut Rattle::new(43, 2, 3));
        assert_ne!(left, right);
    }

    #[test]
    fn the_peak_is_under_half_scale_and_over_silence() {
        let bytes = render(&mut Rattle::new(7, 2, 3));
        let peaks: Vec<i32> = samples(&bytes)
            .into_iter()
            .map(|sample| i32::from(sample).abs())
            .collect();
        let peak = peaks.iter().copied().max().unwrap_or(0);
        assert!(peak <= 16_383, "peak {peak} is over half of full scale");
        assert!(peak > 200, "peak {peak} is silent");
    }

    #[test]
    fn it_ends_in_silence_and_starts_on_a_click() {
        let bytes = render(&mut Rattle::new(11, 2, 3));
        let samples = samples(&bytes);
        let tail = 20 * 16; // 20 ms at 16 kHz, stereo samples counted once
        let last = &samples[samples.len().saturating_sub(tail)..];
        assert!(
            last.iter().all(|sample| *sample == 0),
            "the last 20 ms is not silent"
        );
        let head = &samples[..160]; // 10 ms
        assert!(
            head.iter().any(|sample| *sample != 0),
            "nothing plays within 10 ms"
        );
    }

    #[test]
    fn the_total_length_equals_duration_ms() {
        for dice in 1..=3 {
            let bytes = render(&mut Rattle::new(3, dice, 3));
            let frames = bytes.len() / PCM_BYTES_PER_FRAME;
            assert_eq!(frames as u32, duration_frames(dice), "dice {dice}");
            assert_eq!(duration_ms(dice), RATTLE_MS + KNOCK_MS + TAIL_MS);
        }
    }

    #[test]
    fn chunks_join_to_the_length_and_the_last_one_is_short() {
        let mut rattle = Rattle::new(5, 2, 3);
        let whole = RATTLE_CHUNK_FRAMES * PCM_BYTES_PER_FRAME;
        let total = duration_frames(2) as usize * PCM_BYTES_PER_FRAME;
        let mut seen = 0;
        let mut last = whole;
        loop {
            let mut buffer = [0_u8; RATTLE_CHUNK_FRAMES * PCM_BYTES_PER_FRAME];
            let written = rattle.next_chunk(&mut buffer);
            if written == 0 {
                break;
            }
            assert!(written <= whole);
            seen += written;
            last = written;
        }
        assert_eq!(seen, total);
        assert!(last < whole || total % whole == 0, "last chunk {last}");
        // Further pulls stay at 0.
        let mut buffer = [0_u8; RATTLE_CHUNK_FRAMES * PCM_BYTES_PER_FRAME];
        assert_eq!(rattle.next_chunk(&mut buffer), 0);
    }

    #[test]
    fn more_dice_give_more_clicks() {
        let one = peak_count(&samples(&render(&mut Rattle::new(9, 1, 3))), 400);
        let three = peak_count(&samples(&render(&mut Rattle::new(9, 3, 3))), 400);
        assert!(three > one, "one die {one} clicks, three dice {three}");
    }

    #[test]
    fn a_partial_frame_buffer_is_truncated_to_whole_frames() {
        let mut rattle = Rattle::new(2, 1, 3);
        let mut buffer = [0_u8; PCM_BYTES_PER_FRAME + 3];
        let written = rattle.next_chunk(&mut buffer);
        assert_eq!(written, PCM_BYTES_PER_FRAME, "the tail is not written");
    }

    #[test]
    fn an_empty_buffer_writes_nothing() {
        let mut rattle = Rattle::new(2, 1, 3);
        assert_eq!(rattle.next_chunk(&mut []), 0);
    }
}
