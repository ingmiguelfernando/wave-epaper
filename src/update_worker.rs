//! Runs one update job (a check or an install) on its own thread, so the
//! screen and keys stay responsive while a 3 MB image downloads. The main
//! loop starts the job, cancels it and applies the events it reports, as it
//! does for a voice note (`ai_worker.rs`).

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::JoinHandle,
};

use crate::{
    ota::Release,
    update_state::{Failure, Step, UpdateEvent},
};

/// TLS and the download buffer need some stack; the image itself streams
/// straight to flash.
pub const UPDATE_WORKER_STACK_BYTES: usize = 32 * 1024;

/// What the worker does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateJob {
    /// Read the latest release.
    Check,
    /// Download, verify and write this release's app image.
    Install(Release),
}

/// The network and the flash, so the worker can be tested without either.
pub trait UpdateTransport: Send {
    /// The latest release.
    fn latest_release(&mut self, cancel: &AtomicBool) -> Result<Release, Failure>;

    /// Download the release's app image into the next update slot, verify it
    /// and make it the next boot image. `progress(step, done, total)` is
    /// called as it goes; `cancel` set means stop, leaving the slot unused.
    fn install(
        &mut self,
        release: &Release,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(Step, u64, u64),
    ) -> Result<(), Failure>;
}

/// Run `job` to its end, reporting through `send`.
pub fn run(
    job: &UpdateJob,
    transport: &mut dyn UpdateTransport,
    cancel: &AtomicBool,
    send: &mut dyn FnMut(UpdateEvent),
) {
    match job {
        UpdateJob::Check => {
            let result = transport.latest_release(cancel);
            send(UpdateEvent::Checked(result));
        }
        UpdateJob::Install(release) => {
            let outcome = transport.install(release, cancel, &mut |step, done, total| {
                send(UpdateEvent::Progress { step, done, total });
            });
            send(match outcome {
                Ok(()) => UpdateEvent::Installed,
                Err(failure) => UpdateEvent::Failed(failure),
            });
        }
    }
}

/// One job at a time, on a thread that ends with the job.
#[derive(Default)]
pub struct UpdateWorker {
    thread: Option<JoinHandle<()>>,
    events: Option<Receiver<UpdateEvent>>,
    cancel: Arc<AtomicBool>,
    job: Option<UpdateJob>,
}

impl UpdateWorker {
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Start `job` unless one is running.
    pub fn start(
        &mut self,
        job: UpdateJob,
        mut transport: Box<dyn UpdateTransport>,
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
        let thread_job = job.clone();
        crate::runtime_worker::wait_for_stack_memory(UPDATE_WORKER_STACK_BYTES);
        let builder = std::thread::Builder::new()
            .name("update".into())
            .stack_size(UPDATE_WORKER_STACK_BYTES);
        let thread = crate::photos::worker::spawn_on_second_core(builder, move || {
            run(
                &thread_job,
                transport.as_mut(),
                &thread_cancel,
                &mut |event| {
                    let _ = sender.send(event);
                },
            );
        })?;
        self.thread = Some(thread);
        self.events = Some(receiver);
        self.cancel = cancel;
        self.job = Some(job);
        Ok(())
    }

