//! What can go wrong reading stats or writing the year in review.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};
use deino_llm::LlmError;

/// A stats failure. The `#[error]` text is what the UI shows.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StatsError {
    /// The year in review was asked for outside December without the
    /// debug flag.
    #[error("the year in review opens in December")]
    RecapNotYet,
    /// No model is set up, so the recap cannot be written.
    #[error("set up an AI model in Settings → AI to write the year in review")]
    NoModel,
    /// The model answered with nothing usable.
    #[error("the model's answer was empty; try again")]
    EmptyAnswer,
    /// The model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not read your stats")]
    Storage(#[from] DbError),
}

impl ErrorClass for StatsError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::RecapNotYet => ErrorCode::Conflict,
            Self::NoModel => ErrorCode::Unavailable,
            Self::EmptyAnswer => ErrorCode::Llm,
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
