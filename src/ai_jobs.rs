//! One voice note through the AI providers. The WAV goes to
//! `/audio/transcriptions` in parts of at most ten minutes, the joined
//! transcript to `/chat/completions`, and each step is saved in the note's
//! `.AI` record. Network access goes through `AiTransport`, so host tests
//! run whole jobs against canned replies.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ai_client,
    ai_config::{provider_name, AiConfig, AiLanguage},
    voice_note_record::{self, NoteRecord, NoteState},
    voice_notes::{build_pcm_wav_header, parse_pcm_wav_header},
    watchdog::Pacer,
};

/// Bytes read from the SD card for each write to the connection.
pub const UPLOAD_CHUNK_BYTES: usize = 16 * 1024;
/// Largest transcription reply kept: a ten-minute part is about 10 KB of text.
pub const TRANSCRIPTION_REPLY_LIMIT: usize = 256 * 1024;
/// Largest summary reply kept.
pub const SUMMARY_REPLY_LIMIT: usize = 64 * 1024;
/// Bytes before a recording's PCM.
const WAV_HEADER_BYTES: u64 = 44;

/// Why a request returned no reply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportError {
    /// Recording or sleep needs the device: stop now.
    Cancelled,
    /// No connection, a timeout or a reply too large; worth another try.
    Network(String),
    /// The recording could not be read from the card.
    File(String),
}

/// One HTTPS POST. The body streams from memory or the SD card; the reply
/// is the status and up to `reply_limit` bytes of text.
pub trait AiTransport {
    fn post(
        &mut self,
        url: &str,
        headers: &[(&str, &str)],
        body: &RequestBody<'_>,
        reply_limit: usize,
        cancel: &AtomicBool,
    ) -> Result<(u16, String), TransportError>;
}

/// A request body. A WAV part is the multipart head, a WAV header for the
/// part, the part's PCM read from the file, and the multipart tail.
pub enum RequestBody<'a> {
    Bytes(&'a [u8]),
    WavPart {
        head: &'a [u8],
        header: [u8; 44],
        path: &'a Path,
        /// File offset of the part's first PCM byte.
        offset: u64,
        len: u64,
        tail: &'a [u8],
    },
}

impl RequestBody<'_> {
    #[must_use]
    pub fn content_length(&self) -> u64 {
        match self {
            Self::Bytes(bytes) => bytes.len() as u64,
            Self::WavPart {
                head,
                header,
                len,
                tail,
                ..
            } => (head.len() + header.len() + tail.len()) as u64 + len,
        }
    }

    /// Hand the body to `write` in pieces of at most `UPLOAD_CHUNK_BYTES`,
    /// stopping as soon as `cancel` is set.
    pub fn stream(
        &self,
        cancel: &AtomicBool,
        write: &mut dyn FnMut(&[u8]) -> Result<(), TransportError>,
    ) -> Result<(), TransportError> {
        let check = || {
            if cancel.load(Ordering::Relaxed) {
                Err(TransportError::Cancelled)
            } else {
                Ok(())
            }
        };
        match self {
            Self::Bytes(bytes) => {
                for chunk in bytes.chunks(UPLOAD_CHUNK_BYTES) {
                    check()?;
                    write(chunk)?;
                }
                Ok(())
            }
            Self::WavPart {
                head,
                header,
                path,
                offset,
                len,
                tail,
            } => {
                let unreadable = |error: std::io::Error| {
                    TransportError::File(format!("The recording could not be read: {error}"))
                };
                check()?;
                write(head)?;
                write(header)?;
                let mut file = File::open(path).map_err(unreadable)?;
                file.seek(SeekFrom::Start(*offset)).map_err(unreadable)?;
                let mut buffer = vec![0_u8; UPLOAD_CHUNK_BYTES];
                let mut left = *len;
                let mut pacer = Pacer::start();
                while left > 0 {
                    check()?;
                    let size = left.min(UPLOAD_CHUNK_BYTES as u64) as usize;
                    file.read_exact(&mut buffer[..size]).map_err(unreadable)?;
                    write(&buffer[..size])?;
                    left -= size as u64;
                    pacer.pace();
                }
                check()?;
                write(tail)
            }
        }
    }
}

/// What a job needs. The keys are copies for this job only.
#[derive(Clone)]
pub struct AiJob {
    pub root: PathBuf,
    pub wav_name: String,
    pub config: AiConfig,
    pub transcription_key: String,
    pub summary_key: String,
}

