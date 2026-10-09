//! The diarization model store: `<app_data_dir>/models/diarization/`.
//!
//! Downloads reuse `deino-core`'s verified downloader
//! (`deino_core::models::download::fetch`) over the same [`ByteSource`] port
//! as the transcription models: bytes collect in a `.partial` file, resume
//! after an interruption, and get their real name only once the size and the
//! SHA-256 match the pin. As in `deino_core::models::ModelStore`, a file with
//! its real name passed those checks when it arrived, so it counts as present
//! by its exact size alone.

use std::path::PathBuf;

use deino_core::models::download::{Control, Download, Limits, fetch};
use deino_core::models::{ByteSource, ModelError as FetchError, Sha256Digest};
use deino_domain::{ErrorClass, ErrorCode};
use tokio_util::sync::CancellationToken;

use super::{CATALOG, Catalog, ModelFile};
use crate::pipeline::ModelPaths;

/// Why the models are not usable.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// A model is not downloaded.
    #[error("the speaker model {0} is not downloaded")]
    Missing(&'static str),
    /// Downloading or verifying a model failed (a corrupt download is
    /// deleted; an interrupted one is kept for a resume).
    #[error("downloading the speaker model {file} failed")]
    Download {
        /// The model file.
        file: &'static str,
        /// What went wrong.
        #[source]
        source: FetchError,
    },
    /// Reading the models folder failed.
    #[error("the speaker model files could not be read")]
    Io(#[from] std::io::Error),
}

impl ErrorClass for StoreError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Missing(_) => ErrorCode::Unavailable,
            Self::Download { source, .. } => source.code(),
            Self::Io(_) => ErrorCode::Internal,
        }
    }
}

/// The folder holding the diarization models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelStore {
    dir: PathBuf,
    catalog: Catalog,
}

impl ModelStore {
    /// A store of Deino's pinned models in `dir` (created on the first
    /// download).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::with_catalog(dir, CATALOG)
    }

    /// A store of other pinned files (a mirror, or tests).
    pub fn with_catalog(dir: impl Into<PathBuf>, catalog: Catalog) -> Self {
        Self {
            dir: dir.into(),
            catalog,
        }
    }

    /// Where `model` lives in the store.
    pub fn path(&self, model: &ModelFile) -> PathBuf {
        self.dir.join(model.file_name)
    }

    /// Both models' paths, whether or not they are present.
    pub fn paths(&self) -> ModelPaths {
        ModelPaths {
            segmentation: self.path(&self.catalog.segmentation),
            embedding: self.path(&self.catalog.embedding),
        }
    }

    /// The models' paths once both are present.
    pub fn verified_paths(&self) -> Result<ModelPaths, StoreError> {
        for model in self.catalog.files() {
            if !self.is_present(&model)? {
                return Err(StoreError::Missing(model.file_name));
            }
        }
        Ok(self.paths())
    }

    /// Whether `model` is in the store at its pinned size.
    pub fn is_present(&self, model: &ModelFile) -> Result<bool, StoreError> {
        match std::fs::metadata(self.path(model)) {
            Ok(metadata) => Ok(metadata.len() == model.bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// Downloads every missing model through `source`, verified against its
    /// pin, then returns both paths.
    pub async fn ensure(&self, source: &dyn ByteSource) -> Result<ModelPaths, StoreError> {
        for model in self.catalog.files() {
            if !self.is_present(&model)? {
                self.fetch(&model, source).await?;
            }
        }
        Ok(self.paths())
    }

    async fn fetch(&self, model: &ModelFile, source: &dyn ByteSource) -> Result<(), StoreError> {
        let failed = |source| StoreError::Download {
            file: model.file_name,
            source,
        };
        let sha256: Sha256Digest = model.sha256.parse().map_err(failed)?;
        std::fs::create_dir_all(&self.dir)?;
        let dest = self.path(model);
        let job = Download {
            url: model.url,
            dest: &dest,
            size: model.bytes,
            sha256,
        };
        let cancel = CancellationToken::new();
        let mut control = Control {
            limits: Limits::default(),
            cancel: &cancel,
            progress: &mut |_| {},
        };
        log::info!(target: "deino_diarize", "downloading speaker model {}", model.file_name);
        fetch(source, &job, &mut control).await.map_err(failed)
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
