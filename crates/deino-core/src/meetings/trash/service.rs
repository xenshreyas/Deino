//! [`TrashService`]: the trash's rules (Granola §3.6). Deleting moves
//! meetings to the trash; restoring brings them back with every row intact;
//! deleting for good, by hand or by the 30-day purge, removes the rows
//! first and then the files, so a crash in between never leaves a meeting
//! pointing at missing audio.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId};

use super::{MeetingFiles, TRASH_RETENTION, TrashBin, TrashOutcome, TrashedMeeting};
use crate::meetings::{MeetingError, MeetingWork};

/// What the trash works through.
pub struct TrashDeps {
    /// The meetings' rows.
    pub bin: Arc<dyn TrashBin>,
    /// Their files on disk.
    pub files: Arc<dyn MeetingFiles>,
    /// The time trashing is stamped with and the purge counts from.
    pub clock: Arc<dyn Clock>,
    /// The pipeline, whose running stage trashing a meeting cancels.
    pub work: Arc<dyn MeetingWork>,
}

/// Trashes, restores and deletes meetings.
pub struct TrashService {
    deps: TrashDeps,
}

impl TrashService {
    /// The service over its ports.
    pub fn new(deps: TrashDeps) -> Self {
        Self { deps }
    }

    /// Moves `ids` to the trash and cancels their running pipeline stage.
    /// Search, exports, the CLI, related meetings and people's "last met"
    /// all skip trashed meetings at read time.
    pub async fn trash(&self, ids: &[MeetingId]) -> Result<TrashOutcome, MeetingError> {
        let changed = self.deps.bin.trash_all(ids, self.deps.clock.now()).await?;
        for meeting in &changed {
            self.deps.work.cancel(meeting);
        }
        log::info!(target: "meetings", "moved {} meetings to the trash", changed.len());
        Ok(TrashOutcome { changed })
    }

    /// Takes `ids` out of the trash. Nothing was removed when they went in,
    /// so they come back whole: transcript, notes, tags, folders, people.
    pub async fn restore(&self, ids: &[MeetingId]) -> Result<TrashOutcome, MeetingError> {
        let changed = self
            .deps
            .bin
            .restore_all(ids, self.deps.clock.now())
            .await?;
        log::info!(target: "meetings", "restored {} meetings from the trash", changed.len());
        Ok(TrashOutcome { changed })
    }

    /// The trash, most recently trashed first.
    pub async fn list(&self) -> Result<Vec<TrashedMeeting>, MeetingError> {
        Ok(self.deps.bin.trashed().await?)
    }

    /// Deletes those of `ids` that are in the trash for good, with their
    /// recordings and note images. Meetings outside the trash are skipped.
    pub async fn delete_forever(&self, ids: &[MeetingId]) -> Result<TrashOutcome, MeetingError> {
        let files = self.deps.bin.stored_files(ids).await?;
        let changed = self.deps.bin.delete_trashed(ids).await?;
        for stored in files
            .iter()
            .filter(|files| changed.contains(&files.meeting))
        {
            if let Err(error) = self.deps.files.delete(stored).await {
                // The rows are gone; a leftover file is logged, never fatal.
                log::warn!(target: "meetings", "could not delete files of {}: {error}", stored.owner);
            }
        }
        log::info!(target: "meetings", "deleted {} meetings for good", changed.len());
        Ok(TrashOutcome { changed })
    }

    /// Deletes every meeting that has been in the trash longer than
    /// [`TRASH_RETENTION`].
    pub async fn purge_expired(&self) -> Result<TrashOutcome, MeetingError> {
        let cutoff = self.deps.clock.now() - TRASH_RETENTION;
        let expired = self.deps.bin.expired(cutoff).await?;
        if expired.is_empty() {
            return Ok(TrashOutcome::default());
        }
        self.delete_forever(&expired).await
    }
}
