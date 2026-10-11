//! What can go wrong naming and merging speakers.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, PersonId, SegmentId, SpeakerId};

use crate::voiceprints::VoiceprintError;

/// A speaker naming failure. Messages carry IDs, never names or text.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpeakerError {
    /// The meeting has no speaker with this ID.
    #[error("speaker {0} was not found in this meeting")]
    SpeakerNotFound(SpeakerId),
    /// The meeting has no segment with this ID.
    #[error("segment {0} was not found in this meeting")]
    SegmentNotFound(SegmentId),
    /// No person has this ID.
    #[error("person {0} was not found")]
    PersonNotFound(PersonId),
    /// A new person's name is blank or too long.
    #[error("a name must be between 1 and 100 characters")]
    InvalidName,
    /// A speaker cannot be merged into itself.
    #[error("a speaker cannot be merged into itself")]
    SameSpeaker,
    /// The user's microphone voice cannot be merged away.
    #[error("your own voice cannot be merged into another speaker")]
    MergingMe,
    /// The speaker has no pending suggestion.
    #[error("speaker {0} has no suggested name")]
    NoSuggestion(SpeakerId),
    /// Saving the voice profile failed.
    #[error(transparent)]
    Voiceprint(#[from] VoiceprintError),
    /// The database failed.
    #[error("could not load or save the meeting's speakers")]
    Storage(#[from] DbError),
}

impl ErrorClass for SpeakerError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::SpeakerNotFound(_)
            | Self::SegmentNotFound(_)
            | Self::PersonNotFound(_)
            | Self::NoSuggestion(_) => ErrorCode::NotFound,
            Self::InvalidName | Self::SameSpeaker | Self::MergingMe => ErrorCode::Invalid,
            Self::Voiceprint(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
