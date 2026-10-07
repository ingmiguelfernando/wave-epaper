//! Gallery and viewer state, independent of the hardware.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
};

use super::{
    cache::{self, cache_file, key_from_file_name},
    image::{PhotoFit, Thumbnail},
    scan_photos,
    worker::{PhotoJob, PhotoJobResult},
    PhotoEntry, StarredPhotos, PHOTOS_DIRECTORY, PHOTO_CACHE_DIRECTORY,
};
use crate::{buttons::ButtonEvent, framebuffer::FrameBuffer};

/// Thumbnails per gallery page: three columns, two rows.
pub const PAGE_SIZE: usize = 6;
/// Stale cache files removed per scan, to keep opening the gallery quick.
const MAX_CLEANUP: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhotoStatus {
    /// Waiting for the background worker.
    Pending,
    Ready,
    /// Completes a sentence that starts with the file name.
    Failed(String),
}

/// What ● offers in the viewer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhotoAction {
    ToggleStar,
    OnlyThis,
    Delete,
}

impl PhotoAction {
    pub const ALL: [Self; 3] = [Self::ToggleStar, Self::OnlyThis, Self::Delete];

    #[must_use]
    pub const fn label(self, starred: bool) -> &'static str {
        match self {
            Self::ToggleStar if starred => "Remove from sleep set",
            Self::ToggleStar => "Add to sleep set",
            Self::OnlyThis => "Use only this photo",
            Self::Delete => "Delete photo",
        }
    }
}

/// The full-screen photo and its action list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhotoViewer {
    /// The photo on screen, once its cache file is read.
    pub frame: Option<FrameBuffer>,
    /// Highlighted action while the action list is open.
    pub action: Option<usize>,
    /// Waiting for a second Select before deleting the file.
    pub confirm_delete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotosUiState {
    photos_directory: PathBuf,
    cache_directory: PathBuf,
    pub photos: Vec<PhotoEntry>,
    pub status: Vec<PhotoStatus>,
    pub selected: usize,
    pub starred: StarredPhotos,
    /// How the viewer fits photos; follows Settings › Sleep screen.
    pub fit: PhotoFit,
    pub viewer: PhotoViewer,
    viewer_open: bool,
    /// Why the list is empty, when it is.
    pub note: Option<String>,
    /// Original sizes, read along with thumbnails.
    sizes: BTreeMap<u32, (u16, u16)>,
    /// Thumbnails of the visible page.
    thumbnails: BTreeMap<u32, Thumbnail>,
    starred_changed: bool,
    jobs_changed: bool,
    full_refresh: bool,
    delete_request: Option<String>,
}

impl Default for PhotosUiState {
    fn default() -> Self {
        Self::with_roots(PHOTOS_DIRECTORY, PHOTO_CACHE_DIRECTORY)
    }
}

impl PhotosUiState {
    #[must_use]
    pub fn with_roots(photos: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        Self {
            photos_directory: photos.into(),
            cache_directory: cache.into(),
            photos: Vec::new(),
            status: Vec::new(),
            selected: 0,
            starred: StarredPhotos::default(),
            fit: PhotoFit::default(),
            viewer: PhotoViewer::default(),
            viewer_open: false,
            note: None,
            sizes: BTreeMap::new(),
            thumbnails: BTreeMap::new(),
            starred_changed: false,
            jobs_changed: false,
            full_refresh: false,
            delete_request: None,
        }
    }

    #[must_use]
    pub fn photos_directory(&self) -> &Path {
        &self.photos_directory
    }

    #[must_use]
    pub fn cache_directory(&self) -> &Path {
        &self.cache_directory
    }

    /// List the photos again and match them with their cache files.
    pub fn refresh(&mut self) {
        let folder = self.folder_label();
        match scan_photos(&self.photos_directory) {
            Ok(photos) => {
                self.note = photos
                    .is_empty()
                    .then(|| format!("No JPEG photos in {folder}"));
                self.photos = photos;
            }
            Err(error) => {
                self.note = Some(if error.kind() == io::ErrorKind::NotFound {
                    format!("No {folder} folder on the SD card")
                } else {
                    format!("Could not read {folder}")
                });
                self.photos.clear();
            }
        }
        let cached = self.cached_keys();
        self.status = self
            .photos
            .iter()
            .map(|photo| {
                if cached.contains(&photo.key()) {
                    PhotoStatus::Ready
                } else {
                    PhotoStatus::Pending
                }
            })
            .collect();
        if self.note.is_none() {
            self.remove_stale(&cached);
        }
        self.selected = self.selected.min(self.photos.len().saturating_sub(1));
        self.thumbnails.clear();
        self.load_visible();
        self.jobs_changed = true;
    }

