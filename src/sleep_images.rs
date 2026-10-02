//! Read-only sleep-image discovery and BMP decoding.
//!
//! Sleep pictures live below `/sdcard/RUSTMIX/SLEEP`. Any uncompressed 1-, 4-,
//! 8-, 24- or 32-bit BMP of 800 × 480 (panel orientation) or 480 × 800
//! (portrait, turned like the portrait UI) works; grey and colour pictures are
//! dithered to black and white.

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use embedded_graphics::prelude::Point;

use crate::{
    dither::{floyd_steinberg, luma},
    framebuffer::{FrameBuffer, FRAMEBUFFER_SIZE, HEIGHT, ROW_BYTES, WIDTH},
    storage::SD_MOUNT_POINT,
};

/// Runtime directory containing removable-SD sleep images.
pub const SLEEP_IMAGE_DIRECTORY: &str = "/sdcard/RUSTMIX/SLEEP";
/// Bounded number of files examined on each entry to sleep mode.
pub const MAX_SLEEP_IMAGE_CANDIDATES: usize = 32;
const BMP_FILE_HEADER_BYTES: usize = 14;
const BMP_INFO_HEADER_BYTES: usize = 40;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
/// Enough of a file for any BMP header plus a 256-entry palette.
const HEADER_PROBE_BYTES: u64 = 2048;

/// Scan diagnostics retained with the selected frame so serial logs can
/// distinguish a missing directory from incomplete FAT VFS type hints.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SleepImageScanStats {
    pub raw_entries: usize,
    pub candidate_entries: usize,
    pub metadata_fallbacks: usize,
    pub ignored_entries: usize,
    pub rejected_entries: usize,
    /// Why the first rejected picture cannot be used, e.g. `A.BMP is 24 × 24; ...`.
    pub first_rejection: Option<String>,
}

/// Random-selection diagnostics retained with the selected frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepImageChoice {
    pub random_word: u32,
    pub previous_index: Option<usize>,
    pub selected_index: usize,
    pub anti_repeat: bool,
}

/// One selected sleep image ready for direct native-panel transfer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SleepImageSelection {
    pub file_name: String,
    pub frame: FrameBuffer,
    pub valid_count: usize,
    pub rejected_count: usize,
    pub raw_entries: usize,
    pub candidate_entries: usize,
    pub metadata_fallbacks: usize,
    pub ignored_entries: usize,
    pub scan_error: Option<String>,
    pub choice: Option<SleepImageChoice>,
    pub fallback: bool,
    /// Shown on the built-in sleep screen: why no picture was used.
    pub note: Option<String>,
}

impl SleepImageSelection {
    /// A picture prepared elsewhere, such as a starred photo.
    #[must_use]
    pub fn picture(file_name: String, frame: FrameBuffer) -> Self {
        Self {
            file_name,
            frame,
            valid_count: 1,
            rejected_count: 0,
            raw_entries: 0,
            candidate_entries: 0,
            metadata_fallbacks: 0,
            ignored_entries: 0,
            scan_error: None,
            choice: None,
            fallback: false,
            note: None,
        }
    }
}

/// Read-only SD-backed sleep-image catalog.
#[derive(Clone, Debug)]
pub struct SleepImageCatalog {
    directory: PathBuf,
    last_selected_file_name: Option<String>,
}

impl Default for SleepImageCatalog {
    fn default() -> Self {
        Self::new(SLEEP_IMAGE_DIRECTORY)
    }
}

