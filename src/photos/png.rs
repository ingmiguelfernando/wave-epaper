//! PNG photos: chunks read in order, image data inflated as it streams in,
//! each row unfiltered, turned to grey (transparent parts on white) and
//! box-reduced at once, so a large PNG never sits whole in memory.

use std::io::Read;

use anyhow::{anyhow, bail, Result};
use miniz_oxide::{
    inflate::stream::{inflate, InflateState},
    DataFormat, MZError, MZFlush, MZStatus,
};

use super::{exif::Orientation, image::GreyPhoto};
use crate::dither::luma;

/// The eight bytes every PNG starts with.
pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// The reduced picture keeps at least this many pixels on its short side.
const TARGET: usize = 800;
/// Largest reduced picture kept, in bytes (one per pixel).
const MAX_REDUCED_BYTES: usize = 4 * 1024 * 1024;
/// Widest PNG accepted; one row of it is held while decoding.
const MAX_SIDE: u32 = 16_384;
const MAX_CHUNK_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Color {
    Grey,
    Rgb,
    Palette,
    GreyAlpha,
    Rgba,
}

impl Color {
    const fn channels(self) -> usize {
        match self {
            Self::Grey | Self::Palette => 1,
            Self::GreyAlpha => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Header {
    width: usize,
    height: usize,
    depth: u8,
    color: Color,
}

impl Header {
    fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != 13 {
            bail!("is not a readable PNG");
        }
        let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let (depth, color) = (data[8], data[9]);
        let color = match color {
            0 => Color::Grey,
            2 => Color::Rgb,
            3 => Color::Palette,
            4 => Color::GreyAlpha,
            6 => Color::Rgba,
            _ => bail!("uses an unsupported PNG color type"),
        };
        let valid_depth = match color {
            Color::Grey => matches!(depth, 1 | 2 | 4 | 8 | 16),
            Color::Palette => matches!(depth, 1 | 2 | 4 | 8),
            _ => matches!(depth, 8 | 16),
        };
        if !valid_depth || data[10] != 0 || data[11] != 0 {
            bail!("is not a readable PNG");
        }
        if data[12] != 0 {
            bail!("is an interlaced PNG; save it without interlacing");
        }
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            bail!("is too large ({width} × {height})");
        }
        Ok(Self {
            width: width as usize,
            height: height as usize,
            depth,
            color,
        })
    }

    /// Bytes in one unfiltered row.
    const fn stride(&self) -> usize {
        (self.width * self.color.channels() * self.depth as usize).div_ceil(8)
    }

    /// Bytes per complete pixel, at least one, for the filters.
    const fn filter_step(&self) -> usize {
        let bits = self.color.channels() * self.depth as usize;
        if bits < 8 {
            1
        } else {
            bits / 8
        }
    }
}

/// Box-reduces grey rows by `factor` in both directions as they arrive.
struct Reducer {
    factor: usize,
    width: usize,
    sums: Vec<u32>,
    rows_summed: usize,
    out: Vec<u8>,
}

impl Reducer {
    fn new(width: usize, height: usize) -> Self {
        let mut factor = (width.min(height) / TARGET).max(1);
        while (width / factor) * (height / factor) > MAX_REDUCED_BYTES {
            factor += 1;
        }
        let out_width = (width / factor).max(1);
        Self {
            factor,
            width: out_width,
            sums: vec![0; out_width],
            rows_summed: 0,
            out: Vec::with_capacity(out_width * (height / factor).max(1)),
        }
    }

    fn push(&mut self, grey: &[u8]) {
        for (index, sum) in self.sums.iter_mut().enumerate() {
            let start = index * self.factor;
            let end = (start + self.factor).min(grey.len());
            *sum += grey[start..end].iter().map(|&g| u32::from(g)).sum::<u32>();
        }
        self.rows_summed += 1;
        if self.rows_summed == self.factor {
            let area = (self.factor * self.factor) as u32;
            self.out
                .extend(self.sums.iter().map(|&sum| (sum / area) as u8));
            self.sums.fill(0);
            self.rows_summed = 0;
        }
    }

