//! What can go wrong with a permission.

use deino_domain::{ErrorClass, ErrorCode};

use super::PermissionKind;

/// Why a permission action failed.
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    /// macOS would not open the permission's System Settings page.
    #[error("System Settings could not be opened for {0:?}")]
    SettingsUnavailable(PermissionKind),
}

impl ErrorClass for PermissionError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::SettingsUnavailable(_) => ErrorCode::Unavailable,
        }
    }
}
