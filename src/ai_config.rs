//! AI provider settings from `/RUSTMIX/AI.TXT`: where transcription and
//! summaries go, which models they use, and how they run. Keys never come
//! from the SD card; the main line sets their state.

use std::path::Path;

use anyhow::{Context, Result};

pub const AI_CONFIG_PATH: &str = "/sdcard/RUSTMIX/AI.TXT";

/// Transcription language: `auto` lets the provider detect it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiLanguage {
    Auto,
    Spanish,
    English,
}

impl AiLanguage {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Spanish, Self::English];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Spanish => "Spanish",
            Self::English => "English",
        }
    }

    pub const fn marker(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Spanish => "es",
            Self::English => "en",
        }
    }

    fn from_marker(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.marker() == value)
    }
}

/// How a summary is written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummaryStyle {
    BulletsTodos,
    Bullets,
    Paragraph,
}

impl SummaryStyle {
    pub const ALL: [Self; 3] = [Self::BulletsTodos, Self::Bullets, Self::Paragraph];

    pub const fn label(self) -> &'static str {
        match self {
            Self::BulletsTodos => "Bullets + to-dos",
            Self::Bullets => "Bullets",
            Self::Paragraph => "Paragraph",
        }
    }

    pub const fn marker(self) -> &'static str {
        match self {
            Self::BulletsTodos => "bullets-todos",
            Self::Bullets => "bullets",
            Self::Paragraph => "paragraph",
        }
    }

    fn from_marker(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.marker() == value)
    }
}

/// Whether audio is sent to a provider as it is saved, or only on request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiProcess {
    Online,
    Manual,
}

impl AiProcess {
    pub const ALL: [Self; 2] = [Self::Online, Self::Manual];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Manual => "Manual",
        }
    }

    pub const fn marker(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Manual => "manual",
        }
    }

    fn from_marker(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|process| process.marker() == value)
    }
}

/// The provider a URL points at, named the way the Settings rows show it.
#[must_use]
pub fn provider_name(url: &str) -> String {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', ':'])
        .next()
        .unwrap_or_default();
    match host {
        "api.groq.com" => "Groq".into(),
        "openrouter.ai" => "OpenRouter".into(),
        "api.openai.com" => "OpenAI".into(),
        "localhost" | "127.0.0.1" => "Local".into(),
        other if other.starts_with("192.168.") || other.starts_with("10.") => "Local".into(),
        other => other.to_string(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiConfig {
    pub transcription_url: String,
    pub transcription_model: String,
    pub language: AiLanguage,
    pub summary_url: String,
    pub summary_model: String,
    pub style: SummaryStyle,
    pub process: AiProcess,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            transcription_url: String::new(),
            transcription_model: String::new(),
            language: AiLanguage::Auto,
            summary_url: String::new(),
            summary_model: String::new(),
            style: SummaryStyle::BulletsTodos,
            process: AiProcess::Manual,
        }
    }
}

impl AiConfig {
    /// A missing file or an unreadable one means nothing is set up.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        let Ok(contents) = crate::sd_file::read_to_string(path) else {
            return Ok(None);
        };
        Self::parse(&contents)
            .map(Some)
            .with_context(|| format!("invalid AI settings in {}", path.display()))
    }

    /// Parse `key=value` lines. Unknown keys are ignored; a bad enum value is
    /// an error that names the key.
    pub fn parse(contents: &str) -> Result<Self> {
        let mut config = Self::default();
        for (number, raw) in contents.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                anyhow::bail!("line {} has no '='", number + 1);
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "transcription_url" => config.transcription_url = value.into(),
                "transcription_model" => config.transcription_model = value.into(),
                "summary_url" => config.summary_url = value.into(),
                "summary_model" => config.summary_model = value.into(),
                "language" => {
                    config.language = AiLanguage::from_marker(value)
                        .with_context(|| format!("line {}: unknown language", number + 1))?;
                }
                "style" => {
                    config.style = SummaryStyle::from_marker(value)
                        .with_context(|| format!("line {}: unknown style", number + 1))?;
                }
                "process" => {
                    config.process = AiProcess::from_marker(value)
                        .with_context(|| format!("line {}: unknown process", number + 1))?;
                }
                _ => {}
            }
        }
        Ok(config)
    }

    /// `Ready` needs a URL and a model for both providers.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.transcription_url.is_empty()
            && !self.transcription_model.is_empty()
            && !self.summary_url.is_empty()
            && !self.summary_model.is_empty()
    }

    /// The Settings row value.
    #[must_use]
    pub fn settings_value(&self) -> &'static str {
        if self.is_ready() {
            "Ready"
        } else {
            "Not set up"
        }
    }

    /// Write through `sd_file`, creating the folder first: a card may not have
    /// `/RUSTMIX/` yet.
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        crate::sd_file::replace(path, &self.serialized())
            .with_context(|| format!("write AI settings {}", path.display()))
    }

    #[must_use]
    pub fn serialized(&self) -> String {
        format!(
            "# Wave AI settings v1\ntranscription_url={}\ntranscription_model={}\nlanguage={}\nsummary_url={}\nsummary_model={}\nstyle={}\nprocess={}\n",
            self.transcription_url,
            self.transcription_model,
            self.language.marker(),
            self.summary_url,
            self.summary_model,
            self.style.marker(),
            self.process.marker(),
        )
    }
}

/// The rows of Settings › AI that open an option list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiRow {
    Language,
    Style,
    Process,
}

