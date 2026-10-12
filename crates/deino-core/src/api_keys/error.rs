//! What can go wrong with API keys.

use deino_db::DbError;
use deino_domain::{ApiKeyId, ErrorClass, ErrorCode};

use super::MAX_KEY_NAME_CHARS;

/// An API key failure. Messages are user-safe: never a token or a hash.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiKeyError {
    /// No key has the ID.
    #[error("API key {0} was not found")]
    NotFound(ApiKeyId),
    /// The name is blank or longer than [`MAX_KEY_NAME_CHARS`].
    #[error("a key name must be 1 to {MAX_KEY_NAME_CHARS} characters long")]
    InvalidName,
    /// The token is missing, malformed, unknown, wrong or revoked. One
    /// answer for all of them, so a caller cannot probe which.
    #[error("the API key is missing, invalid or revoked")]
    Unauthorized,
    /// Hashing or the random source failed; the source says how.
    #[error("could not secure the API key")]
    Crypto(String),
    /// The database failed; the source carries the detail.
    #[error("could not load or save API keys")]
    Storage(#[from] DbError),
}

impl ErrorClass for ApiKeyError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::InvalidName => ErrorCode::Invalid,
            Self::Unauthorized => ErrorCode::PermissionDenied,
            Self::Crypto(_) | Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
