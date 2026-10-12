//! In-memory fakes of the watch ports, for tests.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{Seconds, WatchedFileId, WatchedFolderId};

use super::{HandledFile, Sighting, WatchFailure, WatchFs, WatchRepo, WatchedFolder};
use crate::import::{FileImporter, ImportError, ImportRequest, ImportedMeeting};

/// Locks `mutex`, ignoring poisoning (a test that panicked already failed).
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Folders as maps of file name to (size, modified); moves are remembered.
#[derive(Debug, Default)]
pub struct FakeWatchFs {
    folders: Mutex<HashMap<PathBuf, Vec<Sighting>>>,
    /// Files moved to `Processed/`, in order.
    pub moved: Mutex<Vec<PathBuf>>,
}

impl FakeWatchFs {
    /// Creates an empty folder.
    pub fn add_folder(&self, folder: &Path) {
        lock(&self.folders).entry(folder.to_path_buf()).or_default();
    }

    /// Puts a file into its folder, replacing one at the same path.
    pub fn put(&self, file: &Path, size_bytes: u64, modified_ms: i64) {
        let folder = file.parent().unwrap_or(Path::new("/")).to_path_buf();
        let mut folders = lock(&self.folders);
        let files = folders.entry(folder).or_default();
        files.retain(|sighting| sighting.path != file);
        files.push(Sighting {
            path: file.to_path_buf(),
            size_bytes,
            modified_ms,
        });
    }
}

impl WatchFs for FakeWatchFs {
    fn is_folder(&self, path: &Path) -> bool {
        lock(&self.folders).contains_key(path)
    }

    fn scan(&self, folder: &Path) -> io::Result<Vec<Sighting>> {
        lock(&self.folders)
            .get(folder)
            .cloned()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn move_to_processed(&self, file: &Path, folder: &Path) -> io::Result<PathBuf> {
        if let Some(files) = lock(&self.folders).get_mut(folder) {
            files.retain(|sighting| sighting.path != file);
        }
        lock(&self.moved).push(file.to_path_buf());
        Ok(folder
            .join(super::PROCESSED_FOLDER)
            .join(file.file_name().unwrap_or_default()))
    }
}

/// Watched folders and the ledger in memory.
#[derive(Debug, Default)]
pub struct FakeWatchRepo {
    folders: Mutex<Vec<WatchedFolder>>,
    /// The ledger, with each row's dismissal time.
    pub files: Mutex<Vec<(HandledFile, Option<DateTime<Utc>>)>>,
}

#[async_trait]
impl WatchRepo for FakeWatchRepo {
    async fn folders(&self) -> Result<Vec<WatchedFolder>, DbError> {
        Ok(lock(&self.folders).clone())
    }

    async fn add_folder(&self, folder: &WatchedFolder) -> Result<(), DbError> {
        lock(&self.folders).push(folder.clone());
        Ok(())
    }

    async fn remove_folder(&self, id: &WatchedFolderId) -> Result<bool, DbError> {
        let mut folders = lock(&self.folders);
        let before = folders.len();
        folders.retain(|folder| folder.id != *id);
        lock(&self.files).retain(|(file, _)| file.folder_id != *id);
        Ok(folders.len() < before)
    }

    async fn set_move_to_processed(&self, id: &WatchedFolderId, on: bool) -> Result<bool, DbError> {
        let mut folders = lock(&self.folders);
        let found = folders.iter_mut().find(|folder| folder.id == *id);
        Ok(found.map(|folder| folder.move_to_processed = on).is_some())
    }

    async fn handled(&self, folder: &WatchedFolderId, file: &Sighting) -> Result<bool, DbError> {
        let files = lock(&self.files);
        Ok(files
            .iter()
            .any(|(row, _)| row.folder_id == *folder && row.sighting == *file))
    }

    async fn record(&self, file: &HandledFile) -> Result<(), DbError> {
        if !self.handled(&file.folder_id, &file.sighting).await? {
            lock(&self.files).push((file.clone(), None));
        }
        Ok(())
    }

    async fn failures(&self) -> Result<Vec<WatchFailure>, DbError> {
        let files = lock(&self.files);
        let mut failures: Vec<WatchFailure> = files
            .iter()
            .filter(|(_, dismissed)| dismissed.is_none())
            .filter_map(|(row, _)| {
                Some(WatchFailure {
                    id: row.id,
                    folder_id: row.folder_id,
                    path: row.sighting.path.to_string_lossy().into_owned(),
                    failure: row.failure?,
                    failed_at: row.handled_at,
                })
            })
            .collect();
        failures.reverse();
        Ok(failures)
    }

    async fn forget_failure(&self, id: &WatchedFileId) -> Result<Option<WatchedFolderId>, DbError> {
        let mut files = lock(&self.files);
        let index = files
            .iter()
            .position(|(row, _)| row.id == *id && row.failure.is_some());
        Ok(index.map(|index| files.remove(index).0.folder_id))
    }

    async fn dismiss_failure(
        &self,
        id: &WatchedFileId,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let mut files = lock(&self.files);
        let row = files
            .iter_mut()
            .find(|(row, dismissed)| row.id == *id && row.failure.is_some() && dismissed.is_none());
        Ok(row.map(|(_, dismissed)| *dismissed = Some(at)).is_some())
    }
}

/// Imports files by name: `fail()` for paths added with
/// [`FakeImporter::fail_on`], a new meeting for the rest.
#[derive(Debug, Default)]
pub struct FakeImporter {
    failing: Mutex<HashMap<PathBuf, fn() -> ImportError>>,
    /// Every file it was asked to import, in order.
    pub imported: Mutex<Vec<PathBuf>>,
}

impl FakeImporter {
    /// Makes importing `path` fail with `error()`.
    pub fn fail_on(&self, path: impl Into<PathBuf>, error: fn() -> ImportError) {
        lock(&self.failing).insert(path.into(), error);
    }

    /// Lets `path` import again.
    pub fn heal(&self, path: &Path) {
        lock(&self.failing).remove(path);
    }
}

#[async_trait]
impl FileImporter for FakeImporter {
    async fn import(&self, request: &ImportRequest) -> Result<ImportedMeeting, ImportError> {
        let count = {
            let mut imported = lock(&self.imported);
            imported.push(request.source.clone());
            imported.len()
        };
        if let Some(error) = lock(&self.failing).get(&request.source) {
            return Err(error());
        }
        Ok(ImportedMeeting {
            meeting_id: deino_domain::MeetingId::new(format!("meeting-{count}")),
            title: "Imported".into(),
            duration: Seconds(60),
            has_video: false,
        })
    }
}
