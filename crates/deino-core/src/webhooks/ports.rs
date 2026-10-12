//! What the webhooks service is built from ([`WebhookPorts`]), and
//! [`SubjectItems`]: the action item an `action_item.*` event is about.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{ActionItemId, ActionItemStatus, Clock, MeetingId};
use deino_net::{NetClient, ServiceSwitch};

use super::{DeliveryRepo, SubjectItem, WebhookRepo, WebhookSecrets};
use crate::action_items::ActionItemRepo;
use crate::automations::MeetingSource;
use crate::redaction::TextRedactor;

/// The ports of [`WebhookService`](super::WebhookService); the composition
/// root fills them.
#[derive(Clone)]
pub struct WebhookPorts {
    /// The endpoints.
    pub webhooks: Arc<dyn WebhookRepo>,
    /// The delivery queue and log.
    pub deliveries: Arc<dyn DeliveryRepo>,
    /// The signing secrets (the Keychain).
    pub secrets: WebhookSecrets,
    /// Meetings, as exports see them (redacted while NOT-09 is on).
    pub meetings: Arc<dyn MeetingSource>,
    /// The action items events are about.
    pub items: Arc<dyn SubjectItems>,
    /// The `webhooks` scoped client (ADR 0019).
    pub client: Arc<dyn NetClient>,
    /// Switches the `webhooks` service on with the first endpoint and off
    /// with the last.
    pub switch: Arc<dyn ServiceSwitch>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// Reads the action items `action_item.*` events are about.
#[async_trait]
pub trait SubjectItems: Send + Sync {
    /// The meeting `item` belongs to; `None` when it is gone or has none.
    async fn meeting_of(&self, item: &ActionItemId) -> Result<Option<MeetingId>, DbError>;

    /// The item as a payload shows it, redacted like the export.
    async fn subject(&self, item: &ActionItemId) -> Result<Option<SubjectItem>, DbError>;
}

/// [`SubjectItems`] over the action item repository and the export's
/// redactor (NOT-09).
pub struct RepoSubjectItems {
    repo: Arc<dyn ActionItemRepo>,
    redactor: Arc<dyn TextRedactor>,
}

impl RepoSubjectItems {
    /// The items over `repo`, their text through `redactor`.
    pub fn new(repo: Arc<dyn ActionItemRepo>, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { repo, redactor }
    }
}

#[async_trait]
impl SubjectItems for RepoSubjectItems {
    async fn meeting_of(&self, item: &ActionItemId) -> Result<Option<MeetingId>, DbError> {
        Ok(self.repo.find(item).await?.and_then(|item| item.meeting_id))
    }

    async fn subject(&self, item: &ActionItemId) -> Result<Option<SubjectItem>, DbError> {
        let Some(item) = self.repo.find(item).await? else {
            return Ok(None);
        };
        let mut texts = self
            .redactor
            .redact(vec![item.title, item.description])
            .await
            .into_iter();
        Ok(Some(SubjectItem {
            id: item.id,
            title: texts.next().unwrap_or_default(),
            description: texts.next().unwrap_or_default(),
            status: if item.status == ActionItemStatus::Done {
                "done"
            } else {
                "open"
            },
            due_at: item.due_at,
        }))
    }
}