impl SleepImageCatalog {
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            last_selected_file_name: None,
        }
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Scan read-only assets, skip unusable files and select a hardware-seeded
    /// random valid image. When multiple assets exist, the previous image is
    /// removed from the choice space so consecutive entries never repeat it.
    /// Without a usable picture the selection falls back to a built-in frame
    /// and a note that explains why.
    pub fn select_random(&mut self, random_word: u32) -> SleepImageSelection {
        self.select(random_word, false)
    }

    /// Like `select_random`, or the picture after the previous one by name
    /// when `in_order` is set.
    pub fn select(&mut self, random_word: u32, in_order: bool) -> SleepImageSelection {
        match self.scan_valid_images() {
            Ok((valid, mut stats)) if !valid.is_empty() => {
                let previous_index = self
                    .last_selected_file_name
                    .as_deref()
                    .and_then(|previous| {
                        valid
                            .iter()
                            .position(|path| file_name_label(path) == previous)
                    });
                let next = previous_index.map_or(0, |index| (index + 1) % valid.len());
                let (index, anti_repeat) = if in_order {
                    (next, false)
                } else {
                    choose_random_index(valid.len(), previous_index, random_word)
                };
                let path = &valid[index];
                let file_name = file_name_label(path);
                self.last_selected_file_name = Some(file_name.clone());
                let choice = Some(SleepImageChoice {
                    random_word,
                    previous_index,
                    selected_index: index,
                    anti_repeat,
                });
                match decode_sleep_bmp_file(path) {
                    Ok(frame) => {
                        self.selection(file_name, frame, valid.len(), stats, None, choice, false)
                    }
                    Err(error) => {
                        stats.rejected_entries = stats.rejected_entries.saturating_add(1);
                        self.fallback(
                            stats,
                            Some(format!("selected BMP decode failed: {error:#}")),
                            format!("{file_name} {error}"),
                        )
                    }
                }
            }
            Ok((_valid, stats)) => {
                let note = stats
                    .first_rejection
                    .clone()
                    .unwrap_or_else(|| format!("No BMP pictures in {}", self.folder_label()));
                self.fallback(stats, None, note)
            }
            Err(error) => {
                let note = if error.kind() == io::ErrorKind::NotFound {
                    format!("No {} folder on the SD card", self.folder_label())
                } else {
                    format!("Could not read {}", self.folder_label())
                };
                self.fallback(
                    SleepImageScanStats::default(),
                    Some(format!("directory scan failed: {error}")),
                    note,
                )
            }
        }
    }

    /// The folder as it appears on the card, e.g. `/RUSTMIX/SLEEP`.
    fn folder_label(&self) -> String {
        match self.directory.strip_prefix(SD_MOUNT_POINT) {
            Ok(relative) => format!("/{}", relative.display()),
            Err(_) => self.directory.display().to_string(),
        }
    }

    fn scan_valid_images(&self) -> io::Result<(Vec<PathBuf>, SleepImageScanStats)> {
        let mut candidates = Vec::new();
        let mut stats = SleepImageScanStats::default();
        for entry in fs::read_dir(&self.directory)?.take(MAX_SLEEP_IMAGE_CANDIDATES) {
            let entry = entry?;
            stats.raw_entries = stats.raw_entries.saturating_add(1);

            // Hidden files include the "._NAME" companions macOS writes to FAT cards.
            let hinted_type = entry.file_type().ok();
            if entry.file_name().to_string_lossy().starts_with('.')
                || hinted_type
                    .as_ref()
                    .is_some_and(|file_type| file_type.is_symlink())
            {
                stats.ignored_entries = stats.ignored_entries.saturating_add(1);
                continue;
            }

            // ESP-IDF FAT VFS may expose an incomplete d_type hint. Mirror the
            // verified read-only Files browser: use metadata from the
            // immediately following stat call as the final classification.
            let metadata = entry.metadata()?;
            let hint_is_incomplete = hinted_type.as_ref().map_or(true, |file_type| {
                !file_type.is_file() && !file_type.is_dir()
            });
            if hint_is_incomplete {
                stats.metadata_fallbacks = stats.metadata_fallbacks.saturating_add(1);
            }

            if !metadata.file_type().is_file() || !has_bmp_extension(&entry.path()) {
                stats.ignored_entries = stats.ignored_entries.saturating_add(1);
                continue;
            }
            stats.candidate_entries = stats.candidate_entries.saturating_add(1);
            candidates.push(entry.path());
        }
        candidates.sort_by_key(|path| file_name_label(path).to_ascii_uppercase());

        let mut valid = Vec::new();
        for path in candidates {
            match probe_sleep_bmp(&path) {
                Ok(()) => valid.push(path),
                Err(error) => {
                    stats.rejected_entries = stats.rejected_entries.saturating_add(1);
                    if stats.first_rejection.is_none() {
                        stats.first_rejection = Some(format!("{} {error}", file_name_label(&path)));
                    }
                }
            }
        }
        Ok((valid, stats))
    }

    fn fallback(
        &self,
        stats: SleepImageScanStats,
        scan_error: Option<String>,
        note: String,
    ) -> SleepImageSelection {
        let mut selection = self.selection(
            "built-in".into(),
            built_in_sleep_frame(),
            0,
            stats,
            scan_error,
            None,
            true,
        );
        selection.note = Some(note);
        selection
    }

    fn selection(
        &self,
        file_name: String,
        frame: FrameBuffer,
        valid_count: usize,
        stats: SleepImageScanStats,
        scan_error: Option<String>,
        choice: Option<SleepImageChoice>,
        fallback: bool,
    ) -> SleepImageSelection {
        SleepImageSelection {
            file_name,
            frame,
            valid_count,
            rejected_count: stats.rejected_entries,
            raw_entries: stats.raw_entries,
            candidate_entries: stats.candidate_entries,
            metadata_fallbacks: stats.metadata_fallbacks,
            ignored_entries: stats.ignored_entries,
            scan_error,
            choice,
            fallback,
            note: None,
        }
    }
}

