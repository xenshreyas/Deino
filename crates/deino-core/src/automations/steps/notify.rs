//! [`NotifyStep`]: a macOS notification headed with the automation's name,
//! through the same [`Notifier`] port as action item reminders (AI-05).

use std::sync::Arc;

use async_trait::async_trait;

use crate::action_items::{Notice, Notifier};
use crate::automations::{Action, ActionKind, Step, StepContext, StepError, StepOutput};
use crate::insights::MeetingRecords;

/// Runs [`Action::Notify`].
pub struct NotifyStep {
    notifier: Arc<dyn Notifier>,
    records: Arc<dyn MeetingRecords>,
}

impl NotifyStep {
    /// The step over the notifier and the meetings' titles.
    pub fn new(notifier: Arc<dyn Notifier>, records: Arc<dyn MeetingRecords>) -> Self {
        Self { notifier, records }
    }
}

#[async_trait]
impl Step for NotifyStep {
    fn kind(&self) -> ActionKind {
        ActionKind::Notify
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::Notify { message } = action else {
            return Err(StepError::Unavailable);
        };
        let body = if message.is_empty() {
            let title = self
                .records
                .record(ctx.meeting)
                .await?
                .map(|record| record.facts.title)
                .unwrap_or_default();
            format!("Ran on “{title}”.")
        } else {
            message.clone()
        };
        self.notifier.notify(&Notice {
            title: ctx.automation.name.clone(),
            body,
        });
        Ok(StepOutput::Notified)
    }
}
