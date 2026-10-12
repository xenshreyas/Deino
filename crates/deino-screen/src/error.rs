//! [`ScreenError`]: why a capture step failed. None of them stops a
//! recording: the capture logs it once and carries on, or ends quietly.

use deino_domain::{ErrorClass, ErrorCode};

/// Why screen capture could not do something.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ScreenError {
    /// Not on this platform.
    #[error("screen capture is not available on this platform")]
    Unsupported,
    /// Screen Recording is not granted.
    #[error("Screen Recording is not allowed for Deino")]
    NotPermitted,
    /// The window to capture is gone.
    #[error("the meeting window is no longer shared")]
    WindowGone,
    /// A `ScreenCaptureKit`, Core Video or `AVFoundation` call failed.
    #[error("{call} failed")]
    Native {
        /// The call, for the log.
        call: &'static str,
    },
    /// The framework did not answer in time.
    #[error("{0} timed out")]
    Timeout(&'static str),
    /// Encoding a snapshot failed.
    #[error("the snapshot could not be encoded: {0}")]
    Encode(#[from] png::EncodingError),
    /// Writing a file failed.
    #[error("a screen capture file could not be written: {0}")]
    Io(#[from] std::io::Error),
    /// Storing a snapshot row failed.
    #[error("the snapshot could not be stored: {0}")]
    Store(String),
    /// Opening System Settings failed.
    #[error("System Settings could not be opened")]
    SettingsUnavailable,
}

impl ErrorClass for ScreenError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Unsupported | Self::SettingsUnavailable => ErrorCode::Unavailable,
            Self::NotPermitted => ErrorCode::PermissionDenied,
            _ => ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::AppError;

    use super::*;

    #[test]
    fn errors_cross_ipc_with_a_stable_code() {
        assert_eq!(
            AppError::from(ScreenError::Unsupported).code,
            ErrorCode::Unavailable
        );
        assert_eq!(
            AppError::from(ScreenError::NotPermitted).code,
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            AppError::from(ScreenError::WindowGone).code,
            ErrorCode::Internal
        );
    }
}
