//! Ogg pages around Opus packets (RFC 7845), host-tested and free of
//! ESP-IDF. The writer wraps encoder packets as `.ogg`; the reader recovers
//! them after a crash and reports the duration from the file's end.

use std::io::{self, Read, Seek, SeekFrom, Write};

/// Ogg capture pattern at the start of every page.
const CAPTURE: [u8; 4] = *b"OggS";
/// Page version; the only one defined.
const OGG_VERSION: u8 = 0;
/// Header type: a packet continues from the previous page.
const FLAG_CONTINUED: u8 = 0x01;
/// Header type: beginning of stream.
const FLAG_BOS: u8 = 0x02;
/// Header type: end of stream.
const FLAG_EOS: u8 = 0x04;
/// Opus pre-skip default for 16 kHz encoding (samples at 48 kHz).
pub const DEFAULT_PRE_SKIP: u16 = 312;
/// Input sample rate written into `OpusHead`.
pub const INPUT_SAMPLE_RATE: u32 = 16_000;
/// Granule samples per 20 ms packet at 48 kHz.
pub const SAMPLES_PER_PACKET: u64 = 960;
/// Segments one page can hold.
const MAX_SEGMENTS: usize = 255;
/// The biggest packet one page can hold: 254 full segments and one of 254.
const MAX_PACKET_BYTES: usize = MAX_SEGMENTS * 255 - 1;
/// Packets per second of audio; one page holds a second when they fit.
pub const PACKETS_PER_SECOND: usize = 50;

/// Decoded `OpusHead` identification header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpusHead {
    pub channels: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub output_gain: i16,
    pub mapping_family: u8,
}

impl Default for OpusHead {
    fn default() -> Self {
        Self {
            channels: 1,
            pre_skip: DEFAULT_PRE_SKIP,
            input_sample_rate: INPUT_SAMPLE_RATE,
            output_gain: 0,
            mapping_family: 0,
        }
    }
}

impl OpusHead {
    /// The 19-byte identification header of RFC 7845 section 5.1.
    #[must_use]
    pub fn to_bytes(self) -> [u8; 19] {
        let mut bytes = [0_u8; 19];
        bytes[0..4].copy_from_slice(b"Opus");
        bytes[4..8].copy_from_slice(b"Head");
        bytes[8] = 1;
        bytes[9] = self.channels;
        bytes[10..12].copy_from_slice(&self.pre_skip.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.input_sample_rate.to_le_bytes());
        bytes[16..18].copy_from_slice(&self.output_gain.to_le_bytes());
        bytes[18] = self.mapping_family;
        bytes
    }

    /// Parse the 19-byte identification header.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 19 || &bytes[0..8] != b"OpusHead" || bytes[8] != 1 {
            return None;
        }
        Some(Self {
            channels: bytes[9],
            pre_skip: u16::from_le_bytes([bytes[10], bytes[11]]),
            input_sample_rate: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            output_gain: i16::from_le_bytes([bytes[16], bytes[17]]),
            mapping_family: bytes[18],
        })
    }
}