    fn finish(mut self) -> (usize, usize, Vec<u8>) {
        if self.out.is_empty() && self.rows_summed > 0 {
            // A PNG shorter than the factor still gives one row.
            let area = (self.factor * self.rows_summed) as u32;
            self.out
                .extend(self.sums.iter().map(|&sum| (sum / area) as u8));
        }
        let height = self.out.len() / self.width;
        (self.width, height, self.out)
    }
}

/// Rows as they come out of the inflater: unfiltered, then grey.
struct Rows {
    header: Header,
    palette: Vec<[u8; 3]>,
    alpha: Vec<u8>,
    row: Vec<u8>,
    previous: Vec<u8>,
    filled: usize,
    rows_done: usize,
    grey: Vec<u8>,
    reducer: Reducer,
}

impl Rows {
    fn feed(&mut self, mut data: &[u8]) -> Result<()> {
        while !data.is_empty() && self.rows_done < self.header.height {
            let wanted = self.row.len() - self.filled;
            let taken = wanted.min(data.len());
            self.row[self.filled..self.filled + taken].copy_from_slice(&data[..taken]);
            self.filled += taken;
            data = &data[taken..];
            if self.filled == self.row.len() {
                self.finish_row()?;
                self.filled = 0;
            }
        }
        Ok(())
    }

    fn finish_row(&mut self) -> Result<()> {
        let filter = self.row[0];
        let step = self.header.filter_step();
        let (_, current) = self.row.split_at_mut(1);
        unfilter(filter, step, current, &self.previous)?;
        self.previous.copy_from_slice(current);
        self.to_grey();
        self.reducer.push(&self.grey);
        self.rows_done += 1;
        Ok(())
    }

