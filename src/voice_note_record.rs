//! The AI record of one voice note: `VOICE###.AI`, next to its WAV. It holds
//! the processing state, the provider names, the summary and the transcript.
//! No file means the note was never processed.

use anyhow::{bail, Context, Result};

/// Where a note stands in the AI queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteState {
    Queued,
    Transcribing,
    Summarizing,
    Done,
    Failed,
}

impl NoteState {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Transcribing => "transcribing",
            Self::Summarizing => "summarizing",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    fn from_marker(value: &str) -> Option<Self> {
        [
            Self::Queued,
            Self::Transcribing,
            Self::Summarizing,
            Self::Done,
            Self::Failed,
        ]
        .into_iter()
        .find(|state| state.marker() == value)
    }
}

/// Lines that open the two text blocks. Text lines that start with one of
/// these are escaped, so a transcript can never be mistaken for a block.
const SUMMARY_MARK: &str = "[summary]";
const TRANSCRIPT_MARK: &str = "[transcript]";
const ESCAPE: char = '\\';

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoteRecord {
    pub state: NoteState,
    pub error: String,
    pub title: String,
    pub language: String,
    pub transcribed_by: String,
    pub summarized_by: String,
    pub summary: String,
    pub transcript: String,
}

/// The record's file name for a WAV name: `VOICE001.WAV` gives `VOICE001.AI`.
/// `None` for a name that is not a voice WAV.
#[must_use]
pub fn record_file_name(wav_name: &str) -> Option<String> {
    let upper = wav_name.to_ascii_uppercase();
    let stem = upper.strip_suffix(".WAV")?;
    (stem.len() == 8 && stem.starts_with("VOICE")).then(|| format!("{stem}.AI"))
}

/// Load the record of `wav_name` in `root`. A missing record means the note
/// was never processed, which is not an error.
pub fn load_record(root: &std::path::Path, wav_name: &str) -> Result<Option<NoteRecord>> {
    let Some(name) = record_file_name(wav_name) else {
        return Ok(None);
    };
    let path = root.join(name);
    let Ok(text) = crate::sd_file::read_to_string(&path) else {
        return Ok(None);
    };
    NoteRecord::parse(&text)
        .map(Some)
        .with_context(|| format!("invalid AI record {}", path.display()))
}

/// Save the record of `wav_name` through `sd_file`, creating the folder first.
pub fn save_record(root: &std::path::Path, wav_name: &str, record: &NoteRecord) -> Result<()> {
    let name =
        record_file_name(wav_name).with_context(|| format!("not a voice WAV: {wav_name}"))?;
    std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    crate::sd_file::replace(&root.join(name), &record.serialized()).context("saving the AI record")
}

/// Delete the record of `wav_name`, if there is one. A note's delete calls
/// this, so no record outlives its WAV.
pub fn delete_record(root: &std::path::Path, wav_name: &str) -> Result<()> {
    let Some(name) = record_file_name(wav_name) else {
        return Ok(());
    };
    match std::fs::remove_file(root.join(name)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("deleting the AI record"),
    }
}

impl NoteRecord {
    #[must_use]
    pub fn queued() -> Self {
        Self {
            state: NoteState::Queued,
            error: String::new(),
            title: String::new(),
            language: String::new(),
            transcribed_by: String::new(),
            summarized_by: String::new(),
            summary: String::new(),
            transcript: String::new(),
        }
    }

    /// The `key=value` header, then the summary and transcript blocks.
    #[must_use]
    pub fn serialized(&self) -> String {
        let mut out = format!(
            "state={}\nerror={}\ntitle={}\nlanguage={}\ntranscribed_by={}\nsummarized_by={}\n",
            self.state.marker(),
            one_line(&self.error),
            one_line(&self.title),
            one_line(&self.language),
            one_line(&self.transcribed_by),
            one_line(&self.summarized_by),
        );
        out.push_str(SUMMARY_MARK);
        out.push('\n');
        out.push_str(&block(&self.summary));
        out.push_str(TRANSCRIPT_MARK);
        out.push('\n');
        out.push_str(&block(&self.transcript));
        out
    }

