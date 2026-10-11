//! What can go wrong while exporting. Messages are user-safe: they never
//! name a meeting's title, a note or a file path (paths carry titles).

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_export::RenderError;

use crate::settings::SettingsError;

/// An export failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExportError {
    /// No meeting has the ID.
    #[error("meeting {0} was not found")]
    NotFound(MeetingId),
    /// The meeting could not be read.
    #[error("could not read the meeting")]
    Storage(#[from] DbError),
    /// A renderer failed.
    #[error(transparent)]
    Render(#[from] RenderError),
    /// The file could not be written.
    #[error("could not write the export file")]
    Write(#[source] std::io::Error),
    /// A recording could not be added to the account export.
    #[error("could not read a recording")]
    Audio(#[source] std::io::Error),
    /// Auto-export is on but no folder was chosen.
    #[error("choose an export folder in Settings → Export")]
    NoFolder,
    /// The export folder is missing or cannot be written (an unplugged drive).
    #[error("the export folder is not available")]
    FolderUnavailable(#[source] std::io::Error),
    /// An export of every meeting is already running.
    #[error("an export of every meeting is already running")]
    Busy,
    /// The export was cancelled.
    #[error("the export was cancelled")]
    Cancelled,
    /// The clipboard refused the copy.
    #[error("the clipboard is not available")]
    Clipboard,
    /// The mail app could not be opened.
    #[error("the mail app could not be opened")]
    Mail,
    /// The chosen folder could not be saved in the settings.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

impl ExportError {
    /// True when trying again later may succeed (a busy database, a drive
    /// that comes back), so the pipeline retries the stage.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Storage(_) | Self::Write(_) | Self::FolderUnavailable(_)
        )
    }
}

impl ErrorClass for ExportError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::NoFolder => ErrorCode::Invalid,
            Self::Busy => ErrorCode::Conflict,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::FolderUnavailable(_) | Self::Clipboard | Self::Mail => ErrorCode::Unavailable,
            Self::Settings(error) => error.code(),
            Self::Write(_) | Self::Audio(_) | Self::Storage(_) | Self::Render(_) => {
                ErrorCode::Internal
            }
        }
    }
}
