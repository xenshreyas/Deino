//! [`WebhookService`]: the endpoints the user manages on the Webhooks page,
//! the deliveries signals turn into, and the delivery log.
//!
//! Adding the first endpoint switches the `webhooks` network service on;
//! removing the last switches it off (as connecting an integration does,
//! ADR 0031). Its hosts are allowed only while an endpoint names them (the
//! app's policy reads [`WebhookService::hosts`]).

use chrono::TimeDelta;
use deino_domain::{MeetingId, ServiceId, WebhookDeliveryId, WebhookId};

use super::{
    CreatedWebhook, Delivery, DeliveryKind, Dispatcher, MAX_LABEL_CHARS, NewDelivery,
    SigningSecret, Webhook, WebhookError, WebhookInput, WebhookPorts, WebhookSignal, WebhookUrl,
};

/// How long a coalescing event waits for more of the same before it is sent.
pub const COALESCE_DELAY: TimeDelta = TimeDelta::seconds(30);
/// The most deliveries the log returns.
pub const MAX_LOG: u32 = 100;

/// The webhooks feature's business rules.
#[derive(Clone)]
pub struct WebhookService {
    pub(super) ports: WebhookPorts,
    pub(super) dispatcher: Dispatcher,
}

impl WebhookService {
    /// The service over `ports`.
    pub fn new(ports: WebhookPorts) -> Self {
        let dispatcher = Dispatcher::new(ports.clone());
        Self { ports, dispatcher }
    }

    /// The dispatcher, for the loop.
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Every endpoint, oldest first.
    pub async fn list(&self) -> Result<Vec<Webhook>, WebhookError> {
        Ok(self.ports.webhooks.list().await?)
    }

    /// The hosts of every endpoint: the only ones the `webhooks` client may
    /// reach.
    pub async fn hosts(&self) -> Result<Vec<String>, WebhookError> {
        let webhooks = self.ports.webhooks.list().await?;
        Ok(webhooks
            .iter()
            .filter_map(|webhook| WebhookUrl::parse(&webhook.url).ok())
            .map(|url| url.host().to_owned())
            .collect())
    }

    /// Adds an endpoint with a new signing secret, shown this once.
    pub async fn create(&self, input: WebhookInput) -> Result<CreatedWebhook, WebhookError> {
        let input = normalized(input)?;
        let now = self.ports.clock.now();
        let webhook = Webhook {
            id: WebhookId::generate(self.ports.clock.as_ref()),
            url: input.url,
            label: input.label,
            events: input.events,
            fields: input.fields,
            enabled: input.enabled,
            disabled_reason: None,
            failure_streak: 0,
            failing_since: None,
            last_success_at: None,
            created_at: now,
            updated_at: now,
        };
        let secret = SigningSecret::generate()?;
        self.ports.secrets.save(&webhook.id, &secret).await?;
        self.ports.webhooks.create(&webhook).await?;
        self.switch(true).await;
        Ok(CreatedWebhook {
            webhook,
            secret: secret.expose().to_owned(),
        })
    }

    /// Changes an endpoint. Turning it on again clears an auto-disable and
    /// its failure streak.
    pub async fn update(
        &self,
        id: &WebhookId,
        input: WebhookInput,
    ) -> Result<Webhook, WebhookError> {
        let input = normalized(input)?;
        let mut webhook = self.get(id).await?;
        if input.enabled && !webhook.enabled {
            webhook.disabled_reason = None;
            webhook.failure_streak = 0;
            webhook.failing_since = None;
        }
        webhook.url = input.url;
        webhook.label = input.label;
        webhook.events = input.events;
        webhook.fields = input.fields;
        webhook.enabled = input.enabled;
        webhook.updated_at = self.ports.clock.now();
        if !self.ports.webhooks.update(&webhook).await? {
            return Err(WebhookError::NotFound(*id));
        }
        Ok(webhook)
    }

