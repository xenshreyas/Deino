//! The delivery log's entries: [`Delivery`] and its [`DeliveryStatus`].

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, WebhookDeliveryId, WebhookId};
use serde::{Deserialize, Serialize};

use super::DeliveryKind;

/// Where a delivery stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Waiting for its next attempt.
    Pending,
    /// An attempt is in flight.
    Sending,
    /// The endpoint answered 2xx.
    Succeeded,
    /// Deino gave up.
    Failed,
}

impl DeliveryStatus {
    /// The stored name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sending => "sending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// The status stored as `name`.
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Pending, Self::Sending, Self::Succeeded, Self::Failed]
            .into_iter()
            .find(|status| status.as_str() == name)
    }
}

/// One message to one endpoint, as the delivery log shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    /// Its ID (the `webhook-id` header is derived from it).
    pub id: WebhookDeliveryId,
    /// The endpoint.
    pub webhook_id: WebhookId,
    /// What it carries.
    pub kind: DeliveryKind,
    /// The meeting it is about.
    pub meeting_id: Option<MeetingId>,
    /// Where it stands.
    pub status: DeliveryStatus,
    /// Attempts so far.
    pub attempts: u32,
    /// When the next attempt is due, while pending.
    pub next_attempt_at: Option<DateTime<Utc>>,
    /// The last attempt's HTTP status.
    pub last_status: Option<u16>,
    /// The last attempt's error, user-safe.
    pub last_error: Option<String>,
    /// When the event happened.
    pub created_at: DateTime<Utc>,
    /// When it succeeded or Deino gave up.
    pub finished_at: Option<DateTime<Utc>>,
}
