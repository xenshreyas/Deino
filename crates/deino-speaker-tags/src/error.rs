//! The typed ways speaker tags degrade. None of them stops a recording or
//! surfaces as an error to the user: the watcher logs each at debug level
//! when it starts, and the meeting's speakers are named by voice for that
//! stretch (CAP-04).

use deino_domain::{CallApp, ErrorClass, ErrorCode};

use crate::reading::ReadError;

/// Why an app's windows could not be read through Accessibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AxError {
    /// The Accessibility API refused or timed out (`AXError` code).
    #[error("the Accessibility API failed with code {0}")]
    Api(i32),
    /// System Settings would not open its Accessibility page.
    #[error("System Settings could not be opened")]
    SettingsUnavailable,
    /// Accessibility does not exist on this platform.
    #[error("Accessibility is not available on this platform")]
    Unsupported,
}

impl ErrorClass for AxError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Api(_) | Self::SettingsUnavailable | Self::Unsupported => ErrorCode::Unavailable,
        }
    }
}

/// Why a poll found no speaker name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TagError {
    /// Deino has no Accessibility permission. Deino never asks for it on
    /// its own: Settings → Speaker tags opens the System Settings page.
    #[error("Accessibility permission is not granted")]
    PermissionDenied,
    /// An app's windows could not be read.
    #[error("the {app} windows could not be read: {source}")]
    Unreadable {
        /// The app.
        app: CallApp,
        /// What failed.
        source: AxError,
    },
    /// An app's call window has a layout the adapter does not know.
    #[error("the {app} call window was not recognised: {source}")]
    Layout {
        /// The app.
        app: CallApp,
        /// What the adapter said.
        source: ReadError,
    },
}

/// Why finished spans could not be stored. The watcher keeps them and
/// tries again at the next flush.
#[derive(Debug, thiserror::Error)]
#[error("speaker tags could not be stored")]
pub struct SinkError {
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl SinkError {
    /// Wraps the storage's own error.
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
