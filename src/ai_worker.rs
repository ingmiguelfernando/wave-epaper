//! Runs one AI job at a time on its own thread, so the screen and keys stay
//! responsive while a note uploads. The main loop starts jobs, cancels them
//! and applies the steps they report.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    ai_jobs::{self, AiJob, AiTransport, JobOutcome},
    voice_note_record::NoteState,
};

/// TLS needs some stack; the request buffers live on the heap.
pub const AI_WORKER_STACK_BYTES: usize = 32 * 1024;
/// Wait after the first failed try, doubled after each further one.
pub const RETRY_FIRST: Duration = Duration::from_secs(2 * 60);
pub const RETRY_MAX: Duration = Duration::from_secs(60 * 60);

/// What the worker reports to the main loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AiEvent {
    /// The note reached a working step.
    Step { wav_name: String, state: NoteState },
    /// The job ended; the worker is free for the next note.
    Finished {
        wav_name: String,
        outcome: JobOutcome,
    },
}

/// One job at a time, on a thread that ends with the job.
#[derive(Default)]
pub struct AiWorker {
    thread: Option<JoinHandle<()>>,
    events: Option<Receiver<AiEvent>>,
    cancel: Arc<AtomicBool>,
    current: Option<String>,
}

impl AiWorker {
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    /// The note being processed.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Start `job` unless one is running.
    pub fn start(
        &mut self,
        job: AiJob,
        mut transport: Box<dyn AiTransport + Send>,
    ) -> std::io::Result<()> {
        if self.is_busy() {
            return Ok(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = Arc::clone(&cancel);
        let wav_name = job.wav_name.clone();
        crate::runtime_worker::wait_for_stack_memory(AI_WORKER_STACK_BYTES);
        let builder = std::thread::Builder::new()
            .name("ai-notes".into())
            .stack_size(AI_WORKER_STACK_BYTES);
        let thread = crate::photos::worker::spawn_on_second_core(builder, move || {
            let name = job.wav_name.clone();
            let steps = sender.clone();
            let mut progress = |state| {
                let _ = steps.send(AiEvent::Step {
                    wav_name: name.clone(),
                    state,
                });
            };
            let outcome = ai_jobs::run(&job, transport.as_mut(), &thread_cancel, &mut progress);
            let _ = sender.send(AiEvent::Finished {
                wav_name: job.wav_name.clone(),
                outcome,
            });
        })?;
        self.thread = Some(thread);
        self.events = Some(receiver);
        self.cancel = cancel;
        self.current = Some(wav_name);
        Ok(())
    }

    /// Ask the running job to stop; it queues its note again.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Events since the last poll. A thread that ended without a result
    /// reports a retry, so its note is queued again.
    pub fn poll(&mut self) -> Vec<AiEvent> {
        let mut events: Vec<AiEvent> = self
            .events
            .as_ref()
            .map(|events| events.try_iter().collect())
            .unwrap_or_default();
        let finished = events
            .iter()
            .any(|event| matches!(event, AiEvent::Finished { .. }));
        let ended = self.thread.as_ref().is_some_and(JoinHandle::is_finished);
        if !finished && ended {
            // A last look: the result may have arrived after the first one.
            if let Some(receiver) = self.events.as_ref() {
                events.extend(receiver.try_iter());
            }
        }
        let finished = events
            .iter()
            .any(|event| matches!(event, AiEvent::Finished { .. }));
        if !finished && ended {
            if let Some(wav_name) = self.current.clone() {
                events.push(AiEvent::Finished {
                    wav_name,
                    outcome: JobOutcome::Retry("The AI worker stopped.".into()),
                });
            }
        }
        if finished || ended {
            self.current = None;
            self.events = None;
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
        events
    }
}

/// The wait before the next try after failed ones: two minutes, doubled
/// after each further failure, at most an hour.
#[derive(Clone, Copy, Debug, Default)]
pub struct AiBackoff {
    failures: u32,
    until: Option<Instant>,
}

impl AiBackoff {
    #[must_use]
    pub fn ready(&self, now: Instant) -> bool {
        self.until.map_or(true, |until| now >= until)
    }

    pub fn failed(&mut self, now: Instant) {
        let doublings = self.failures.min(10);
        self.failures = self.failures.saturating_add(1);
        let wait = RETRY_FIRST.saturating_mul(1 << doublings).min(RETRY_MAX);
        self.until = Some(now + wait);
    }

    /// A finished job, or the owner asking again, clears the wait.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use std::{sync::atomic::AtomicBool, time::Duration};

    use embedded_svc::http::Method;
    use esp_idf_svc::{
        http::client::{Configuration, EspHttpConnection},
        sys::{self, EspError},
    };

    use crate::ai_jobs::{AiTransport, RequestBody, TransportError};

    /// How long the connection waits on the provider. Transcribing a long
    /// part takes a while after the upload ends.
    const AI_HTTP_TIMEOUT: Duration = Duration::from_secs(90);

    /// HTTPS through ESP-IDF's client and certificate bundle, with a new
    /// connection per request so a failed one cannot affect the next.
    pub struct EspAiTransport;

    fn network(what: &str, error: EspError) -> TransportError {
        TransportError::Network(format!("{what}: {error}"))
    }

    impl AiTransport for EspAiTransport {
        fn post(
            &mut self,
            url: &str,
            headers: &[(&str, &str)],
            body: &RequestBody<'_>,
            reply_limit: usize,
            cancel: &AtomicBool,
        ) -> Result<(u16, String), TransportError> {
            let config = Configuration {
                crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
                timeout: Some(AI_HTTP_TIMEOUT),
                buffer_size: Some(4096),
                // Request line and headers, the key included.
                buffer_size_tx: Some(2048),
                ..Default::default()
            };
            let mut connection =
                EspHttpConnection::new(&config).map_err(|error| network("HTTP client", error))?;
            connection
                .initiate_request(Method::Post, url, headers)
                .map_err(|error| network("Connecting", error))?;
            body.stream(cancel, &mut |bytes| {
                connection
                    .write_all(bytes)
                    .map_err(|error| network("Sending", error))
            })?;
            connection
                .initiate_response()
                .map_err(|error| network("Waiting for the reply", error))?;
            let status = connection.status();
            let mut reply = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = connection
                    .read(&mut buffer)
                    .map_err(|error| network("Reading the reply", error))?;
                if read == 0 {
                    break;
                }
                if reply.len() + read > reply_limit {
                    return Err(TransportError::Network("The reply is too large.".into()));
                }
                reply.extend_from_slice(&buffer[..read]);
            }
            Ok((status, String::from_utf8_lossy(&reply).into_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

    use super::{AiBackoff, AiEvent, AiWorker, RETRY_FIRST, RETRY_MAX};
    use crate::{
        ai_config::AiConfig,
        ai_jobs::{AiJob, AiTransport, JobOutcome, RequestBody, TransportError},
        voice_note_record::{save_record, NoteRecord, NoteState},
        voice_notes::build_pcm_wav_header,
    };

    /// Answers every request with a refusal.
    struct Refusing;

    impl AiTransport for Refusing {
        fn post(
            &mut self,
            _url: &str,
            _headers: &[(&str, &str)],
            _body: &RequestBody<'_>,
            _reply_limit: usize,
            _cancel: &AtomicBool,
        ) -> Result<(u16, String), TransportError> {
            Ok((403, r#"{"error":{"message":"Forbidden"}}"#.into()))
        }
    }

    fn wait_for_finish(worker: &mut AiWorker) -> Vec<AiEvent> {
        let mut events = Vec::new();
        for _ in 0..500 {
            events.extend(worker.poll());
            if !worker.is_busy() {
                return events;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the job did not finish");
    }

    #[test]
    fn a_job_runs_on_its_thread_and_reports_its_steps() {
        let root = std::env::temp_dir().join(format!("wave-ai-worker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut wav = build_pcm_wav_header(32_000).to_vec();
        wav.resize(44 + 32_000, 0);
        std::fs::write(root.join("VOICE001.WAV"), wav).unwrap();
        save_record(&root, "VOICE001.WAV", &NoteRecord::queued()).unwrap();
        let job = AiJob {
            root: root.clone(),
            wav_name: "VOICE001.WAV".into(),
            config: AiConfig::default(),
            transcription_key: "key".into(),
            summary_key: "key".into(),
        };
        let mut worker = AiWorker::default();
        worker.start(job, Box::new(Refusing)).unwrap();
        assert_eq!(worker.current(), Some("VOICE001.WAV"));
        let events = wait_for_finish(&mut worker);
        assert_eq!(
            events,
            [
                AiEvent::Step {
                    wav_name: "VOICE001.WAV".into(),
                    state: NoteState::Transcribing,
                },
                AiEvent::Finished {
                    wav_name: "VOICE001.WAV".into(),
                    outcome: JobOutcome::Failed("HTTP 403: Forbidden".into()),
                },
            ]
        );
        assert_eq!(worker.current(), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn failures_wait_longer_each_time_up_to_an_hour() {
        let now = Instant::now();
        let mut backoff = AiBackoff::default();
        assert!(backoff.ready(now));
        backoff.failed(now);
        assert!(!backoff.ready(now + RETRY_FIRST - Duration::from_secs(1)));
        assert!(backoff.ready(now + RETRY_FIRST));
        backoff.failed(now);
        assert!(!backoff.ready(now + RETRY_FIRST));
        assert!(backoff.ready(now + RETRY_FIRST * 2));
        for _ in 0..20 {
            backoff.failed(now);
        }
        assert!(backoff.ready(now + RETRY_MAX));
        backoff.clear();
        assert!(backoff.ready(now));
    }
}