    /// Ask the running job to stop. An install that is cancelled leaves its
    /// slot unused.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Events since the last poll. A thread that ended without a final
    /// event reports a failure, so the screen never waits for nothing.
    pub fn poll(&mut self) -> Vec<UpdateEvent> {
        let mut events: Vec<UpdateEvent> = self
            .events
            .as_ref()
            .map(|events| events.try_iter().collect())
            .unwrap_or_default();
        let is_final = |event: &UpdateEvent| {
            matches!(
                event,
                UpdateEvent::Checked(_) | UpdateEvent::Installed | UpdateEvent::Failed(_)
            )
        };
        let ended = self.thread.as_ref().is_some_and(JoinHandle::is_finished);
        if ended && !events.iter().any(is_final) {
            // A last look: the result may have arrived after the first one.
            if let Some(receiver) = self.events.as_ref() {
                events.extend(receiver.try_iter());
            }
            if !events.iter().any(is_final) && self.job.is_some() {
                events.push(UpdateEvent::Failed(Failure::Other(
                    "The update worker stopped.".into(),
                )));
            }
        }
        if events.iter().any(is_final) || ended {
            self.job = None;
            self.events = None;
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };

    use super::{run, UpdateJob, UpdateTransport, UpdateWorker};
    use crate::{
        ota::{parse_release, Release},
        update_state::{Failure, Step, UpdateEvent},
    };

    fn release() -> Release {
        parse_release(r#"{"tag_name":"v1.0.0","assets":[]}"#).unwrap()
    }

    /// A transport that answers from a script.
    struct Script {
        latest: Result<Release, Failure>,
        install: Result<(), Failure>,
        steps: Vec<(Step, u64, u64)>,
        stop_on_cancel: bool,
    }

    impl UpdateTransport for Script {
        fn latest_release(&mut self, _cancel: &AtomicBool) -> Result<Release, Failure> {
            self.latest.clone()
        }

        fn install(
            &mut self,
            _release: &Release,
            cancel: &AtomicBool,
            progress: &mut dyn FnMut(Step, u64, u64),
        ) -> Result<(), Failure> {
            for &(step, done, total) in &self.steps {
                if self.stop_on_cancel && cancel.load(Ordering::Relaxed) {
                    return Err(Failure::Stopped);
                }
                progress(step, done, total);
            }
            self.install.clone()
        }
    }

    fn script() -> Script {
        Script {
            latest: Ok(release()),
            install: Ok(()),
            steps: vec![(Step::Download, 0, 10), (Step::Download, 10, 10)],
            stop_on_cancel: false,
        }
    }

    fn collect(job: &UpdateJob, transport: &mut Script, cancel: &AtomicBool) -> Vec<UpdateEvent> {
        let mut events = Vec::new();
        run(job, transport, cancel, &mut |event| events.push(event));
        events
    }

    #[test]
    fn a_check_reports_the_release_or_the_failure() {
        let cancel = AtomicBool::new(false);
        let events = collect(&UpdateJob::Check, &mut script(), &cancel);
        assert_eq!(events, vec![UpdateEvent::Checked(Ok(release()))]);
        let mut down = script();
        down.latest = Err(Failure::Unreachable);
        assert_eq!(
            collect(&UpdateJob::Check, &mut down, &cancel),
            vec![UpdateEvent::Checked(Err(Failure::Unreachable))]
        );
    }

    #[test]
    fn an_install_reports_progress_then_installed() {
        let events = collect(
            &UpdateJob::Install(release()),
            &mut script(),
            &AtomicBool::new(false),
        );
        assert_eq!(
            events,
            vec![
                UpdateEvent::Progress {
                    step: Step::Download,
                    done: 0,
                    total: 10
                },
                UpdateEvent::Progress {
                    step: Step::Download,
                    done: 10,
                    total: 10
                },
                UpdateEvent::Installed,
            ]
        );
    }

    #[test]
    fn an_install_that_fails_or_is_cancelled_ends_in_failed() {
        let mut bad = script();
        bad.install = Err(Failure::WrongImage("checksum differs".into()));
        let events = collect(
            &UpdateJob::Install(release()),
            &mut bad,
            &AtomicBool::new(false),
        );
        assert_eq!(
            events.last(),
            Some(&UpdateEvent::Failed(Failure::WrongImage(
                "checksum differs".into()
            )))
        );
        let mut stoppable = script();
        stoppable.stop_on_cancel = true;
        let cancelled = AtomicBool::new(true);
        let events = collect(&UpdateJob::Install(release()), &mut stoppable, &cancelled);
        assert_eq!(events, vec![UpdateEvent::Failed(Failure::Stopped)]);
    }

    /// Poll until the worker has nothing running, or time runs out.
    fn drain(worker: &mut UpdateWorker) -> Vec<UpdateEvent> {
        let started = Instant::now();
        let mut all = Vec::new();
        while started.elapsed() < Duration::from_secs(5) {
            all.extend(worker.poll());
            if !worker.is_busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        all
    }

    #[test]
    fn the_worker_runs_a_job_on_its_thread_and_is_free_after() {
        let mut worker = UpdateWorker::default();
        assert!(!worker.is_busy());
        worker
            .start(UpdateJob::Install(release()), Box::new(script()))
            .unwrap();
        assert!(worker.is_busy());
        let events = drain(&mut worker);
        assert_eq!(events.last(), Some(&UpdateEvent::Installed));
        assert!(!worker.is_busy(), "free for the next job");
        // A second job starts after the first.
        worker.start(UpdateJob::Check, Box::new(script())).unwrap();
        let events = drain(&mut worker);
        assert_eq!(events, vec![UpdateEvent::Checked(Ok(release()))]);
    }

    #[test]
    fn a_thread_that_dies_without_a_result_reports_a_failure() {
        struct Dies;
        impl UpdateTransport for Dies {
            fn latest_release(&mut self, _cancel: &AtomicBool) -> Result<Release, Failure> {
                panic!("the transport panicked");
            }
            fn install(
                &mut self,
                _release: &Release,
                _cancel: &AtomicBool,
                _progress: &mut dyn FnMut(Step, u64, u64),
            ) -> Result<(), Failure> {
                Err(Failure::Stopped)
            }
        }
        let mut worker = UpdateWorker::default();
        worker.start(UpdateJob::Check, Box::new(Dies)).unwrap();
        let events = drain(&mut worker);
        assert!(
            matches!(events.last(), Some(UpdateEvent::Failed(Failure::Other(_)))),
            "{events:?}"
        );
        assert!(!worker.is_busy());
    }
}
