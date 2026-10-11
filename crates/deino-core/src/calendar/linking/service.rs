//! [`EventLinkService`]: links meetings to calendar events.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{CalendarEventId, Clock, MeetingId};

use super::nearest::{link_range, nearest_event, rank_suggestions, suggestion_range};
use super::{EventAttendance, EventLinkChange, LinkError, TitleRule};
use crate::calendar::{
    CalendarError, CalendarEvent, CalendarEventRepo, CalendarService, TimeRange,
};
use crate::meetings::{
    MAX_TITLE_CHARS, MeetingLifecycleRepo, MeetingLink, MeetingRepo, MeetingState, MeetingTitle,
};

/// The events the user sees on their calendar: visible calendars of active
/// sources, declined events and duplicates left out.
#[async_trait]
pub trait EventAgenda: Send + Sync {
    /// The visible events overlapping `range`, by start.
    async fn events(&self, range: TimeRange) -> Result<Vec<CalendarEvent>, CalendarError>;
}

#[async_trait]
impl EventAgenda for CalendarService {
    async fn events(&self, range: TimeRange) -> Result<Vec<CalendarEvent>, CalendarError> {
        self.upcoming(range).await
    }
}

/// What the service is built over.
pub struct EventLinkDeps {
    /// Titles and creation times.
    pub meetings: Arc<dyn MeetingRepo>,
    /// The link itself (`meetings.calendar_event_id`, `series_id`).
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// Stored events, by ID.
    pub events: Arc<dyn CalendarEventRepo>,
    /// The visible events in a span.
    pub agenda: Arc<dyn EventAgenda>,
    /// Invitees into attendees.
    pub attendance: EventAttendance,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// Links meetings to calendar events: automatically when recording starts,
/// and from the meeting header's picker.
pub struct EventLinkService {
    meetings: Arc<dyn MeetingRepo>,
    lifecycle: Arc<dyn MeetingLifecycleRepo>,
    events: Arc<dyn CalendarEventRepo>,
    agenda: Arc<dyn EventAgenda>,
    attendance: EventAttendance,
    clock: Arc<dyn Clock>,
}

impl EventLinkService {
    /// The service over its ports.
    pub fn new(deps: EventLinkDeps) -> Self {
        Self {
            meetings: deps.meetings,
            lifecycle: deps.lifecycle,
            events: deps.events,
            agenda: deps.agenda,
            attendance: deps.attendance,
            clock: deps.clock,
        }
    }

    /// The event `meeting` is linked to; `None` when it has none, or the
    /// event was deleted from the calendar.
    pub async fn linked(&self, meeting: &MeetingId) -> Result<Option<CalendarEvent>, LinkError> {
        let state = self.state(meeting).await?;
        self.event_of(&state).await
    }

    /// The events the picker offers for `meeting`: those of the day before
    /// it started (or was created) up to an hour after, closest first.
    pub async fn suggestions(&self, meeting: &MeetingId) -> Result<Vec<CalendarEvent>, LinkError> {
        let state = self.state(meeting).await?;
        let anchor = match state.started_at {
            Some(started_at) => started_at,
            None => self.created_at(meeting).await?,
        };
        let events = self.agenda.events(suggestion_range(anchor)).await?;
        Ok(rank_suggestions(events, anchor))
    }

    /// Links `meeting` to `event` (re-linking replaces the previous event's
    /// attendees) and titles it after the event. Works at any point of the
    /// meeting's life, recording included.
    pub async fn link(
        &self,
        meeting: &MeetingId,
        event: &CalendarEventId,
    ) -> Result<EventLinkChange, LinkError> {
        let next = self
            .events
            .event(event)
            .await?
            .ok_or(LinkError::EventNotFound(*event))?;
        let state = self.state(meeting).await?;
        self.apply(&state, Some(next), TitleRule::FromEvent).await
    }

    /// Unlinks `meeting` from its event and series and takes the event's
    /// invitees off it; the title stays.
    pub async fn unlink(&self, meeting: &MeetingId) -> Result<EventLinkChange, LinkError> {
        let state = self.state(meeting).await?;
        self.apply(&state, None, TitleRule::Keep).await
    }

    /// Links a meeting that just started recording to the event nearest to
    /// now ([`nearest_event`]); `None` when it is already linked or no event
    /// is near.
    pub async fn auto_link(
        &self,
        meeting: &MeetingId,
        title: TitleRule,
    ) -> Result<Option<EventLinkChange>, LinkError> {
        let state = self.state(meeting).await?;
        if state.calendar_event_id.is_some() {
            return Ok(None);
        }
        let now = self.clock.now();
        let events = self.agenda.events(link_range(now)).await?;
        let Some(event) = nearest_event(&events, now).cloned() else {
            return Ok(None);
        };
        self.apply(&state, Some(event), title).await.map(Some)
    }

    /// Stores the link to `next`, then the attendees, then the title.
    async fn apply(
        &self,
        state: &MeetingState,
        next: Option<CalendarEvent>,
        title: TitleRule,
    ) -> Result<EventLinkChange, LinkError> {
        let meeting = &state.id;
        let previous = self.event_of(state).await?;
        let link = MeetingLink {
            calendar_event_id: next.as_ref().map(|event| event.id),
            series_id: next.as_ref().and_then(|event| event.series_id.clone()),
        };
        self.lifecycle
            .link(meeting, &link, self.clock.now())
            .await?;
        let attendees = self
            .attendance
            .replace(meeting, previous.as_ref(), next.as_ref())
            .await?;
        let renamed = match (&next, title) {
            (Some(event), TitleRule::FromEvent) => self.retitle(meeting, event).await?,
            _ => None,
        };
        Ok(EventLinkChange {
            meeting_id: meeting.clone(),
            event: next,
            title: renamed,
            attendees_added: attendees.added,
            attendees_removed: attendees.removed,
        })
    }

    /// Renames the meeting after `event`; its new title, or `None` when the
    /// event has no title or the meeting already had it.
    async fn retitle(
        &self,
        meeting: &MeetingId,
        event: &CalendarEvent,
    ) -> Result<Option<String>, LinkError> {
        let shortened: String = event.title.trim().chars().take(MAX_TITLE_CHARS).collect();
        let Ok(title) = MeetingTitle::parse(&shortened) else {
            return Ok(None);
        };
        let current = self.meetings.find(meeting).await?;
        if current.is_some_and(|found| found.title == title.as_str()) {
            return Ok(None);
        }
        let renamed = self
            .meetings
            .rename(meeting, &title, self.clock.now())
            .await?;
        Ok(renamed.map(|found| found.title))
    }

    async fn state(&self, meeting: &MeetingId) -> Result<MeetingState, LinkError> {
        self.lifecycle
            .state(meeting)
            .await?
            .ok_or_else(|| LinkError::MeetingNotFound(meeting.clone()))
    }

    async fn created_at(&self, meeting: &MeetingId) -> Result<DateTime<Utc>, LinkError> {
        let found = self.meetings.find(meeting).await?;
        found
            .map(|found| found.created_at)
            .ok_or_else(|| LinkError::MeetingNotFound(meeting.clone()))
    }

    async fn event_of(&self, state: &MeetingState) -> Result<Option<CalendarEvent>, LinkError> {
        match &state.calendar_event_id {
            Some(id) => Ok(self.events.event(id).await?),
            None => Ok(None),
        }
    }
}
