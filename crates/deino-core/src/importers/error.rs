//! What can go wrong importing from another note-taker.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, ImportSource};
use deino_net::NetError;
use deino_secrets::SecretError;

use crate::transcript_import::TranscriptImportError;

/// An import failure. Messages are user-safe: never a key or meeting text.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImporterError {
    /// No API key is saved for the source.
    #[error("save a {0} API key first")]
    MissingKey(ImportSource),
    /// The source refused the saved key.
    #[error("{0} did not accept the API key; check it and save it again")]
    Unauthorized(ImportSource),
    /// The source kept answering "too many requests".
    #[error("{0} is limiting requests; try again in a few minutes")]
    RateLimited(ImportSource),
    /// The source answered with something Deino cannot read; the text says
    /// what.
    #[error("{0} sent a response Deino cannot read ({1})")]
    Unreadable(ImportSource, &'static str),
    /// An import is already running.
    #[error("an import is already running")]
    Busy,
    /// The chosen folder is not a readable folder.
    #[error("the folder cannot be read")]
    Folder(#[source] std::io::Error),
    /// The request could not be sent or failed (offline mode, a switched-off
    /// service, no connection).
    #[error(transparent)]
    Net(#[from] NetError),
    /// The Keychain could not be read.
    #[error(transparent)]
    Secret(#[from] SecretError),
    /// One meeting could not be written.
    #[error(transparent)]
    Import(#[from] TranscriptImportError),
    /// The ledger could not be read or written.
    #[error("could not save the import")]
    Storage(#[from] DbError),
}

impl ImporterError {
    /// True when nothing more can be imported in this run: the key, the
    /// network policy or the source itself is the problem, not one meeting.
    pub fn stops_the_run(&self) -> bool {
        match self {
            Self::Unreadable(..) | Self::Import(_) => false,
            Self::Net(error) => error.is_blocked() || error.code() == ErrorCode::Network,
            _ => true,
        }
    }
}

impl ErrorClass for ImporterError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MissingKey(_) => ErrorCode::NotFound,
            Self::Unauthorized(_) => ErrorCode::PermissionDenied,
            Self::RateLimited(_) => ErrorCode::Unavailable,
            Self::Unreadable(..) => ErrorCode::Network,
            Self::Busy => ErrorCode::Conflict,
            Self::Folder(_) => ErrorCode::Invalid,
            Self::Net(error) => error.code(),
            Self::Secret(error) => error.code(),
            Self::Import(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
