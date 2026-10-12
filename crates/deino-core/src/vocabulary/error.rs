//! What can go wrong with the vocabulary. Messages are user-safe: limits and
//! IDs, never the user's words.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, VocabTermId};

use super::{MAX_TERM_CHARS, MAX_VARIANTS};

/// A vocabulary failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VocabError {
    /// The spelling or a variant is blank once whitespace is trimmed.
    #[error("a vocabulary entry cannot be empty")]
    EmptyTerm,
    /// The spelling or a variant is longer than [`MAX_TERM_CHARS`].
    #[error("a vocabulary entry can be at most {MAX_TERM_CHARS} characters long")]
    TermTooLong,
    /// More than [`MAX_VARIANTS`] sounds-like variants.
    #[error("an entry can have at most {MAX_VARIANTS} sounds-like spellings")]
    TooManyVariants,
    /// A replacement rule without anything to replace.
    #[error("a replacement needs at least one word to replace")]
    NothingToReplace,
    /// An entry of the same kind is already spelled this way.
    #[error("this word is already in your vocabulary")]
    Duplicate,
    /// No entry has this ID.
    #[error("vocabulary entry {0} does not exist")]
    NotFound(VocabTermId),
    /// The database failed; the source carries the detail for the log.
    #[error("could not load or save the vocabulary")]
    Storage(#[source] DbError),
}

impl From<DbError> for VocabError {
    fn from(error: DbError) -> Self {
        if error.is_unique_violation() {
            Self::Duplicate
        } else {
            Self::Storage(error)
        }
    }
}

impl ErrorClass for VocabError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EmptyTerm
            | Self::TermTooLong
            | Self::TooManyVariants
            | Self::NothingToReplace => ErrorCode::Invalid,
            Self::Duplicate => ErrorCode::Conflict,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
