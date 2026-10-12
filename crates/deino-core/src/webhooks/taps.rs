//! Taps: the existing services' event ports, wrapped so each moment also
//! reaches the webhooks as a [`WebhookSignal`]. Nothing polls; the inner
//! port always runs first and keeps its own behaviour.
//!
//! | Event | Tap | Port |
//! |---|---|---|
//! | `meeting.ready` | [`ReadyObserver`] | the lifecycle's `MeetingStatusObserver` (`processing` → `ready`) |
//! | `notes.generated`, `notes.edited` | [`TappedNotesWriter`] | `EnhancedNotesWriter` (version 1 is generated; later versions and edits are edits) |
//! | `action_item.created`, `action_item.updated` | [`TappedActionItemEvents`] | `ActionItemEvents` |
//! | `insight.generated` | [`TappedInsightGenerator`] | `InsightGenerator` (a run that answered) |

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{MeetingId, MeetingStatus};

use super::{WebhookEvent, WebhookSignal, WebhookSink};
use crate::action_items::{ActionItemChange, ActionItemChangeKind, ActionItemEvents};
use crate::enhance::{EnhancedNotesWriter, NewVersion, VersionEdit};
use crate::insights::{Attempt, GenerateOutcome, InsightError, InsightGenerator};
use crate::meetings::{MeetingStatusObserver, StatusChange};

/// Announces `meeting.ready` when a meeting leaves `processing` for `ready`.
pub struct ReadyObserver {
    sink: Arc<dyn WebhookSink>,
}

impl ReadyObserver {
    /// The observer, announcing to `sink`.
    pub fn new(sink: Arc<dyn WebhookSink>) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl MeetingStatusObserver for ReadyObserver {
    async fn status_changed(&self, id: &MeetingId, change: StatusChange) {
        if change.from == MeetingStatus::Processing && change.to == MeetingStatus::Ready {
            self.sink.emit(WebhookSignal::meeting(
                WebhookEvent::MeetingReady,
                id.clone(),
            ));
        }
    }
}

/// [`ActionItemEvents`] that also announces `action_item.*`.
pub struct TappedActionItemEvents {
    inner: Arc<dyn ActionItemEvents>,
    sink: Arc<dyn WebhookSink>,
}

impl TappedActionItemEvents {
    /// `inner`, tapped into `sink`.
    pub fn new(inner: Arc<dyn ActionItemEvents>, sink: Arc<dyn WebhookSink>) -> Self {
        Self { inner, sink }
    }
}

impl ActionItemEvents for TappedActionItemEvents {
    fn changed(&self, change: &ActionItemChange) {
        self.inner.changed(change);
        let event = match change.kind {
            ActionItemChangeKind::Created => WebhookEvent::ActionItemCreated,
            ActionItemChangeKind::Updated
            | ActionItemChangeKind::Completed
            | ActionItemChangeKind::Reopened => WebhookEvent::ActionItemUpdated,
            ActionItemChangeKind::Deleted => return,
        };
        for id in &change.ids {
            self.sink.emit(WebhookSignal::item(event, *id));
        }
    }
}

/// [`EnhancedNotesWriter`] that also announces `notes.generated` and
/// `notes.edited`.
pub struct TappedNotesWriter {
    inner: Arc<dyn EnhancedNotesWriter>,
    sink: Arc<dyn WebhookSink>,
}

impl TappedNotesWriter {
    /// `inner`, tapped into `sink`.
    pub fn new(inner: Arc<dyn EnhancedNotesWriter>, sink: Arc<dyn WebhookSink>) -> Self {
        Self { inner, sink }
    }
}

#[async_trait]
impl EnhancedNotesWriter for TappedNotesWriter {
    async fn insert(&self, version: &NewVersion) -> Result<u32, DbError> {
        let number = self.inner.insert(version).await?;
        let event = if number == 1 && !version.is_user_edited {
            WebhookEvent::NotesGenerated
        } else {
            WebhookEvent::NotesEdited
        };
        self.sink
            .emit(WebhookSignal::meeting(event, version.meeting_id.clone()));
        Ok(number)
    }

    async fn save_edit(&self, edit: &VersionEdit) -> Result<bool, DbError> {
        let saved = self.inner.save_edit(edit).await?;
        if saved {
            let signal = WebhookSignal::meeting(WebhookEvent::NotesEdited, edit.meeting_id.clone());
            self.sink.emit(signal);
        }
        Ok(saved)
    }
}

/// [`InsightGenerator`] that also announces `insight.generated`.
pub struct TappedInsightGenerator {
    inner: Arc<dyn InsightGenerator>,
    sink: Arc<dyn WebhookSink>,
}

impl TappedInsightGenerator {
    /// `inner`, tapped into `sink`.
    pub fn new(inner: Arc<dyn InsightGenerator>, sink: Arc<dyn WebhookSink>) -> Self {
        Self { inner, sink }
    }
}

#[async_trait]
impl InsightGenerator for TappedInsightGenerator {
    async fn generate(
        &self,
        meeting: &MeetingId,
        attempt: Attempt,
    ) -> Result<GenerateOutcome, InsightError> {
        let outcome = self.inner.generate(meeting, attempt).await?;
        if matches!(outcome, GenerateOutcome::Answered { insights } if insights > 0) {
            let signal = WebhookSignal::meeting(WebhookEvent::InsightGenerated, meeting.clone());
            self.sink.emit(signal);
        }
        Ok(outcome)
    }
}
