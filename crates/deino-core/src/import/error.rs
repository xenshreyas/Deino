//! Why an import failed.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, ImportFailure};

use crate::meetings::MeetingError;

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImportError {
    /// The file does not exist (any more).
    #[error("the file could not be found")]
    Missing,
    /// The file's extension is not an audio or video format imports accept.
    #[error("this kind of file cannot be imported")]
    UnsupportedFile,
    /// The file is larger than [`MAX_IMPORT_BYTES`](super::MAX_IMPORT_BYTES).
    #[error("the file is too large to import")]
    TooLarge,
    /// The file holds no audio.
    #[error("the file has no audio")]
    NoAudio,
    /// No decoder reads the file's container or codec.
    #[error("the file's format is not supported")]
    UnsupportedFormat,
    /// The file's audio is damaged.
    #[error("the file's audio could not be decoded")]
    Damaged(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// Importing needs the native audio encoder (macOS).
    #[error("importing recordings is only available on macOS")]
    Unavailable,
    /// The converted audio or the kept video could not be written.
    #[error("the recording could not be saved")]
    Files(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The meeting could not be created or started processing.
    #[error(transparent)]
    Meeting(#[from] MeetingError),
    /// The database failed.
    #[error("could not save the imported meeting")]
    Storage(#[from] DbError),
}

impl ImportError {
    /// What a watched folder's failure list shows for this error.
    pub fn failure(&self) -> ImportFailure {
        match self {
            Self::Missing => ImportFailure::Missing,
            Self::TooLarge => ImportFailure::TooLarge,
            Self::NoAudio => ImportFailure::NoAudio,
            Self::UnsupportedFile | Self::UnsupportedFormat | Self::Unavailable => {
                ImportFailure::Unsupported
            }
            Self::Damaged(_) => ImportFailure::Damaged,
            Self::Files(_) | Self::Meeting(_) | Self::Storage(_) => ImportFailure::Failed,
        }
    }
}

impl ErrorClass for ImportError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Missing => ErrorCode::NotFound,
            Self::UnsupportedFile
            | Self::TooLarge
            | Self::NoAudio
            | Self::UnsupportedFormat
            | Self::Damaged(_) => ErrorCode::Invalid,
            Self::Unavailable => ErrorCode::Unavailable,
            Self::Meeting(error) => error.code(),
            Self::Files(_) | Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
