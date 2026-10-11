//! What can go wrong with voice profiles.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, VoiceprintId};

/// A voiceprint failure. Messages carry IDs, never names or vectors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VoiceprintError {
    /// No voiceprint has the requested ID.
    #[error("voiceprint {0} was not found")]
    NotFound(VoiceprintId),
    /// The voice cannot become a voiceprint: wrong size, not finite, or no
    /// model name.
    #[error("this voice cannot be saved as a voice profile")]
    Unusable,
    /// The database failed.
    #[error("could not load or save voice profiles")]
    Storage(#[from] DbError),
}

impl ErrorClass for VoiceprintError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Unusable => ErrorCode::Invalid,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
