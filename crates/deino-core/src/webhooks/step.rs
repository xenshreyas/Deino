//! [`WebhookStep`]: AUT-01's webhook step, run through the webhooks
//! service. The body is the automation payload (`deino.automation` v1, the
//! same as a shell script's stdin) with the step's fields; the delivery is
//! keyed `run:<run>:<endpoint>`, so a rerun of the stage finds it instead
//! of posting twice, and failed attempts are retried by the webhooks'
//! own schedule rather than the pipeline's.

use std::sync::Arc;

use async_trait::async_trait;

use super::{DeliveryStatus, PayloadDelivery, WebhookError, WebhookService};
use crate::automations::{
    Action, ActionKind, MeetingSource, Step, StepContext, StepError, StepOutput, build_payload,
};

/// Runs [`Action::Webhook`].
pub struct WebhookStep {
    service: WebhookService,
    meetings: Arc<dyn MeetingSource>,
}

impl WebhookStep {
    /// The step over the webhooks service and the meetings the payload
    /// reads (redacted like every export).
    pub fn new(service: WebhookService, meetings: Arc<dyn MeetingSource>) -> Self {
        Self { service, meetings }
    }
}

#[async_trait]
impl Step for WebhookStep {
    fn kind(&self) -> ActionKind {
        ActionKind::Webhook
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::Webhook { webhook_id, fields } = action else {
            return Err(StepError::Unavailable);
        };
        let payload = build_payload(self.meetings.as_ref(), ctx, *fields).await?;
        let body = serde_json::to_string(&payload).map_err(WebhookError::Encode)?;
        let key = format!("run:{}:{webhook_id}", ctx.run_id);
        let delivery = self
            .service
            .deliver_payload(PayloadDelivery {
                webhook: webhook_id,
                meeting: ctx.meeting,
                key: &key,
                body: &body,
            })
            .await?;
        if delivery.status == DeliveryStatus::Failed {
            let reason = delivery.last_error.unwrap_or_default();
            return Err(StepError::Webhook(WebhookError::Delivery(reason)));
        }
        Ok(StepOutput::Webhook {
            delivery_id: delivery.id,
            delivered: delivery.status == DeliveryStatus::Succeeded,
        })
    }
}
