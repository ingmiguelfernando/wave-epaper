//! A voice note's PCM as an Ogg Opus file, for the upload. The recording
//! stays a WAV on the card (portable, played back as it is, recovered after a
//! power cut); a note sent to the AI providers goes as Opus at 24 kbps, about
//! ten times smaller, so it uploads faster and the radio stays on for less.
//!
//! The pipeline is plain Rust over a `FrameEncoder`, so the host tests it
//! with a stand-in; the device's encoder is the `esp_audio_codec` one the
//! XiaoZhi call uses, behind the `espidf` module.

use std::{
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ogg_opus::{OggOpusWriter, OpusHead},
    watchdog::Pacer,
};

/// Length of one Opus frame, the 20 ms `ogg_opus` counts its packets in.
pub const FRAME_MS: u32 = 20;
/// Target bitrate: speech stays clear for transcription at 24 kbps.
pub const BITRATE_BPS: i32 = 24_000;
/// Ogg stream serial of a part (one stream per file, so any number works).
const SERIAL: u32 = 0x5741_5645;
/// Frames between looks at the cancel flag: one second of audio.
const CANCEL_EVERY_FRAMES: u32 = 50;

/// Why a part did not become a file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The job was stopped (recording, sleep): not a failure.
    Cancelled,
    /// The encoder or the recording failed; the caller can send the WAV.
    Failed(String),
}

/// One frame of mono 16-bit PCM in, one Opus packet out.
pub trait FrameEncoder {
    /// Bytes of PCM one frame takes (640 for 20 ms at 16 kHz).
    fn frame_bytes(&self) -> usize;
    /// Encode exactly `frame_bytes()` of PCM into `packet`, replacing it.
    fn encode(&mut self, pcm: &[u8], packet: &mut Vec<u8>) -> Result<(), String>;
}

