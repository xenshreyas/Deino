//! [`FakeIngest`]: an in-memory [`MediaIngest`] for tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;

use super::{ImportError, IngestedMedia, MediaFacts, MediaIngest, MediaKind};

/// A known file: its facts, and its length or the error converting it gives.
type Entry = (MediaFacts, Result<Duration, fn() -> ImportError>);

/// Files it knows, each with its facts and length; anything else is
/// missing. Remembers every folder it wrote and discarded.
#[derive(Debug, Default)]
pub struct FakeIngest {
    files: Mutex<HashMap<PathBuf, Entry>>,
    /// Folders written to, in order.
    pub written: Mutex<Vec<PathBuf>>,
    /// Folders discarded, in order.
    pub discarded: Mutex<Vec<PathBuf>>,
}

impl FakeIngest {
    /// Adds a readable file of `duration`.
    pub fn add(&self, path: impl Into<PathBuf>, facts: MediaFacts, duration: Duration) {
        self.lock().insert(path.into(), (facts, Ok(duration)));
    }

    /// Adds a file whose conversion fails with `error()`.
    pub fn add_broken(
        &self,
        path: impl Into<PathBuf>,
        facts: MediaFacts,
        error: fn() -> ImportError,
    ) {
        self.lock().insert(path.into(), (facts, Err(error)));
    }

    /// Forgets a file, as if it were deleted.
    pub fn remove(&self, path: &Path) {
        self.lock().remove(path);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, Entry>> {
        self.files.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl MediaIngest for FakeIngest {
    async fn facts(&self, path: &Path) -> Result<MediaFacts, ImportError> {
        self.lock()
            .get(path)
            .map(|(facts, _)| *facts)
            .ok_or(ImportError::Missing)
    }

    async fn ingest(
        &self,
        source: &Path,
        kind: MediaKind,
        folder: &Path,
    ) -> Result<IngestedMedia, ImportError> {
        let duration = match self.lock().get(source) {
            None => return Err(ImportError::Missing),
            Some((_, Err(error))) => return Err(error()),
            Some((_, Ok(duration))) => *duration,
        };
        self.written
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(folder.to_path_buf());
        Ok(IngestedMedia {
            audio: folder.join("audio.m4a"),
            video: (kind == MediaKind::Video).then(|| folder.join("video.mp4")),
            duration,
        })
    }

    async fn discard(&self, folder: &Path) {
        self.discarded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(folder.to_path_buf());
    }
}