    /// Parse a record. A header line with an unknown state is an error that
    /// names the line; unknown header keys are ignored.
    pub fn parse(text: &str) -> Result<Self> {
        let mut record = Self::queued();
        let mut block_now: Option<&str> = None;
        let mut summary = Vec::new();
        let mut transcript = Vec::new();
        for (number, line) in text.lines().enumerate() {
            if line == SUMMARY_MARK {
                block_now = Some(SUMMARY_MARK);
                continue;
            }
            if line == TRANSCRIPT_MARK {
                block_now = Some(TRANSCRIPT_MARK);
                continue;
            }
            match block_now {
                Some(SUMMARY_MARK) => summary.push(unescape(line)),
                Some(_) => transcript.push(unescape(line)),
                None => {
                    let Some((key, value)) = line.split_once('=') else {
                        bail!("record line {} has no '='", number + 1);
                    };
                    match key {
                        "state" => {
                            record.state = NoteState::from_marker(value).with_context(|| {
                                format!("record line {}: unknown state", number + 1)
                            })?;
                        }
                        "error" => record.error = value.into(),
                        "title" => record.title = value.into(),
                        "language" => record.language = value.into(),
                        "transcribed_by" => record.transcribed_by = value.into(),
                        "summarized_by" => record.summarized_by = value.into(),
                        _ => {}
                    }
                }
            }
        }
        record.summary = summary.join("\n");
        record.transcript = transcript.join("\n");
        Ok(record)
    }
}

/// A header value stays on one line: newlines become spaces.
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

/// One text block: each line escaped if it looks like a block marker.
fn block(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with('[') || line.starts_with(ESCAPE) {
            out.push(ESCAPE);
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn unescape(line: &str) -> String {
    match line.strip_prefix(ESCAPE) {
        Some(rest) if rest.starts_with('[') || rest.starts_with(ESCAPE) => rest.to_string(),
        _ => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_record_round_trips() {
        let record = NoteRecord {
            state: NoteState::Done,
            error: String::new(),
            title: "Compra de pan".into(),
            language: "es".into(),
            transcribed_by: "Groq whisper-large-v3-turbo".into(),
            summarized_by: "OpenRouter llama-3.3-70b".into(),
            summary: "- Comprar pan\n- Llamar a Ana".into(),
            transcript: "Hoy tengo que comprar pan.\n\nLlamar a Ana mañana.".into(),
        };
        assert_eq!(NoteRecord::parse(&record.serialized()).unwrap(), record);
    }

    #[test]
    fn a_block_marker_inside_the_text_round_trips() {
        let record = NoteRecord {
            transcript: "[summary]\n\\[transcript]\nfin".into(),
            ..NoteRecord::queued()
        };
        let parsed = NoteRecord::parse(&record.serialized()).unwrap();
        assert_eq!(
            parsed.transcript, record.transcript,
            "the text is kept as written"
        );
        assert_eq!(parsed.summary, "");
    }

    #[test]
    fn a_failed_note_keeps_its_error_on_one_line() {
        let record = NoteRecord {
            state: NoteState::Failed,
            error: "HTTP 429\nrate limit".into(),
            ..NoteRecord::queued()
        };
        let parsed = NoteRecord::parse(&record.serialized()).unwrap();
        assert_eq!(parsed.state, NoteState::Failed);
        assert_eq!(parsed.error, "HTTP 429 rate limit");
    }

    #[test]
    fn an_unknown_state_names_its_line() {
        let error = NoteRecord::parse("state=flying\n").unwrap_err();
        assert!(format!("{error:#}").contains("line 1"));
    }

    #[test]
    fn unknown_header_keys_are_ignored() {
        let parsed = NoteRecord::parse("state=queued\nfuture=1\n").unwrap();
        assert_eq!(parsed.state, NoteState::Queued);
    }

    #[test]
    fn the_record_name_shares_the_wav_stem() {
        assert_eq!(
            record_file_name("VOICE001.WAV").as_deref(),
            Some("VOICE001.AI")
        );
        assert_eq!(record_file_name("notes.wav"), None, "not a voice note");
        assert_eq!(
            record_file_name("VOICE1.WAV"),
            None,
            "the stem is eight characters"
        );
    }

    #[test]
    fn a_record_saves_loads_and_is_deleted_with_its_note() {
        let root = std::env::temp_dir().join(format!("wave-voice-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // The voice folder may not exist yet: the save creates it.
        assert_eq!(
            load_record(&root, "VOICE001.WAV").unwrap(),
            None,
            "unprocessed"
        );
        let record = NoteRecord {
            state: NoteState::Done,
            title: "Compra".into(),
            summary: "- Pan".into(),
            ..NoteRecord::queued()
        };
        save_record(&root, "VOICE001.WAV", &record).unwrap();
        assert_eq!(load_record(&root, "VOICE001.WAV").unwrap(), Some(record));
        delete_record(&root, "VOICE001.WAV").unwrap();
        assert_eq!(
            load_record(&root, "VOICE001.WAV").unwrap(),
            None,
            "deleted with the note"
        );
        delete_record(&root, "VOICE001.WAV").unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
