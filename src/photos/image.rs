//! Photo decoding and conversion for the panel: grey levels, upright
//! orientation, area-average scaling, a contrast stretch and dithering.

use std::{io::Read, ops::Range};

use anyhow::{anyhow, bail, Result};
use embedded_graphics::prelude::Point;
use jpeg_decoder::{CodingProcess, Decoder, PixelFormat};

use super::exif::Orientation;
use crate::{
    dither::{floyd_steinberg, luma},
    framebuffer::FrameBuffer,
};

/// Portrait screen size in pixels.
pub const SCREEN_WIDTH: usize = 480;
pub const SCREEN_HEIGHT: usize = 800;
/// Gallery thumbnail size in pixels.
pub const THUMB_WIDTH: usize = 144;
pub const THUMB_HEIGHT: usize = 216;
pub const THUMB_ROW_BYTES: usize = THUMB_WIDTH.div_ceil(8);
/// The decoder picks the smallest DCT scale that reaches this in one axis.
const DECODE_TARGET: u16 = 800;
/// Largest decoded picture accepted, in bytes.
const MAX_DECODED_BYTES: usize = 4 * 1024 * 1024;
/// Progressive and lossless JPEGs keep the whole picture while decoding.
const MAX_PROGRESSIVE_PIXELS: usize = 1_000_000;

/// How a photo covers the portrait screen.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PhotoFit {
    /// Cover the screen and crop what overflows.
    #[default]
    Fill,
    /// Show the whole photo on white.
    Whole,
}

impl PhotoFit {
    pub const ALL: [Self; 2] = [Self::Fill, Self::Whole];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill (crop)",
            Self::Whole => "Whole photo",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Whole => "whole",
        }
    }
}

/// A 1-bit gallery thumbnail: rows of `THUMB_ROW_BYTES`, bit set = white.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Thumbnail {
    pub bits: Vec<u8>,
}

impl Thumbnail {
    #[must_use]
    pub fn is_black(&self, x: usize, y: usize) -> bool {
        let byte = self.bits.get(y * THUMB_ROW_BYTES + x / 8).copied();
        byte.is_some_and(|byte| byte & (0x80 >> (x % 8)) == 0)
    }
}

/// A decoded photo in grey levels as stored, with the turn that makes it
/// upright.
pub struct GreyPhoto {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    orientation: Orientation,
    /// Upright size of the original file, before the decode scale.
    original: (u16, u16),
    /// Levels mapped to black and white by the contrast stretch.
    levels: (u8, u8),
}

impl GreyPhoto {
    #[must_use]
    pub fn new(
        (width, height): (usize, usize),
        pixels: Vec<u8>,
        orientation: Orientation,
        original: (u16, u16),
    ) -> Self {
        let levels = stretch_levels(&pixels);
        Self {
            width,
            height,
            pixels,
            orientation,
            original,
            levels,
        }
    }

    /// Upright size of the original photo, for display.
    #[must_use]
    pub const fn original_size(&self) -> (u16, u16) {
        self.original
    }

    /// Upright size of the decoded pixels.
    #[must_use]
    pub const fn upright_size(&self) -> (usize, usize) {
        self.orientation.upright_size(self.width, self.height)
    }

    fn level(&self, u: usize, v: usize) -> u8 {
        let (x, y) = self.orientation.source(u, v, self.width, self.height);
        self.pixels[y * self.width + x]
    }

    fn stretched(&self, level: u8) -> u8 {
        let (low, high) = self.levels;
        let span = u32::from(high - low).max(1);
        let level = u32::from(level.clamp(low, high) - low);
        (level * 255 / span) as u8
    }