    /// Replaces an endpoint's signing secret; returns the new one, shown once.
    pub async fn rotate_secret(&self, id: &WebhookId) -> Result<String, WebhookError> {
        self.get(id).await?;
        let secret = SigningSecret::generate()?;
        self.ports.secrets.save(id, &secret).await?;
        Ok(secret.expose().to_owned())
    }

    /// Removes an endpoint, its deliveries and its secret.
    pub async fn delete(&self, id: &WebhookId) -> Result<(), WebhookError> {
        if !self.ports.webhooks.delete(id).await? {
            return Err(WebhookError::NotFound(*id));
        }
        self.ports.secrets.forget(id).await?;
        if self.ports.webhooks.list().await?.is_empty() {
            self.switch(false).await;
        }
        Ok(())
    }

    /// The newest deliveries of `id`.
    pub async fn deliveries(&self, id: &WebhookId) -> Result<Vec<Delivery>, WebhookError> {
        Ok(self.ports.deliveries.list(id, MAX_LOG).await?)
    }

    /// Turns `signal` into a delivery for every enabled endpoint subscribed
    /// to it; returns how many were queued (or joined a waiting one).
    pub async fn accept(&self, signal: &WebhookSignal) -> Result<usize, WebhookError> {
        let targets: Vec<Webhook> = self
            .ports
            .webhooks
            .list()
            .await?
            .into_iter()
            .filter(|webhook| webhook.enabled && webhook.events.contains(&signal.event))
            .collect();
        if targets.is_empty() {
            return Ok(0);
        }
        let meeting = match (&signal.meeting, &signal.item) {
            (Some(meeting), _) => Some(meeting.clone()),
            (None, Some(item)) => self.ports.items.meeting_of(item).await?,
            (None, None) => None,
        };
        let Some(meeting) = meeting else {
            return Ok(0);
        };
        for webhook in &targets {
            let delivery = self.event_delivery(webhook, signal, &meeting);
            self.ports.deliveries.enqueue(&delivery).await?;
        }
        Ok(targets.len())
    }

    /// The delivery of `signal` about `meeting` to `webhook`: due now, or
    /// after [`COALESCE_DELAY`] for an event that coalesces.
    fn event_delivery(
        &self,
        webhook: &Webhook,
        signal: &WebhookSignal,
        meeting: &MeetingId,
    ) -> NewDelivery {
        let now = self.ports.clock.now();
        let coalesce = signal.event.coalesces();
        NewDelivery {
            id: WebhookDeliveryId::generate(self.ports.clock.as_ref()),
            webhook_id: webhook.id,
            kind: DeliveryKind::Event(signal.event),
            meeting_id: Some(meeting.clone()),
            subject_id: signal.item.map(|item| item.to_string()),
            idempotency_key: None,
            body: None,
            coalesce,
            due_at: if coalesce { now + COALESCE_DELAY } else { now },
            created_at: now,
        }
    }

    /// One endpoint.
    pub(super) async fn get(&self, id: &WebhookId) -> Result<Webhook, WebhookError> {
        self.ports
            .webhooks
            .get(id)
            .await?
            .ok_or(WebhookError::NotFound(*id))
    }

    /// Switches the `webhooks` network service; a failure is logged, since
    /// the user can still switch it in Settings → Privacy → Network.
    async fn switch(&self, enabled: bool) {
        if let Err(error) = self
            .ports
            .switch
            .set_enabled(ServiceId::Webhooks, enabled)
            .await
        {
            log::warn!(target: "webhooks", "could not switch the webhooks service: {error}");
        }
    }
}

/// `input` trimmed and checked.
fn normalized(input: WebhookInput) -> Result<WebhookInput, WebhookError> {
    let url = WebhookUrl::parse(&input.url)?;
    let label = input.label.trim().to_owned();
    if label.chars().count() > MAX_LABEL_CHARS {
        return Err(WebhookError::invalid("name", "is too long"));
    }
    let mut events = input.events;
    events.sort_unstable();
    events.dedup();
    Ok(WebhookInput {
        url: url.as_str().to_owned(),
        label,
        events,
        ..input
    })
}
