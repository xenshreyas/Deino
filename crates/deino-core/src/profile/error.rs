//! What can go wrong when saving the profile.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use crate::settings::SettingsError;

/// A profile failure. Messages name fields and positions, never what the
/// user typed (emails count as content).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProfileError {
    /// The address at this 1-based position does not parse.
    #[error("email address {0} is not valid")]
    InvalidEmail(usize),
    /// The profile breaks a settings rule, or could not be saved.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The "Me" person could not be read or written.
    #[error("could not update your person")]
    Storage(#[from] DbError),
}

impl ErrorClass for ProfileError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidEmail(_) => ErrorCode::Invalid,
            Self::Settings(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
