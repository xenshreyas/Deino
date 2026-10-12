//! [`Dispatcher`]: makes one attempt of a delivery, records it, and keeps
//! the endpoint's failure streak (auto-disable).
//!
//! An attempt claims the delivery (`pending` → `sending`), fixes its body
//! on the first attempt (assembled from the meeting at that moment), signs
//! it with the endpoint's secret, posts it, and stores what happened: done,
//! the time of the next try ([`next_attempt`]), or given up.

use deino_domain::{ActionItemId, WebhookDeliveryId};

use super::payload::{PayloadRequest, build_event_payload};
use super::schedule::{AttemptResult, disable_reason, next_attempt};
use super::sender::{Outgoing, send};
use super::signing::SignedMessage;
use super::{
    AttemptRecord, Claimed, DeliveryKind, DeliveryStatus, OtherKind, Webhook, WebhookError,
    WebhookPorts,
};
use crate::export::ExportError;

/// The most deliveries one pass attempts.
const BATCH: u32 = 50;

/// Attempts deliveries.
#[derive(Clone)]
pub struct Dispatcher {
    ports: WebhookPorts,
}

impl Dispatcher {
    /// The dispatcher over `ports`.
    pub fn new(ports: WebhookPorts) -> Self {
        Self { ports }
    }

    /// Attempts every delivery due now; returns how many it tried.
    pub async fn run_due(&self) -> Result<usize, WebhookError> {
        let now = self.ports.clock.now();
        let due = self.ports.deliveries.due(now, BATCH).await?;
        for id in &due {
            if let Err(error) = self.attempt(id).await {
                log::warn!(target: "webhooks", "delivery {id} could not be attempted: {error}");
            }
        }
        Ok(due.len())
    }

    /// Makes one attempt of `id` if it is pending.
    pub async fn attempt(&self, id: &WebhookDeliveryId) -> Result<(), WebhookError> {
        let now = self.ports.clock.now();
        let Some(claimed) = self.ports.deliveries.claim(id, now).await? else {
            return Ok(());
        };
        let Some(webhook) = self.ports.webhooks.get(&claimed.webhook_id).await? else {
            return Ok(());
        };
        let (result, body) = match self.body(&claimed, &webhook).await {
            Ok(body) => (self.post(&claimed, &webhook, &body).await, Some(body)),
            Err(result) => (result, None),
        };
        self.conclude(&claimed, &webhook, (result, body)).await
    }

    /// The body to send, or why there is nothing to send.
    async fn body(&self, claimed: &Claimed, webhook: &Webhook) -> Result<String, AttemptResult> {
        let test = claimed.kind == DeliveryKind::Other(OtherKind::Test);
        if !webhook.enabled && !test {
            return Err(AttemptResult::Rejected(WebhookError::Disabled.to_string()));
        }
        if let Some(body) = &claimed.body {
            return Ok(body.clone());
        }
        let kind = match claimed.kind {
            DeliveryKind::Event(event) => event.as_str(),
            DeliveryKind::Other(OtherKind::Test) => "test",
            DeliveryKind::Other(OtherKind::Automation) => {
                return Err(AttemptResult::Rejected(
                    "the automation's payload is missing".into(),
                ));
            }
        };
        self.assemble(claimed, webhook, kind).await
    }

    /// Assembles an event body from the meeting as it is now.
    async fn assemble(
        &self,
        claimed: &Claimed,
        webhook: &Webhook,
        kind: &'static str,
    ) -> Result<String, AttemptResult> {
        let gone = || AttemptResult::Rejected("the meeting no longer exists".into());
        let meeting = claimed.meeting_id.clone().ok_or_else(gone)?;
        let item = match claimed
            .subject_id
            .as_deref()
            .and_then(|id| id.parse::<ActionItemId>().ok())
        {
            Some(item) => self.ports.items.subject(&item).await.map_err(busy)?,
            None => None,
        };
        let request = PayloadRequest {
            kind,
            meeting: &meeting,
            fields: webhook.fields,
            at: self.ports.clock.now(),
        };
        let payload = build_event_payload(self.ports.meetings.as_ref(), request, item)
            .await
            .map_err(|error| match error {
                ExportError::NotFound(_) => gone(),
                other => busy(other),
            })?;
        serde_json::to_string(&payload)
            .map_err(|error| AttemptResult::Rejected(WebhookError::Encode(error).to_string()))
    }

