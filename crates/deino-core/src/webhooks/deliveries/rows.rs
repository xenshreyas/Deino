//! The delivery repository's inputs and rows.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, WebhookDeliveryId, WebhookId};

use crate::webhooks::{Delivery, DeliveryKind, DeliveryStatus};

/// A delivery to queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewDelivery {
    /// Its ID.
    pub id: WebhookDeliveryId,
    /// The endpoint.
    pub webhook_id: WebhookId,
    /// What it carries.
    pub kind: DeliveryKind,
    /// The meeting it is about.
    pub meeting_id: Option<MeetingId>,
    /// The action item it is about.
    pub subject_id: Option<String>,
    /// One delivery per key and endpoint (automation steps).
    pub idempotency_key: Option<String>,
    /// The body, when the caller fixes it (automation steps); otherwise
    /// it is assembled at the first attempt.
    pub body: Option<String>,
    /// Join a delivery of the same event still waiting for its first attempt.
    pub coalesce: bool,
    /// When to make the first attempt.
    pub due_at: DateTime<Utc>,
    /// When the event happened.
    pub created_at: DateTime<Utc>,
}

/// A delivery an attempt has claimed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claimed {
    /// Its ID.
    pub id: WebhookDeliveryId,
    /// The endpoint.
    pub webhook_id: WebhookId,
    /// What it carries.
    pub kind: DeliveryKind,
    /// The meeting it is about.
    pub meeting_id: Option<MeetingId>,
    /// The action item it is about.
    pub subject_id: Option<String>,
    /// The body fixed by the first attempt, or by the caller.
    pub body: Option<String>,
    /// Attempts before this one.
    pub attempts: u32,
    /// When the event happened.
    pub created_at: DateTime<Utc>,
}

/// One attempt's outcome, to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRecord {
    /// The delivery.
    pub id: WebhookDeliveryId,
    /// Attempts so far, this one included.
    pub attempts: u32,
    /// Pending (with `next_attempt_at`), succeeded or failed.
    pub status: DeliveryStatus,
    /// When to try again.
    pub next_attempt_at: Option<DateTime<Utc>>,
    /// The HTTP status.
    pub last_status: Option<u16>,
    /// The user-safe error.
    pub last_error: Option<String>,
    /// The body to resend; dropped once the delivery ends.
    pub body: Option<String>,
    /// When the attempt ended.
    pub at: DateTime<Utc>,
}

/// The row [`super::DeliveryRepo::claim`] returns.
pub(super) struct ClaimRow {
    pub(super) id: WebhookDeliveryId,
    pub(super) webhook_id: WebhookId,
    pub(super) event: String,
    pub(super) meeting_id: Option<MeetingId>,
    pub(super) subject_id: Option<String>,
    pub(super) body: Option<String>,
    pub(super) attempts: i64,
    pub(super) created_at: DateTime<Utc>,
}

impl ClaimRow {
    /// The claimed delivery; `None` for an event name this build does not know.
    pub(super) fn into_claimed(self) -> Option<Claimed> {
        Some(Claimed {
            kind: DeliveryKind::parse(&self.event)?,
            id: self.id,
            webhook_id: self.webhook_id,
            meeting_id: self.meeting_id,
            subject_id: self.subject_id,
            body: self.body,
            attempts: u32::try_from(self.attempts).unwrap_or(u32::MAX),
            created_at: self.created_at,
        })
    }
}

/// One delivery log row.
pub(super) struct DeliveryRow {
    pub(super) id: WebhookDeliveryId,
    pub(super) webhook_id: WebhookId,
    pub(super) event: String,
    pub(super) meeting_id: Option<MeetingId>,
    pub(super) status: String,
    pub(super) attempts: i64,
    pub(super) next_attempt_at: Option<DateTime<Utc>>,
    pub(super) last_status: Option<i64>,
    pub(super) last_error: Option<String>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) finished_at: Option<DateTime<Utc>>,
}

impl DeliveryRow {
    /// The log entry; `None` for a name this build does not know.
    pub(super) fn into_delivery(self) -> Option<Delivery> {
        Some(Delivery {
            kind: DeliveryKind::parse(&self.event)?,
            status: DeliveryStatus::parse(&self.status)?,
            id: self.id,
            webhook_id: self.webhook_id,
            meeting_id: self.meeting_id,
            attempts: u32::try_from(self.attempts).unwrap_or(u32::MAX),
            next_attempt_at: self.next_attempt_at,
            last_status: self.last_status.and_then(|code| u16::try_from(code).ok()),
            last_error: self.last_error,
            created_at: self.created_at,
            finished_at: self.finished_at,
        })
    }
}
