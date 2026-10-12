//! What can go wrong importing the sample meeting.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

/// Why the sample meeting could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum SampleError {
    /// The bundled resource is not valid JSON of the expected shape.
    #[error("the bundled sample meeting does not parse")]
    Parse(#[from] serde_json::Error),
    /// The bundled resource parses but breaks a rule; names the rule.
    #[error("the bundled sample meeting is invalid: {0}")]
    Invalid(&'static str),
    /// A repository write failed; nothing of the sample is left behind.
    #[error("could not save the sample meeting")]
    Write(#[from] DbError),
}

impl ErrorClass for SampleError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}