    /// The folder as it appears on the card, e.g. `/RUSTMIX/PHOTOS`.
    fn folder_label(&self) -> String {
        let path = self.photos_directory.to_string_lossy();
        path.strip_prefix("/sdcard").unwrap_or(&path).to_string()
    }

    fn cached_keys(&self) -> BTreeSet<u32> {
        let Ok(entries) = fs::read_dir(&self.cache_directory) else {
            return BTreeSet::new();
        };
        entries
            .filter_map(|entry| {
                let name = entry.ok()?.file_name();
                key_from_file_name(&name.to_string_lossy())
            })
            .collect()
    }

    fn remove_stale(&self, cached: &BTreeSet<u32>) {
        let current: BTreeSet<u32> = self.photos.iter().map(PhotoEntry::key).collect();
        for key in cached.difference(&current).take(MAX_CLEANUP) {
            let _ = fs::remove_file(cache_file(&self.cache_directory, *key));
        }
    }

    #[must_use]
    pub fn page(&self) -> usize {
        self.selected / PAGE_SIZE
    }

    #[must_use]
    pub fn page_count(&self) -> usize {
        self.photos.len().div_ceil(PAGE_SIZE).max(1)
    }

    /// Indexes of the photos on the current page.
    #[must_use]
    pub fn visible(&self) -> Range<usize> {
        let start = self.page() * PAGE_SIZE;
        start..(start + PAGE_SIZE).min(self.photos.len())
    }

    #[must_use]
    pub fn selected_photo(&self) -> Option<&PhotoEntry> {
        self.photos.get(self.selected)
    }

    #[must_use]
    pub fn thumbnail(&self, key: u32) -> Option<&Thumbnail> {
        self.thumbnails.get(&key)
    }

    #[must_use]
    pub fn size(&self, key: u32) -> Option<(u16, u16)> {
        self.sizes.get(&key).copied()
    }

    /// Starred photos that are in the list.
    #[must_use]
    pub fn starred_count(&self) -> usize {
        let starred = &self.starred;
        self.photos
            .iter()
            .filter(|photo| starred.contains(&photo.name))
            .count()
    }

    /// For example `IMG_0412.jpg · 4032×3024 · Sep 28, 2026`.
    #[must_use]
    pub fn info_label(&self) -> Option<String> {
        let photo = self.selected_photo()?;
        let size = match self.size(photo.key()) {
            Some((width, height)) => format!("{width}\u{d7}{height}"),
            None => photo.size_label(),
        };
        Some(format!(
            "{} \u{b7} {size} \u{b7} {}",
            photo.name,
            photo.date_label()
        ))
    }

    /// Read thumbnails for the visible page, dropping the rest.
    fn load_visible(&mut self) {
        let visible = self.visible();
        let keys: Vec<u32> = self.photos[visible.clone()]
            .iter()
            .map(PhotoEntry::key)
            .collect();
        self.thumbnails.retain(|key, _| keys.contains(key));
        for index in visible {
            let key = self.photos[index].key();
            if self.status[index] != PhotoStatus::Ready || self.thumbnails.contains_key(&key) {
                continue;
            }
            match cache::read_thumbnail(&cache_file(&self.cache_directory, key)) {
                Ok((size, thumbnail)) => {
                    self.sizes.insert(key, size);
                    self.thumbnails.insert(key, thumbnail);
                }
                Err(_) => {
                    // Unreadable or from an older version: prepare it again.
                    let _ = fs::remove_file(cache_file(&self.cache_directory, key));
                    self.status[index] = PhotoStatus::Pending;
                    self.jobs_changed = true;
                }
            }
        }
    }

    /// Photos still to prepare: the visible page first, then the following
    /// pages, then the earlier ones.
    #[must_use]
    pub fn cache_jobs(&self) -> Vec<PhotoJob> {
        let start = self.page() * PAGE_SIZE;
        (start..self.photos.len())
            .chain(0..start)
            .filter(|index| self.status[*index] == PhotoStatus::Pending)
            .map(|index| PhotoJob {
                name: self.photos[index].name.clone(),
                key: self.photos[index].key(),
            })
            .collect()
    }

    /// Record a worker result; true when the visible screen changed.
    pub fn on_job_result(&mut self, result: &PhotoJobResult) -> bool {
        let (key, status) = match result {
            PhotoJobResult::Ready { key } => (*key, PhotoStatus::Ready),
            PhotoJobResult::Failed { key, reason } => (*key, PhotoStatus::Failed(reason.clone())),
        };
        let Some(index) = self.photos.iter().position(|photo| photo.key() == key) else {
            return false;
        };
        self.status[index] = status;
        let mut changed = false;
        if self.visible().contains(&index) {
            self.load_visible();
            changed = true;
        }
        if index == self.selected && self.viewer_open && self.viewer.frame.is_none() {
            self.load_viewer_frame();
            changed = true;
        }
        changed
    }

