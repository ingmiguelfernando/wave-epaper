//! Character set shared by every bitmap font: printable ASCII, Latin-1 and a
//! few typographic extras. `scripts/fonts/generate.py` emits glyphs in this
//! order, so Spanish text (á, ñ, ¿, ¡, «», —) renders without substitutes.

/// Glyphs in one complete font atlas.
pub const GLYPH_COUNT: usize = 95 + 96 + EXTRAS.len();

const EXTRAS: [char; 15] = [
    '–', '—', '‘', '’', '‚', '“', '”', '„', '•', '…', '‹', '›', '€', '™', '−',
];

/// Atlas index of `character`, or `None` when the fonts cannot draw it.
#[must_use]
pub fn glyph_index(character: char) -> Option<usize> {
    let code = character as usize;
    match code {
        0x20..=0x7E => Some(code - 0x20),
        0xA0..=0xFF => Some(code - 0xA0 + 95),
        _ => EXTRAS
            .iter()
            .position(|&extra| extra == character)
            .map(|index| index + 191),
    }
}

#[cfg(test)]
mod tests {
    use super::{glyph_index, GLYPH_COUNT};

    #[test]
    fn indexes_ascii_latin1_and_extras() {
        assert_eq!(glyph_index(' '), Some(0));
        assert_eq!(glyph_index('~'), Some(94));
        assert_eq!(glyph_index('\u{a0}'), Some(95));
        assert_eq!(glyph_index('ñ'), Some(95 + 0xF1 - 0xA0));
        assert_eq!(glyph_index('−'), Some(GLYPH_COUNT - 1));
        assert_eq!(glyph_index('\u{7f}'), None);
        assert_eq!(glyph_index('ā'), None);
    }
}
