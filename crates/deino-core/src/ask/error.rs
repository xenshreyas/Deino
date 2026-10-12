//! What can go wrong when asking about a meeting.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_llm::LlmError;

use crate::recipes::RecipeError;

/// An ask failure. Messages are user-safe: IDs only, never the question,
/// the answer or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AskError {
    /// No meeting has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// The meeting has no notes and no transcript yet.
    #[error("this meeting has no notes or transcript to answer from yet")]
    NothingToRead,
    /// Neither a question nor a recipe.
    #[error("type a question or choose a recipe")]
    EmptyQuestion,
    /// The recipe could not be run.
    #[error(transparent)]
    Recipe(#[from] RecipeError),
    /// The model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the conversation")]
    Storage(#[from] DbError),
}

impl ErrorClass for AskError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::NothingToRead => ErrorCode::Unavailable,
            Self::EmptyQuestion => ErrorCode::Invalid,
            Self::Recipe(error) => error.code(),
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
