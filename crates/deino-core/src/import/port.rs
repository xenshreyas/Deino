//! The ports an import works through.

use std::path::Path;

use async_trait::async_trait;

use super::{ImportError, ImportRequest, ImportedMeeting, IngestedMedia, MediaFacts, MediaKind};

/// Reads and converts media files (the file system and the decoders).
#[async_trait]
pub trait MediaIngest: Send + Sync {
    /// The file's size and modification time; [`ImportError::Missing`]
    /// when it does not exist.
    async fn facts(&self, path: &Path) -> Result<MediaFacts, ImportError>;

    /// Converts the audio of `source` into `folder`/`audio.m4a` (48 kHz
    /// mono AAC) and, for a video, copies the video beside it. Creates
    /// `folder`, and removes what it wrote there when it fails.
    async fn ingest(
        &self,
        source: &Path,
        kind: MediaKind,
        folder: &Path,
    ) -> Result<IngestedMedia, ImportError>;

    /// Removes `folder` and everything in it (an import that failed after
    /// its files were written).
    async fn discard(&self, folder: &Path);
}

/// The open dialog, filtered to the formats imports accept.
#[async_trait]
pub trait MediaPicker: Send + Sync {
    /// The chosen file, or `None` when the user cancelled.
    async fn pick_media(&self) -> Option<std::path::PathBuf>;
}

/// Imports one file; what a watched folder needs from [`super::ImportService`].
#[async_trait]
pub trait FileImporter: Send + Sync {
    /// Imports `request` as a new meeting.
    async fn import(&self, request: &ImportRequest) -> Result<ImportedMeeting, ImportError>;
}