/// Choose a bounded random slot while excluding the previous slot when possible.
fn choose_random_index(
    valid_count: usize,
    previous_index: Option<usize>,
    random_word: u32,
) -> (usize, bool) {
    if valid_count <= 1 {
        return (0, false);
    }

    if let Some(previous_index) = previous_index.filter(|index| *index < valid_count) {
        let slot = random_word as usize % (valid_count - 1);
        let selected_index = if slot >= previous_index {
            slot + 1
        } else {
            slot
        };
        (selected_index, true)
    } else {
        (random_word as usize % valid_count, false)
    }
}

/// Decode one sleep picture file into native panel bytes.
pub fn decode_sleep_bmp_file(path: &Path) -> Result<FrameBuffer> {
    let bytes = fs::read(path).map_err(|_| anyhow!("could not be read"))?;
    decode_sleep_bmp(&bytes)
}

/// Check a picture's header and length without reading all of its pixels.
fn probe_sleep_bmp(path: &Path) -> Result<()> {
    let unreadable = |_| anyhow!("could not be read");
    let mut file = fs::File::open(path).map_err(unreadable)?;
    let file_len = file.metadata().map_err(unreadable)?.len() as usize;
    let mut head = Vec::new();
    file.by_ref()
        .take(HEADER_PROBE_BYTES)
        .read_to_end(&mut head)
        .map_err(unreadable)?;
    BmpLayout::parse(&head, file_len).map(|_| ())
}

/// Decode a whole BMP file held in memory. Error messages complete a sentence
/// that starts with the file name, e.g. `SLEEP.BMP is truncated`.
pub fn decode_sleep_bmp(bytes: &[u8]) -> Result<FrameBuffer> {
    let layout = BmpLayout::parse(bytes, bytes.len())?;
    if let Some(invert) = layout.native_black_and_white() {
        let mut native = vec![0_u8; FRAMEBUFFER_SIZE];
        for (y, destination) in native.chunks_exact_mut(ROW_BYTES).enumerate() {
            let source = &layout.row(bytes, y)[..ROW_BYTES];
            for (destination, source) in destination.iter_mut().zip(source) {
                *destination = if invert { !*source } else { *source };
            }
        }
        return FrameBuffer::from_native_bytes(native).map_err(|message| anyhow!(message));
    }

    let mut frame = FrameBuffer::new_white();
    floyd_steinberg(
        layout.width,
        layout.height,
        |x, y| layout.grey(layout.row(bytes, y), x),
        |x, y| frame.set_native_black(layout.native_point(x, y), true),
    );
    Ok(frame)
}

