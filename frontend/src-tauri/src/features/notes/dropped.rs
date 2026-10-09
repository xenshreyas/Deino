//! Files dropped on the window: the only paths the notepad will read.
//!
//! With Tauri's native drag and drop, the webview learns a dropped file's
//! path but cannot read it (the fs scope is the app data folder). The window
//! event handler records each drop here; `attach_dropped_image` accepts a
//! path only if it is in the latest drop, so the webview can never make Rust
//! read an arbitrary file.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use deino_core::notes::{MAX_IMAGE_BYTES, NotesError};
use deino_domain::{ErrorClass, ErrorCode};

/// The paths of the most recent drop on any window.
#[derive(Debug, Default)]
pub struct DroppedFiles {
    paths: Mutex<Vec<PathBuf>>,
}

/// Why a dropped file could not become a note image.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DroppedImageError {
    /// The path was not part of the latest drop.
    #[error("that file was not dropped on the window")]
    NotDropped,
    /// The file is larger than an image may be.
    #[error("images can be at most {} MB", MAX_IMAGE_BYTES / 1024 / 1024)]
    TooLarge,
    /// The file could not be read.
    #[error("could not read the dropped file")]
    Unreadable(#[source] io::Error),
    /// The notes service refused or failed to store it.
    #[error(transparent)]
    Notes(#[from] NotesError),
}

impl ErrorClass for DroppedImageError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotDropped | Self::TooLarge => ErrorCode::Invalid,
            Self::Unreadable(_) => ErrorCode::Internal,
            Self::Notes(error) => error.code(),
        }
    }
}

impl DroppedFiles {
    /// Replaces the record with a new drop.
    pub fn remember(&self, paths: &[PathBuf]) {
        *self.paths.lock().unwrap_or_else(PoisonError::into_inner) = paths.to_vec();
    }

    /// Takes `path` out of the latest drop, if it is there.
    fn take(&self, path: &Path) -> Option<PathBuf> {
        let mut paths = self.paths.lock().unwrap_or_else(PoisonError::into_inner);
        let index = paths.iter().position(|dropped| dropped == path)?;
        Some(paths.swap_remove(index))
    }

    /// Reads a dropped file (once), refusing paths that were not dropped
    /// and files over [`MAX_IMAGE_BYTES`] before reading them.
    pub(crate) async fn read(&self, path: &Path) -> Result<Vec<u8>, DroppedImageError> {
        let path = self.take(path).ok_or(DroppedImageError::NotDropped)?;
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(DroppedImageError::Unreadable)?;
        if usize::try_from(metadata.len()).map_or(true, |len| len > MAX_IMAGE_BYTES) {
            return Err(DroppedImageError::TooLarge);
        }
        tokio::fs::read(&path)
            .await
            .map_err(DroppedImageError::Unreadable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn only_dropped_paths_are_read_and_only_once() {
        let folder = tempfile::tempdir().unwrap();
        let picture = folder.path().join("a.png");
        std::fs::write(&picture, b"png").unwrap();
        let dropped = DroppedFiles::default();

        let before = dropped.read(&picture).await.unwrap_err();
        dropped.remember(std::slice::from_ref(&picture));
        let bytes = dropped.read(&picture).await.unwrap();
        let again = dropped.read(&picture).await.unwrap_err();

        assert!(matches!(before, DroppedImageError::NotDropped));
        assert_eq!(bytes, b"png");
        assert!(matches!(again, DroppedImageError::NotDropped));
    }
}
