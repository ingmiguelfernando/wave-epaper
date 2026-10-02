//! Hyphenation points for Reader line breaking.
//!
//! Spanish and English use the TeX hyphenation patterns embedded by the
//! `hypher` crate (see `docs/licenses/HYPHENATION_NOTICES.md`). Books in other
//! languages only break at spaces and existing hyphens.

use hypher::Lang;

/// Frequent Spanish words that are not English words.
const SPANISH_WORDS: [&str; 20] = [
    "de", "la", "que", "el", "los", "las", "del", "por", "con", "una", "para", "como", "pero",
    "más", "su", "se", "y", "es", "lo", "al",
];
/// Frequent English words that are not Spanish words.
const ENGLISH_WORDS: [&str; 20] = [
    "the", "and", "of", "to", "is", "that", "it", "was", "for", "with", "his", "as", "on", "be",
    "at", "by", "you", "had", "her", "which",
];
/// Fewest recognised words needed before guessing a language.
const DETECTION_MIN_WORDS: usize = 8;

/// A language whose hyphenation patterns are built in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    English,
    Spanish,
}

impl Language {
    /// Language named by a BCP 47 tag such as `es`, `es-MX` or `en-GB`.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_']).next().unwrap_or("");
        match primary.to_ascii_lowercase().as_str() {
            "es" | "spa" => Some(Self::Spanish),
            "en" | "eng" => Some(Self::English),
            _ => None,
        }
    }

    /// Guess the language of a text sample by counting frequent short words.
    /// `None` when the sample is too short or neither language dominates.
    #[must_use]
    pub fn detect(sample: &str) -> Option<Self> {
        let mut spanish = 0;
        let mut english = 0;
        for word in sample.split(|character: char| !character.is_alphabetic()) {
            if word.is_empty() || word.len() > 5 {
                continue;
            }
            let word = word.to_lowercase();
            if SPANISH_WORDS.contains(&word.as_str()) {
                spanish += 1;
            } else if ENGLISH_WORDS.contains(&word.as_str()) {
                english += 1;
            }
        }
        if spanish + english < DETECTION_MIN_WORDS {
            None
        } else if spanish >= 2 * english {
            Some(Self::Spanish)
        } else if english >= 2 * spanish {
            Some(Self::English)
        } else {
            None
        }
    }

    const fn patterns(self) -> Lang {
        match self {
            Self::English => Lang::English,
            Self::Spanish => Lang::Spanish,
        }
    }
}

/// Byte offsets inside `word` where it may break with an added hyphen, in
/// ascending order. Only the alphabetic core is hyphenated, so leading and
/// trailing punctuation stays with its fragment. Words with inner digits or
/// punctuation and all-capital words (acronyms) are never hyphenated.
#[must_use]
pub fn break_offsets(word: &str, language: Language) -> Vec<usize> {
    let Some(start) = word.find(char::is_alphabetic) else {
        return Vec::new();
    };
    let Some((last, character)) = word
        .char_indices()
        .rev()
        .find(|(_, character)| character.is_alphabetic())
    else {
        return Vec::new();
    };
    let core = &word[start..last + character.len_utf8()];
    if !core.chars().all(char::is_alphabetic) || !core.chars().any(char::is_lowercase) {
        return Vec::new();
    }
    let mut offsets = Vec::new();
    let mut offset = start;
    for syllable in hypher::hyphenate(core, language.patterns()) {
        offset += syllable.len();
        offsets.push(offset);
    }
    // The last syllable ends the word; it is not a break.
    offsets.pop();
    offsets
}

#[cfg(test)]
mod tests {
    use super::{break_offsets, Language};

    #[test]
    fn maps_language_tags() {
        assert_eq!(Language::from_tag("es"), Some(Language::Spanish));
        assert_eq!(Language::from_tag(" es-MX "), Some(Language::Spanish));
        assert_eq!(Language::from_tag("EN_gb"), Some(Language::English));
        assert_eq!(Language::from_tag("fr"), None);
        assert_eq!(Language::from_tag(""), None);
    }

    #[test]
    fn detects_spanish_and_english_samples() {
        let spanish = "En el principio creó Dios los cielos y la tierra. Y la tierra \
                       estaba desordenada y vacía, y las tinieblas estaban sobre la faz \
                       del abismo.";
        let english = "In the beginning God created the heaven and the earth. And the \
                       earth was without form, and void; and darkness was upon the \
                       face of the deep.";
        assert_eq!(Language::detect(spanish), Some(Language::Spanish));
        assert_eq!(Language::detect(english), Some(Language::English));
        assert_eq!(Language::detect("1234 5678"), None);
    }

    #[test]
    fn breaks_words_at_syllables_inside_their_punctuation() {
        assert_eq!(break_offsets("extensive", Language::English), [2, 5]);
        assert_eq!(break_offsets("palabra", Language::Spanish), [2, 4]);
        // "«" is two bytes, so every break moves by two.
        assert_eq!(break_offsets("«palabra»,", Language::Spanish), [4, 6]);
        let word = "extraordinariamente";
        let offsets = break_offsets(word, Language::Spanish);
        assert!(offsets.len() >= 4, "{offsets:?}");
        assert!(offsets.iter().all(|offset| word.is_char_boundary(*offset)));
    }

    #[test]
    fn leaves_acronyms_numbers_and_compounds_whole() {
        assert!(break_offsets("UNESCO", Language::Spanish).is_empty());
        assert!(break_offsets("1990", Language::English).is_empty());
        assert!(break_offsets("well-known", Language::English).is_empty());
        assert!(break_offsets("a", Language::English).is_empty());
    }
}
