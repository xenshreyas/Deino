//! Deliveries: one write to a provider, sent at most once per
//! [`IdempotencyKey`] and recorded in the delivery log
//! (`integration_deliveries`) the Integrations page shows.

mod repo;

use chrono::{DateTime, Utc};
use deino_domain::{
    Capability, ConnectionId, DeliveryId, DeliveryStatus, ErrorCode, IntegrationProvider, MeetingId,
};
use serde::{Deserialize, Serialize};

pub use repo::{DeliveryRepo, NewDelivery, SqliteDeliveryRepo};

use crate::{Action, IntegrationError};

/// The longest idempotency key.
pub const MAX_KEY_CHARS: usize = 300;

/// Names one intended write, so sending it twice writes once: an automation
/// run's step (`automation:<run>:<step>`), a share
/// (`share:<meeting>:<connection>:<target>`). Chosen by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Validates `key`: non-blank, at most [`MAX_KEY_CHARS`] characters.
    pub fn parse(key: &str) -> Result<Self, IntegrationError> {
        let key = key.trim();
        if key.is_empty() || key.chars().count() > MAX_KEY_CHARS {
            return Err(IntegrationError::Invalid(format!(
                "an idempotency key needs 1 to {MAX_KEY_CHARS} characters"
            )));
        }
        Ok(Self(key.to_owned()))
    }

    /// The key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = IntegrationError;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        Self::parse(&key)
    }
}

impl From<IdempotencyKey> for String {
    fn from(key: IdempotencyKey) -> Self {
        key.0
    }
}

/// A write to send.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryRequest {
    /// The account to send it with.
    pub connection: ConnectionId,
    /// What to write.
    pub action: Action,
    /// Sending the same key again returns the first result.
    pub idempotency_key: IdempotencyKey,
    /// The meeting it comes from, if any.
    pub meeting: Option<MeetingId>,
}

/// What the provider created.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRef {
    /// The provider's ID for it (a Slack message `ts`, a Notion page ID).
    pub id: String,
    /// Where to open it, if the provider gives a link.
    pub url: Option<String>,
}

/// A user-safe delivery failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryFailure {
    /// What kind of failure.
    pub code: ErrorCode,
    /// What happened, user-safe.
    pub message: String,
}

/// One row of the delivery log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    /// The delivery's ID.
    pub id: DeliveryId,
    /// The account it was sent with; `None` once that was disconnected.
    pub connection_id: Option<ConnectionId>,
    /// The provider.
    pub provider: IntegrationProvider,
    /// What it wrote.
    pub capability: Capability,
    /// The meeting it came from, if any.
    pub meeting_id: Option<MeetingId>,
    /// Its key.
    pub idempotency_key: String,
    /// Where it stands.
    pub status: DeliveryStatus,
    /// How many times it was tried.
    pub attempts: u32,
    /// What the provider created, once it succeeded.
    pub external: Option<ExternalRef>,
    /// The last failure, if the last attempt failed.
    pub error: Option<DeliveryFailure>,
    /// When it was first requested.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_trimmed_and_bounded() {
        assert_eq!(
            IdempotencyKey::parse(" share:a ").unwrap().as_str(),
            "share:a"
        );
        assert!(IdempotencyKey::parse("  ").is_err());
        assert!(IdempotencyKey::parse(&"k".repeat(MAX_KEY_CHARS + 1)).is_err());
        assert!(serde_json::from_str::<IdempotencyKey>(r#""""#).is_err());
    }
}
