//! What can go wrong managing models. Messages are user-safe.

use deino_domain::{ErrorClass, ErrorCode};

use super::{FetchError, ModelFileId};

/// A model-management failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelError {
    /// The bundled catalog is malformed (a build defect).
    #[error("the model catalog is invalid: {0}")]
    InvalidCatalog(String),
    /// No catalog file has this ID.
    #[error("unknown model {0}")]
    UnknownModel(ModelFileId),
    /// The file is not downloaded.
    #[error("model {0} is not downloaded")]
    NotDownloaded(ModelFileId),
    /// The file is already downloading.
    #[error("model {0} is already downloading")]
    AlreadyDownloading(ModelFileId),
    /// The download was cancelled; what arrived is kept for a resume.
    #[error("the download was cancelled")]
    Cancelled,
    /// The transfer failed; what arrived is kept for a resume.
    #[error("the download failed: {0}")]
    Network(#[from] FetchError),
    /// The server sent more or less than the catalog size.
    #[error("the download was {received} bytes, expected {expected}")]
    SizeMismatch {
        /// The catalog size.
        expected: u64,
        /// What arrived.
        received: u64,
    },
    /// The file's content does not match the catalog hash; it was deleted.
    #[error("the downloaded file is corrupt and was deleted; try again")]
    HashMismatch,
    /// Reading or writing the models folder failed.
    #[error("could not read or write the models folder")]
    Io(#[from] std::io::Error),
}

impl ErrorClass for ModelError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::UnknownModel(_) => ErrorCode::NotFound,
            Self::NotDownloaded(_) => ErrorCode::Unavailable,
            Self::AlreadyDownloading(_) => ErrorCode::Conflict,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Network(_) | Self::SizeMismatch { .. } | Self::HashMismatch => ErrorCode::Network,
            Self::InvalidCatalog(_) | Self::Io(_) => ErrorCode::Internal,
        }
    }
}
