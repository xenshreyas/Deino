//! What can go wrong with retention. Messages name meetings by ID only.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use crate::meetings::MeetingError;

/// A retention failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RetentionError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// Moving expired meetings to the trash failed.
    #[error("could not move expired meetings to the trash")]
    Trash(#[source] MeetingError),
    /// The database failed; the source carries the detail.
    #[error("could not read or update what retention keeps")]
    Storage(#[from] DbError),
}

impl ErrorClass for RetentionError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::Trash(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