    /// Area-average the upright photo into a `width` × `height` grey picture.
    #[must_use]
    pub fn resample(&self, width: usize, height: usize, fit: PhotoFit) -> Vec<u8> {
        let (source_width, source_height) = self.upright_size();
        let x_ratio = width as f32 / source_width as f32;
        let y_ratio = height as f32 / source_height as f32;
        let scale = match fit {
            PhotoFit::Fill => x_ratio.max(y_ratio),
            PhotoFit::Whole => x_ratio.min(y_ratio),
        };
        let step = 1.0 / scale;
        // Where target pixel (0, 0) starts in the photo; negative over white bars.
        let origin_x = (source_width as f32 - width as f32 * step) / 2.0;
        let origin_y = (source_height as f32 - height as f32 * step) / 2.0;
        let mut out = vec![255_u8; width * height];
        for y in 0..height {
            let top = origin_y + y as f32 * step;
            let Some(rows) = span(top, top + step, source_height) else {
                continue;
            };
            for x in 0..width {
                let left = origin_x + x as f32 * step;
                let Some(columns) = span(left, left + step, source_width) else {
                    continue;
                };
                let mut sum = 0_u32;
                for v in rows.clone() {
                    for u in columns.clone() {
                        sum += u32::from(self.level(u, v));
                    }
                }
                let count = (rows.len() * columns.len()) as u32;
                out[y * width + x] = (sum / count) as u8;
            }
        }
        out
    }

    /// The photo dithered onto the portrait screen, in native panel layout.
    #[must_use]
    pub fn screen_frame(&self, fit: PhotoFit) -> FrameBuffer {
        let grey = self.resample(SCREEN_WIDTH, SCREEN_HEIGHT, fit);
        let mut frame = FrameBuffer::new_white();
        floyd_steinberg(
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            |x, y| self.stretched(grey[y * SCREEN_WIDTH + x]),
            |x, y| frame.set_native_black(native_point(x, y), true),
        );
        frame
    }

    #[must_use]
    pub fn thumbnail(&self) -> Thumbnail {
        let grey = self.resample(THUMB_WIDTH, THUMB_HEIGHT, PhotoFit::Fill);
        let mut bits = vec![0xFF_u8; THUMB_ROW_BYTES * THUMB_HEIGHT];
        floyd_steinberg(
            THUMB_WIDTH,
            THUMB_HEIGHT,
            |x, y| self.stretched(grey[y * THUMB_WIDTH + x]),
            |x, y| bits[y * THUMB_ROW_BYTES + x / 8] &= !(0x80 >> (x % 8)),
        );
        Thumbnail { bits }
    }
}

/// Decode a JPEG at the smallest scale that still covers the screen. Errors
/// complete a sentence that starts with the file name.
pub fn decode_jpeg(reader: impl Read) -> Result<GreyPhoto> {
    let mut decoder = Decoder::new(reader);
    decoder
        .read_info()
        .map_err(|_| anyhow!("is not a readable JPEG"))?;
    let info = decoder
        .info()
        .ok_or_else(|| anyhow!("is not a readable JPEG"))?;
    let pixels = usize::from(info.width) * usize::from(info.height);
    let sequential = matches!(info.coding_process, CodingProcess::DctSequential);
    if !sequential && pixels > MAX_PROGRESSIVE_PIXELS {
        bail!("is a progressive JPEG; save it as a standard JPEG");
    }
    decoder.set_max_decoding_buffer_size(MAX_DECODED_BYTES);
    let (width, height) = decoder
        .scale(DECODE_TARGET, DECODE_TARGET)
        .map_err(|_| anyhow!("is not a readable JPEG"))?;
    let data = decoder
        .decode()
        .map_err(|error| anyhow!("could not be decoded ({error})"))?;
    let orientation = decoder
        .exif_data()
        .map_or(Orientation::Upright, Orientation::from_exif);
    drop(decoder);
    let size = (usize::from(width), usize::from(height));
    let grey = match info.pixel_format {
        PixelFormat::L8 => data,
        PixelFormat::RGB24 => data
            .chunks_exact(3)
            .map(|rgb| luma(rgb[0], rgb[1], rgb[2]))
            .collect(),
        _ => bail!("uses an unsupported color format"),
    };
    if grey.len() < size.0 * size.1 {
        bail!("is truncated");
    }
    let (original_width, original_height) =
        orientation.upright_size(usize::from(info.width), usize::from(info.height));
    let original = (original_width as u16, original_height as u16);
    Ok(GreyPhoto::new(size, grey, orientation, original))
}

/// Photo pixels under [start, end), or `None` when that is outside the photo.
fn span(start: f32, end: f32, limit: usize) -> Option<Range<usize>> {
    if end <= 0.0 || start >= limit as f32 {
        return None;
    }
    let first = start.max(0.0) as usize;
    let last = (end.ceil() as usize).clamp(first + 1, limit);
    Some(first..last)
}

