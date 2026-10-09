//! What can go wrong in detection. Messages are user-safe: they name what
//! failed and an OS status code, never an app's window title or a URL.

use deino_domain::{ErrorClass, ErrorCode};

/// A detection failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DetectError {
    /// Meeting detection needs macOS.
    #[error("meeting detection is only available on macOS")]
    Unsupported,
    /// A Core Audio, CoreMediaIO or Accessibility call failed.
    #[error("could not read {what} (OS status {status})")]
    Os {
        /// The property or call, for example "the audio process list".
        what: &'static str,
        /// The `OSStatus` the call returned.
        status: i32,
    },
    /// The prompt a command named has already closed.
    #[error("that meeting prompt has already closed")]
    PromptClosed,
    /// Recording could not be started.
    #[error("could not start recording")]
    Recording(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The ignore list could not be saved.
    #[error("could not save the ignored apps")]
    IgnoreList(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The detector thread could not be started.
    #[error("could not start meeting detection")]
    Thread(#[source] std::io::Error),
}

impl DetectError {
    /// A failed OS call reading `what`.
    pub fn os(what: &'static str, status: i32) -> Self {
        Self::Os { what, status }
    }
}

impl ErrorClass for DetectError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Unsupported => ErrorCode::Unavailable,
            Self::PromptClosed => ErrorCode::NotFound,
            Self::Os { .. } | Self::Recording(_) | Self::IgnoreList(_) | Self::Thread(_) => {
                ErrorCode::Internal
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_prompt_is_not_found_and_os_failures_are_internal() {
        assert_eq!(DetectError::PromptClosed.code(), ErrorCode::NotFound);
        assert_eq!(
            DetectError::os("the camera list", -50).code(),
            ErrorCode::Internal
        );
        assert_eq!(
            DetectError::os("the camera list", -50).to_string(),
            "could not read the camera list (OS status -50)"
        );
    }
}
