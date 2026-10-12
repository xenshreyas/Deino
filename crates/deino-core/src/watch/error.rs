//! Why a watched-folder operation failed.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

/// Why a watched folder could not be added, changed or scanned.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WatchError {
    /// The path is not a folder that exists.
    #[error("the folder could not be found")]
    NotAFolder,
    /// The folder is already watched.
    #[error("this folder is already watched")]
    AlreadyWatched,
    /// No watched folder or failure has the ID.
    #[error("the watched folder or file was not found")]
    NotFound,
    /// The folder could not be listed.
    #[error("the folder could not be read")]
    Unreadable(#[source] std::io::Error),
    /// The database failed.
    #[error("could not load or save watched folders")]
    Storage(#[from] DbError),
}

impl ErrorClass for WatchError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotAFolder | Self::Unreadable(_) => ErrorCode::Invalid,
            Self::AlreadyWatched => ErrorCode::Conflict,
            Self::NotFound => ErrorCode::NotFound,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
