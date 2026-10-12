//! The typed ways the chat notice degrades. None of them stops a recording
//! or reaches the user as an error: each is logged once per meeting at
//! debug level ([`LoggedOnce`]) and the notice is simply not posted.

use std::collections::HashSet;

use deino_domain::{CallApp, ErrorClass, ErrorCode};
use deino_speaker_tags::AxError;

/// Why the notice could not be posted, or not yet.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ChatError {
    /// Deino has no Accessibility permission. It never asks on its own.
    #[error("Accessibility permission is not granted")]
    PermissionDenied,
    /// The Accessibility API refused or timed out.
    #[error("the {app} window could not be read or used: {source}")]
    Ax {
        /// The app.
        app: CallApp,
        /// What failed.
        source: AxError,
    },
    /// The element is no longer where the snapshot had it, or has another
    /// role: the window changed between the read and the action.
    #[error("the {0} chat changed before Deino could use it")]
    Moved(CallApp),
    /// Setting the composer's text did not take: the app ignores
    /// Accessibility writes to it.
    #[error("the {0} chat composer did not accept the notice")]
    NotTyped(CallApp),
    /// The composer changed while the notice was being written (the user
    /// typed): the notice is left unsent for this meeting.
    #[error("the {0} chat composer changed while the notice was written")]
    Interrupted(CallApp),
    /// Send was pressed but the composer kept the notice: it did not go.
    #[error("the {0} chat did not send the notice")]
    NotSent(CallApp),
    /// The chat panel is open but has no send button the adapter knows.
    #[error("the {0} chat has no send button Deino recognises")]
    NoSendButton(CallApp),
    /// Accessibility does not exist on this platform.
    #[error("the chat notice needs macOS Accessibility")]
    Unsupported,
}

impl ErrorClass for ChatError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Unavailable
    }
}

/// Remembers which errors were logged, so each is logged once per meeting.
#[derive(Debug, Default)]
pub struct LoggedOnce(HashSet<String>);

impl LoggedOnce {
    /// Logs `error` at debug level, unless an equal one was logged before.
    /// Returns whether it was logged now.
    pub fn log(&mut self, error: &ChatError) -> bool {
        let first = self.0.insert(error.to_string());
        if first {
            log::debug!(target: "deino_consent", "chat notice: {error}");
        }
        first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_error_is_logged_once() {
        let mut logged = LoggedOnce::default();

        assert!(logged.log(&ChatError::PermissionDenied));
        assert!(!logged.log(&ChatError::PermissionDenied));
        assert!(logged.log(&ChatError::Moved(CallApp::Zoom)));
        assert!(logged.log(&ChatError::Moved(CallApp::Meet)));
    }
}
