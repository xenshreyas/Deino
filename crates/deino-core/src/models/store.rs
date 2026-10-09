//! Model files on disk: `<app_data_dir>/models/parakeet-unified/<file>`.
//!
//! A file with its real name passed the size and hash checks when it was
//! downloaded, so it counts as ready by its size alone (hashing 700 MB on
//! every launch would cost seconds). Anything mid-download is a `.partial`.

use std::io::ErrorKind;
use std::path::PathBuf;

use tokio::fs;

use super::download::partial_path;
use super::{CatalogFile, ModelError, ModelState};

/// The folder holding downloaded model files.
#[derive(Clone, Debug)]
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    /// A store in `root` (created on the first download).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Where `file` lives once downloaded.
    pub fn path(&self, file: &CatalogFile) -> PathBuf {
        self.root.join(&file.filename)
    }

    /// Creates the folder if needed.
    pub async fn ensure_root(&self) -> Result<(), ModelError> {
        Ok(fs::create_dir_all(&self.root).await?)
    }

    /// Whether `file` is downloaded, part-way, or missing.
    pub async fn state(&self, file: &CatalogFile) -> Result<ModelState, ModelError> {
        let path = self.path(file);
        if len(&path).await? == Some(file.size_bytes) {
            return Ok(ModelState::Ready);
        }
        Ok(match len(&partial_path(&path)).await? {
            Some(downloaded_bytes) => ModelState::Partial { downloaded_bytes },
            None => ModelState::Missing,
        })
    }

    /// Deletes `file` and any partial download of it.
    pub async fn delete(&self, file: &CatalogFile) -> Result<(), ModelError> {
        let path = self.path(file);
        remove(&path).await?;
        remove(&partial_path(&path)).await
    }
}

async fn len(path: &std::path::Path) -> Result<Option<u64>, ModelError> {
    match fs::metadata(path).await {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn remove(path: &std::path::Path) -> Result<(), ModelError> {
    match fs::remove_file(path).await {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}
