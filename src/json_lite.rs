//! A small JSON reader for the AI endpoints. Numbers stay text; nothing
//! parses floats, because the Xtensa backend cannot reliably build serde's
//! float visitor.

use core::fmt;

/// Maximum accepted input, in bytes.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Maximum nesting depth of arrays and objects.
const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// Digits kept as written, never converted to a float.
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

/// A parse failure at a byte offset into the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonError {
    pub offset: usize,
    pub kind: JsonErrorKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonErrorKind {
    UnexpectedEnd,
    UnexpectedCharacter,
    BadEscape,
    LoneSurrogate,
    MissingColon,
    MissingComma,
    TrailingCharacters,
    TooLarge,
    TooDeep,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "json error at byte {}: ", self.offset)?;
        match self.kind {
            JsonErrorKind::UnexpectedEnd => formatter.write_str("unexpected end of input"),
            JsonErrorKind::UnexpectedCharacter => formatter.write_str("unexpected character"),
            JsonErrorKind::BadEscape => formatter.write_str("bad escape"),
            JsonErrorKind::LoneSurrogate => formatter.write_str("lone surrogate"),
            JsonErrorKind::MissingColon => formatter.write_str("missing ':'"),
            JsonErrorKind::MissingComma => formatter.write_str("missing ',' or '}'"),
            JsonErrorKind::TrailingCharacters => {
                formatter.write_str("trailing characters after the value")
            }
            JsonErrorKind::TooLarge => formatter.write_str("input too large"),
            JsonErrorKind::TooDeep => formatter.write_str("nesting too deep"),
        }
    }
}

impl std::error::Error for JsonError {}

/// Byte length of one UTF-8 sequence from its leading byte.
const fn utf8_width(leading: u8) -> usize {
    match leading {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// Parse one complete JSON document.
pub fn parse(text: &str) -> Result<JsonValue, JsonError> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(JsonError {
            offset: MAX_INPUT_BYTES,
            kind: JsonErrorKind::TooLarge,
        });
    }
    let bytes = text.as_bytes();
    let mut parser = Parser { bytes, position: 0 };
    parser.skip_space();
    let value = parser.value(0)?;
    parser.skip_space();
    if parser.position != bytes.len() {
        return Err(parser.error(JsonErrorKind::TrailingCharacters));
    }
    Ok(value)
}

/// Escape `text` for embedding in a request string: quotes, backslash and
/// control characters escaped, other Unicode kept as-is.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn error(&self, kind: JsonErrorKind) -> JsonError {
        JsonError {
            offset: self.position,
            kind,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, byte: u8, kind: JsonErrorKind) -> Result<(), JsonError> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(kind))
        }
    }

    fn literal(&mut self, word: &str) -> bool {
        if self.bytes[self.position..].starts_with(word.as_bytes()) {
            self.position += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error(JsonErrorKind::TooDeep));
        }
        self.skip_space();
        match self.peek() {
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(JsonValue::String(self.string()?)),
            Some(b't') => self.keyword("true", JsonValue::Bool(true)),
            Some(b'f') => self.keyword("false", JsonValue::Bool(false)),
            Some(b'n') => self.keyword("null", JsonValue::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error(JsonErrorKind::UnexpectedCharacter)),
        }
    }

    fn keyword(&mut self, word: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.literal(word) {
            Ok(value)
        } else {
            Err(self.error(JsonErrorKind::UnexpectedCharacter))
        }
    }

    fn number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        let digits_start = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
        if self.position == digits_start {
            return Err(self.error(JsonErrorKind::UnexpectedCharacter));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            // Fractions and exponents stay verbatim text.
            while matches!(
                self.peek(),
                Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
            ) {
                self.position += 1;
            }
        }
        Ok(JsonValue::Number(
            String::from_utf8_lossy(&self.bytes[start..self.position]).into_owned(),
        ))
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"', JsonErrorKind::UnexpectedCharacter)?;
        let mut out = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error(JsonErrorKind::UnexpectedEnd));
            };
            self.position += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(escape) = self.peek() else {
                        return Err(self.error(JsonErrorKind::UnexpectedEnd));
                    };
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return Err(self.error(JsonErrorKind::BadEscape)),
                    }
                }
                other => {
                    // Copy one full UTF-8 sequence: the length comes from the
                    // leading byte, so accents and emoji stay intact.
                    if other < 0x80 {
                        out.push(other as char);
                    } else {
                        let width = utf8_width(other);
                        let end = (self.position - 1 + width).min(self.bytes.len());
                        let text = String::from_utf8_lossy(&self.bytes[self.position - 1..end]);
                        out.push_str(&text);
                        self.position = end;
                    }
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, JsonError> {
        let Some(hex) = self.bytes.get(self.position..self.position + 4) else {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        };
        self.position += 4;
        let text = core::str::from_utf8(hex).map_err(|_| self.error(JsonErrorKind::BadEscape))?;
        u16::from_str_radix(text, 16).map_err(|_| self.error(JsonErrorKind::BadEscape))
    }

    /// One `\uXXXX`, joining surrogate pairs; a lone surrogate is an error.
    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let first = self.hex4()?;
        match first {
            0xD800..=0xDBFF => {
                if self.bytes.get(self.position) == Some(&b'\\')
                    && self.bytes.get(self.position + 1) == Some(&b'u')
                {
                    self.position += 2;
                } else {
                    return Err(self.error(JsonErrorKind::LoneSurrogate));
                }
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.error(JsonErrorKind::LoneSurrogate));
                }
                let combined =
                    0x10000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00);
                char::from_u32(combined).ok_or_else(|| self.error(JsonErrorKind::BadEscape))
            }
            0xDC00..=0xDFFF => Err(self.error(JsonErrorKind::LoneSurrogate)),
            plain => {
                char::from_u32(u32::from(plain)).ok_or_else(|| self.error(JsonErrorKind::BadEscape))
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect(b'[', JsonErrorKind::UnexpectedCharacter)?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(JsonValue::Array(items));
                }
                None => return Err(self.error(JsonErrorKind::UnexpectedEnd)),
                Some(_) => return Err(self.error(JsonErrorKind::MissingComma)),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect(b'{', JsonErrorKind::UnexpectedCharacter)?;
        let mut entries = Vec::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.expect(b':', JsonErrorKind::MissingColon)?;
            let value = self.value(depth + 1)?;
            entries.push((key, value));
            self.skip_space();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(JsonValue::Object(entries));
                }
                None => return Err(self.error(JsonErrorKind::UnexpectedEnd)),
                Some(_) => return Err(self.error(JsonErrorKind::MissingComma)),
            }
        }
    }
}

