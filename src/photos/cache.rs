//! Prepared photos on the SD card: a thumbnail and two ready-to-show frames,
//! so the gallery and the sleep screen never decode a JPEG twice.

use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use super::image::{PhotoFit, Thumbnail, THUMB_HEIGHT, THUMB_ROW_BYTES, THUMB_WIDTH};
use crate::framebuffer::{FrameBuffer, FRAMEBUFFER_SIZE};

const MAGIC: &[u8; 4] = b"WPC1";
const HEADER_BYTES: usize = 12;
const THUMB_BYTES: usize = THUMB_ROW_BYTES * THUMB_HEIGHT;
/// Size of a complete cache file.
pub const CACHE_FILE_BYTES: usize = HEADER_BYTES + THUMB_BYTES + 2 * FRAMEBUFFER_SIZE;

/// What the gallery and the sleep screen need from one photo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedPhoto {
    /// Upright size of the original photo.
    pub size: (u16, u16),
    pub thumbnail: Thumbnail,
    pub fill: FrameBuffer,
    pub whole: FrameBuffer,
}

/// Cache file of the photo with `key`.
#[must_use]
pub fn cache_file(directory: &Path, key: u32) -> PathBuf {
    directory.join(format!("{key:08X}.PIC"))
}

/// Key of a cache file name such as `1A2B3C4D.PIC`.
#[must_use]
pub fn key_from_file_name(name: &str) -> Option<u32> {
    let stem = name
        .strip_suffix(".PIC")
        .or_else(|| name.strip_suffix(".pic"))?;
    if stem.len() == 8 {
        u32::from_str_radix(stem, 16).ok()
    } else {
        None
    }
}

/// Write through a temporary file so a power cut never leaves half a cache
/// file behind.
pub fn write(path: &Path, photo: &PreparedPhoto) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(CACHE_FILE_BYTES);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&photo.size.0.to_le_bytes());
    bytes.extend_from_slice(&photo.size.1.to_le_bytes());
    bytes.extend_from_slice(&(THUMB_WIDTH as u16).to_le_bytes());
    bytes.extend_from_slice(&(THUMB_HEIGHT as u16).to_le_bytes());
    bytes.extend_from_slice(&photo.thumbnail.bits);
    bytes.extend_from_slice(photo.fill.as_bytes());
    bytes.extend_from_slice(photo.whole.as_bytes());
    let partial = path.with_extension("TMP");
    fs::write(&partial, &bytes)?;
    // FAT cannot rename over an existing file.
    let _ = fs::remove_file(path);
    fs::rename(&partial, path)
}

/// Original size and thumbnail stored in a cache file.
pub fn read_thumbnail(path: &Path) -> io::Result<((u16, u16), Thumbnail)> {
    let mut head = vec![0_u8; HEADER_BYTES + THUMB_BYTES];
    File::open(path)?.read_exact(&mut head)?;
    let size = check_header(&head)?;
    let bits = head.split_off(HEADER_BYTES);
    Ok((size, Thumbnail { bits }))
}

/// One of the two screen frames stored in a cache file.
pub fn read_frame(path: &Path, fit: PhotoFit) -> io::Result<FrameBuffer> {
    let mut file = File::open(path)?;
    let mut head = [0_u8; HEADER_BYTES];
    file.read_exact(&mut head)?;
    check_header(&head)?;
    let skip = match fit {
        PhotoFit::Fill => 0,
        PhotoFit::Whole => FRAMEBUFFER_SIZE,
    };
    file.seek(SeekFrom::Start((HEADER_BYTES + THUMB_BYTES + skip) as u64))?;
    let mut frame = vec![0_u8; FRAMEBUFFER_SIZE];
    file.read_exact(&mut frame)?;
    FrameBuffer::from_native_bytes(frame).map_err(invalid)
}

/// The original size, if `head` starts a cache file of this version.
fn check_header(head: &[u8]) -> io::Result<(u16, u16)> {
    let word = |index: usize| u16::from_le_bytes([head[4 + index * 2], head[5 + index * 2]]);
    if &head[..4] != MAGIC || (word(2), word(3)) != (THUMB_WIDTH as u16, THUMB_HEIGHT as u16) {
        return Err(invalid("not a photo cache file of this version"));
    }
    Ok((word(0), word(1)))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        cache_file, key_from_file_name, read_frame, read_thumbnail, write, PreparedPhoto,
        CACHE_FILE_BYTES,
    };
    use crate::{
        framebuffer::FrameBuffer,
        photos::image::{PhotoFit, Thumbnail},
    };

    #[test]
    fn cache_files_round_trip() {
        let root = std::env::temp_dir().join(format!("wave-photo-cache-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = cache_file(&root, 0x1A2B_3C4D);
        assert!(path.ends_with("1A2B3C4D.PIC"));
        assert_eq!(key_from_file_name("1A2B3C4D.PIC"), Some(0x1A2B_3C4D));
        assert_eq!(key_from_file_name("NOTES.TXT"), None);

        let mut fill = vec![0xFF_u8; 48_000];
        fill[0] = 0x0F;
        let photo = PreparedPhoto {
            size: (4032, 3024),
            thumbnail: Thumbnail {
                bits: vec![0xAA; 18 * 216],
            },
            fill: FrameBuffer::from_native_bytes(fill).unwrap(),
            whole: FrameBuffer::new_white(),
        };
        write(&path, &photo).unwrap();
        write(&path, &photo).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), CACHE_FILE_BYTES as u64);
        let (size, thumbnail) = read_thumbnail(&path).unwrap();
        assert_eq!(size, (4032, 3024));
        assert_eq!(thumbnail, photo.thumbnail);
        assert_eq!(read_frame(&path, PhotoFit::Fill).unwrap(), photo.fill);
        assert_eq!(read_frame(&path, PhotoFit::Whole).unwrap(), photo.whole);

        fs::write(&path, b"WPC0 old").unwrap();
        assert!(read_thumbnail(&path).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
