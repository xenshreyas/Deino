//! What can go wrong editing notes by asking. Every message is user-safe:
//! no notes, transcript or provider bodies.

use deino_db::DbError;
use deino_doc::DocError;
use deino_domain::{ErrorClass, ErrorCode};
use deino_llm::LlmError;

use crate::enhance::EnhanceError;

/// An edit failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The request is empty.
    #[error("Say what to change in the notes.")]
    EmptyInstruction,
    /// The meeting has no enhanced notes.
    #[error("There are no enhanced notes to edit yet.")]
    NoNotes,
    /// The version is not the newest: the notes changed since.
    #[error("The notes changed since you asked. Ask again on the newest version.")]
    NotLatest,
    /// The AI (or the rename) changed nothing.
    #[error("Nothing in the notes matched that request. Try saying it another way.")]
    NoChange,
    /// The edited document breaks the editor's schema.
    #[error("the edited notes are not a valid document")]
    InvalidDoc(#[source] DocError),
    /// The model call, or reading and storing the notes, failed.
    #[error("{0}")]
    Notes(#[from] EnhanceError),
}

impl From<DbError> for EditError {
    fn from(error: DbError) -> Self {
        Self::Notes(EnhanceError::Storage(error))
    }
}

impl From<LlmError> for EditError {
    fn from(error: LlmError) -> Self {
        Self::Notes(EnhanceError::Llm(error))
    }
}

impl ErrorClass for EditError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EmptyInstruction | Self::NoChange | Self::InvalidDoc(_) => ErrorCode::Invalid,
            Self::NoNotes => ErrorCode::NotFound,
            Self::NotLatest => ErrorCode::Conflict,
            Self::Notes(error) => error.code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_llm::ProviderId;

    use super::*;

    #[test]
    fn a_model_failure_says_where_to_fix_it() {
        let error = EditError::from(LlmError::Auth(ProviderId::Anthropic));

        assert!(error.to_string().contains("Settings → AI"), "{error}");
        assert_eq!(EditError::NotLatest.code(), ErrorCode::Conflict);
    }
}
