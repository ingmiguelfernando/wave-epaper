//! The API keys of the two AI providers. Keys arrive through the Wi-Fi
//! portal and live only in the encrypted NVS partition: never on the SD
//! card, never in a log, never shown again. This module checks keys and
//! picks the one a request uses; `espidf` stores them.

use crate::ai_config::AiConfig;

/// NVS namespace of the keys.
pub const AI_KEYS_NAMESPACE: &str = "ai_keys";
/// Longest key accepted. Provider keys run from about 50 to 200 characters.
pub const AI_KEY_MAX_CHARS: usize = 256;

/// The provider a key belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiKeySlot {
    Transcription,
    Summary,
}

impl AiKeySlot {
    pub const ALL: [Self; 2] = [Self::Transcription, Self::Summary];

    /// The NVS entry name, which is also the portal's `slot` value.
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Transcription => "transcription",
            Self::Summary => "summary",
        }
    }

    #[must_use]
    pub fn from_marker(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.marker() == value)
    }

    const fn other(self) -> Self {
        match self {
            Self::Transcription => Self::Summary,
            Self::Summary => Self::Transcription,
        }
    }

    fn url(self, config: &AiConfig) -> &str {
        match self {
            Self::Transcription => &config.transcription_url,
            Self::Summary => &config.summary_url,
        }
    }
}

/// Which keys the device holds; never the keys themselves.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AiKeyPresence {
    /// The encrypted store opened. Without it no key can be saved.
    pub available: bool,
    pub transcription: bool,
    pub summary: bool,
}

impl AiKeyPresence {
    #[must_use]
    pub const fn has(self, slot: AiKeySlot) -> bool {
        match slot {
            AiKeySlot::Transcription => self.transcription,
            AiKeySlot::Summary => self.summary,
        }
    }

    /// The stored key a request to `slot` uses: its own, or the other
    /// provider's when both URLs point at the same host, so one Groq key
    /// serves both requests.
    #[must_use]
    pub fn source(self, slot: AiKeySlot, config: &AiConfig) -> Option<AiKeySlot> {
        let other = slot.other();
        if self.has(slot) {
            Some(slot)
        } else if self.has(other) && same_host(slot.url(config), other.url(config)) {
            Some(other)
        } else {
            None
        }
    }

    /// Both requests have a key.
    #[must_use]
    pub fn ready(self, config: &AiConfig) -> bool {
        AiKeySlot::ALL
            .into_iter()
            .all(|slot| self.source(slot, config).is_some())
    }

    /// The Settings › AI row value.
    #[must_use]
    pub fn label(self, config: &AiConfig) -> &'static str {
        if !self.available {
            return "Unavailable";
        }
        let transcription = self.source(AiKeySlot::Transcription, config).is_some();
        let summary = self.source(AiKeySlot::Summary, config).is_some();
        match (transcription, summary) {
            (true, true) => "Set",
            (true, false) => "Summary missing",
            (false, true) => "Transcription missing",
            (false, false) => "Not set",
        }
    }
}

/// `api.groq.com` from `https://api.groq.com/openai/v1`.
fn host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', ':'])
        .next()
        .unwrap_or_default()
}

fn same_host(left: &str, right: &str) -> bool {
    let left = host(left);
    !left.is_empty() && left.eq_ignore_ascii_case(host(right))
}

