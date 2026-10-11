//! What can go wrong in search. Messages never quote the query or any
//! indexed text.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use crate::models::ModelError;

/// A search failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SearchError {
    /// The local embedding model is not downloaded (semantic indexing).
    #[error("the embedding model is not downloaded")]
    ModelMissing,
    /// Downloading, finding or deleting the embedding model failed.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// The embedding model failed or returned something unusable.
    #[error("the embedding model could not embed the text: {0}")]
    Embedding(String),
    /// The database failed; the source carries the detail.
    #[error("could not read or update the search index")]
    Storage(#[from] DbError),
    /// A write through adapters opened read-only (another process's
    /// [`Retriever`](super::Retriever)).
    #[error("the search index was opened read-only")]
    ReadOnly,
}

impl From<sqlx::Error> for SearchError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl SearchError {
    /// Whether trying again may succeed: a busy database, or a sidecar that
    /// failed and restarts. A missing model or a bad file needs the user.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Storage(_) | Self::Embedding(_))
    }
}

impl ErrorClass for SearchError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::ModelMissing => ErrorCode::Unavailable,
            Self::Model(error) => error.code(),
            Self::Embedding(_) => ErrorCode::Llm,
            Self::Storage(_) | Self::ReadOnly => ErrorCode::Internal,
        }
    }
}