/// How a job ended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobOutcome {
    /// The summary is saved.
    Done(NoteRecord),
    /// The provider refused the request, or the recording cannot be read.
    /// The record keeps the reason until the note is queued again.
    Failed(String),
    /// A network problem or a busy provider. The note is queued again and
    /// the queue waits before the next try.
    Retry(String),
    /// Recording or sleep stopped the job; the note is queued again.
    Cancelled,
}

enum Stop {
    Cancelled,
    Retry(String),
    Failed(String),
}

impl From<TransportError> for Stop {
    fn from(error: TransportError) -> Self {
        match error {
            TransportError::Cancelled => Self::Cancelled,
            TransportError::Network(reason) => Self::Retry(reason),
            TransportError::File(reason) => Self::Failed(reason),
        }
    }
}

fn card_error(error: anyhow::Error) -> Stop {
    Stop::Failed(format!("SD card: {error:#}"))
}

/// Run one queued note to the end. `progress` hears each working step.
pub fn run(
    job: &AiJob,
    transport: &mut dyn AiTransport,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(NoteState),
) -> JobOutcome {
    match steps(job, transport, cancel, progress) {
        Ok(record) => JobOutcome::Done(record),
        Err(Stop::Cancelled) => {
            let _ = voice_note_record::queue(&job.root, &job.wav_name);
            JobOutcome::Cancelled
        }
        Err(Stop::Retry(reason)) => {
            let _ = voice_note_record::queue(&job.root, &job.wav_name);
            JobOutcome::Retry(reason)
        }
        Err(Stop::Failed(reason)) => {
            let _ = voice_note_record::fail(&job.root, &job.wav_name, &reason);
            JobOutcome::Failed(reason)
        }
    }
}

fn steps(
    job: &AiJob,
    transport: &mut dyn AiTransport,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(NoteState),
) -> Result<NoteRecord, Stop> {
    let config = &job.config;
    let record =
        voice_note_record::begin_transcription(&job.root, &job.wav_name).map_err(card_error)?;
    progress(NoteState::Transcribing);
    // A note queued again after its transcript was saved only needs a summary.
    let transcript = if record.transcript.trim().is_empty() {
        transcribe(job, transport, cancel)?
    } else {
        record.transcript
    };
    if transcript.trim().is_empty() {
        return Err(Stop::Failed("No speech found in the recording.".into()));
    }
    let language = match config.language {
        AiLanguage::Auto => "",
        other => other.label(),
    };
    voice_note_record::finish_transcription(
        &job.root,
        &job.wav_name,
        &transcript,
        language,
        &named(&config.transcription_url, &config.transcription_model),
    )
    .map_err(card_error)?;
    progress(NoteState::Summarizing);
    let content = summarize(job, &transcript, transport, cancel)?;
    voice_note_record::finish_summary(
        &job.root,
        &job.wav_name,
        &ai_client::clean_reply(&content),
        &named(&config.summary_url, &config.summary_model),
    )
    .map_err(card_error)
}

/// `Groq whisper-large-v3-turbo`, as the result screen names a provider.
fn named(url: &str, model: &str) -> String {
    format!("{} {model}", provider_name(url))
}

/// 2xx passes; a busy or broken server is worth another try; anything else
/// is the provider's answer to keep.
fn check_status(status: u16, reply: &str) -> Result<(), Stop> {
    if (200..300).contains(&status) {
        Ok(())
    } else if ai_client::is_retryable_status(status) {
        Err(Stop::Retry(ai_client::api_error(status, reply)))
    } else {
        Err(Stop::Failed(ai_client::api_error(status, reply)))
    }
}

fn pcm_bytes(path: &Path) -> Result<u32, Stop> {
    let unreadable =
        |error: String| Stop::Failed(format!("The recording could not be read: {error}"));
    let mut file = File::open(path).map_err(|error| unreadable(error.to_string()))?;
    let mut header = [0_u8; 44];
    file.read_exact(&mut header).map_err(|error| unreadable(error.to_string()))?;
    parse_pcm_wav_header(&header).map_err(|error| unreadable(error.to_string()))
}