/// Encodes a stretch of a recording as a complete Ogg Opus file.
pub trait PartEncoder {
    /// Read `pcm_bytes` of mono 16-bit PCM from `pcm` and return the file.
    fn encode_part(
        &mut self,
        pcm: &mut dyn Read,
        pcm_bytes: u64,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, EncodeError>;
}

/// Read up to `buffer.len()` bytes, until the reader ends.
fn read_full(reader: &mut dyn Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

/// Encode `pcm_bytes` of PCM, frame by frame, into an Ogg Opus file. The
/// last frame is padded with silence. A reader that ends early gives the
/// frames it had; no audio at all is a failure.
pub fn encode_to_ogg(
    encoder: &mut dyn FrameEncoder,
    pcm: &mut dyn Read,
    pcm_bytes: u64,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, EncodeError> {
    let frame_bytes = encoder.frame_bytes();
    if frame_bytes == 0 || frame_bytes % 2 != 0 {
        return Err(EncodeError::Failed(format!(
            "the encoder's frame of {frame_bytes} bytes is not whole samples"
        )));
    }
    let failed = |error: io::Error| EncodeError::Failed(format!("the Ogg file: {error}"));
    // About a tenth of the PCM, and room for the head and tags.
    let capacity = usize::try_from(pcm_bytes / 10 + 512).unwrap_or(0);
    let mut writer = OggOpusWriter::new(Vec::with_capacity(capacity), OpusHead::default(), SERIAL)
        .map_err(failed)?;
    let mut frame = vec![0_u8; frame_bytes];
    let mut packet = Vec::with_capacity(512);
    let mut left = pcm_bytes;
    let mut frames = 0_u32;
    let mut pacer = Pacer::start();
    while left > 0 {
        if frames % CANCEL_EVERY_FRAMES == 0 && cancel.load(Ordering::Relaxed) {
            return Err(EncodeError::Cancelled);
        }
        let want = usize::try_from(left.min(frame_bytes as u64)).unwrap_or(frame_bytes);
        let read = read_full(pcm, &mut frame[..want]).map_err(|error| {
            EncodeError::Failed(format!("the recording could not be read: {error}"))
        })?;
        if read == 0 {
            break;
        }
        // Silence after the last sample, so the frame is whole.
        frame[read..].fill(0);
        encoder
            .encode(&frame, &mut packet)
            .map_err(|error| EncodeError::Failed(format!("Opus encoder: {error}")))?;
        writer.write_packet(&packet).map_err(failed)?;
        left -= read as u64;
        frames += 1;
        pacer.pace();
    }
    if frames == 0 {
        return Err(EncodeError::Failed("the recording has no audio".into()));
    }
    writer.finish().map_err(failed)?;
    Ok(writer.into_inner())
}

/// A `PartEncoder` over any frame encoder made fresh for each part.
pub struct OggPartEncoder<F, E>
where
    F: FnMut() -> Result<E, String>,
    E: FrameEncoder,
{
    make: F,
}

impl<F, E> OggPartEncoder<F, E>
where
    F: FnMut() -> Result<E, String>,
    E: FrameEncoder,
{
    pub fn new(make: F) -> Self {
        Self { make }
    }
}

impl<F, E> PartEncoder for OggPartEncoder<F, E>
where
    F: FnMut() -> Result<E, String>,
    E: FrameEncoder,
{
    fn encode_part(
        &mut self,
        pcm: &mut dyn Read,
        pcm_bytes: u64,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, EncodeError> {
        let mut encoder = (self.make)().map_err(EncodeError::Failed)?;
        encode_to_ogg(&mut encoder, pcm, pcm_bytes, cancel)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::atomic::AtomicBool};

    use super::{encode_to_ogg, EncodeError, FrameEncoder, OggPartEncoder, PartEncoder};
    use crate::ogg_opus::{duration_ms, read_stream};

    /// A stand-in codec: a packet of two bytes, the frame's first sample.
    struct Fake {
        frame: usize,
        fail_at: Option<usize>,
        calls: usize,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                frame: 640,
                fail_at: None,
                calls: 0,
            }
        }
    }

    impl FrameEncoder for Fake {
        fn frame_bytes(&self) -> usize {
            self.frame
        }
        fn encode(&mut self, pcm: &[u8], packet: &mut Vec<u8>) -> Result<(), String> {
            assert_eq!(pcm.len(), self.frame, "a whole frame, always");
            if self.fail_at == Some(self.calls) {
                return Err("boom".into());
            }
            self.calls += 1;
            packet.clear();
            packet.extend_from_slice(&pcm[..2]);
            Ok(())
        }
    }

    /// `frames` frames of PCM whose first byte counts up.
    fn pcm(frames: usize, extra: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        for frame in 0..frames {
            bytes.extend((0..640).map(|index| if index == 0 { frame as u8 } else { 7 }));
        }
        bytes.extend((0..extra).map(|_| 9));
        bytes
    }

    fn encode(bytes: &[u8], encoder: &mut Fake) -> Result<Vec<u8>, EncodeError> {
        encode_to_ogg(
            encoder,
            &mut Cursor::new(bytes.to_vec()),
            bytes.len() as u64,
            &AtomicBool::new(false),
        )
    }

    #[test]
    fn a_recording_becomes_one_packet_per_twenty_milliseconds_in_order() {
        let bytes = pcm(120, 0);
        let ogg = encode(&bytes, &mut Fake::new()).unwrap();
        let stream = read_stream(&mut Cursor::new(ogg.clone())).unwrap();
        assert_eq!(stream.packets.len(), 120);
        for (index, packet) in stream.packets.iter().enumerate() {
            assert_eq!(packet[0], index as u8, "packet {index} in order");
        }
        // 120 frames are 2 400 ms of packets less the decoder's pre-skip.
        let ms = duration_ms(&mut Cursor::new(ogg), stream.head.pre_skip)
            .unwrap()
            .unwrap();
        assert!((2_390..=2_400).contains(&ms), "{ms} ms");
    }

    #[test]
    fn a_partial_last_frame_is_padded_with_silence_and_still_encoded() {
        let bytes = pcm(10, 100);
        let mut encoder = Fake::new();
        let ogg = encode(&bytes, &mut encoder).unwrap();
        assert_eq!(encoder.calls, 11, "ten frames and the padded eleventh");
        assert_eq!(
            read_stream(&mut Cursor::new(ogg)).unwrap().packets.len(),
            11
        );
    }

    #[test]
    fn a_reader_that_ends_early_gives_the_frames_it_had() {
        // The part says 50 frames but the file holds 12.
        let bytes = pcm(12, 0);
        let ogg = encode_to_ogg(
            &mut Fake::new(),
            &mut Cursor::new(bytes),
            50 * 640,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            read_stream(&mut Cursor::new(ogg)).unwrap().packets.len(),
            12
        );
    }

    #[test]
    fn no_audio_an_odd_frame_or_a_failing_encoder_is_a_failure_not_a_panic() {
        assert!(matches!(
            encode(&[], &mut Fake::new()),
            Err(EncodeError::Failed(_))
        ));
        let mut odd = Fake::new();
        odd.frame = 641;
        assert!(matches!(
            encode(&pcm(3, 0), &mut odd),
            Err(EncodeError::Failed(_))
        ));
        let mut zero = Fake::new();
        zero.frame = 0;
        assert!(matches!(
            encode(&pcm(3, 0), &mut zero),
            Err(EncodeError::Failed(_))
        ));
        let mut breaks = Fake::new();
        breaks.fail_at = Some(5);
        let error = encode(&pcm(20, 0), &mut breaks).unwrap_err();
        assert!(
            matches!(&error, EncodeError::Failed(text) if text.contains("boom")),
            "{error:?}"
        );
    }

    #[test]
    fn cancelling_stops_within_a_second_of_audio() {
        let bytes = pcm(500, 0);
        let cancel = AtomicBool::new(true);
        let outcome = encode_to_ogg(
            &mut Fake::new(),
            &mut Cursor::new(bytes),
            500 * 640,
            &cancel,
        );
        assert_eq!(outcome, Err(EncodeError::Cancelled));
    }

    #[test]
    fn a_read_error_is_reported_as_the_recording() {
        struct Broken;
        impl std::io::Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("card removed"))
            }
        }
        let outcome = encode_to_ogg(
            &mut Fake::new(),
            &mut Broken,
            10_000,
            &AtomicBool::new(false),
        );
        assert!(
            matches!(&outcome, Err(EncodeError::Failed(text)) if text.contains("card removed")),
            "{outcome:?}"
        );
    }

    #[test]
    fn the_part_encoder_makes_a_fresh_encoder_for_every_part() {
        let mut made = 0;
        let mut parts = OggPartEncoder::new(|| {
            made += 1;
            Ok(Fake::new())
        });
        let cancel = AtomicBool::new(false);
        for _ in 0..3 {
            let bytes = pcm(5, 0);
            let ogg = parts
                .encode_part(&mut Cursor::new(bytes.clone()), bytes.len() as u64, &cancel)
                .unwrap();
            assert_eq!(read_stream(&mut Cursor::new(ogg)).unwrap().packets.len(), 5);
        }
        drop(parts);
        assert_eq!(made, 3);
        let mut refuses = OggPartEncoder::new(|| Err::<Fake, _>("no memory".to_string()));
        assert_eq!(
            refuses.encode_part(&mut Cursor::new(vec![0; 640]), 640, &cancel),
            Err(EncodeError::Failed("no memory".into()))
        );
    }
}