/// Where the pixels of an accepted BMP live and how to read them as grey.
struct BmpLayout {
    width: usize,
    height: usize,
    top_down: bool,
    bits_per_pixel: u16,
    pixel_offset: usize,
    stride: usize,
    /// Grey level of each palette entry, for 1-, 4- and 8-bit pictures.
    palette: Vec<u8>,
}

impl BmpLayout {
    /// Validate a header held in `bytes` against the whole file's length.
    fn parse(bytes: &[u8], file_len: usize) -> Result<Self> {
        if bytes.len() < BMP_FILE_HEADER_BYTES + BMP_INFO_HEADER_BYTES || &bytes[0..2] != b"BM" {
            bail!("is not a BMP picture");
        }
        let pixel_offset = read_u32(bytes, 10)? as usize;
        let header_bytes = read_u32(bytes, 14)? as usize;
        let signed_width = read_i32(bytes, 18)?;
        let signed_height = read_i32(bytes, 22)?;
        let bits_per_pixel = read_u16(bytes, 28)?;
        let compression = read_u32(bytes, 30)?;
        if header_bytes < BMP_INFO_HEADER_BYTES || read_u16(bytes, 26)? != 1 {
            bail!("uses an unsupported BMP header");
        }
        let width = signed_width.unsigned_abs() as usize;
        let height = signed_height.unsigned_abs() as usize;
        let (native_width, native_height) = (WIDTH as usize, HEIGHT as usize);
        if (width, height) != (native_width, native_height)
            && (width, height) != (native_height, native_width)
        {
            bail!("is {width}\u{d7}{height}; it must be 480\u{d7}800 or 800\u{d7}480");
        }
        if !matches!(bits_per_pixel, 1 | 4 | 8 | 24 | 32) {
            bail!("has {bits_per_pixel}-bit color; use 1, 4, 8, 24 or 32-bit");
        }
        match compression {
            BI_RGB => {}
            BI_BITFIELDS if bits_per_pixel == 32 => {
                let masks = (
                    read_u32(bytes, 54)?,
                    read_u32(bytes, 58)?,
                    read_u32(bytes, 62)?,
                );
                if masks != (0x00FF_0000, 0x0000_FF00, 0x0000_00FF) {
                    bail!("uses an unsupported 32-bit color layout");
                }
            }
            _ => bail!("is compressed; save it uncompressed"),
        }
        let palette = if bits_per_pixel <= 8 {
            let most = 1_usize << bits_per_pixel;
            let used = read_u32(bytes, 46)? as usize;
            let colors = if used == 0 || used > most { most } else { used };
            let start = BMP_FILE_HEADER_BYTES + header_bytes;
            (0..colors)
                .map(|index| palette_grey(bytes, start + index * 4))
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let stride = (width * usize::from(bits_per_pixel)).div_ceil(32) * 4;
        if pixel_offset.saturating_add(stride * height) > file_len {
            bail!("is truncated");
        }
        Ok(Self {
            width,
            height,
            top_down: signed_height < 0,
            bits_per_pixel,
            pixel_offset,
            stride,
            palette,
        })
    }

    /// For a 1-bit picture in panel orientation with a pure black and white
    /// palette, whether its bits must be inverted to match the panel.
    fn native_black_and_white(&self) -> Option<bool> {
        if self.bits_per_pixel != 1 || self.width != WIDTH as usize {
            return None;
        }
        match self.palette.as_slice() {
            [0, 255] => Some(false),
            [255, 0] => Some(true),
            _ => None,
        }
    }

    /// Stored bytes of picture row `y`, counted from the top.
    fn row<'a>(&self, bytes: &'a [u8], y: usize) -> &'a [u8] {
        let stored = if self.top_down {
            y
        } else {
            self.height - 1 - y
        };
        let start = self.pixel_offset + stored * self.stride;
        &bytes[start..start + self.stride]
    }

    fn grey(&self, row: &[u8], x: usize) -> u8 {
        let entry = |index: u8| *self.palette.get(usize::from(index)).unwrap_or(&255);
        match self.bits_per_pixel {
            1 => entry((row[x / 8] >> (7 - x % 8)) & 1),
            4 => entry((row[x / 2] >> (4 - 4 * (x % 2))) & 0x0F),
            8 => entry(row[x]),
            bits => {
                let at = x * usize::from(bits / 8);
                luma(row[at + 2], row[at + 1], row[at])
            }
        }
    }

    /// Panel position of picture pixel (x, y); portrait pictures turn the
    /// same way as the portrait UI.
    fn native_point(&self, x: usize, y: usize) -> Point {
        if self.width == WIDTH as usize {
            Point::new(x as i32, y as i32)
        } else {
            Point::new(y as i32, HEIGHT as i32 - 1 - x as i32)
        }
    }
}