/// The `OpusTags` comment header: vendor `wave`, no user comments.
#[must_use]
pub fn opus_tags_bytes() -> Vec<u8> {
    let vendor = b"wave";
    let mut bytes = Vec::with_capacity(8 + 4 + vendor.len() + 4);
    bytes.extend_from_slice(b"OpusTags");
    bytes.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    bytes.extend_from_slice(vendor);
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

/// Ogg CRC: polynomial `0x04C11DB7`, initial 0, no reflection, no final
/// XOR. `123456789` checks to `0x89A1897F`.
#[must_use]
pub fn ogg_crc(data: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &byte in data {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Lacing values for one packet: `0` ends a packet, `255` continues. A
/// packet whose size is a multiple of 255 ends with a `0` segment.
#[must_use]
pub fn lacing_values(packet_len: usize) -> Vec<u8> {
    let mut values = Vec::new();
    let mut left = packet_len;
    while left >= 255 {
        values.push(255);
        left -= 255;
    }
    values.push(left as u8);
    values
}

/// Build one page around `body`, with its CRC filled in.
fn build_page(
    header_type: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    body: &[u8],
    segment_table: &[u8],
) -> Vec<u8> {
    let mut page = Vec::with_capacity(27 + segment_table.len() + body.len());
    page.extend_from_slice(&CAPTURE);
    page.push(OGG_VERSION);
    page.push(header_type);
    page.extend_from_slice(&granule.to_le_bytes());
    page.extend_from_slice(&serial.to_le_bytes());
    page.extend_from_slice(&sequence.to_le_bytes());
    page.extend_from_slice(&0_u32.to_le_bytes());
    page.push(segment_table.len() as u8);
    page.extend_from_slice(segment_table);
    page.extend_from_slice(body);
    let crc = ogg_crc(&page);
    page[22..26].copy_from_slice(&crc.to_le_bytes());
    page
}

/// Writer of Ogg Opus: `OpusHead`, `OpusTags`, then one page per second.
pub struct OggOpusWriter<W: Write> {
    inner: W,
    head: OpusHead,
    serial: u32,
    sequence: u32,
    /// Packets waiting for the next page.
    packets: Vec<Vec<u8>>,
    /// Granule at the end of the last written audio packet.
    granule: u64,
    finished: bool,
}

impl<W: Write> OggOpusWriter<W> {
    /// Start a stream with the identification and comment pages.
    pub fn new(mut inner: W, head: OpusHead, serial: u32) -> io::Result<Self> {
        let head_page = build_page(
            FLAG_BOS,
            0,
            serial,
            0,
            &head.to_bytes(),
            &[head.to_bytes().len() as u8],
        );
        inner.write_all(&head_page)?;
        let tags = opus_tags_bytes();
        let tag_segments = lacing_values(tags.len());
        let tag_page = build_page(0, 0, serial, 1, &tags, &tag_segments);
        inner.write_all(&tag_page)?;
        Ok(Self {
            inner,
            head,
            serial,
            sequence: 2,
            packets: Vec::new(),
            granule: 0,
            finished: false,
        })
    }

    /// The head written at the start.
    #[must_use]
    pub fn head(&self) -> OpusHead {
        self.head
    }

    /// Queue one 20 ms Opus packet. Pages flush every second of audio.
    pub fn write_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        assert!(!self.finished, "stream already finished");
        if lacing_values(packet.len()).len() > MAX_SEGMENTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "packet does not fit in one Ogg page",
            ));
        }
        self.packets.push(packet.to_vec());
        if self.packets.len() >= PACKETS_PER_SECOND {
            self.flush_page(false)?;
        }
        Ok(())
    }

    /// The sink the file was written to, after `finish`.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Write the remaining packets as the end-of-stream page.
    pub fn finish(&mut self) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        self.flush_page(true)?;
        self.finished = true;
        self.inner.flush()
    }

    fn flush_page(&mut self, eos: bool) -> io::Result<()> {
        if self.packets.is_empty() && !eos {
            return Ok(());
        }
        let mut body = Vec::new();
        let mut segment_table = Vec::new();
        let mut kept = 0;
        for packet in &self.packets {
            let values = lacing_values(packet.len());
            if segment_table.len() + values.len() > MAX_SEGMENTS {
                break;
            }
            segment_table.extend_from_slice(&values);
            body.extend_from_slice(packet);
            kept += 1;
        }
        // The granule is the sample count at the end of the page's last packet.
        self.granule = self
            .granule
            .saturating_add(u64::try_from(kept).unwrap_or(0) * SAMPLES_PER_PACKET);
        let header_type = if eos { FLAG_EOS } else { 0 };
        let page = build_page(
            header_type,
            self.granule,
            self.serial,
            self.sequence,
            &body,
            &segment_table,
        );
        self.inner.write_all(&page)?;
        self.sequence = self.sequence.wrapping_add(1);
        self.packets.drain(..kept);
        if eos && !self.packets.is_empty() {
            // Rare: the last second split across pages. Write the rest as EOS.
            return self.flush_page(true);
        }
        Ok(())
    }
}

/// A page header parsed from the stream.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PageHeader {
    header_type: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    segment_table: Vec<u8>,
    body: Vec<u8>,
}

