//! EXIF orientation: how a camera picture must turn to stand upright.

/// The eight EXIF orientations. Mirrored ones are rare but cheap to honour.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    #[default]
    Upright,
    MirrorHorizontal,
    Rotate180,
    MirrorVertical,
    Transpose,
    Rotate90,
    Transverse,
    Rotate270,
}

impl Orientation {
    /// Orientation from EXIF data that starts at the TIFF header, as returned
    /// by `jpeg_decoder::Decoder::exif_data`.
    #[must_use]
    pub fn from_exif(exif: &[u8]) -> Self {
        match orientation_tag(exif) {
            Some(2) => Self::MirrorHorizontal,
            Some(3) => Self::Rotate180,
            Some(4) => Self::MirrorVertical,
            Some(5) => Self::Transpose,
            Some(6) => Self::Rotate90,
            Some(7) => Self::Transverse,
            Some(8) => Self::Rotate270,
            _ => Self::Upright,
        }
    }

    const fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::Transpose | Self::Rotate90 | Self::Transverse | Self::Rotate270
        )
    }

    /// Size once upright, for a stored `width` × `height` picture.
    #[must_use]
    pub const fn upright_size(self, width: usize, height: usize) -> (usize, usize) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// Stored pixel shown at upright position (`u`, `v`).
    #[must_use]
    pub const fn source(self, u: usize, v: usize, width: usize, height: usize) -> (usize, usize) {
        match self {
            Self::Upright => (u, v),
            Self::MirrorHorizontal => (width - 1 - u, v),
            Self::Rotate180 => (width - 1 - u, height - 1 - v),
            Self::MirrorVertical => (u, height - 1 - v),
            Self::Transpose => (v, u),
            Self::Rotate90 => (v, height - 1 - u),
            Self::Transverse => (width - 1 - v, height - 1 - u),
            Self::Rotate270 => (width - 1 - v, u),
        }
    }
}

/// Value of tag 0x0112 in the first image directory, if present.
fn orientation_tag(exif: &[u8]) -> Option<u16> {
    let little_endian = match exif.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |offset: usize| -> Option<u16> {
        let bytes = [*exif.get(offset)?, *exif.get(offset + 1)?];
        Some(if little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    };
    let u32_at = |offset: usize| -> Option<usize> {
        let first = u32::from(u16_at(offset)?);
        let second = u32::from(u16_at(offset + 2)?);
        let value = if little_endian {
            second << 16 | first
        } else {
            first << 16 | second
        };
        usize::try_from(value).ok()
    };
    if u16_at(2)? != 42 {
        return None;
    }
    let directory = u32_at(4)?;
    let entries = usize::from(u16_at(directory)?);
    (0..entries).find_map(|index| {
        let entry = directory + 2 + index * 12;
        if u16_at(entry)? == 0x0112 {
            u16_at(entry + 8)
        } else {
            None
        }
    })
}

/// A TIFF header and one directory holding only the orientation tag.
#[cfg(test)]
pub(crate) fn test_exif(orientation: u16, little_endian: bool) -> Vec<u8> {
    let u16_bytes = |value: u16| {
        if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    let u32_bytes = |value: u32| {
        if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    let mut exif = Vec::new();
    exif.extend_from_slice(if little_endian { b"II" } else { b"MM" });
    exif.extend_from_slice(&u16_bytes(42));
    exif.extend_from_slice(&u32_bytes(8));
    exif.extend_from_slice(&u16_bytes(1));
    exif.extend_from_slice(&u16_bytes(0x0112));
    exif.extend_from_slice(&u16_bytes(3));
    exif.extend_from_slice(&u32_bytes(1));
    // A SHORT value fills the first half of its four-byte slot.
    exif.extend_from_slice(&u16_bytes(orientation));
    exif.extend_from_slice(&[0, 0]);
    exif
}

#[cfg(test)]
mod tests {
    use super::{test_exif, Orientation};

    #[test]
    fn reads_the_orientation_tag_in_both_byte_orders() {
        let rotated = test_exif(6, true);
        assert_eq!(Orientation::from_exif(&rotated), Orientation::Rotate90);
        let flipped = test_exif(3, false);
        assert_eq!(Orientation::from_exif(&flipped), Orientation::Rotate180);
        assert_eq!(Orientation::from_exif(b"junk"), Orientation::Upright);
        assert_eq!(Orientation::from_exif(&rotated[..12]), Orientation::Upright);
    }

    #[test]
    fn rotations_map_upright_pixels_back_to_stored_ones() {
        // A 4 × 2 stored picture shown upright as 2 × 4 after a quarter turn.
        let turned = Orientation::Rotate90;
        assert_eq!(turned.upright_size(4, 2), (2, 4));
        // The stored bottom-left pixel ends up top-left once turned clockwise.
        assert_eq!(turned.source(0, 0, 4, 2), (0, 1));
        assert_eq!(turned.source(1, 3, 4, 2), (3, 0));
        assert_eq!(Orientation::Rotate270.source(0, 0, 4, 2), (3, 0));
        assert_eq!(Orientation::Rotate180.source(0, 0, 4, 2), (3, 1));
    }
}
