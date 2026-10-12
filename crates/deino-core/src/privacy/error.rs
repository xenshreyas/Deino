//! What stops the database from opening at startup.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};
use deino_secrets::SecretError;

/// Why the database cannot be prepared for opening.
///
/// A failed conversion is not an error (the file is unchanged and opens in
/// its current format); these are the cases where it cannot open at all.
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// The file is encrypted but no key is saved in the Keychain.
    #[error("the Deino database is encrypted and its key is not in the Keychain")]
    KeyMissing,
    /// The Keychain refused to hand over the key, or failed.
    #[error("the Keychain did not hand over the database key")]
    KeyUnreadable(#[source] SecretError),
    /// The saved key is not a database key.
    #[error("the saved database key is not valid")]
    KeyMalformed,
    /// The file could not be inspected or recovered.
    #[error("the database file could not be prepared")]
    Database(#[from] DbError),
}

impl ErrorClass for PrivacyError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::KeyMissing => ErrorCode::NotFound,
            Self::KeyUnreadable(_) => ErrorCode::PermissionDenied,
            Self::KeyMalformed | Self::Database(_) => ErrorCode::Internal,
        }
    }
}
