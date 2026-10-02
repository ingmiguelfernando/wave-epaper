//! Persistent global UI display preferences.
//!
//! Preferences are loaded from `/sdcard/RUSTMIX/DISPLAY.TXT` at boot. The UI
//! remains usable when the SD card or file is unavailable: Inter + Standard is
//! always the safe default. Changes are persisted best-effort by the runtime.

use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

/// SD-backed global UI typography preference file.
pub const DISPLAY_CONFIG_PATH: &str = "/sdcard/RUSTMIX/DISPLAY.TXT";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiFontFamily {
    #[default]
    Inter,
    AtkinsonHyperlegible,
}

impl UiFontFamily {
    pub const ALL: [Self; 2] = [Self::Inter, Self::AtkinsonHyperlegible];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::AtkinsonHyperlegible => "Atkinson Hyperlegible",
        }
    }

    #[must_use]
    pub const fn compact_label(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::AtkinsonHyperlegible => "Atkinson",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Inter => "inter",
            Self::AtkinsonHyperlegible => "atkinson-hyperlegible",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "inter" => Ok(Self::Inter),
            "atkinson-hyperlegible" | "atkinson_hyperlegible" | "atkinson" => {
                Ok(Self::AtkinsonHyperlegible)
            }
            other => bail!("unsupported font_family value {other:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiFontSize {
    Compact,
    #[default]
    Standard,
    Large,
}

impl UiFontSize {
    pub const ALL: [Self; 3] = [Self::Compact, Self::Standard, Self::Large];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Standard => "Standard",
            Self::Large => "Large",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Large => "large",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "compact" => Ok(Self::Compact),
            "standard" => Ok(Self::Standard),
            "large" => Ok(Self::Large),
            other => bail!("unsupported font_size value {other:?}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisplayPreferences {
    pub font_family: UiFontFamily,
    pub font_size: UiFontSize,
}

impl DisplayPreferences {
    /// Labels of the choices for `action` and the index of the value in use.
    /// Action 0 is the UI font family, anything else the UI font size.
    #[must_use]
    pub fn options(self, action: usize) -> (Vec<&'static str>, usize) {
        if action == 0 {
            (
                UiFontFamily::ALL
                    .iter()
                    .map(|family| family.label())
                    .collect(),
                UiFontFamily::ALL
                    .iter()
                    .position(|&family| family == self.font_family)
                    .unwrap_or(0),
            )
        } else {
            (
                UiFontSize::ALL.iter().map(|size| size.label()).collect(),
                UiFontSize::ALL
                    .iter()
                    .position(|&size| size == self.font_size)
                    .unwrap_or(0),
            )
        }
    }

    /// Apply the option-list choice `index` to `action`.
    pub fn choose(&mut self, action: usize, index: usize) {
        if action == 0 {
            self.font_family = UiFontFamily::ALL[index % UiFontFamily::ALL.len()];
        } else {
            self.font_size = UiFontSize::ALL[index % UiFontSize::ALL.len()];
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("read display config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut preferences = Self::default();
        let mut saw_family = false;
        let mut saw_size = false;
        for (line_number, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("line {} must contain '='", line_number + 1))?;
            match key.trim() {
                "font_family" => {
                    if saw_family {
                        bail!("duplicate font_family entry");
                    }
                    preferences.font_family = UiFontFamily::parse(value)?;
                    saw_family = true;
                }
                "font_size" => {
                    if saw_size {
                        bail!("duplicate font_size entry");
                    }
                    preferences.font_size = UiFontSize::parse(value)?;
                    saw_size = true;
                }
                other => bail!("unsupported display config key {other:?}"),
            }
        }
        Ok(preferences)
    }

    pub fn save_to_path(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.serialized())
            .with_context(|| format!("write display config {}", path.display()))
    }

    #[must_use]
    pub fn serialized(self) -> String {
        format!(
            "# Wave UI typography\nfont_family={}\nfont_size={}\n",
            self.font_family.marker(),
            self.font_size.marker()
        )
    }

    #[must_use]
    pub const fn persistence_label(self) -> &'static str {
        "SD FILE"
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{DisplayPreferences, UiFontFamily, UiFontSize};

    #[test]
    fn defaults_to_inter_standard() {
        let preferences = DisplayPreferences::default();
        assert_eq!(preferences.font_family, UiFontFamily::Inter);
        assert_eq!(preferences.font_size, UiFontSize::Standard);
        assert_eq!(preferences.persistence_label(), "SD FILE");
    }

    #[test]
    fn parses_and_serializes_supported_preferences() {
        let parsed =
            DisplayPreferences::parse("font_family=atkinson-hyperlegible\nfont_size=large\n")
                .unwrap();
        assert_eq!(parsed.font_family, UiFontFamily::AtkinsonHyperlegible);
        assert_eq!(parsed.font_size, UiFontSize::Large);
        assert!(parsed
            .serialized()
            .contains("font_family=atkinson-hyperlegible"));
        assert!(parsed.serialized().contains("font_size=large"));
    }

    #[test]
    fn saves_and_loads_display_file() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rustmix-display-{nanos}.txt"));
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::AtkinsonHyperlegible,
            font_size: UiFontSize::Compact,
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(
            DisplayPreferences::load_from_path(&path).unwrap(),
            preferences
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_unknown_keys_and_values() {
        assert!(DisplayPreferences::parse("font_family=comic-sans\n").is_err());
        assert!(DisplayPreferences::parse("font_size=huge\n").is_err());
        assert!(DisplayPreferences::parse("other=value\n").is_err());
    }
}
