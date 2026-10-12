//! [`WebhookError`]: what can go wrong with webhooks. Messages name fields,
//! IDs and HTTP statuses, never a URL's path, a secret or meeting content.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, WebhookId};
use deino_secrets::SecretError;

use crate::export::ExportError;

/// A webhooks failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WebhookError {
    /// No endpoint has the ID.
    #[error("webhook {0} was not found")]
    NotFound(WebhookId),
    /// A field breaks a rule.
    #[error("{field} {rule}")]
    Invalid {
        /// The field, for example "URL".
        field: &'static str,
        /// What is wrong.
        rule: &'static str,
    },
    /// There is no finished meeting to send as a test.
    #[error("there is no finished meeting to send yet")]
    NoMeeting,
    /// The endpoint is turned off.
    #[error("this webhook is turned off")]
    Disabled,
    /// The endpoint's signing secret is missing or unreadable.
    #[error("this webhook's signing secret is missing; add the webhook again")]
    BadSecret,
    /// The system could not provide random bytes for a secret.
    #[error("could not create a signing secret")]
    Random,
    /// The endpoint did not take the delivery; the user-safe reason.
    #[error("the webhook was not delivered: {0}")]
    Delivery(String),
    /// The Keychain refused.
    #[error(transparent)]
    Secret(#[from] SecretError),
    /// The meeting could not be read for the payload.
    #[error(transparent)]
    Payload(#[from] ExportError),
    /// The payload could not be encoded (a bug).
    #[error("could not encode the payload")]
    Encode(#[source] serde_json::Error),
    /// The database failed.
    #[error("could not load or save webhooks")]
    Storage(#[from] DbError),
}

impl WebhookError {
    /// A broken rule of `field`.
    pub fn invalid(field: &'static str, rule: &'static str) -> Self {
        Self::Invalid { field, rule }
    }
}

impl ErrorClass for WebhookError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::NoMeeting => ErrorCode::NotFound,
            Self::Invalid { .. } => ErrorCode::Invalid,
            Self::Disabled => ErrorCode::Conflict,
            Self::Delivery(_) => ErrorCode::Network,
            Self::BadSecret | Self::Random | Self::Encode(_) => ErrorCode::Internal,
            Self::Secret(error) => error.code(),
            Self::Payload(error) => error.code(),
            Self::Storage(error) => error.code(),
        }
    }
}