/// The ESP32-S3's Opus encoder (`esp_audio_codec`, as the XiaoZhi call uses).
#[cfg(target_os = "espidf")]
pub mod espidf {
    use core::ffi::c_void;
    use std::{sync::atomic::AtomicBool, time::Instant};

    use esp_idf_svc::sys::opus as codec;

    use super::{encode_to_ogg, EncodeError, FrameEncoder, PartEncoder, BITRATE_BPS, FRAME_MS};

    /// 16 kHz mono, 20 ms frames, 24 kbps variable bitrate, no DTX (every
    /// packet is 20 ms, as `ogg_opus` counts), the lowest complexity (the
    /// encoder must run faster than the audio on the worker's core).
    pub struct EspFrameEncoder {
        handle: *mut c_void,
        frame_bytes: usize,
        packet_bytes: usize,
    }

    impl EspFrameEncoder {
        pub fn new() -> Result<Self, String> {
            let mut config: codec::esp_opus_enc_config_t = unsafe { core::mem::zeroed() };
            config.sample_rate = 16_000;
            config.channel = 1;
            config.bits_per_sample = 16;
            config.bitrate = BITRATE_BPS;
            config.frame_duration = match FRAME_MS {
                20 => codec::esp_opus_enc_frame_duration_t_ESP_OPUS_ENC_FRAME_DURATION_20_MS,
                _ => codec::esp_opus_enc_frame_duration_t_ESP_OPUS_ENC_FRAME_DURATION_60_MS,
            };
            config.application_mode =
                codec::esp_opus_enc_application_t_ESP_OPUS_ENC_APPLICATION_VOIP;
            config.complexity = 0;
            config.enable_dtx = false;
            config.enable_vbr = true;
            let mut handle = core::ptr::null_mut();
            let size = u32::try_from(core::mem::size_of::<codec::esp_opus_enc_config_t>())
                .map_err(|error| error.to_string())?;
            // SAFETY: `config` is a fully initialised config of the size passed.
            let opened = unsafe {
                codec::esp_opus_enc_open(
                    (&mut config as *mut codec::esp_opus_enc_config_t).cast(),
                    size,
                    &mut handle,
                )
            };
            if opened != 0 || handle.is_null() {
                return Err(format!("error {opened} opening the encoder"));
            }
            let mut encoder = Self {
                handle,
                frame_bytes: 0,
                packet_bytes: 0,
            };
            let (mut frame_bytes, mut packet_bytes) = (0, 0);
            // SAFETY: the handle is open; both are out parameters.
            unsafe {
                codec::esp_opus_enc_get_frame_size(handle, &mut frame_bytes, &mut packet_bytes);
            }
            encoder.frame_bytes = usize::try_from(frame_bytes).unwrap_or(0);
            encoder.packet_bytes = usize::try_from(packet_bytes).unwrap_or(0).max(256);
            if encoder.frame_bytes == 0 {
                return Err("the encoder reports no frame size".into());
            }
            Ok(encoder)
        }
    }

