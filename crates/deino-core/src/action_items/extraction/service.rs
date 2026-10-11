//! [`ExtractionService`]: the `extract_action_items` stage's rules.
//!
//! 1. Load the meeting's context ([`ContextSources::load`]).
//! 2. Ask the model for its commitments ([`ActionItemModel`]).
//! 3. Resolve each item: title, assignee, due day, evidence
//!    ([`resolve_items`]).
//! 4. Drop items the meeting already holds in a form the user touched
//!    ([`without_repeats`]).
//! 5. Replace the meeting's untouched AI items with the rest, in one
//!    transaction ([`ActionItemBatch::replace_untouched_ai`]).
//!
//! Running it again replaces only what it wrote and the user never touched,
//! so the stage is idempotent and never loses an edit or a tick.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{ActionItemId, Clock, LocalZone, MeetingId};

use super::context::ContextSources;
use super::llm::ActionItemModel;
use super::resolve::{Resolution, resolve_items, without_repeats};
use super::{ActionItemExtractor, ExtractionError, ExtractionOutcome};
use crate::action_items::{
    ActionItem, ActionItemBatch, ActionItemChange, ActionItemChangeKind, ActionItemEvents,
    ActionItemRepo,
};

/// The ports extraction works through.
#[derive(Clone)]
pub struct ExtractionDeps {
    /// The meeting's context.
    pub sources: ContextSources,
    /// The language model.
    pub model: Arc<dyn ActionItemModel>,
    /// The meeting's current items.
    pub items: Arc<dyn ActionItemRepo>,
    /// The replacement write.
    pub batch: Arc<dyn ActionItemBatch>,
    /// Where changes are announced.
    pub events: Arc<dyn ActionItemEvents>,
    /// Creation times and IDs.
    pub clock: Arc<dyn Clock>,
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
}

/// Extracts a meeting's action items with the LLM.
pub struct ExtractionService {
    deps: ExtractionDeps,
}

impl ExtractionService {
    /// The service over its ports.
    pub fn new(deps: ExtractionDeps) -> Self {
        Self { deps }
    }
}

#[async_trait]
impl ActionItemExtractor for ExtractionService {
    async fn extract(&self, meeting: &MeetingId) -> Result<ExtractionOutcome, ExtractionError> {
        let zone = self.deps.zone.as_ref();
        let context = self
            .deps
            .sources
            .load(meeting, zone)
            .await?
            .ok_or_else(|| ExtractionError::MeetingNotFound(meeting.clone()))?;
        if context.transcript.lines.is_empty() {
            return Ok(ExtractionOutcome::NoTranscript);
        }
        let answer = self.deps.model.extract(&context.prompt).await?;
        let resolution = Resolution {
            meeting,
            context: &context,
            zone,
            clock: self.deps.clock.as_ref(),
        };
        let existing = self.deps.items.list_for_meeting(meeting).await?;
        let fresh = without_repeats(resolve_items(&answer.items, resolution), &existing);
        let removed = self
            .deps
            .batch
            .replace_untouched_ai(meeting, &fresh)
            .await?;
        self.announce(&existing, &fresh);
        log::info!(
            target: "deino_core::action_items",
            "extracted action items for meeting {meeting}: {} answered, {} saved, {removed} replaced",
            answer.items.len(),
            fresh.len()
        );
        Ok(ExtractionOutcome::Extracted {
            saved: fresh.len(),
            replaced: removed,
        })
    }
}

impl ExtractionService {
    /// Announces the replaced items as deleted and the new ones as created.
    fn announce(&self, existing: &[ActionItem], fresh: &[ActionItem]) {
        let replaced: Vec<ActionItemId> = existing
            .iter()
            .filter(|item| item.is_untouched_ai())
            .map(|item| item.id)
            .collect();
        let created: Vec<ActionItemId> = fresh.iter().map(|item| item.id).collect();
        for (kind, ids) in [
            (ActionItemChangeKind::Deleted, replaced),
            (ActionItemChangeKind::Created, created),
        ] {
            if !ids.is_empty() {
                self.deps.events.changed(&ActionItemChange { kind, ids });
            }
        }
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