/// Native panel position of portrait screen pixel (x, y).
fn native_point(x: usize, y: usize) -> Point {
    Point::new(y as i32, (SCREEN_WIDTH - 1 - x) as i32)
}

/// Levels at the 1st and 99th percentiles, or the full range when the photo
/// is too flat to stretch.
fn stretch_levels(pixels: &[u8]) -> (u8, u8) {
    let mut histogram = [0_usize; 256];
    for level in pixels {
        histogram[usize::from(*level)] += 1;
    }
    let tail = pixels.len() / 100;
    let low = first_beyond(&histogram, tail, 0..256);
    let high = first_beyond(&histogram, tail, (0..256).rev());
    if high < low + 48 {
        (0, 255)
    } else {
        (low as u8, high as u8)
    }
}

/// First level along `levels` at which more than `tail` pixels were counted.
fn first_beyond(
    histogram: &[usize; 256],
    tail: usize,
    levels: impl Iterator<Item = usize>,
) -> usize {
    let mut seen = 0;
    for level in levels {
        seen += histogram[level];
        if seen > tail {
            return level;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{decode_jpeg, stretch_levels, GreyPhoto, PhotoFit};
    use crate::photos::{
        exif::Orientation,
        test_photos::{color_jpeg, grey_jpeg, progressive_jpeg},
    };

    #[test]
    fn decodes_grey_and_color_jpegs() {
        let photo = decode_jpeg(Cursor::new(grey_jpeg(96, 64, None))).unwrap();
        assert_eq!(photo.upright_size(), (96, 64));
        assert_eq!(photo.original_size(), (96, 64));
        let color = decode_jpeg(Cursor::new(color_jpeg(48, 32))).unwrap();
        assert_eq!(color.upright_size(), (48, 32));
        let error = decode_jpeg(Cursor::new(b"not a jpeg")).err().unwrap();
        assert_eq!(error.to_string(), "is not a readable JPEG");
    }

    #[test]
    fn exif_rotation_stands_the_photo_upright() {
        let photo = decode_jpeg(Cursor::new(grey_jpeg(96, 64, Some(6)))).unwrap();
        assert_eq!(photo.upright_size(), (64, 96));
        assert_eq!(photo.original_size(), (64, 96));
    }

    #[test]
    fn large_progressive_jpegs_are_refused_before_decoding() {
        assert!(decode_jpeg(Cursor::new(progressive_jpeg(64, 48))).is_ok());
        let error = decode_jpeg(Cursor::new(progressive_jpeg(1100, 1000)))
            .err()
            .unwrap();
        assert!(error.to_string().starts_with("is a progressive JPEG"));
    }

    #[test]
    fn fill_crops_and_whole_letterboxes_on_white() {
        // A 4 × 2 photo: left half black, right half white.
        let pixels = vec![0, 0, 255, 255, 0, 0, 255, 255];
        let photo = GreyPhoto::new((4, 2), pixels, Orientation::Upright, (4, 2));
        let fill = photo.resample(2, 2, PhotoFit::Fill);
        assert_eq!(fill, [0, 255, 0, 255]);
        let whole = photo.resample(4, 4, PhotoFit::Whole);
        assert_eq!(&whole[..4], &[255; 4]);
        assert_eq!(&whole[4..8], &[0, 0, 255, 255]);
    }

    #[test]
    fn screen_frames_and_thumbnails_have_panel_sizes() {
        let photo = decode_jpeg(Cursor::new(grey_jpeg(96, 64, None))).unwrap();
        let frame = photo.screen_frame(PhotoFit::Whole);
        assert_eq!(frame.as_bytes().len(), 48_000);
        // 144 × 216 pixels at one bit each.
        assert_eq!(photo.thumbnail().bits.len(), 18 * 216);
    }

    #[test]
    fn contrast_stretch_ignores_flat_photos() {
        assert_eq!(stretch_levels(&[128; 100]), (0, 255));
        let mut pixels = vec![60_u8; 50];
        pixels.extend([200_u8; 50]);
        assert_eq!(stretch_levels(&pixels), (60, 200));
    }
}