/// Check a key typed in the portal: printable ASCII without spaces, at most
/// `AI_KEY_MAX_CHARS`. Whitespace around it, such as a pasted line break,
/// is dropped.
pub fn validate_key(raw: &str) -> Result<&str, &'static str> {
    let key = raw.trim();
    if key.is_empty() {
        return Err("The key is empty.");
    }
    if key.len() > AI_KEY_MAX_CHARS {
        return Err("The key is too long.");
    }
    if !key.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("The key has spaces or characters a key never has.");
    }
    Ok(key)
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use anyhow::{Context, Result};
    use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

    use super::{AiKeyPresence, AiKeySlot, AI_KEYS_NAMESPACE, AI_KEY_MAX_CHARS};

    /// The keys in the default NVS partition, which this firmware encrypts.
    #[derive(Clone)]
    pub struct AiKeyStore {
        partition: EspDefaultNvsPartition,
    }

    impl AiKeyStore {
        #[must_use]
        pub fn new(partition: EspDefaultNvsPartition) -> Self {
            Self { partition }
        }

        fn open(&self) -> Result<EspNvs<NvsDefault>> {
            EspNvs::new(self.partition.clone(), AI_KEYS_NAMESPACE, true)
                .context("opening the key store")
        }

        /// Which keys are stored. A store that does not open has none.
        #[must_use]
        pub fn presence(&self) -> AiKeyPresence {
            let Ok(nvs) = self.open() else {
                return AiKeyPresence::default();
            };
            let has = |slot: AiKeySlot| nvs.str_len(slot.marker()).ok().flatten().is_some();
            AiKeyPresence {
                available: true,
                transcription: has(AiKeySlot::Transcription),
                summary: has(AiKeySlot::Summary),
            }
        }

        pub fn get(&self, slot: AiKeySlot) -> Result<Option<String>> {
            let nvs = self.open()?;
            let mut buffer = [0_u8; AI_KEY_MAX_CHARS + 1];
            let key = nvs
                .get_str(slot.marker(), &mut buffer)
                .context("reading a key")?
                .map(str::to_owned);
            buffer.fill(0);
            Ok(key)
        }

        pub fn set(&self, slot: AiKeySlot, key: &str) -> Result<()> {
            self.open()?
                .set_str(slot.marker(), key)
                .context("saving the key")
        }

        pub fn clear(&self, slot: AiKeySlot) -> Result<()> {
            self.open()?
                .remove(slot.marker())
                .map(|_| ())
                .context("clearing the key")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_key, AiKeyPresence, AiKeySlot, AI_KEY_MAX_CHARS};
    use crate::ai_config::AiConfig;

    fn config(transcription_url: &str, summary_url: &str) -> AiConfig {
        AiConfig {
            transcription_url: transcription_url.into(),
            summary_url: summary_url.into(),
            ..AiConfig::default()
        }
    }

    const GROQ: &str = "https://api.groq.com/openai/v1";
    const OPENROUTER: &str = "https://openrouter.ai/api/v1";

    #[test]
    fn slots_round_trip_through_their_markers() {
        for slot in AiKeySlot::ALL {
            assert_eq!(AiKeySlot::from_marker(slot.marker()), Some(slot));
            assert!(slot.marker().len() <= 15, "NVS names are at most 15 bytes");
        }
        assert_eq!(AiKeySlot::from_marker("other"), None);
    }

    #[test]
    fn each_provider_needs_its_own_key() {
        let both = config(GROQ, OPENROUTER);
        let only_groq = AiKeyPresence {
            available: true,
            transcription: true,
            summary: false,
        };
        assert_eq!(
            only_groq.source(AiKeySlot::Transcription, &both),
            Some(AiKeySlot::Transcription)
        );
        assert_eq!(only_groq.source(AiKeySlot::Summary, &both), None);
        assert!(!only_groq.ready(&both));
        assert_eq!(only_groq.label(&both), "Summary missing");
    }

    #[test]
    fn one_key_serves_both_requests_on_the_same_host() {
        let groq_twice = config(GROQ, "https://API.groq.com/openai/v1/");
        let summary_only = AiKeyPresence {
            available: true,
            transcription: false,
            summary: true,
        };
        assert_eq!(
            summary_only.source(AiKeySlot::Transcription, &groq_twice),
            Some(AiKeySlot::Summary)
        );
        assert!(summary_only.ready(&groq_twice));
        assert_eq!(summary_only.label(&groq_twice), "Set");
    }

    #[test]
    fn labels_say_what_is_missing() {
        let both = config(GROQ, OPENROUTER);
        let none = AiKeyPresence {
            available: true,
            ..AiKeyPresence::default()
        };
        assert_eq!(none.label(&both), "Not set");
        let summary = AiKeyPresence {
            summary: true,
            ..none
        };
        assert_eq!(summary.label(&both), "Transcription missing");
        assert_eq!(AiKeyPresence::default().label(&both), "Unavailable");
        // Empty URLs share no host, so nothing is borrowed.
        let empty = config("", "");
        assert_eq!(summary.source(AiKeySlot::Transcription, &empty), None);
    }

    #[test]
    fn keys_are_trimmed_and_checked() {
        assert_eq!(validate_key("  gsk_abc123\r\n"), Ok("gsk_abc123"));
        assert!(validate_key("   ").is_err());
        assert!(validate_key("sk or v1").is_err());
        assert!(validate_key("clé").is_err());
        assert!(validate_key(&"k".repeat(AI_KEY_MAX_CHARS)).is_ok());
        assert!(validate_key(&"k".repeat(AI_KEY_MAX_CHARS + 1)).is_err());
    }
}