    fn to_grey(&mut self) {
        let header = self.header;
        let row = &self.previous;
        let sample = |index: usize| -> u8 {
            match header.depth {
                16 => row[index * 2],
                8 => row[index],
                bits => {
                    let bits = usize::from(bits);
                    let per_byte = 8 / bits;
                    let byte = row[index / per_byte];
                    let shift = 8 - bits * (index % per_byte + 1);
                    let max = (1_u16 << bits) - 1;
                    (byte >> shift) & max as u8
                }
            }
        };
        let scale_up = |value: u8| -> u8 {
            match header.depth {
                1 => value * 255,
                2 => value * 85,
                4 => value * 17,
                _ => value,
            }
        };
        let on_white = |grey: u8, alpha: u8| -> u8 {
            let (grey, alpha) = (u32::from(grey), u32::from(alpha));
            ((grey * alpha + 255 * (255 - alpha)) / 255) as u8
        };
        for x in 0..header.width {
            self.grey[x] = match header.color {
                Color::Grey => scale_up(sample(x)),
                Color::GreyAlpha => on_white(sample(x * 2), sample(x * 2 + 1)),
                Color::Rgb => luma(sample(x * 3), sample(x * 3 + 1), sample(x * 3 + 2)),
                Color::Rgba => {
                    let grey = luma(sample(x * 4), sample(x * 4 + 1), sample(x * 4 + 2));
                    on_white(grey, sample(x * 4 + 3))
                }
                Color::Palette => {
                    let index = usize::from(sample(x));
                    let [r, g, b] = self.palette.get(index).copied().unwrap_or([0; 3]);
                    let alpha = self.alpha.get(index).copied().unwrap_or(255);
                    on_white(luma(r, g, b), alpha)
                }
            };
        }
    }
}

/// Undo one row's PNG filter in place against the row above.
fn unfilter(filter: u8, step: usize, row: &mut [u8], above: &[u8]) -> Result<()> {
    match filter {
        0 => {}
        1 => {
            for i in step..row.len() {
                row[i] = row[i].wrapping_add(row[i - step]);
            }
        }
        2 => {
            for (byte, up) in row.iter_mut().zip(above) {
                *byte = byte.wrapping_add(*up);
            }
        }
        3 => {
            for i in 0..row.len() {
                let left = if i >= step { u16::from(row[i - step]) } else { 0 };
                let average = ((left + u16::from(above[i])) / 2) as u8;
                row[i] = row[i].wrapping_add(average);
            }
        }
        4 => {
            for i in 0..row.len() {
                let left = if i >= step { row[i - step] } else { 0 };
                let corner = if i >= step { above[i - step] } else { 0 };
                row[i] = row[i].wrapping_add(paeth(left, above[i], corner));
            }
        }
        _ => bail!("is a damaged PNG"),
    }
    Ok(())
}

fn paeth(left: u8, up: u8, corner: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(corner);
    let to_left = (estimate - i16::from(left)).abs();
    let to_up = (estimate - i16::from(up)).abs();
    let to_corner = (estimate - i16::from(corner)).abs();
    if to_left <= to_up && to_left <= to_corner {
        left
    } else if to_up <= to_corner {
        up
    } else {
        corner
    }
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<()> {
    reader
        .read_exact(buffer)
        .map_err(|_| anyhow!("is a truncated PNG"))
}

/// Decode a PNG, reduced so its short side stays at least 800 pixels where
/// the original has them. Errors complete a sentence that starts with the
/// file name.
pub fn decode_png(mut reader: impl Read) -> Result<GreyPhoto> {
    let mut signature = [0_u8; 8];
    read_exact(&mut reader, &mut signature)?;
    if signature != SIGNATURE {
        bail!("is not a readable PNG");
    }
    let mut rows: Option<Rows> = None;
    let mut inflater = InflateState::new_boxed(DataFormat::Zlib);
    let mut inflated = vec![0_u8; 8 * 1024];
    let mut stream_ended = false;
    loop {
        let mut head = [0_u8; 8];
        read_exact(&mut reader, &mut head)?;
        let length = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        if length > MAX_CHUNK_BYTES {
            bail!("is a damaged PNG");
        }
        let kind = [head[4], head[5], head[6], head[7]];
        if &kind == b"IDAT" {
            let Some(rows) = rows.as_mut() else {
                bail!("is a damaged PNG");
            };
            let mut left = length as usize;
            let mut chunk = [0_u8; 4 * 1024];
            while left > 0 {
                let take = left.min(chunk.len());
                read_exact(&mut reader, &mut chunk[..take])?;
                left -= take;
                let mut input = &chunk[..take];
                while !input.is_empty() && !stream_ended {
                    let result = inflate(&mut inflater, input, &mut inflated, MZFlush::None);
                    input = &input[result.bytes_consumed..];
                    rows.feed(&inflated[..result.bytes_written])?;
                    match result.status {
                        Ok(MZStatus::StreamEnd) => stream_ended = true,
                        Ok(_) => {}
                        Err(MZError::Buf)
                            if result.bytes_consumed > 0 || result.bytes_written > 0 => {}
                        Err(_) => bail!("is a damaged PNG"),
                    }
                }
            }
            // Output held back for lack of room comes out with no new input.
            while !stream_ended {
                let result = inflate(&mut inflater, &[], &mut inflated, MZFlush::None);
                if result.bytes_written == 0 {
                    break;
                }
                rows.feed(&inflated[..result.bytes_written])?;
                if result.status == Ok(MZStatus::StreamEnd) {
                    stream_ended = true;
                }
            }
        } else {
            let mut data = vec![0_u8; length as usize];
            read_exact(&mut reader, &mut data)?;
            match &kind {
                b"IHDR" => {
                    let header = Header::parse(&data)?;
                    let stride = header.stride();
                    rows = Some(Rows {
                        header,
                        palette: Vec::new(),
                        alpha: Vec::new(),
                        row: vec![0; stride + 1],
                        previous: vec![0; stride],
                        filled: 0,
                        rows_done: 0,
                        grey: vec![255; header.width],
                        reducer: Reducer::new(header.width, header.height),
                    });
                }
                b"PLTE" => {
                    if let Some(rows) = rows.as_mut() {
                        rows.palette = data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                    }
                }
                b"tRNS" => {
                    if let Some(rows) = rows.as_mut().filter(|r| r.header.color == Color::Palette) {
                        rows.alpha = data;
                    }
                }
                b"IEND" => break,
                _ => {}
            }
        }
        let mut crc = [0_u8; 4];
        read_exact(&mut reader, &mut crc)?;
    }
    let rows = rows.ok_or_else(|| anyhow!("is not a readable PNG"))?;
    if rows.header.color == Color::Palette && rows.palette.is_empty() {
        bail!("is a damaged PNG");
    }
    if rows.rows_done < rows.header.height {
        bail!("is a truncated PNG");
    }
    let original = (rows.header.width as u16, rows.header.height as u16);
    let (width, height, pixels) = rows.reducer.finish();
    Ok(GreyPhoto::new(
        (width, height),
        pixels,
        Orientation::Upright,
        original,
    ))
}

#[cfg(test)]
pub(crate) mod test_png {
    use super::SIGNATURE;

    /// A PNG of `color_type` (0, 2, 3, 4 or 6) at 8 bits, every row with the
    /// filter `filter`, pixels from `pixel(x, y)` as channel bytes.
    pub fn png(
        width: u32,
        height: u32,
        color_type: u8,
        filter: u8,
        pixel: impl Fn(u32, u32) -> Vec<u8>,
        palette: Option<&[[u8; 3]]>,
    ) -> Vec<u8> {
        let mut raw = Vec::new();
        let mut previous: Vec<u8> = Vec::new();
        for y in 0..height {
            let row: Vec<u8> = (0..width).flat_map(|x| pixel(x, y)).collect();
            let step = row.len() / width as usize;
            if previous.is_empty() {
                previous = vec![0; row.len()];
            }
            raw.push(filter);
            for i in 0..row.len() {
                let left = if i >= step { row[i - step] } else { 0 };
                let up = previous[i];
                let corner = if i >= step { previous[i - step] } else { 0 };
                let predicted = match filter {
                    0 => 0,
                    1 => left,
                    2 => up,
                    3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                    _ => super::paeth(left, up, corner),
                };
                raw.push(row[i].wrapping_sub(predicted));
            }
            previous = row;
        }
        let mut out = SIGNATURE.to_vec();
        let mut header = Vec::new();
        header.extend(width.to_be_bytes());
        header.extend(height.to_be_bytes());
        header.extend([8, color_type, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &header);
        if let Some(palette) = palette {
            let bytes: Vec<u8> = palette.iter().flatten().copied().collect();
            chunk(&mut out, b"PLTE", &bytes);
        }
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        // Split the data over two chunks, as encoders may.
        let half = compressed.len() / 2;
        chunk(&mut out, b"IDAT", &compressed[..half]);
        chunk(&mut out, b"IDAT", &compressed[half..]);
        chunk(&mut out, b"IEND", &[]);
        out
    }

    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        out.extend(kind);
        out.extend(data);
        out.extend([0, 0, 0, 0]);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{decode_png, test_png::png};

    fn half_dark(x: u32, width: u32) -> u8 {
        if x < width / 2 {
            30
        } else {
            220
        }
    }

    #[test]
    fn every_filter_and_color_type_decodes_to_the_same_grey() {
        for filter in 0..=4 {
            for color_type in [0_u8, 2, 4, 6] {
                let file = png(
                    64,
                    48,
                    color_type,
                    filter,
                    |x, _| {
                        let g = half_dark(x, 64);
                        match color_type {
                            0 => vec![g],
                            2 => vec![g, g, g],
                            4 => vec![g, 255],
                            _ => vec![g, g, g, 255],
                        }
                    },
                    None,
                );
                let photo = decode_png(Cursor::new(file)).unwrap();
                assert_eq!(photo.upright_size(), (64, 48));
                let grey = photo.resample(64, 48, super::super::image::PhotoFit::Whole);
                assert_eq!(grey[10], 30, "filter {filter} color {color_type}");
                assert_eq!(grey[50], 220, "filter {filter} color {color_type}");
            }
        }
    }

    #[test]
    fn palettes_and_transparency_land_on_white() {
        let palette = [[0, 0, 0], [255, 255, 255]];
        let file = png(
            8,
            4,
            3,
            1,
            |x, _| vec![u8::from(x >= 4)],
            Some(&palette),
        );
        let photo = decode_png(Cursor::new(file)).unwrap();
        let grey = photo.resample(8, 4, super::super::image::PhotoFit::Whole);
        assert_eq!((grey[0], grey[7]), (0, 255));

        let clear = png(4, 4, 6, 4, |_, _| vec![0, 0, 0, 0], None);
        let photo = decode_png(Cursor::new(clear)).unwrap();
        let grey = photo.resample(4, 4, super::super::image::PhotoFit::Whole);
        assert!(grey.iter().all(|&g| g == 255), "transparent is white");
    }

    #[test]
    fn large_pngs_are_reduced_while_decoding() {
        let file = png(2_400, 1_700, 0, 2, |x, _| vec![half_dark(x, 2_400)], None);
        let photo = decode_png(Cursor::new(file)).unwrap();
        assert_eq!(photo.original_size(), (2_400, 1_700));
        // 1700 / 800 = 2: half size, the short side still over 800.
        assert_eq!(photo.upright_size(), (1_200, 850));
    }

    #[test]
    fn damaged_and_unsupported_files_say_why() {
        let reason = |bytes: Vec<u8>| decode_png(Cursor::new(bytes)).err().unwrap().to_string();
        assert_eq!(reason(b"PNG".to_vec()), "is a truncated PNG");
        assert_eq!(reason(b"0123456789abcdef".to_vec()), "is not a readable PNG");
        let mut cut = png(32, 32, 0, 0, |x, _| vec![x as u8], None);
        cut.truncate(cut.len() - 30);
        assert!(reason(cut).contains("PNG"));
        let mut interlaced = png(8, 8, 0, 0, |_, _| vec![0], None);
        // IHDR data starts after the signature and the chunk head; byte 12 is interlace.
        interlaced[8 + 8 + 12] = 1;
        assert_eq!(
            reason(interlaced),
            "is an interlaced PNG; save it without interlacing"
        );
    }
}