impl AiConfig {
    /// The labels of a row's list and the index of the value in use.
    #[must_use]
    pub fn options(&self, row: AiRow) -> (Vec<&'static str>, usize) {
        match row {
            AiRow::Language => option_marks(&AiLanguage::ALL, self.language, AiLanguage::label),
            AiRow::Style => option_marks(&SummaryStyle::ALL, self.style, SummaryStyle::label),
            AiRow::Process => option_marks(&AiProcess::ALL, self.process, AiProcess::label),
        }
    }

    /// Apply the chosen index of a row's list. Returns whether the value
    /// changed; choosing the value in use changes nothing and saves nothing.
    pub fn choose(&mut self, row: AiRow, index: usize) -> bool {
        let before = self.clone();
        match row {
            AiRow::Language => {
                if let Some(&value) = AiLanguage::ALL.get(index) {
                    self.language = value;
                }
            }
            AiRow::Style => {
                if let Some(&value) = SummaryStyle::ALL.get(index) {
                    self.style = value;
                }
            }
            AiRow::Process => {
                if let Some(&value) = AiProcess::ALL.get(index) {
                    self.process = value;
                }
            }
        }
        *self != before
    }
}

/// Labels of a list and the position of the value in use, defaulting to 0.
fn option_marks<T: Copy + PartialEq>(
    all: &[T],
    current: T,
    label: fn(T) -> &'static str,
) -> (Vec<&'static str>, usize) {
    (
        all.iter().copied().map(label).collect(),
        all.iter().position(|&value| value == current).unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosing_the_value_in_use_changes_nothing() {
        let mut config = AiConfig::default();
        let (labels, current) = config.options(AiRow::Style);
        assert_eq!(labels.len(), 3);
        assert!(
            !config.choose(AiRow::Style, current),
            "the value in use is kept"
        );
        assert_eq!(config.style, SummaryStyle::BulletsTodos);
    }

    #[test]
    fn choosing_another_value_reports_the_change() {
        let mut config = AiConfig::default();
        assert!(config.choose(AiRow::Process, 0), "Manual to Online changes");
        assert_eq!(config.process, AiProcess::Online);
        assert!(config.choose(AiRow::Language, 1));
        assert_eq!(config.language, AiLanguage::Spanish);
    }

    #[test]
    fn an_out_of_range_index_changes_nothing() {
        let mut config = AiConfig::default();
        assert!(!config.choose(AiRow::Language, 99));
        assert_eq!(config.language, AiLanguage::Auto);
    }

    #[test]
    fn a_save_creates_its_folder_and_loads_back_exactly() {
        let root = std::env::temp_dir().join(format!("wave-ai-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // The card may not have /RUSTMIX/ yet: the save must create the folder.
        let path = root.join("RUSTMIX").join("AI.TXT");
        let config = AiConfig {
            transcription_url: "https://api.groq.com/openai/v1".into(),
            transcription_model: "whisper-large-v3-turbo".into(),
            language: AiLanguage::English,
            ..AiConfig::default()
        };
        config.save_to_path(&path).unwrap();
        assert_eq!(AiConfig::load_from_path(&path).unwrap(), Some(config));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_file_means_not_set_up() {
        let path = std::env::temp_dir().join("wave-ai-config-missing.txt");
        let _ = std::fs::remove_file(&path);
        assert_eq!(AiConfig::load_from_path(&path).unwrap(), None);
    }

    #[test]
    fn defaults_are_manual_and_auto() {
        let config = AiConfig::default();
        assert_eq!(config.language, AiLanguage::Auto);
        assert_eq!(config.style, SummaryStyle::BulletsTodos);
        assert_eq!(config.process, AiProcess::Manual);
        assert_eq!(config.settings_value(), "Not set up");
    }

    #[test]
    fn a_round_trip_keeps_every_field() {
        let config = AiConfig {
            transcription_url: "https://api.groq.com/openai/v1".into(),
            transcription_model: "whisper-large-v3-turbo".into(),
            language: AiLanguage::Spanish,
            summary_url: "https://openrouter.ai/api/v1".into(),
            summary_model: "llama-3.3-70b".into(),
            style: SummaryStyle::Paragraph,
            process: AiProcess::Online,
        };
        assert_eq!(AiConfig::parse(&config.serialized()).unwrap(), config);
    }

    #[test]
    fn unknown_keys_are_ignored_and_ready_needs_both_providers() {
        let config = AiConfig::parse(
            "transcription_url=https://api.groq.com/x\ntranscription_model=w\nfuture=1\n",
        )
        .unwrap();
        assert!(!config.is_ready(), "the summary provider is missing");
        assert_eq!(config.settings_value(), "Not set up");
    }

    #[test]
    fn a_bad_enum_value_names_its_line() {
        let error = AiConfig::parse("language=klingon\n").unwrap_err();
        assert!(format!("{error:#}").contains("line 1"));
    }

    #[test]
    fn providers_are_named_from_the_host() {
        assert_eq!(provider_name("https://api.groq.com/openai/v1"), "Groq");
        assert_eq!(provider_name("https://openrouter.ai/api/v1"), "OpenRouter");
        assert_eq!(provider_name("https://api.openai.com/v1"), "OpenAI");
        assert_eq!(provider_name("http://192.168.1.20:8080/v1"), "Local");
        assert_eq!(provider_name("https://example.org/v1"), "example.org");
    }

    #[test]
    fn option_lists_mark_the_value_in_use_by_marker() {
        let style = SummaryStyle::Bullets;
        assert_eq!(SummaryStyle::ALL.iter().position(|&s| s == style), Some(1));
        assert_eq!(
            SummaryStyle::from_marker("paragraph"),
            Some(SummaryStyle::Paragraph)
        );
    }
}
