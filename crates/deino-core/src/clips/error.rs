//! What can go wrong with a clip.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use super::CutFailed;
use crate::player::PlayerError;

/// A clip failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClipError {
    /// The database failed; the source carries the detail.
    #[error("could not save the clip")]
    Storage(#[from] DbError),
    /// The meeting's recording could not be looked up.
    #[error("could not find the meeting's recording")]
    Recording(#[from] PlayerError),
    /// The meeting has no recording on this Mac.
    #[error("this meeting has no recording to clip")]
    NoRecording,
    /// The clip's range is empty or reversed.
    #[error("choose a range to clip")]
    EmptyRange,
    /// The clip's name is empty.
    #[error("give the clip a name")]
    EmptyName,
    /// No such clip (or its meeting is in the trash).
    #[error("this clip no longer exists")]
    NotFound,
    /// The clip's file is not inside the app's recordings folder.
    #[error("this clip's file is missing")]
    FileMissing,
    /// Cutting the recording failed.
    #[error("could not cut the clip")]
    Cut(#[from] CutFailed),
    /// A file operation failed; the source carries the detail.
    #[error("could not write the clip's file")]
    File(#[source] std::io::Error),
}

impl ErrorClass for ClipError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) | Self::Cut(_) | Self::File(_) => ErrorCode::Internal,
            Self::Recording(error) => error.code(),
            Self::NoRecording | Self::NotFound | Self::FileMissing => ErrorCode::NotFound,
            Self::EmptyRange | Self::EmptyName => ErrorCode::Invalid,
        }
    }
}
