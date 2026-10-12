//! [`WatchService`]: the watched folders, their failures, and one scan.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use deino_domain::{Clock, WatchedFileId, WatchedFolderId};

use super::{
    FolderPicker, HandledFile, Sighting, StabilityTracker, WatchError, WatchFailure, WatchFs,
    WatchRepo, WatchedFolder,
};
use crate::import::{FileImporter, ImportRequest};

/// What the service works through.
#[derive(Clone)]
pub struct WatchDeps {
    /// Folders and the ledger.
    pub repo: Arc<dyn WatchRepo>,
    /// Listing folders and moving files.
    pub fs: Arc<dyn WatchFs>,
    /// Imports a ready file (the app's `ImportService`).
    pub importer: Arc<dyn FileImporter>,
    /// IDs, timestamps and the 10 s rule.
    pub clock: Arc<dyn Clock>,
}

/// Watches folders and imports what lands in them.
pub struct WatchService {
    deps: WatchDeps,
    /// Each folder's files and how long they have been unchanged.
    trackers: Mutex<HashMap<WatchedFolderId, StabilityTracker>>,
}

impl WatchService {
    /// The service over its ports.
    pub fn new(deps: WatchDeps) -> Self {
        Self {
            deps,
            trackers: Mutex::default(),
        }
    }

    /// Every watched folder, oldest first.
    pub async fn folders(&self) -> Result<Vec<WatchedFolder>, WatchError> {
        Ok(self.deps.repo.folders().await?)
    }

    /// Starts watching `path`. Files already in it are entered in the
    /// ledger as handled, so only recordings added from now on import.
    pub async fn add_folder(&self, path: &Path) -> Result<WatchedFolder, WatchError> {
        if !self.deps.fs.is_folder(path) {
            return Err(WatchError::NotAFolder);
        }
        let path = path.to_string_lossy().into_owned();
        if self
            .deps
            .repo
            .folders()
            .await?
            .iter()
            .any(|folder| folder.path == path)
        {
            return Err(WatchError::AlreadyWatched);
        }
        let existing = self
            .deps
            .fs
            .scan(Path::new(&path))
            .map_err(WatchError::Unreadable)?;
        let folder = WatchedFolder {
            id: WatchedFolderId::generate(self.deps.clock.as_ref()),
            path,
            move_to_processed: false,
            created_at: self.deps.clock.now(),
        };
        self.deps.repo.add_folder(&folder).await?;
        for sighting in existing {
            self.record(&folder.id, sighting, Ok(None)).await?;
        }
        log::info!(target: "watch", "watching folder {}", folder.id);
        Ok(folder)
    }

    /// Asks `picker` for a folder and watches it; `None` when cancelled.
    pub async fn add_picked(
        &self,
        picker: &dyn FolderPicker,
    ) -> Result<Option<WatchedFolder>, WatchError> {
        match picker.pick_folder().await {
            Some(path) => self.add_folder(&path).await.map(Some),
            None => Ok(None),
        }
    }

    /// Stops watching a folder and forgets its ledger.
    pub async fn remove_folder(&self, id: &WatchedFolderId) -> Result<(), WatchError> {
        if !self.deps.repo.remove_folder(id).await? {
            return Err(WatchError::NotFound);
        }
        self.trackers().remove(id);
        Ok(())
    }

    /// Turns "Move to processed" on or off for a folder.
    pub async fn set_move_to_processed(
        &self,
        id: &WatchedFolderId,
        on: bool,
    ) -> Result<(), WatchError> {
        if self.deps.repo.set_move_to_processed(id, on).await? {
            Ok(())
        } else {
            Err(WatchError::NotFound)
        }
    }

    /// Files that could not be imported, newest first.
    pub async fn failures(&self) -> Result<Vec<WatchFailure>, WatchError> {
        Ok(self.deps.repo.failures().await?)
    }

    /// Tries a failed file again: it imports on a later scan, once it has
    /// been unchanged for 10 s.
    pub async fn retry(&self, id: &WatchedFileId) -> Result<(), WatchError> {
        let folder = self
            .deps
            .repo
            .forget_failure(id)
            .await?
            .ok_or(WatchError::NotFound)?;
        // The tracker reported the file once; start it afresh.
        self.trackers().remove(&folder);
        Ok(())
    }

    /// Removes a failure from the list without trying again.
    pub async fn dismiss(&self, id: &WatchedFileId) -> Result<(), WatchError> {
        let now = self.deps.clock.now();
        if self.deps.repo.dismiss_failure(id, now).await? {
            Ok(())
        } else {
            Err(WatchError::NotFound)
        }
    }

    /// Scans every folder once and imports the files that are ready.
    pub async fn scan(&self) -> Result<(), WatchError> {
        for folder in self.deps.repo.folders().await? {
            self.scan_folder(&folder).await?;
        }
        Ok(())
    }

    /// Scans one folder; a folder that cannot be read (an unplugged drive)
    /// is skipped until it can.
    async fn scan_folder(&self, folder: &WatchedFolder) -> Result<(), WatchError> {
        let scan = match self.deps.fs.scan(Path::new(&folder.path)) {
            Ok(scan) => scan,
            Err(error) => {
                log::debug!(target: "watch", "cannot read folder {}: {error}", folder.id);
                return Ok(());
            }
        };
        let now = self.deps.clock.now();
        let ready = self
            .trackers()
            .entry(folder.id)
            .or_default()
            .settled(&scan, now);
        for sighting in ready {
            if !self.deps.repo.handled(&folder.id, &sighting).await? {
                self.import(folder, sighting).await?;
            }
        }
        Ok(())
    }

    /// Imports one ready file, enters the outcome in the ledger and moves
    /// the file aside when the folder asks.
    async fn import(&self, folder: &WatchedFolder, sighting: Sighting) -> Result<(), WatchError> {
        let request = ImportRequest {
            source: sighting.path.clone(),
            title: None,
        };
        let outcome = self.deps.importer.import(&request).await;
        let imported = outcome.is_ok();
        if let Err(error) = &outcome {
            log::warn!(target: "watch", "a file in folder {} failed to import: {error}", folder.id);
        }
        let path = sighting.path.clone();
        self.record(
            &folder.id,
            sighting,
            outcome.map(|meeting| Some(meeting.meeting_id)),
        )
        .await?;
        if imported && folder.move_to_processed {
            let moved = self
                .deps
                .fs
                .move_to_processed(&path, Path::new(&folder.path));
            if let Err(error) = moved {
                log::warn!(target: "watch", "could not move an imported file aside: {error}");
            }
        }
        Ok(())
    }

    /// Enters `sighting` in the folder's ledger with its outcome.
    async fn record(
        &self,
        folder: &WatchedFolderId,
        sighting: Sighting,
        outcome: Result<Option<deino_domain::MeetingId>, crate::import::ImportError>,
    ) -> Result<(), WatchError> {
        let (meeting_id, failure) = match outcome {
            Ok(meeting) => (meeting, None),
            Err(error) => (None, Some(error.failure())),
        };
        let file = HandledFile {
            id: WatchedFileId::generate(self.deps.clock.as_ref()),
            folder_id: *folder,
            sighting,
            meeting_id,
            failure,
            handled_at: self.deps.clock.now(),
        };
        Ok(self.deps.repo.record(&file).await?)
    }

    fn trackers(&self) -> std::sync::MutexGuard<'_, HashMap<WatchedFolderId, StabilityTracker>> {
        self.trackers.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