/// Read one page; `Ok(None)` at a clean end. A cut or damaged page is also
/// `None`, so the pages before it stay available.
fn read_page(reader: &mut impl Read) -> io::Result<Option<PageHeader>> {
    let mut prefix = [0_u8; 27];
    if !read_exact_or_eof(reader, &mut prefix)? {
        return Ok(None);
    }
    if prefix[0..4] != CAPTURE || prefix[4] != OGG_VERSION {
        return Ok(None);
    }
    let segment_count = prefix[26] as usize;
    let mut segment_table = vec![0_u8; segment_count];
    if !read_exact_or_eof(reader, &mut segment_table)? {
        return Ok(None);
    }
    let body_len: usize = segment_table.iter().map(|&value| usize::from(value)).sum();
    let mut body = vec![0_u8; body_len];
    if !read_exact_or_eof(reader, &mut body)? {
        return Ok(None);
    }
    let stored_crc = u32::from_le_bytes([prefix[22], prefix[23], prefix[24], prefix[25]]);
    // CRC covers the header (with checksum zeroed) and the body.
    let mut zeroed = prefix;
    zeroed[22..26].copy_from_slice(&0_u32.to_le_bytes());
    let mut crc_input = Vec::with_capacity(27 + segment_count + body.len());
    crc_input.extend_from_slice(&zeroed);
    crc_input.extend_from_slice(&segment_table);
    crc_input.extend_from_slice(&body);
    if ogg_crc(&crc_input) != stored_crc {
        return Ok(None);
    }
    Ok(Some(PageHeader {
        header_type: prefix[5],
        granule: u64::from_le_bytes([
            prefix[6], prefix[7], prefix[8], prefix[9], prefix[10], prefix[11], prefix[12],
            prefix[13],
        ]),
        serial: u32::from_le_bytes([prefix[14], prefix[15], prefix[16], prefix[17]]),
        sequence: u32::from_le_bytes([prefix[18], prefix[19], prefix[20], prefix[21]]),
        segment_table,
        body,
    }))
}

fn read_exact_or_eof(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<bool> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => return Ok(false),
            Ok(n) => filled += n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}

/// Packets and the identification header recovered from an Ogg Opus stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OggOpusStream {
    pub head: OpusHead,
    pub packets: Vec<Vec<u8>>,
}

/// Walk every complete audio packet: `visit` sees each one in order. The
/// `OpusHead` and `OpusTags` packets are not visited. A cut or damaged page
/// ends the walk without an error for the packets before it. Returns the
/// head (default when the stream has none) and how many packets were seen.
fn walk_packets(
    reader: &mut impl Read,
    mut visit: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<(OpusHead, usize)> {
    let mut head = OpusHead::default();
    let mut count = 0;
    let mut pending: Vec<u8> = Vec::new();
    let mut started = false;
    loop {
        let Some(page) = read_page(reader)? else {
            break;
        };
        let mut offset = 0;
        let mut first = true;
        for &lace in &page.segment_table {
            let take = usize::from(lace);
            if offset + take > page.body.len() {
                return Ok((head, count));
            }
            let piece = &page.body[offset..offset + take];
            offset += take;
            if first && page.header_type & FLAG_CONTINUED != 0 {
                // Continues a packet from the previous page.
            } else if first {
                pending.clear();
            }
            pending.extend_from_slice(piece);
            if pending.len() > MAX_PACKET_BYTES {
                // Only a damaged file continues a packet this far; stop
                // before it fills the memory.
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "packet does not fit in one Ogg page",
                ));
            }
            first = false;
            if lace < 255 {
                // A complete packet.
                if !started {
                    if let Some(parsed) = OpusHead::parse(&pending) {
                        head = parsed;
                        pending.clear();
                        continue;
                    }
                    if pending.starts_with(b"OpusTags") {
                        started = true;
                        pending.clear();
                        continue;
                    }
                    // Unknown leading packet: ignore it.
                    pending.clear();
                    continue;
                }
                let packet = core::mem::take(&mut pending);
                visit(&packet)?;
                count += 1;
            }
        }
    }
    Ok((head, count))
}