fn transcribe(
    job: &AiJob,
    transport: &mut dyn AiTransport,
    cancel: &AtomicBool,
) -> Result<String, Stop> {
    let path = job.root.join(&job.wav_name);
    let pcm_bytes = pcm_bytes(&path)?;
    let config = &job.config;
    let (head, tail) = ai_client::transcription_form(
        &config.transcription_model,
        ai_client::language_code(config.language),
        ai_client::MULTIPART_BOUNDARY,
    );
    let url = ai_client::endpoint(&config.transcription_url, "/audio/transcriptions");
    let authorization = format!("Bearer {}", job.transcription_key);
    let content_type = format!(
        "multipart/form-data; boundary={}",
        ai_client::MULTIPART_BOUNDARY
    );
    let mut texts = Vec::new();
    for (offset, len) in ai_client::wav_parts(pcm_bytes) {
        let body = RequestBody::WavPart {
            head: &head,
            header: build_pcm_wav_header(len),
            path: &path,
            offset: WAV_HEADER_BYTES + u64::from(offset),
            len: u64::from(len),
            tail: &tail,
        };
        let length = body.content_length().to_string();
        let headers = [
            ("Authorization", authorization.as_str()),
            ("Content-Type", content_type.as_str()),
            ("Content-Length", length.as_str()),
            ("Accept", "application/json"),
        ];
        let (status, reply) =
            transport.post(&url, &headers, &body, TRANSCRIPTION_REPLY_LIMIT, cancel)?;
        check_status(status, &reply)?;
        let text = ai_client::parse_transcription(&reply).map_err(|error| {
            Stop::Failed(format!("Unexpected transcription reply: {error}"))
        })?;
        texts.push(text.trim().to_string());
    }
    texts.retain(|text| !text.is_empty());
    Ok(texts.join("\n\n"))
}

