//! What can go wrong in the example feature.
//!
//! Every variant documents when it happens, and its `#[error]` text is the
//! user-safe message the UI receives: IDs and limits, never user content.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use super::MAX_NOTE_CHARS;

/// An example-feature failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExampleError {
    /// The note text is blank once whitespace is trimmed.
    #[error("a note cannot be empty")]
    EmptyText,
    /// The note text is longer than [`MAX_NOTE_CHARS`].
    #[error("a note can be at most {MAX_NOTE_CHARS} characters long")]
    TextTooLong,
    /// The database failed; the source carries the detail for the log.
    #[error("could not load or save notes")]
    Storage(#[from] DbError),
}

impl ErrorClass for ExampleError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EmptyText | Self::TextTooLong => ErrorCode::Invalid,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
