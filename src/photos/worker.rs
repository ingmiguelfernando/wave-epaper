//! Background preparation of photo cache files, so the gallery stays
//! responsive while JPEGs decode.

use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, MutexGuard, PoisonError,
    },
    thread::JoinHandle,
};

use anyhow::{anyhow, Result};

use super::{
    cache::{self, cache_file, PreparedPhoto},
    image::{decode_jpeg, PhotoFit},
};
use crate::watchdog::Pacer;

/// Decoding uses the heap; the stack only holds the decoder's tables.
pub const PHOTO_WORKER_STACK_BYTES: usize = 48 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotoJob {
    pub name: String,
    pub key: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhotoJobResult {
    Ready {
        key: u32,
    },
    /// `reason` completes a sentence that starts with the file name.
    Failed {
        key: u32,
        reason: String,
    },
}

/// One thread at a time works through the queue, then exits.
pub struct PhotoWorker {
    photos: PathBuf,
    cache: PathBuf,
    queue: Arc<Mutex<VecDeque<PhotoJob>>>,
    sender: Sender<PhotoJobResult>,
    results: Receiver<PhotoJobResult>,
    thread: Option<JoinHandle<()>>,
}

impl PhotoWorker {
    #[must_use]
    pub fn new(photos: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        let (sender, results) = mpsc::channel();
        Self {
            photos: photos.into(),
            cache: cache.into(),
            queue: Arc::default(),
            sender,
            results,
            thread: None,
        }
    }

    /// Replace the waiting jobs; a photo already being prepared finishes.
    pub fn set_jobs(&mut self, jobs: Vec<PhotoJob>) {
        *lock(&self.queue) = jobs.into();
        self.start_if_needed();
    }

    /// True while a thread prepares photos.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    /// Results so far. Also restarts the thread when jobs are still waiting.
    pub fn poll(&mut self) -> Vec<PhotoJobResult> {
        let results = self.results.try_iter().collect();
        self.start_if_needed();
        results
    }

    fn start_if_needed(&mut self) {
        if self.is_busy() || lock(&self.queue).is_empty() {
            return;
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let queue = Arc::clone(&self.queue);
        let sender = self.sender.clone();
        let (photos, cache) = (self.photos.clone(), self.cache.clone());
        let builder = std::thread::Builder::new()
            .name("photos".into())
            .stack_size(PHOTO_WORKER_STACK_BYTES);
        let spawned = spawn_on_second_core(builder, move || loop {
            let Some(job) = lock(&queue).pop_front() else {
                break;
            };
            if sender.send(prepare(&photos, &cache, &job)).is_err() {
                break;
            }
        });
        match spawned {
            Ok(thread) => self.thread = Some(thread),
            Err(error) => log::warn!("rustmix-wave=photo-worker status=failed error={error}"),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The main task owns core 0; on core 1 a decode never delays button polling.
#[cfg(target_os = "espidf")]
pub(crate) fn spawn_on_second_core<F>(
    builder: std::thread::Builder,
    task: F,
) -> io::Result<JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    use esp_idf_svc::hal::{cpu::Core, task::thread::ThreadSpawnConfiguration};

    let previous = ThreadSpawnConfiguration::get().unwrap_or_default();
    let pinned = ThreadSpawnConfiguration {
        priority: 1,
        pin_to_core: Some(Core::Core1),
        ..Default::default()
    };
    if let Err(error) = pinned.set() {
        log::warn!("rustmix-wave=photo-worker status=unpinned error={error}");
    }
    let spawned = builder.spawn(task);
    let _ = previous.set();
    spawned
}

#[cfg(not(target_os = "espidf"))]
pub(crate) fn spawn_on_second_core<F>(
    builder: std::thread::Builder,
    task: F,
) -> io::Result<JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    builder.spawn(task)
}

/// Decode one photo and write its cache file, unless that file exists.
#[must_use]
pub fn prepare(photos: &Path, cache: &Path, job: &PhotoJob) -> PhotoJobResult {
    match prepare_file(photos, cache, job) {
        Ok(()) => PhotoJobResult::Ready { key: job.key },
        Err(error) => PhotoJobResult::Failed {
            key: job.key,
            reason: error.to_string(),
        },
    }
}

fn prepare_file(photos: &Path, cache: &Path, job: &PhotoJob) -> Result<()> {
    let target = cache_file(cache, job.key);
    if target.exists() {
        return Ok(());
    }
    let file = File::open(photos.join(&job.name)).map_err(|_| anyhow!("could not be read"))?;
    let photo = decode_jpeg(BufReader::new(PacedReader::new(file)))?;
    let mut pacer = Pacer::start();
    let thumbnail = photo.thumbnail();
    pacer.pace();
    let fill = photo.screen_frame(PhotoFit::Fill);
    pacer.pace();
    let whole = photo.screen_frame(PhotoFit::Whole);
    let prepared = PreparedPhoto {
        size: photo.original_size(),
        thumbnail,
        fill,
        whole,
    };
    drop(photo);
    fs::create_dir_all(cache)
        .and_then(|()| cache::write(&target, &prepared))
        .map_err(|error| anyhow!("could not be saved ({error})"))
}

/// Pauses for a tick between reads, so a long decode never starves the idle
/// task that feeds the watchdog. Wrap it in a `BufReader` to keep reads rare.
struct PacedReader<R> {
    inner: R,
    pacer: Pacer,
}

impl<R> PacedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            pacer: Pacer::start(),
        }
    }
}

impl<R: Read> Read for PacedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.pacer.pace();
        self.inner.read(buffer)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, thread, time::Duration};

    use super::{PhotoJob, PhotoJobResult, PhotoWorker};
    use crate::photos::{
        cache::{cache_file, read_thumbnail},
        test_photos::grey_jpeg,
    };

    #[test]
    fn prepares_queued_photos_in_the_background() {
        let root = std::env::temp_dir().join(format!("wave-photo-worker-{}", std::process::id()));
        let (photos, cache) = (root.join("PHOTOS"), root.join("CACHE"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&photos).unwrap();
        fs::write(photos.join("A.jpg"), grey_jpeg(96, 64, None)).unwrap();
        fs::write(photos.join("B.jpg"), b"broken").unwrap();

        let mut worker = PhotoWorker::new(&photos, &cache);
        worker.set_jobs(vec![
            PhotoJob {
                name: "A.jpg".into(),
                key: 1,
            },
            PhotoJob {
                name: "B.jpg".into(),
                key: 2,
            },
        ]);
        let mut results = Vec::new();
        for _ in 0..500 {
            results.extend(worker.poll());
            if results.len() == 2 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(results[0], PhotoJobResult::Ready { key: 1 });
        let PhotoJobResult::Failed { key: 2, reason } = &results[1] else {
            panic!("{:?}", results[1]);
        };
        assert_eq!(reason, "is not a readable JPEG");
        let (size, _) = read_thumbnail(&cache_file(&cache, 1)).unwrap();
        assert_eq!(size, (96, 64));
        let _ = fs::remove_dir_all(root);
    }
}
