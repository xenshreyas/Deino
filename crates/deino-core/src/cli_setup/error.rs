//! What can go wrong installing the CLI. Messages name paths, which the
//! user needs to act on, and nothing else.

use deino_domain::{ErrorClass, ErrorCode};

/// An install failure.
#[derive(Debug, thiserror::Error)]
pub enum CliSetupError {
    /// This build of the app has no CLI inside it.
    #[error("this build of Deino does not include the command-line tool")]
    NotBundled,
    /// Something Deino did not make is already at the path.
    #[error("{path} already exists and is not a link Deino made; remove it first")]
    Occupied {
        /// The occupied path.
        path: String,
    },
    /// The folder cannot be written by the app.
    #[error("Deino cannot write to that folder; run this in Terminal instead: {command}")]
    NotWritable {
        /// The command that installs it with administrator rights.
        command: String,
    },
    /// The file system refused.
    #[error("could not install the command-line tool")]
    Io(#[from] std::io::Error),
}

impl ErrorClass for CliSetupError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotBundled => ErrorCode::Unavailable,
            Self::Occupied { .. } => ErrorCode::Conflict,
            Self::NotWritable { .. } => ErrorCode::PermissionDenied,
            Self::Io(_) => ErrorCode::Internal,
        }
    }
}