/// Read every complete packet. A cut or damaged page ends the stream without
/// an error for the pages before it. The `OpusHead` and `OpusTags` packets
/// are not returned as audio.
pub fn read_stream(reader: &mut impl Read) -> io::Result<OggOpusStream> {
    let mut packets: Vec<Vec<u8>> = Vec::new();
    let (head, _) = walk_packets(reader, |packet| {
        packets.push(packet.to_vec());
        Ok(())
    })?;
    Ok(OggOpusStream { head, packets })
}

/// Read the pages once and count the audio packets (the head and tags are
/// not counted). A cut or damaged page ends the count without an error for
/// the pages before it, as `read_stream` does.
pub fn count_packets(reader: &mut impl Read) -> io::Result<(OpusHead, usize)> {
    walk_packets(reader, |_| Ok(()))
}

/// Duration in milliseconds from the last page's granule less the pre-skip,
/// read from the end of the file.
pub fn duration_ms(reader: &mut (impl Read + Seek), pre_skip: u16) -> io::Result<Option<u64>> {
    const TAIL: u64 = 64 * 1024;
    let end = reader.seek(SeekFrom::End(0))?;
    let start = end.saturating_sub(TAIL);
    reader.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    reader.take(TAIL).read_to_end(&mut tail)?;
    // Scan backward for the last complete page that starts in the tail.
    let mut last_granule = None;
    let mut index = 0;
    while index + 27 <= tail.len() {
        if tail[index..index + 4] == CAPTURE {
            if let Some(page) = read_page(&mut &tail[index..])? {
                if page.granule != u64::MAX {
                    last_granule = Some(page.granule);
                }
                let body_len: usize = page
                    .segment_table
                    .iter()
                    .map(|&value| usize::from(value))
                    .sum();
                index += 27 + page.segment_table.len() + body_len;
                continue;
            }
        }
        index += 1;
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(last_granule.map(|granule| {
        let samples = granule.saturating_sub(u64::from(pre_skip));
        samples * 1_000 / 48_000
    }))
}

/// How many packets go in each balanced part, the way `ai_client::wav_parts`
/// splits a recording, so none is a short tail. Never zero.
fn balanced_part_size(total: usize, max_seconds: u32) -> usize {
    let limit = (max_seconds as usize)
        .saturating_mul(PACKETS_PER_SECOND)
        .max(1);
    let count = total.div_ceil(limit).max(1);
    total.div_ceil(count).max(1)
}

/// Split a stream into complete Ogg Opus files of at most `max_seconds` of
/// packets each (50 a second). The parts hold balanced packet counts, the
/// way `ai_client::wav_parts` splits a recording, so none is a short tail.
/// An empty stream gives no parts. The caller passes
/// `ai_client::MAX_PART_SECONDS` as the limit; this module does not copy
/// the number. A packet too big for one Ogg page (read from a damaged file)
/// is an error, not a panic.
pub fn split_parts(stream: &OggOpusStream, max_seconds: u32) -> io::Result<Vec<Vec<u8>>> {
    let size = balanced_part_size(stream.packets.len(), max_seconds);
    let mut parts = Vec::new();
    for chunk in stream.packets.chunks(size) {
        let mut writer = OggOpusWriter::new(Vec::new(), stream.head, 0)?;
        for packet in chunk {
            writer.write_packet(packet)?;
        }
        writer.finish()?;
        parts.push(writer.inner);
    }
    Ok(parts)
}

/// Split an Ogg Opus file into balanced parts without holding the stream in
/// memory: count the packets, rewind, then write each packet to a complete
/// Ogg Opus file through `make_part(index)`. Each part is finished before
/// the next starts. At most one page and one packet are held at a time.
/// Returns the number of parts; an empty stream calls `make_part` zero
/// times. Errors from `make_part` or a writer are returned.
pub fn split_file<W: Write>(
    reader: &mut (impl Read + Seek),
    max_seconds: u32,
    mut make_part: impl FnMut(usize) -> io::Result<W>,
) -> io::Result<usize> {
    let (head, total) = count_packets(reader)?;
    reader.seek(SeekFrom::Start(0))?;
    if total == 0 {
        return Ok(0);
    }
    let size = balanced_part_size(total, max_seconds);
    let mut current: Option<OggOpusWriter<W>> = None;
    let mut in_part = 0;
    let mut opened = 0;
    walk_packets(reader, |packet| {
        if in_part == size {
            if let Some(mut done) = current.take() {
                done.finish()?;
            }
            in_part = 0;
        }
        if current.is_none() {
            current = Some(OggOpusWriter::new(make_part(opened)?, head, 0)?);
            opened += 1;
        }
        if let Some(writer) = current.as_mut() {
            writer.write_packet(packet)?;
        }
        in_part += 1;
        Ok(())
    })?;
    if let Some(mut done) = current.take() {
        done.finish()?;
    }
    Ok(opened)
}

#[cfg(test)]
mod tests {
    use super::{
        build_page, count_packets, duration_ms, lacing_values, ogg_crc, opus_tags_bytes,
        read_stream, split_file, split_parts, OggOpusStream, OggOpusWriter, OpusHead, FLAG_BOS,
        FLAG_CONTINUED, FLAG_EOS, PACKETS_PER_SECOND, SAMPLES_PER_PACKET,
    };
    use std::{
        cell::RefCell,
        io::{Cursor, Write},
        rc::Rc,
    };

    #[test]
    fn crc_matches_the_check_value() {
        assert_eq!(ogg_crc(b"123456789"), 0x89A1_897F);
    }

    #[test]
    fn opus_head_is_nineteen_bytes_per_rfc_7845() {
        let head = OpusHead {
            channels: 1,
            pre_skip: 312,
            input_sample_rate: 16_000,
            output_gain: 0,
            mapping_family: 0,
        };
        let bytes = head.to_bytes();
        assert_eq!(bytes.len(), 19);
        assert_eq!(&bytes[0..8], b"OpusHead");
        assert_eq!(bytes[8], 1);
        assert_eq!(bytes[9], 1);
        assert_eq!(u16::from_le_bytes([bytes[10], bytes[11]]), 312);
        assert_eq!(
            u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            16_000
        );
        assert_eq!(i16::from_le_bytes([bytes[16], bytes[17]]), 0);
        assert_eq!(bytes[18], 0);
        assert_eq!(OpusHead::parse(&bytes), Some(head));
    }

    #[test]
    fn opus_tags_is_vendor_wave_and_no_comments() {
        let tags = opus_tags_bytes();
        assert_eq!(&tags[0..8], b"OpusTags");
        assert_eq!(
            u32::from_le_bytes([tags[8], tags[9], tags[10], tags[11]]),
            4
        );
        assert_eq!(&tags[12..16], b"wave");
        assert_eq!(
            u32::from_le_bytes([tags[16], tags[17], tags[18], tags[19]]),
            0
        );
        assert_eq!(tags.len(), 20);
    }

    #[test]
    fn lacing_covers_zero_254_255_256_and_510() {
        assert_eq!(lacing_values(0), vec![0]);
        assert_eq!(lacing_values(254), vec![254]);
        assert_eq!(lacing_values(255), vec![255, 0]);
        assert_eq!(lacing_values(256), vec![255, 1]);
        assert_eq!(lacing_values(510), vec![255, 255, 0]);
    }

    fn sample_packet(index: usize) -> Vec<u8> {
        vec![0xA0 | (index as u8 & 0x0F); 40]
    }

    #[test]
    fn round_trips_a_few_hundred_packets() {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        let count = 300;
        for index in 0..count {
            writer.write_packet(&sample_packet(index)).unwrap();
        }
        writer.finish().unwrap();
        let bytes = writer.inner.into_inner();
        let stream = read_stream(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(stream.head, OpusHead::default());
        assert_eq!(stream.packets.len(), count);
        for (index, packet) in stream.packets.iter().enumerate() {
            assert_eq!(packet, &sample_packet(index));
        }
    }

    #[test]
    fn pages_hold_a_second_of_audio_with_flags_and_granules() {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        for index in 0..PACKETS_PER_SECOND {
            writer.write_packet(&sample_packet(index)).unwrap();
        }
        writer.finish().unwrap();
        let bytes = writer.inner.into_inner();
        // BOS on the first page.
        assert_eq!(bytes[5], FLAG_BOS);
        let stream = read_stream(&mut Cursor::new(bytes.clone())).unwrap();
        assert_eq!(stream.packets.len(), PACKETS_PER_SECOND);
        // The audio page's granule is 50 packets of 960 samples at 48 kHz.
        let mut last = 0_u64;
        let mut index = 0;
        while index + 27 <= bytes.len() {
            if &bytes[index..index + 4] == b"OggS" {
                last = u64::from_le_bytes(bytes[index + 6..index + 14].try_into().unwrap());
                let segments = bytes[index + 26] as usize;
                let body: usize = bytes[index + 27..index + 27 + segments]
                    .iter()
                    .map(|&value| usize::from(value))
                    .sum();
                index += 27 + segments + body;
            } else {
                index += 1;
            }
        }
        assert_eq!(
            last,
            50 * SAMPLES_PER_PACKET,
            "granule is 50 packets of 960 samples"
        );
    }

    #[test]
    fn a_cut_file_keeps_the_packets_of_earlier_pages() {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        for index in 0..120 {
            writer.write_packet(&sample_packet(index)).unwrap();
        }
        writer.finish().unwrap();
        let mut bytes = writer.inner.into_inner();
        // Cut inside the last page.
        bytes.truncate(bytes.len() - 10);
        let stream = read_stream(&mut Cursor::new(bytes)).unwrap();
        assert!(
            stream.packets.len() >= 50 && stream.packets.len() < 120,
            "keeps whole pages only, got {}",
            stream.packets.len()
        );
        for (index, packet) in stream.packets.iter().enumerate() {
            assert_eq!(packet, &sample_packet(index));
        }
    }

    #[test]
    fn a_packet_too_big_for_a_page_is_refused_and_big_packets_still_round_trip() {
        let mut writer = OggOpusWriter::new(Vec::new(), OpusHead::default(), 7).unwrap();
        assert!(writer.write_packet(&vec![1_u8; 255 * 255]).is_err());
        // 50 packets of 2000 bytes need 400 segments: more than one page.
        let packets: Vec<Vec<u8>> = (0..50).map(|n| vec![n as u8; 2000]).collect();
        for packet in &packets {
            writer.write_packet(packet).unwrap();
        }
        writer.finish().unwrap();
        let bytes = writer.inner.clone();
        let stream = read_stream(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(stream.packets, packets);
    }

    #[test]
    fn a_changed_byte_fails_that_page() {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        for index in 0..60 {
            writer.write_packet(&sample_packet(index)).unwrap();
        }
        writer.finish().unwrap();
        let mut bytes = writer.inner.into_inner();
        // Flip a body byte of the audio page (after the first two headers).
        let at = bytes.len() / 2;
        bytes[at] ^= 0xFF;
        let stream = read_stream(&mut Cursor::new(bytes)).unwrap();
        assert!(
            stream.packets.len() < 60,
            "the damaged page contributes no packets"
        );
    }

    #[test]
    fn duration_of_a_327_file_comes_from_its_end() {
        // 3:27 = 207 s of 20 ms packets.
        let seconds = 207_u64;
        let count = seconds as usize * PACKETS_PER_SECOND;
        let mut writer = OggOpusWriter::new(
            Cursor::new(Vec::new()),
            OpusHead {
                pre_skip: 0,
                ..OpusHead::default()
            },
            7,
        )
        .unwrap();
        for index in 0..count {
            writer.write_packet(&sample_packet(index % 16)).unwrap();
        }
        writer.finish().unwrap();
        let bytes = writer.inner.into_inner();
        let mut cursor = Cursor::new(bytes);
        let ms = duration_ms(&mut cursor, 0).unwrap().unwrap();
        assert_eq!(ms, 207_000);
    }

    /// Build a stream of `count` 20 ms packets.
    fn stream_of(count: usize) -> OggOpusStream {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        for index in 0..count {
            writer.write_packet(&sample_packet(index % 16)).unwrap();
        }
        writer.finish().unwrap();
        read_stream(&mut Cursor::new(writer.inner.into_inner())).unwrap()
    }

    /// The header type of the last page of an Ogg file.
    fn last_page_flags(bytes: &[u8]) -> u8 {
        let mut flags = 0;
        let mut index = 0;
        while index + 27 <= bytes.len() {
            if &bytes[index..index + 4] == b"OggS" {
                flags = bytes[index + 5];
                let segments = bytes[index + 26] as usize;
                let body: usize = bytes[index + 27..index + 27 + segments]
                    .iter()
                    .map(|&value| usize::from(value))
                    .sum();
                index += 27 + segments + body;
            } else {
                index += 1;
            }
        }
        flags
    }

    #[test]
    fn split_parts_makes_equal_complete_files_that_read_back_in_order() {
        // 30 001 packets at 600 s (30 000 packets) a part: two balanced
        // parts, not one full and a sliver.
        let stream = stream_of(30_001);
        let parts = split_parts(&stream, 600).unwrap();
        assert_eq!(parts.len(), 2);
        let mut joined = Vec::new();
        let mut counts = Vec::new();
        for part in &parts {
            let mut bytes = Cursor::new(part.as_slice());
            let head = read_stream(&mut bytes).unwrap();
            assert_eq!(head.head, stream.head, "the part keeps the stream's head");
            assert!(head.packets.len() <= 600 * PACKETS_PER_SECOND);
            counts.push(head.packets.len());
            joined.extend(head.packets);
            // The first page is the head (BOS); the last is end of stream.
            assert_eq!(part[5], FLAG_BOS);
            assert_eq!(last_page_flags(part), FLAG_EOS);
            let mut cursor = Cursor::new(part.as_slice());
            let ms = duration_ms(&mut cursor, stream.head.pre_skip)
                .unwrap()
                .expect("a part has a duration");
            assert!(ms < 600 * 1_000, "part is under the limit: {ms} ms");
        }
        assert_eq!(joined, stream.packets, "every packet, in order");
        // Balanced: 15 001 and 15 000, not 30 000 and 1.
        assert_eq!(counts, vec![15_001, 15_000]);
    }

    #[test]
    fn exactly_the_limit_gives_one_part_and_an_empty_stream_none() {
        let one = split_parts(&stream_of(30_000), 600).unwrap();
        assert_eq!(one.len(), 1);
        let mut bytes = Cursor::new(one[0].as_slice());
        assert_eq!(read_stream(&mut bytes).unwrap().packets.len(), 30_000);

        let empty = OggOpusStream {
            head: OpusHead::default(),
            packets: Vec::new(),
        };
        assert!(split_parts(&empty, 600).unwrap().is_empty());
    }

    #[test]
    fn a_packet_too_big_for_a_page_is_an_error() {
        // A damaged file can hold a packet over 255 segments; the device
        // must not panic on it.
        let stream = OggOpusStream {
            head: OpusHead::default(),
            packets: vec![sample_packet(0), vec![0x55; 255 * 255]],
        };
        assert!(split_parts(&stream, 600).is_err());
    }

    /// A `Write` that stores each part in a shared list, so the test can
    /// compare the bytes after `split_file` drops its writers.
    struct SlotWriter {
        parts: Rc<RefCell<Vec<Vec<u8>>>>,
        index: usize,
    }

    impl Write for SlotWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.parts.borrow_mut()[self.index].extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Run `split_file` over `bytes`, collecting each part.
    fn split_file_collect(bytes: &[u8], max_seconds: u32) -> Vec<Vec<u8>> {
        let parts = Rc::new(RefCell::new(Vec::new()));
        let mut cursor = Cursor::new(bytes);
        let opened = split_file(&mut cursor, max_seconds, |index| {
            parts.borrow_mut().push(Vec::new());
            Ok(SlotWriter {
                parts: parts.clone(),
                index,
            })
        })
        .unwrap();
        let collected = parts.borrow().clone();
        assert_eq!(opened, collected.len());
        collected
    }

    /// Bytes of a stream of `count` packets of `packet_len`.
    fn file_of(count: usize, packet_len: usize) -> Vec<u8> {
        let mut writer =
            OggOpusWriter::new(Cursor::new(Vec::new()), OpusHead::default(), 7).unwrap();
        for index in 0..count {
            writer
                .write_packet(&vec![0xA0 | (index as u8 & 0x0F); packet_len])
                .unwrap();
        }
        writer.finish().unwrap();
        writer.inner.into_inner()
    }

    #[test]
    fn split_file_matches_split_parts_byte_for_byte() {
        for count in [1_usize, 29_999, 30_000, 30_001, 60_001] {
            let bytes = file_of(count, 40);
            let stream = read_stream(&mut Cursor::new(bytes.clone())).unwrap();
            let expected = split_parts(&stream, 600).unwrap();
            let got = split_file_collect(&bytes, 600);
            assert_eq!(got.len(), expected.len(), "count {count}");
            for (index, (a, b)) in got.iter().zip(&expected).enumerate() {
                assert_eq!(a, b, "count {count} part {index}");
            }
        }
    }

    #[test]
    fn packets_that_cross_a_page_split_the_same_way() {
        // 2000-byte packets need many segments each; the writer puts a few
        // per page, so packets continue across the page boundary.
        let bytes = file_of(120, 2000);
        let stream = read_stream(&mut Cursor::new(bytes.clone())).unwrap();
        assert_eq!(stream.packets.len(), 120);
        let expected = split_parts(&stream, 2).unwrap();
        let got = split_file_collect(&bytes, 2);
        assert_eq!(got, expected);
    }

    #[test]
    fn a_cut_file_gives_the_parts_of_the_complete_packets_before_the_cut() {
        let bytes = file_of(120, 40);
        let mut cut = bytes.clone();
        cut.truncate(cut.len() - 10);
        let stream = read_stream(&mut Cursor::new(cut.clone())).unwrap();
        let expected = split_parts(&stream, 2).unwrap();
        let got = split_file_collect(&cut, 2);
        assert_eq!(got, expected);
        assert!(!got.is_empty());
    }

    #[test]
    fn an_empty_stream_calls_make_part_zero_times() {
        let empty = file_of(0, 40);
        let mut calls = 0;
        let mut cursor = Cursor::new(empty);
        let opened = split_file(&mut cursor, 600, |_| {
            calls += 1;
            Ok(Vec::new())
        })
        .unwrap();
        assert_eq!(opened, 0);
        assert_eq!(calls, 0);
    }

    #[test]
    fn an_error_from_make_part_or_a_writer_is_returned() {
        let bytes = file_of(10, 40);
        let mut cursor = Cursor::new(bytes.clone());
        let error = split_file(&mut cursor, 600, |_| {
            Err::<Vec<u8>, _>(std::io::Error::new(std::io::ErrorKind::Other, "no file"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);

        struct Fails;
        impl Write for Fails {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut cursor = Cursor::new(bytes);
        let error = split_file(&mut cursor, 600, |_| Ok(Fails)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn a_packet_continued_past_one_page_is_an_error_not_a_huge_buffer() {
        // Pages that only ever continue (every lace 255) build a packet
        // with no end; reading stops once it passes what one page holds.
        let mut bytes = file_of(0, 40);
        let body = vec![0x55; 255 * 255];
        for sequence in 2..6 {
            let flags = if sequence == 2 { 0 } else { FLAG_CONTINUED };
            bytes.extend(build_page(flags, 0, 7, sequence, &body, &[255; 255]));
        }
        let error = count_packets(&mut Cursor::new(bytes.clone())).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(read_stream(&mut Cursor::new(bytes)).is_err());
    }

    #[test]
    fn count_packets_matches_read_stream_and_stops_at_a_cut() {
        let bytes = file_of(120, 40);
        let mut cursor = Cursor::new(bytes.clone());
        let (head, count) = count_packets(&mut cursor).unwrap();
        let stream = read_stream(&mut Cursor::new(bytes.clone())).unwrap();
        assert_eq!(head, stream.head);
        assert_eq!(count, stream.packets.len());

        let mut cut = bytes;
        cut.truncate(cut.len() - 10);
        let (_, cut_count) = count_packets(&mut Cursor::new(cut.clone())).unwrap();
        assert_eq!(
            cut_count,
            read_stream(&mut Cursor::new(cut)).unwrap().packets.len()
        );
    }
}