fn built_in_sleep_frame() -> FrameBuffer {
    let mut frame = FrameBuffer::new_white();
    let width = WIDTH as i32;
    let height = HEIGHT as i32;
    for x in 0..width {
        frame.set_native_black(Point::new(x, 0), true);
        frame.set_native_black(Point::new(x, height - 1), true);
    }
    for y in 0..height {
        frame.set_native_black(Point::new(0, y), true);
        frame.set_native_black(Point::new(width - 1, y), true);
    }
    for x in 240..560 {
        frame.set_native_black(Point::new(x, 220), true);
        frame.set_native_black(Point::new(x, 260), true);
    }
    for y in 220..=260 {
        frame.set_native_black(Point::new(240, y), true);
        frame.set_native_black(Point::new(559, y), true);
    }
    frame
}

fn has_bmp_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bmp"))
}

fn file_name_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sleep.bmp")
        .to_string()
}

fn palette_grey(bytes: &[u8], offset: usize) -> Result<u8> {
    let entry = bytes
        .get(offset..offset + 3)
        .ok_or_else(|| anyhow!("is truncated"))?;
    Ok(luma(entry[2], entry[1], entry[0]))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow!("is truncated"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("is truncated"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("is truncated"))?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{decode_sleep_bmp, SleepImageCatalog};
    use crate::framebuffer::{FRAMEBUFFER_SIZE, HEIGHT, ROW_BYTES, WIDTH};
    use embedded_graphics::prelude::Point;

    fn fixture() -> Vec<u8> {
        include_bytes!("../examples/sd-card/RUSTMIX/SLEEP/SLEEP.BMP").to_vec()
    }

    /// An uncompressed 8- or 24-bit BMP; `pixel(x, y)` counts rows from the
    /// top and returns a palette index or `0xRRGGBB`.
    fn build_bmp(
        (width, height): (usize, usize),
        bits: u16,
        header_bytes: usize,
        top_down: bool,
        palette: &[[u8; 4]],
        pixel: impl Fn(usize, usize) -> u32,
    ) -> Vec<u8> {
        let stride = (width * usize::from(bits)).div_ceil(32) * 4;
        let pixel_offset = 14 + header_bytes + palette.len() * 4;
        let mut bytes = vec![0_u8; pixel_offset + stride * height];
        let file_size = bytes.len() as u32;
        let signed_height = if top_down {
            -(height as i32)
        } else {
            height as i32
        };
        bytes[0..2].copy_from_slice(b"BM");
        bytes[2..6].copy_from_slice(&file_size.to_le_bytes());
        bytes[10..14].copy_from_slice(&(pixel_offset as u32).to_le_bytes());
        bytes[14..18].copy_from_slice(&(header_bytes as u32).to_le_bytes());
        bytes[18..22].copy_from_slice(&(width as i32).to_le_bytes());
        bytes[22..26].copy_from_slice(&signed_height.to_le_bytes());
        bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
        bytes[28..30].copy_from_slice(&bits.to_le_bytes());
        for (index, entry) in palette.iter().enumerate() {
            let at = 14 + header_bytes + index * 4;
            bytes[at..at + 4].copy_from_slice(entry);
        }
        for y in 0..height {
            let stored = if top_down { y } else { height - 1 - y };
            let row = pixel_offset + stored * stride;
            for x in 0..width {
                let value = pixel(x, y).to_le_bytes();
                let size = usize::from(bits / 8);
                bytes[row + x * size..row + (x + 1) * size].copy_from_slice(&value[..size]);
            }
        }
        bytes
    }

    #[test]
    fn accepts_uploaded_native_sleep_fixture() {
        let frame = decode_sleep_bmp(&fixture()).unwrap();
        assert_eq!(frame.as_bytes().len(), FRAMEBUFFER_SIZE);
    }

    #[test]
    fn rejects_wrong_dimensions_and_color_depth() {
        let mut wrong_width = fixture();
        wrong_width[18..22].copy_from_slice(&799_i32.to_le_bytes());
        let error = decode_sleep_bmp(&wrong_width).unwrap_err().to_string();
        assert!(error.starts_with("is 799×480"), "{error}");
        let mut wrong_depth = fixture();
        wrong_depth[28..30].copy_from_slice(&16_u16.to_le_bytes());
        let error = decode_sleep_bmp(&wrong_depth).unwrap_err().to_string();
        assert!(error.starts_with("has 16-bit color"), "{error}");
    }

    #[test]
    fn rejects_compressed_and_truncated_payloads() {
        let mut compressed = fixture();
        compressed[30..34].copy_from_slice(&1_u32.to_le_bytes());
        assert!(decode_sleep_bmp(&compressed).is_err());
        let mut truncated = fixture();
        truncated.truncate(100);
        assert!(decode_sleep_bmp(&truncated).is_err());
    }

    #[test]
    fn reverses_bottom_up_rows_into_native_panel_order() {
        let mut bytes = fixture();
        let pixel_offset = 62;
        bytes[pixel_offset..pixel_offset + FRAMEBUFFER_SIZE].fill(0xFF);
        let bottom_row = pixel_offset + (HEIGHT as usize - 1) * ROW_BYTES;
        bytes[bottom_row] = 0x7F;
        let frame = decode_sleep_bmp(&bytes).unwrap();
        assert_eq!(frame.as_bytes()[0], 0x7F);
        assert_eq!(frame.as_bytes()[ROW_BYTES], 0xFF);
    }

    #[test]
    fn inverts_reversed_black_white_palette() {
        let mut bytes = fixture();
        bytes[54..58].copy_from_slice(&[255, 255, 255, 0]);
        bytes[58..62].copy_from_slice(&[0, 0, 0, 0]);
        let pixel_offset = 62;
        bytes[pixel_offset..pixel_offset + FRAMEBUFFER_SIZE].fill(0x00);
        let frame = decode_sleep_bmp(&bytes).unwrap();
        assert!(frame.as_bytes().iter().all(|byte| *byte == 0xFF));
    }

    #[test]
    fn dithers_grey_portrait_pictures_in_portrait_orientation() {
        let grey: Vec<[u8; 4]> = (0..=255).map(|level| [level, level, level, 0]).collect();
        let bytes = build_bmp((480, 800), 8, 40, false, &grey, |x, y| match (x, y) {
            (_, 400..) => 128,
            (0..=239, _) => 0,
            _ => 255,
        });
        let frame = decode_sleep_bmp(&bytes).unwrap();
        // The picture's top-left corner lands at the bottom of the native left edge.
        assert_eq!(frame.is_black(Point::new(0, 479)), Some(true));
        assert_eq!(frame.is_black(Point::new(0, 0)), Some(false));
        let black = (400..800)
            .flat_map(|x| (0..480).map(move |y| Point::new(x, y)))
            .filter(|point| frame.is_black(*point) == Some(true))
            .count();
        let percent = black * 100 / (400 * 480);
        assert!((40..=60).contains(&percent), "{percent}% black");
    }

    #[test]
    fn reads_top_down_24_bit_pictures_with_large_headers() {
        let bytes = build_bmp((800, 480), 24, 124, true, &[], |x, y| {
            if x < 10 && y < 10 {
                0x00_0000
            } else {
                0xFF_FFFF
            }
        });
        let frame = decode_sleep_bmp(&bytes).unwrap();
        assert_eq!(frame.is_black(Point::new(0, 0)), Some(true));
        assert_eq!(frame.is_black(Point::new(9, 9)), Some(true));
        assert_eq!(frame.is_black(Point::new(10, 10)), Some(false));
        assert_eq!(frame.is_black(Point::new(799, 479)), Some(false));
    }

    #[test]
    fn scan_skips_hidden_files_and_explains_rejections() {
        let root = unique_temp_dir("rejections");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("._SLEEP.BMP"), b"macOS resource fork").unwrap();
        let mut large = fixture();
        large[18..22].copy_from_slice(&1024_i32.to_le_bytes());
        large[22..26].copy_from_slice(&768_i32.to_le_bytes());
        fs::write(root.join("SLEEP.BMP"), large).unwrap();
        let selection = SleepImageCatalog::new(&root).select_random(0);
        assert!(selection.fallback);
        assert_eq!(selection.ignored_entries, 1);
        assert_eq!(selection.rejected_count, 1);
        assert_eq!(
            selection.note.as_deref(),
            Some("SLEEP.BMP is 1024×768; it must be 480×800 or 800×480")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn empty_and_missing_folders_explain_the_fallback() {
        let root = unique_temp_dir("empty");
        fs::create_dir_all(&root).unwrap();
        let note = SleepImageCatalog::new(&root).select_random(0).note.unwrap();
        assert!(note.starts_with("No BMP pictures in"), "{note}");
        let _ = fs::remove_dir_all(&root);
        let note = SleepImageCatalog::new(&root).select_random(0).note.unwrap();
        assert!(note.ends_with("folder on the SD card"), "{note}");
    }

    #[test]
    fn random_selection_avoids_immediate_repeat_when_multiple_assets_exist() {
        let root = unique_temp_dir("random-anti-repeat");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("SLEEP01.BMP"), fixture()).unwrap();
        fs::write(root.join("SLEEP.BMP"), fixture()).unwrap();
        let mut catalog = SleepImageCatalog::new(&root);
        let first = catalog.select_random(0);
        let second = catalog.select_random(0);
        assert_eq!(first.file_name, "SLEEP.BMP");
        assert_eq!(second.file_name, "SLEEP01.BMP");
        assert_ne!(first.file_name, second.file_name);
        assert!(!first.choice.unwrap().anti_repeat);
        assert!(second.choice.unwrap().anti_repeat);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_reports_candidate_diagnostics_for_valid_assets() {
        let root = unique_temp_dir("diagnostics");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("SLEEP.BMP"), fixture()).unwrap();
        fs::write(root.join("README.TXT"), b"ignored").unwrap();
        let mut catalog = SleepImageCatalog::new(&root);
        let selection = catalog.select_random(0);
        assert!(!selection.fallback);
        assert_eq!(selection.valid_count, 1);
        assert_eq!(selection.raw_entries, 2);
        assert_eq!(selection.candidate_entries, 1);
        assert_eq!(selection.ignored_entries, 1);
        assert!(selection.scan_error.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_directory_reports_scan_error() {
        let mut catalog = SleepImageCatalog::new(unique_temp_dir("missing-error"));
        let selection = catalog.select_random(0);
        assert!(selection.fallback);
        assert!(selection
            .scan_error
            .as_deref()
            .is_some_and(|error| error.contains("directory scan failed")));
    }

    #[test]
    fn missing_directory_uses_built_in_fallback() {
        let mut catalog = SleepImageCatalog::new(unique_temp_dir("missing"));
        let selection = catalog.select_random(0);
        assert!(selection.fallback);
        assert_eq!(selection.file_name, "built-in");
        assert_eq!(
            selection.frame.as_bytes().len(),
            (WIDTH as usize / 8) * HEIGHT as usize
        );
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-wave-sleep-images-{label}-{nanos}"))
    }
}