fn summarize(
    job: &AiJob,
    transcript: &str,
    transport: &mut dyn AiTransport,
    cancel: &AtomicBool,
) -> Result<String, Stop> {
    let config = &job.config;
    let url = ai_client::endpoint(&config.summary_url, "/chat/completions");
    let body = ai_client::summary_request(&config.summary_model, transcript, config.style);
    let authorization = format!("Bearer {}", job.summary_key);
    let length = body.len().to_string();
    let headers = [
        ("Authorization", authorization.as_str()),
        ("Content-Type", "application/json"),
        ("Content-Length", length.as_str()),
        ("Accept", "application/json"),
    ];
    let body = RequestBody::Bytes(body.as_bytes());
    let (status, reply) = transport.post(&url, &headers, &body, SUMMARY_REPLY_LIMIT, cancel)?;
    check_status(status, &reply)?;
    ai_client::parse_chat_completion(&reply)
        .map_err(|error| Stop::Failed(format!("Unexpected summary reply: {error}")))
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicBool, Ordering},
    };

    use super::{run, AiJob, AiTransport, JobOutcome, RequestBody, TransportError};
    use crate::{
        ai_config::{AiConfig, AiLanguage, AiProcess, SummaryStyle},
        voice_note_record::{load_record, save_record, NoteRecord, NoteState},
        voice_notes::{build_pcm_wav_header, bytes_per_second},
    };

    /// One request as the provider would see it.
    struct Seen {
        url: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    /// Replies in order, and every request it was sent.
    struct FakeTransport {
        replies: Vec<Result<(u16, String), TransportError>>,
        seen: Vec<Seen>,
        /// Set the cancel flag after this many requests.
        cancel_after: Option<usize>,
    }

    impl FakeTransport {
        fn new(replies: Vec<Result<(u16, String), TransportError>>) -> Self {
            Self {
                replies,
                seen: Vec::new(),
                cancel_after: None,
            }
        }
    }

    impl AiTransport for FakeTransport {
        fn post(
            &mut self,
            url: &str,
            headers: &[(&str, &str)],
            body: &RequestBody<'_>,
            _reply_limit: usize,
            cancel: &AtomicBool,
        ) -> Result<(u16, String), TransportError> {
            if self.cancel_after == Some(self.seen.len()) {
                cancel.store(true, Ordering::Relaxed);
            }
            let mut bytes = Vec::new();
            body.stream(cancel, &mut |chunk| {
                bytes.extend_from_slice(chunk);
                Ok(())
            })?;
            assert_eq!(bytes.len() as u64, body.content_length());
            self.seen.push(Seen {
                url: url.into(),
                headers: headers
                    .iter()
                    .map(|(name, value)| ((*name).into(), (*value).into()))
                    .collect(),
                body: bytes,
            });
            self.replies.remove(0)
        }
    }

    fn config() -> AiConfig {
        AiConfig {
            transcription_url: "https://api.groq.com/openai/v1".into(),
            transcription_model: "whisper-large-v3-turbo".into(),
            language: AiLanguage::Spanish,
            summary_url: "https://openrouter.ai/api/v1/".into(),
            summary_model: "llama-3.3-70b-instruct".into(),
            style: SummaryStyle::BulletsTodos,
            process: AiProcess::Online,
        }
    }

    /// A queued note of `seconds` seconds of a rising PCM ramp.
    fn queued_note(name: &str, seconds: u32) -> (PathBuf, AiJob) {
        let root = std::env::temp_dir().join(format!("wave-ai-job-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let pcm_bytes = seconds * bytes_per_second();
        let mut wav = build_pcm_wav_header(pcm_bytes).to_vec();
        wav.extend((0..pcm_bytes).map(|index| (index % 251) as u8));
        std::fs::write(root.join("VOICE001.WAV"), wav).unwrap();
        save_record(&root, "VOICE001.WAV", &NoteRecord::queued()).unwrap();
        let job = AiJob {
            root: root.clone(),
            wav_name: "VOICE001.WAV".into(),
            config: config(),
            transcription_key: "gsk_test".into(),
            summary_key: "sk-or-test".into(),
        };
        (root, job)
    }

    fn transcription(text: &str) -> Result<(u16, String), TransportError> {
        Ok((200, format!("{{\"text\":\"{text}\"}}")))
    }

    fn completion(content: &str) -> Result<(u16, String), TransportError> {
        Ok((
            200,
            format!("{{\"choices\":[{{\"message\":{{\"content\":\"{content}\"}}}}]}}"),
        ))
    }

    fn header<'a>(seen: &'a Seen, name: &str) -> Option<&'a str> {
        seen.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn record(root: &Path) -> NoteRecord {
        load_record(root, "VOICE001.WAV").unwrap().unwrap()
    }

    fn run_job(job: &AiJob, transport: &mut FakeTransport) -> (JobOutcome, Vec<NoteState>) {
        let mut steps = Vec::new();
        let cancel = AtomicBool::new(false);
        let outcome = run(job, transport, &cancel, &mut |state| steps.push(state));
        (outcome, steps)
    }

    #[test]
    fn a_note_is_transcribed_summarized_and_saved() {
        let (root, job) = queued_note("done", 2);
        let mut transport = FakeTransport::new(vec![
            transcription("Hay que comprar pan."),
            completion("**Compras**\\n- Comprar pan"),
        ]);
        let (outcome, steps) = run_job(&job, &mut transport);
        let saved = record(&root);
        assert_eq!(outcome, JobOutcome::Done(saved.clone()));
        assert_eq!(steps, [NoteState::Transcribing, NoteState::Summarizing]);
        assert_eq!(saved.state, NoteState::Done);
        assert_eq!(saved.title, "Compras");
        assert_eq!(saved.summary, "\u{2022} Comprar pan");
        assert_eq!(saved.transcript, "Hay que comprar pan.");
        assert_eq!(saved.language, "Spanish");
        assert_eq!(saved.transcribed_by, "Groq whisper-large-v3-turbo");
        assert_eq!(saved.summarized_by, "OpenRouter llama-3.3-70b-instruct");

        let [upload, summary] = transport.seen.as_slice() else {
            panic!("two requests");
        };
        assert_eq!(
            upload.url,
            "https://api.groq.com/openai/v1/audio/transcriptions"
        );
        assert_eq!(header(upload, "Authorization"), Some("Bearer gsk_test"));
        assert_eq!(
            header(upload, "Content-Length"),
            Some(upload.body.len().to_string().as_str())
        );
        // The part carries its own WAV header, then the note's PCM unchanged.
        let wav = std::fs::read(root.join("VOICE001.WAV")).unwrap();
        let start = upload
            .body
            .windows(4)
            .position(|window| window == b"RIFF")
            .unwrap();
        assert_eq!(&upload.body[start..start + wav.len()], wav.as_slice());
        assert_eq!(summary.url, "https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(
            header(summary, "Authorization"),
            Some("Bearer sk-or-test")
        );
        let request = String::from_utf8(summary.body.clone()).unwrap();
        assert!(request.contains("Hay que comprar pan."));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_long_note_goes_up_in_parts_and_the_texts_join() {
        // Twelve minutes go up as two parts of six.
        let (root, job) = queued_note("parts", 0);
        let pcm_bytes = 12 * 60 * bytes_per_second();
        let mut wav = build_pcm_wav_header(pcm_bytes).to_vec();
        wav.resize(44 + pcm_bytes as usize, 7);
        std::fs::write(root.join("VOICE001.WAV"), wav).unwrap();
        let mut transport = FakeTransport::new(vec![
            transcription("Primera parte."),
            transcription("Segunda parte."),
            completion("Dos partes\\n- Una nota larga"),
        ]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert!(matches!(outcome, JobOutcome::Done(_)));
        assert_eq!(transport.seen.len(), 3);
        let half = build_pcm_wav_header(6 * 60 * bytes_per_second());
        for upload in &transport.seen[..2] {
            let start = upload
                .body
                .windows(4)
                .position(|window| window == b"RIFF")
                .unwrap();
            assert_eq!(upload.body[start..start + 44], half);
        }
        assert_eq!(
            record(&root).transcript,
            "Primera parte.\n\nSegunda parte."
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_saved_transcript_only_needs_a_new_summary() {
        let (root, job) = queued_note("resummary", 1);
        let queued = NoteRecord {
            transcript: "Ya transcrito.".into(),
            ..NoteRecord::queued()
        };
        save_record(&root, "VOICE001.WAV", &queued).unwrap();
        let mut transport = FakeTransport::new(vec![completion("Resumen\\n- Nuevo")]);
        let (outcome, steps) = run_job(&job, &mut transport);
        assert!(matches!(outcome, JobOutcome::Done(_)));
        assert_eq!(steps, [NoteState::Transcribing, NoteState::Summarizing]);
        assert_eq!(transport.seen.len(), 1, "no audio upload");
        assert!(transport.seen[0].url.ends_with("/chat/completions"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_refused_key_fails_the_note_with_the_providers_words() {
        let (root, job) = queued_note("refused", 1);
        let refusal = r#"{"error":{"message":"Invalid API Key"}}"#;
        let mut transport = FakeTransport::new(vec![Ok((401, refusal.into()))]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert_eq!(
            outcome,
            JobOutcome::Failed("HTTP 401: Invalid API Key".into())
        );
        let saved = record(&root);
        assert_eq!(saved.state, NoteState::Failed);
        assert_eq!(saved.error, "HTTP 401: Invalid API Key");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_busy_provider_or_lost_connection_queues_the_note_again() {
        let (root, job) = queued_note("retry", 1);
        let busy = r#"{"error":{"message":"Rate limit reached"}}"#;
        let mut transport = FakeTransport::new(vec![Ok((429, busy.into()))]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert_eq!(
            outcome,
            JobOutcome::Retry("HTTP 429: Rate limit reached".into())
        );
        assert_eq!(record(&root).state, NoteState::Queued);

        let mut transport = FakeTransport::new(vec![
            transcription("Texto."),
            Err(TransportError::Network("connection reset".into())),
        ]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert_eq!(outcome, JobOutcome::Retry("connection reset".into()));
        let saved = record(&root);
        assert_eq!(saved.state, NoteState::Queued);
        assert_eq!(saved.transcript, "Texto.", "the next try skips the upload");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_cancel_stops_the_upload_and_queues_the_note_again() {
        let (root, job) = queued_note("cancel", 1);
        let mut transport = FakeTransport::new(vec![transcription("unused")]);
        transport.cancel_after = Some(0);
        let (outcome, _) = run_job(&job, &mut transport);
        assert_eq!(outcome, JobOutcome::Cancelled);
        assert!(transport.seen.is_empty(), "nothing was sent");
        assert_eq!(record(&root).state, NoteState::Queued);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn silence_fails_without_asking_for_a_summary() {
        let (root, job) = queued_note("silence", 1);
        let mut transport = FakeTransport::new(vec![transcription("  ")]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert_eq!(
            outcome,
            JobOutcome::Failed("No speech found in the recording.".into())
        );
        assert_eq!(transport.seen.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn auto_language_leaves_the_language_out() {
        let (root, mut job) = queued_note("auto", 1);
        job.config.language = AiLanguage::Auto;
        let mut transport =
            FakeTransport::new(vec![transcription("Hello."), completion("Hi\\n- Hello")]);
        let (outcome, _) = run_job(&job, &mut transport);
        assert!(matches!(outcome, JobOutcome::Done(_)));
        let upload = &transport.seen[0].body;
        let field = b"name=\"language\"";
        assert!(!upload.windows(field.len()).any(|window| window == field));
        assert_eq!(record(&root).language, "");
        let _ = std::fs::remove_dir_all(&root);
    }
}
