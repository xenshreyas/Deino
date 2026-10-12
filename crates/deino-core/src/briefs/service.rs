//! [`BriefService`]: prepares an event's brief (only when its inputs
//! changed) and serves it to the UI: the read API ORG-02's event note and
//! 1-minute notification use is [`BriefService::for_event`].

use std::sync::Arc;

use deino_domain::{CalendarEventId, Clock, MeetingId};

use super::{
    Brief, BriefError, BriefFeedback, BriefRepo, BriefRun, BriefSources, BriefStatus, BriefWriter,
    StoredBrief,
};
use crate::calendar::{CalendarEvent, CalendarEventRepo};
use crate::meetings::MeetingLifecycleRepo;

/// What [`BriefService`] is built from.
#[derive(Clone)]
pub struct BriefDeps {
    /// The `briefs` table.
    pub store: Arc<dyn BriefRepo>,
    /// Calendar events by ID.
    pub events: Arc<dyn CalendarEventRepo>,
    /// A meeting's calendar link.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// What briefs are written from.
    pub sources: BriefSources,
    /// The model.
    pub writer: Arc<dyn BriefWriter>,
    /// Timestamps.
    pub clock: Arc<dyn Clock>,
}

/// Prepares and serves pre-meeting briefs.
pub struct BriefService {
    deps: BriefDeps,
}

impl BriefService {
    /// A service over `deps`.
    pub fn new(deps: BriefDeps) -> Self {
        Self { deps }
    }

    /// The brief of `event`, or `None` when there is none (not prepared
    /// yet, or too little to say).
    pub async fn for_event(&self, event: &CalendarEventId) -> Result<Option<Brief>, BriefError> {
        let stored = self.deps.store.find(event).await?;
        Ok(stored.and_then(StoredBrief::into_brief))
    }

    /// The brief of the event `meeting` is linked to, if any.
    pub async fn for_meeting(&self, meeting: &MeetingId) -> Result<Option<Brief>, BriefError> {
        let state = self.deps.meetings.state(meeting).await?;
        match state.and_then(|state| state.calendar_event_id) {
            Some(event) => self.for_event(&event).await,
            None => Ok(None),
        }
    }

    /// Records that the user saw the brief; the card collapses from now on.
    pub async fn mark_read(&self, event: &CalendarEventId) -> Result<(), BriefError> {
        let now = self.deps.clock.now();
        Ok(self.deps.store.mark_read(event, now).await?)
    }

    /// Stores the user's verdict on the brief.
    pub async fn give_feedback(
        &self,
        event: &CalendarEventId,
        feedback: BriefFeedback,
    ) -> Result<(), BriefError> {
        let now = self.deps.clock.now();
        Ok(self.deps.store.set_feedback(event, feedback, now).await?)
    }

    /// Prepares the brief of the stored event `id`.
    pub async fn prepare_event(&self, id: &CalendarEventId) -> Result<BriefRun, BriefError> {
        let event = self.deps.events.event(id).await?;
        let event = event.ok_or(BriefError::EventNotFound(*id))?;
        self.prepare(&event).await
    }

    /// Writes `event`'s brief unless its inputs are unchanged since the last
    /// run: the local check first, then the model, whose "insufficient"
    /// answer (or too few cited bullets) also means no brief.
    pub async fn prepare(&self, event: &CalendarEvent) -> Result<BriefRun, BriefError> {
        let own_notes = self.deps.store.notes_of(&event.id).await?;
        let inputs = self.deps.sources.gather(event, &own_notes).await?;
        let rendered = inputs.render();
        let previous = self.deps.store.find(&event.id).await?;
        if previous.is_some_and(|stored| stored.inputs_hash == rendered.fingerprint) {
            return Ok(BriefRun::Unchanged);
        }
        let bullets = if inputs.is_sufficient() {
            let answer = self.deps.writer.write(rendered.prompt).await?;
            rendered.index.bullets(answer)
        } else {
            None
        };
        let (status, run) = match bullets {
            Some(_) => (BriefStatus::Ready, BriefRun::Written),
            None => (BriefStatus::Insufficient, BriefRun::Insufficient),
        };
        let brief = StoredBrief {
            event_id: event.id,
            status,
            inputs_hash: rendered.fingerprint,
            bullets: bullets.unwrap_or_default(),
            generated_at: self.deps.clock.now(),
            read_at: None,
            feedback: None,
        };
        self.deps.store.save(&brief).await?;
        log::debug!(target: "briefs", "brief for event {}: {run:?}", event.id);
        Ok(run)
    }
}
