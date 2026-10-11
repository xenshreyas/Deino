//! [`Connection`]: a connected account, as stored in
//! `integration_connections` (no secrets) and shown on the Integrations page.

mod repo;

use chrono::{DateTime, Utc};
use deino_domain::{
    AppError, AuthMethod, ConnectionId, ConnectionStatus, ErrorCode, IntegrationProvider,
};
use serde::Serialize;

pub use repo::{ConnectionRepo, SqliteConnectionRepo};

/// The longest stored error message.
pub const MAX_ERROR_CHARS: usize = 500;

/// A connected account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    /// The connection's ID.
    pub id: ConnectionId,
    /// The provider.
    pub provider: IntegrationProvider,
    /// The provider's stable ID for the account.
    pub account_id: String,
    /// What the user recognises: `Acme · ana@acme.com`.
    pub account_label: String,
    /// How it was connected.
    pub auth_method: AuthMethod,
    /// Whether it can be used.
    pub status: ConnectionStatus,
    /// The scopes it was granted, sorted.
    pub scopes: Vec<String>,
    /// Where the account's API lives, when the provider decides it per
    /// account. Not shown in the UI.
    #[serde(skip)]
    pub api_base_url: Option<String>,
    /// When the stored access token expires, if it does.
    pub token_expires_at: Option<DateTime<Utc>>,
    /// When it was first connected.
    pub connected_at: DateTime<Utc>,
    /// When anything about it last changed.
    pub updated_at: DateTime<Utc>,
    /// The last request that worked.
    pub last_used_at: Option<DateTime<Utc>>,
    /// The last failure, until a request works again.
    pub last_error: Option<ConnectionError>,
}

/// A user-safe failure and when it happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionError {
    /// What kind of failure.
    pub code: ErrorCode,
    /// What happened, user-safe.
    pub message: String,
    /// When.
    pub at: DateTime<Utc>,
}

impl ConnectionError {
    /// `error` at `at`, its message capped at [`MAX_ERROR_CHARS`].
    pub fn new(error: &AppError, at: DateTime<Utc>) -> Self {
        Self {
            code: error.code,
            message: capped(&error.message),
            at,
        }
    }
}

/// `message`, cut to [`MAX_ERROR_CHARS`] characters.
pub(crate) fn capped(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        return "unknown error".into();
    }
    message.chars().take(MAX_ERROR_CHARS).collect()
}