    pub fn take_jobs_changed(&mut self) -> bool {
        std::mem::take(&mut self.jobs_changed)
    }

    pub fn take_starred_changed(&mut self) -> bool {
        std::mem::take(&mut self.starred_changed)
    }

    /// True once after the viewer changed photo: worth a full refresh.
    pub fn take_full_refresh(&mut self) -> bool {
        std::mem::take(&mut self.full_refresh)
    }

    /// File waiting to be deleted once the worker is idle.
    #[must_use]
    pub fn delete_request(&self) -> Option<&str> {
        self.delete_request.as_deref()
    }

    /// ▲▼ move through the photos; true when Select asks for the viewer.
    pub fn apply_grid(&mut self, event: ButtonEvent) -> bool {
        let count = self.photos.len();
        if count == 0 {
            return false;
        }
        let page = self.page();
        match event {
            ButtonEvent::Up => self.selected = (self.selected + count - 1) % count,
            ButtonEvent::Down => self.selected = (self.selected + 1) % count,
            ButtonEvent::Select => return true,
        }
        if self.page() != page {
            self.load_visible();
            self.jobs_changed = true;
        }
        false
    }

    /// Star or unstar the selected photo.
    pub fn toggle_star(&mut self) {
        if let Some(name) = self.selected_photo().map(|photo| photo.name.clone()) {
            self.starred.toggle(&name);
            self.starred_changed = true;
        }
    }

    pub fn open_viewer(&mut self) {
        self.viewer = PhotoViewer::default();
        self.viewer_open = true;
        self.load_viewer_frame();
        self.full_refresh = true;
    }

    /// Back to the grid; the frame is dropped to free its 48 KB.
    pub fn close_viewer(&mut self) {
        self.viewer = PhotoViewer::default();
        self.viewer_open = false;
        self.full_refresh = true;
    }

    fn load_viewer_frame(&mut self) {
        self.viewer.frame = None;
        let Some(photo) = self.selected_photo() else {
            return;
        };
        if self.status[self.selected] == PhotoStatus::Ready {
            let path = cache_file(&self.cache_directory, photo.key());
            self.viewer.frame = cache::read_frame(&path, self.fit).ok();
        }
    }

    /// One button in the viewer. Returns true when the viewer should close.
    pub fn apply_viewer(&mut self, event: ButtonEvent) -> bool {
        if self.viewer.confirm_delete {
            if event == ButtonEvent::Select {
                self.delete_request = self.selected_photo().map(|photo| photo.name.clone());
                self.viewer.confirm_delete = false;
            }
            return false;
        }
        if let Some(highlighted) = self.viewer.action {
            let count = PhotoAction::ALL.len();
            match event {
                ButtonEvent::Up => self.viewer.action = Some((highlighted + count - 1) % count),
                ButtonEvent::Down => self.viewer.action = Some((highlighted + 1) % count),
                ButtonEvent::Select => self.run_action(PhotoAction::ALL[highlighted]),
            }
            return false;
        }
        let count = self.photos.len();
        if count == 0 {
            return true;
        }
        match event {
            ButtonEvent::Up => self.selected = (self.selected + count - 1) % count,
            ButtonEvent::Down => self.selected = (self.selected + 1) % count,
            ButtonEvent::Select => {
                self.viewer.action = Some(0);
                return false;
            }
        }
        self.load_viewer_frame();
        self.load_visible();
        self.jobs_changed = true;
        self.full_refresh = true;
        false
    }

    fn run_action(&mut self, action: PhotoAction) {
        let Some(name) = self.selected_photo().map(|photo| photo.name.clone()) else {
            return;
        };
        match action {
            PhotoAction::ToggleStar => {
                self.starred.toggle(&name);
                self.starred_changed = true;
            }
            PhotoAction::OnlyThis => {
                self.starred.only(&name);
                self.starred_changed = true;
            }
            PhotoAction::Delete => {
                self.viewer.confirm_delete = true;
                return;
            }
        }
        self.viewer.action = None;
    }

    /// Hold BOOT in the viewer: close the confirmation or the action list
    /// first. Returns true when something was closed.
    pub fn close_viewer_layer(&mut self) -> bool {
        if self.viewer.confirm_delete {
            self.viewer.confirm_delete = false;
            true
        } else {
            self.viewer.action.take().is_some()
        }
    }