    /// Signs and posts `body`.
    async fn post(&self, claimed: &Claimed, webhook: &Webhook, body: &str) -> AttemptResult {
        let secret = match self.ports.secrets.load(&webhook.id).await {
            Ok(secret) => secret,
            Err(error) => return AttemptResult::Rejected(error.to_string()),
        };
        let id = message_id(&claimed.id);
        let message = SignedMessage {
            id: &id,
            timestamp: self.ports.clock.now().timestamp(),
            body: body.as_bytes(),
        };
        let headers = match secret.sign(&message) {
            Ok(headers) => headers,
            Err(error) => return AttemptResult::Rejected(error.to_string()),
        };
        let outgoing = Outgoing {
            url: &webhook.url,
            body,
            headers: &headers,
        };
        send(self.ports.client.as_ref(), outgoing).await
    }

    /// Stores the attempt and updates the endpoint.
    async fn conclude(
        &self,
        claimed: &Claimed,
        webhook: &Webhook,
        (result, body): (AttemptResult, Option<String>),
    ) -> Result<(), WebhookError> {
        let now = self.ports.clock.now();
        let attempts = claimed.attempts.saturating_add(1);
        let test = claimed.kind == DeliveryKind::Other(OtherKind::Test);
        let next = next_attempt(&result, attempts, (claimed.created_at, now)).filter(|_| !test);
        let status = match (&result, next) {
            (AttemptResult::Delivered(_), _) => DeliveryStatus::Succeeded,
            (_, Some(_)) => DeliveryStatus::Pending,
            (_, None) => DeliveryStatus::Failed,
        };
        log::debug!(target: "webhooks", "delivery {} attempt {attempts}: {status:?}", claimed.id);
        let record = AttemptRecord {
            id: claimed.id,
            attempts,
            status,
            next_attempt_at: next,
            last_status: result.status(),
            last_error: result.error(),
            body,
            at: now,
        };
        self.ports.deliveries.record(&record).await?;
        self.track_endpoint(webhook, &result).await?;
        if status != DeliveryStatus::Pending {
            self.ports.deliveries.prune(&webhook.id).await?;
        }
        Ok(())
    }

    /// The endpoint's streak after `result`, turning it off when it keeps
    /// failing.
    async fn track_endpoint(
        &self,
        webhook: &Webhook,
        result: &AttemptResult,
    ) -> Result<(), WebhookError> {
        let now = self.ports.clock.now();
        let webhooks = &self.ports.webhooks;
        match result {
            AttemptResult::Delivered(_) => {
                return Ok(webhooks.record_success(&webhook.id, now).await?);
            }
            AttemptResult::Status(_) | AttemptResult::NoAnswer(_) => {}
            AttemptResult::Blocked(_) | AttemptResult::Rejected(_) => return Ok(()),
        }
        let streak = webhooks.record_failure(&webhook.id, now).await?;
        let Some(reason) = disable_reason(result, streak.failures, (streak.since, now)) else {
            return Ok(());
        };
        log::warn!(target: "webhooks", "webhook {} turned off ({})", webhook.id, reason.as_str());
        webhooks.disable(&webhook.id, reason, now).await?;
        let error = "the webhook was turned off after it kept failing";
        Ok(self
            .ports
            .deliveries
            .fail_pending(&webhook.id, error, now)
            .await?)
    }
}

/// The `webhook-id` of a delivery: `msg_` and its UUID's hex digits.
pub fn message_id(id: &WebhookDeliveryId) -> String {
    format!("msg_{}", id.as_uuid().simple())
}

/// A failure reading Deino's own data: try again later, without blaming
/// the endpoint.
fn busy(error: impl std::fmt::Display) -> AttemptResult {
    AttemptResult::Blocked(format!("could not read the meeting: {error}"))
}