    impl FrameEncoder for EspFrameEncoder {
        fn frame_bytes(&self) -> usize {
            self.frame_bytes
        }

        fn encode(&mut self, pcm: &[u8], packet: &mut Vec<u8>) -> Result<(), String> {
            packet.resize(self.packet_bytes, 0);
            let mut input: codec::esp_audio_enc_in_frame_t = unsafe { core::mem::zeroed() };
            input.buffer = pcm.as_ptr().cast_mut();
            input.len = u32::try_from(pcm.len()).map_err(|error| error.to_string())?;
            let mut output: codec::esp_audio_enc_out_frame_t = unsafe { core::mem::zeroed() };
            output.buffer = packet.as_mut_ptr();
            output.len = u32::try_from(packet.len()).map_err(|error| error.to_string())?;
            // SAFETY: the handle is open; both frames point at live buffers
            // of the lengths they carry.
            let code = unsafe { codec::esp_opus_enc_process(self.handle, &mut input, &mut output) };
            if code != 0 {
                return Err(format!("error {code} encoding"));
            }
            packet.truncate(output.encoded_bytes as usize);
            Ok(())
        }
    }

    impl Drop for EspFrameEncoder {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by `new` and is closed once.
            unsafe { codec::esp_opus_enc_close(self.handle) };
        }
    }

    /// Encoder parts on the device: a fresh codec for each, with its speed
    /// in the log (`took-ms` against `audio-ms`).
    #[derive(Default)]
    pub struct EspPartEncoder;

    impl PartEncoder for EspPartEncoder {
        fn encode_part(
            &mut self,
            pcm: &mut dyn std::io::Read,
            pcm_bytes: u64,
            cancel: &AtomicBool,
        ) -> Result<Vec<u8>, EncodeError> {
            let started = Instant::now();
            let mut encoder = EspFrameEncoder::new().map_err(EncodeError::Failed)?;
            let result = encode_to_ogg(&mut encoder, pcm, pcm_bytes, cancel);
            // 16 kHz mono 16-bit: 32 bytes a millisecond.
            let audio_ms = pcm_bytes / 32;
            match &result {
                Ok(ogg) => log::info!(
                    "rustmix-wave=opus-encode status=done audio-ms={audio_ms} took-ms={} bytes={}",
                    started.elapsed().as_millis(),
                    ogg.len()
                ),
                Err(error) => log::warn!(
                    "rustmix-wave=opus-encode status=stopped error={error:?} took-ms={}",
                    started.elapsed().as_millis()
                ),
            }
            result
        }
    }
}
