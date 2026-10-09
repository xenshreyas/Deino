//! What can go wrong when storing or reading a secret.
//!
//! Messages are user-safe: they name the secret (`OpenAI API key`), never
//! its value. Platform errors stay in the source chain for the log.

use std::error::Error;

use deino_domain::{ErrorClass, ErrorCode};

use crate::{MAX_SECRET_CHARS, SecretName};

/// A platform error carried as a source, for the log only.
pub(crate) type PlatformError = Box<dyn Error + Send + Sync>;

/// A secrets failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SecretError {
    /// The value is blank once whitespace is trimmed.
    #[error("a key cannot be empty")]
    Empty,
    /// The value is longer than [`MAX_SECRET_CHARS`].
    #[error("a key can be at most {MAX_SECRET_CHARS} characters long")]
    TooLong,
    /// The secret exists, but the OS refused to hand it over: the user
    /// clicked Deny, or the app's signature changed since the key was saved
    /// (ad-hoc builds). Entering the key again fixes it.
    #[error("macOS denied access to the saved {}; enter it again in Settings", .0.label())]
    AccessDenied(SecretName),
    /// The keychain itself is missing, locked or read-only.
    #[error("the keychain is not available")]
    Unavailable(#[source] PlatformError),
    /// Any other keychain failure.
    #[error("could not reach the keychain")]
    Backend(#[source] PlatformError),
}

impl ErrorClass for SecretError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Empty | Self::TooLong => ErrorCode::Invalid,
            Self::AccessDenied(_) => ErrorCode::PermissionDenied,
            Self::Unavailable(_) => ErrorCode::Unavailable,
            Self::Backend(_) => ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::AppError;

    use super::*;

    #[test]
    fn access_denied_names_the_secret_and_asks_for_it_again() {
        let error = AppError::from(SecretError::AccessDenied(SecretName::OpenAiApiKey));

        assert_eq!(error.code, ErrorCode::PermissionDenied);
        assert_eq!(
            error.message,
            "macOS denied access to the saved OpenAI API key; enter it again in Settings"
        );
    }
}