    /// After `main.rs` deleted the requested file: drop it from the list, its
    /// star and its cache file. Returns false when the list is now empty.
    pub fn finish_delete(&mut self, deleted: bool) -> bool {
        let Some(name) = self.delete_request.take() else {
            return !self.photos.is_empty();
        };
        if deleted {
            if let Some(index) = self.photos.iter().position(|photo| photo.name == name) {
                let key = self.photos[index].key();
                let _ = fs::remove_file(cache_file(&self.cache_directory, key));
                self.photos.remove(index);
                self.status.remove(index);
            }
            if self.starred.contains(&name) {
                self.starred.remove(&name);
                self.starred_changed = true;
            }
        }
        self.selected = self.selected.min(self.photos.len().saturating_sub(1));
        self.viewer = PhotoViewer::default();
        self.load_viewer_frame();
        self.load_visible();
        self.full_refresh = true;
        !self.photos.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{PhotoAction, PhotoStatus, PhotosUiState};
    use crate::{
        buttons::ButtonEvent,
        photos::{
            test_photos::grey_jpeg,
            worker::{prepare, PhotoJobResult},
        },
    };

    fn gallery(name: &str, count: usize) -> PhotosUiState {
        let unique = format!("wave-gallery-{name}-{}", std::process::id());
        let root = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&root);
        let photos = root.join("PHOTOS");
        fs::create_dir_all(&photos).unwrap();
        let jpeg = grey_jpeg(96, 64, None);
        for index in 0..count {
            fs::write(photos.join(format!("IMG_{index:04}.jpg")), &jpeg).unwrap();
        }
        let mut state = PhotosUiState::with_roots(photos, root.join("CACHE"));
        state.refresh();
        state
    }

    #[test]
    fn empty_and_missing_folders_explain_themselves() {
        let mut state = gallery("empty", 0);
        let note = state.note.clone().unwrap();
        assert!(note.starts_with("No JPEG photos in"), "{note}");
        let _ = fs::remove_dir_all(state.photos_directory());
        state.refresh();
        assert!(state.note.unwrap().ends_with("folder on the SD card"));
    }

    #[test]
    fn grid_pages_follow_the_selection_and_jobs_start_with_the_page() {
        let mut state = gallery("pages", 8);
        assert_eq!(state.page_count(), 2);
        assert!(state.take_jobs_changed());
        for _ in 0..6 {
            assert!(!state.apply_grid(ButtonEvent::Down));
        }
        assert_eq!(state.page(), 1);
        assert_eq!(state.visible(), 6..8);
        let jobs = state.cache_jobs();
        assert_eq!(jobs.len(), 8);
        assert_eq!(jobs[0].name, state.photos[6].name);
        assert!(state.apply_grid(ButtonEvent::Select));
    }

    #[test]
    fn prepared_photos_show_thumbnails_and_open_in_the_viewer() {
        let mut state = gallery("viewer", 2);
        let job = state.cache_jobs().remove(0);
        let result = prepare(state.photos_directory(), state.cache_directory(), &job);
        assert_eq!(result, PhotoJobResult::Ready { key: job.key });
        assert!(state.on_job_result(&result));
        assert!(state.thumbnail(job.key).is_some());
        assert!(state.info_label().unwrap().contains("96\u{d7}64"));

        state.open_viewer();
        assert!(state.viewer.frame.is_some());
        assert!(state.take_full_refresh());
        state.toggle_star();
        assert_eq!(state.starred_count(), 1);
        assert!(state.take_starred_changed());
    }

    #[test]
    fn viewer_actions_star_and_delete_with_confirmation() {
        let mut state = gallery("actions", 2);
        state.open_viewer();
        state.apply_viewer(ButtonEvent::Select);
        assert_eq!(state.viewer.action, Some(0));
        assert_eq!(PhotoAction::ALL[0].label(false), "Add to sleep set");
        state.apply_viewer(ButtonEvent::Select);
        assert_eq!(state.starred_count(), 1);
        assert_eq!(state.viewer.action, None);

        state.apply_viewer(ButtonEvent::Select);
        state.apply_viewer(ButtonEvent::Up);
        state.apply_viewer(ButtonEvent::Select);
        assert!(state.viewer.confirm_delete);
        assert!(state.close_viewer_layer());
        assert_eq!(state.viewer.action, Some(2));

        state.apply_viewer(ButtonEvent::Select);
        state.apply_viewer(ButtonEvent::Select);
        let name = state.delete_request().unwrap().to_string();
        fs::remove_file(state.photos_directory().join(&name)).unwrap();
        assert!(state.finish_delete(true));
        assert_eq!(state.photos.len(), 1);
        assert_eq!(state.starred_count(), 0);
        assert_eq!(state.status, [PhotoStatus::Pending]);
    }
}
