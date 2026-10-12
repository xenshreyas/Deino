//! Deliveries the user or an automation asks for: "Send test" with the
//! latest meeting, and an automation's webhook step. Both are attempted at
//! once, so the caller sees the first attempt's outcome.

use deino_domain::{MeetingId, WebhookDeliveryId, WebhookId};

use super::{Delivery, DeliveryKind, NewDelivery, OtherKind, WebhookError, WebhookService};

/// An automation step's payload for one endpoint.
#[derive(Clone, Copy, Debug)]
pub struct PayloadDelivery<'a> {
    /// The endpoint.
    pub webhook: &'a WebhookId,
    /// The meeting the payload is about.
    pub meeting: &'a MeetingId,
    /// `run:<run>:<endpoint>`: the step's retries reuse its delivery.
    pub key: &'a str,
    /// The JSON body.
    pub body: &'a str,
}

impl WebhookService {
    /// Sends the newest finished meeting to `id` now, once (a test is not
    /// retried); returns the delivery after the attempt.
    pub async fn send_test(&self, id: &WebhookId) -> Result<Delivery, WebhookError> {
        self.get(id).await?;
        let meeting = self
            .ports
            .webhooks
            .latest_meeting()
            .await?
            .ok_or(WebhookError::NoMeeting)?;
        let delivery = self.new_delivery(id, DeliveryKind::Other(OtherKind::Test), meeting);
        self.send_now(&delivery).await
    }

    /// Queues an automation step's payload under its key and makes the
    /// first attempt now; a step run again finds its earlier delivery.
    pub async fn deliver_payload(
        &self,
        request: PayloadDelivery<'_>,
    ) -> Result<Delivery, WebhookError> {
        let webhook = self.get(request.webhook).await?;
        if !webhook.enabled {
            return Err(WebhookError::Disabled);
        }
        let kind = DeliveryKind::Other(OtherKind::Automation);
        let mut delivery = self.new_delivery(request.webhook, kind, request.meeting.clone());
        delivery.idempotency_key = Some(request.key.to_owned());
        delivery.body = Some(request.body.to_owned());
        self.send_now(&delivery).await
    }

    /// A delivery of `kind` to `webhook` about `meeting`, due now.
    fn new_delivery(
        &self,
        webhook: &WebhookId,
        kind: DeliveryKind,
        meeting: MeetingId,
    ) -> NewDelivery {
        let now = self.ports.clock.now();
        NewDelivery {
            id: WebhookDeliveryId::generate(self.ports.clock.as_ref()),
            webhook_id: *webhook,
            kind,
            meeting_id: Some(meeting),
            subject_id: None,
            idempotency_key: None,
            body: None,
            coalesce: false,
            due_at: now,
            created_at: now,
        }
    }

    /// Queues `delivery`, attempts it if it is still pending, and returns
    /// it as it then stands.
    async fn send_now(&self, delivery: &NewDelivery) -> Result<Delivery, WebhookError> {
        let id = self.ports.deliveries.enqueue(delivery).await?;
        self.dispatcher.attempt(&id).await?;
        self.ports
            .deliveries
            .get(&id)
            .await?
            .ok_or(WebhookError::NotFound(delivery.webhook_id))
    }
}