impl JsonValue {
    /// The first entry under `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// The entry at `index`.
    #[must_use]
    pub fn index(&self, index: usize) -> Option<&JsonValue> {
        match self {
            Self::Array(items) => items.get(index),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Parse the stored number as an integer on demand; fractions fail.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(text) => text.parse().ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{escape, parse, JsonErrorKind, JsonValue, MAX_INPUT_BYTES};

    #[test]
    fn parses_every_value_type() {
        let source = r#"{"n":null,"yes":true,"no":false,"count":-42,"big":1.5e3,"name":"Ana","list":[1,[2,3],{}]}"#;
        let value = parse(source).unwrap();
        assert_eq!(value.get("n"), Some(&JsonValue::Null));
        assert_eq!(value.get("yes").and_then(JsonValue::as_bool), Some(true));
        assert_eq!(value.get("no").and_then(JsonValue::as_bool), Some(false));
        assert_eq!(value.get("count").and_then(JsonValue::as_i64), Some(-42));
        // Fractions stay text and never pretend to be integers.
        assert_eq!(value.get("big").and_then(JsonValue::as_i64), None);
        assert_eq!(value.get("big"), Some(&JsonValue::Number("1.5e3".into())));
        assert_eq!(value.get("name").and_then(JsonValue::as_str), Some("Ana"));
        let list = value.get("list").unwrap();
        assert_eq!(list.index(0).and_then(JsonValue::as_i64), Some(1));
        let inner = list.index(1).unwrap();
        assert_eq!(inner.index(1).and_then(JsonValue::as_i64), Some(3));
        assert!(matches!(list.index(2), Some(JsonValue::Object(entries)) if entries.is_empty()));
    }

    #[test]
    fn parses_spanish_text_and_emoji_escaped_both_ways() {
        let value = parse(r#""¿Dónde está el niño? \ud83d\ude00 ñandú""#).unwrap();
        assert_eq!(value.as_str(), Some("¿Dónde está el niño? \u{1F600} ñandú"));
        let escaped = escape("¿Dónde está el niño? \u{1F600} \"quoted\" \\ back");
        assert_eq!(
            escaped,
            "¿Dónde está el niño? \u{1F600} \\\"quoted\\\" \\\\ back"
        );
        let round = parse(&format!("\"{escaped}\"")).unwrap();
        assert_eq!(
            round.as_str(),
            Some("¿Dónde está el niño? \u{1F600} \"quoted\" \\ back")
        );
    }

    #[test]
    fn control_characters_escape_as_unicode() {
        assert_eq!(escape("a\u{07}b\u{1F}c"), "a\\u0007b\\u001fc");
        assert_eq!(escape("tab\there"), "tab\\there");
    }

    #[test]
    fn nested_structures_reach_the_depth_limit() {
        let ok = format!("{}{}", "[".repeat(32), "]".repeat(32));
        assert!(parse(&ok).is_ok());
        let too_deep = format!("{}x{}", "[".repeat(33), "]".repeat(33));
        let error = parse(&too_deep).unwrap_err();
        assert_eq!(error.kind, JsonErrorKind::TooDeep);
    }

    #[test]
    fn oversized_input_is_rejected() {
        let text = format!("\"{}\"", "x".repeat(MAX_INPUT_BYTES));
        let error = parse(&text).unwrap_err();
        assert_eq!(error.kind, JsonErrorKind::TooLarge);
    }

    #[test]
    fn malformed_input_names_the_byte_offset() {
        for (text, kind) in [
            // A value after the comma: the parser wanted a key string.
            (r#"{"a":1,}"#, JsonErrorKind::UnexpectedCharacter),
            (r#""open"#, JsonErrorKind::UnexpectedEnd),
            (r#""bad \x escape""#, JsonErrorKind::BadEscape),
            (r#""lone \ud83d""#, JsonErrorKind::LoneSurrogate),
            (r#"{"a" 1}"#, JsonErrorKind::MissingColon),
            (r#"[1 2]"#, JsonErrorKind::MissingComma),
            ("1 2", JsonErrorKind::TrailingCharacters),
            ("tru", JsonErrorKind::UnexpectedCharacter),
            ("@#$", JsonErrorKind::UnexpectedCharacter),
        ] {
            let error = parse(text).unwrap_err();
            assert_eq!(error.kind, kind, "{text}");
            assert!(error.offset <= text.len());
        }
    }

    #[test]
    fn unicode_escapes_include_short_and_bmp_forms() {
        assert_eq!(parse(r#""\u00f1""#).unwrap().as_str(), Some("ñ"));
        assert_eq!(parse(r#""\u00e1""#).unwrap().as_str(), Some("á"));
        let error = parse(r#""\u0""#).unwrap_err();
        assert_eq!(error.kind, JsonErrorKind::UnexpectedEnd);
    }
}
