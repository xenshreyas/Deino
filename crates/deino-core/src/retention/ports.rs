//! The ports retention works through besides its store, and their
//! implementations over the settings, the trash and the app's folders.

use std::io;

use async_trait::async_trait;
use deino_domain::MeetingId;

use crate::meetings::trash::StoredFiles;
use crate::meetings::{DiskMeetingFiles, MeetingError, TrashService};
use crate::settings::{RetentionSettings, SettingsService};

/// The retention periods the user chose.
#[async_trait]
pub trait RetentionPolicy: Send + Sync {
    /// `privacy.retention` as it is now.
    async fn retention(&self) -> RetentionSettings;
}

#[async_trait]
impl RetentionPolicy for SettingsService {
    async fn retention(&self) -> RetentionSettings {
        self.get().await.privacy.retention
    }
}

/// Deletes a meeting's recording files.
#[async_trait]
pub trait RecordingFiles: Send + Sync {
    /// Deletes `files` inside the app's recordings folder and the meeting's
    /// `recordings/<id>` folder; anything else stays. Missing files are not
    /// an error.
    async fn delete_recordings(&self, files: &StoredFiles) -> io::Result<()>;
}

#[async_trait]
impl RecordingFiles for DiskMeetingFiles {
    async fn delete_recordings(&self, files: &StoredFiles) -> io::Result<()> {
        Self::delete_recordings(self, files).await
    }
}

/// Moves meetings to the trash.
#[async_trait]
pub trait MeetingTrash: Send + Sync {
    /// Moves `ids` to the trash; returns those that were not there yet.
    async fn trash(&self, ids: &[MeetingId]) -> Result<Vec<MeetingId>, MeetingError>;
}

#[async_trait]
impl MeetingTrash for TrashService {
    async fn trash(&self, ids: &[MeetingId]) -> Result<Vec<MeetingId>, MeetingError> {
        Ok(Self::trash(self, ids).await?.changed)
    }
}
