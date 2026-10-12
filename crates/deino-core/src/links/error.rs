//! What can go wrong with a link.
//!
//! The messages name the route or parameter, never the link's values (an
//! ID, a query or a recipe could be anything the user typed).

use deino_domain::{ErrorClass, ErrorCode};

use super::model::MAX_LINK_CHARS;
use super::ports::RecorderError;

/// A link Deino cannot follow, or a recording it could not start or stop.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinkError {
    /// The text is not a URL, or not a `deino://` one.
    #[error("not a deino:// link")]
    NotDeinoLink,
    /// The link is longer than [`MAX_LINK_CHARS`].
    #[error("a deino:// link can be at most {MAX_LINK_CHARS} characters long")]
    TooLong,
    /// The route is not one Deino knows.
    #[error("Deino has no link for that")]
    UnknownRoute,
    /// A required parameter (`q`, `data`) or path part (the meeting ID) is
    /// missing or empty.
    #[error("the link is missing its {0}")]
    Missing(&'static str),
    /// A parameter or path part is malformed (a non-numeric `t`, a bad ID).
    #[error("the link's {0} is not valid")]
    Invalid(&'static str),
    /// The recorder refused (already recording, nothing to stop) or failed.
    #[error("recording: {0}")]
    Recorder(#[source] RecorderError),
}

impl ErrorClass for LinkError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotDeinoLink
            | Self::TooLong
            | Self::UnknownRoute
            | Self::Missing(_)
            | Self::Invalid(_) => ErrorCode::Invalid,
            Self::Recorder(error) => error.code,
        }
    }
}
