//! Join & record (ORG-02): one path for Home's Join button and the
//! reminder window's Join & record and Record only.
//!
//! [`JoinService::join`] starts the recorder and opens the call link at the
//! same time, so capture never waits for the browser or the meeting app
//! (the card's "starts capture within 2 s"). The recorder and the opener
//! are ports: the app wires the live recorder and the system opener; tests
//! pass fakes.

use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{CalendarEventId, ErrorClass, ErrorCode, MeetingId};
use serde::Deserialize;

use super::{CalendarError, CalendarEvent, CalendarService};

/// A failure from a port, kept for the log.
pub type PortError = Box<dyn Error + Send + Sync>;

/// Reads one stored event.
#[async_trait]
pub trait EventLookup: Send + Sync {
    /// The event `id`.
    async fn event(&self, id: CalendarEventId) -> Result<CalendarEvent, CalendarError>;
}

#[async_trait]
impl EventLookup for CalendarService {
    async fn event(&self, id: CalendarEventId) -> Result<CalendarEvent, CalendarError> {
        Self::event(self, id).await
    }
}

/// Opens a call link in the browser or the meeting app.
#[async_trait]
pub trait CallOpener: Send + Sync {
    /// Opens `url`.
    async fn open_call(&self, url: &str) -> Result<(), PortError>;
}

/// Starts recording a meeting for a calendar event.
#[async_trait]
pub trait EventRecorder: Send + Sync {
    /// Starts recording `event`; resolves once audio is captured.
    async fn record(&self, event: &CalendarEvent) -> Result<MeetingId, PortError>;
}

/// What the user pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum JoinMode {
    /// Open the call and record it.
    JoinAndRecord,
    /// Open the call only.
    JoinOnly,
    /// Record without opening the call (the user is already in it).
    RecordOnly,
}

impl JoinMode {
    const fn opens(self) -> bool {
        matches!(self, Self::JoinAndRecord | Self::JoinOnly)
    }

    const fn records(self) -> bool {
        matches!(self, Self::JoinAndRecord | Self::RecordOnly)
    }
}

/// Why joining failed. The text is user-safe: no titles, no links.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JoinError {
    /// The event could not be read.
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    /// The event has no call link to open.
    #[error("this event has no call link")]
    NoCallLink,
    /// The call link could not be opened.
    #[error("could not open the call link")]
    Open(#[source] PortError),
    /// The recorder did not start.
    #[error("could not start recording")]
    Record(#[source] PortError),
}

impl ErrorClass for JoinError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Calendar(error) => error.code(),
            Self::NoCallLink => ErrorCode::Invalid,
            Self::Open(_) | Self::Record(_) => ErrorCode::Unavailable,
        }
    }
}

/// Joins calls and records them.
pub struct JoinService {
    events: Arc<dyn EventLookup>,
    opener: Arc<dyn CallOpener>,
    recorder: Arc<dyn EventRecorder>,
}

impl JoinService {
    /// A service reading events from `events` (the [`CalendarService`]).
    pub fn new(
        events: Arc<dyn EventLookup>,
        opener: Arc<dyn CallOpener>,
        recorder: Arc<dyn EventRecorder>,
    ) -> Self {
        Self {
            events,
            opener,
            recorder,
        }
    }

    /// Does what `mode` says for event `id`; returns the meeting recording
    /// it, if one started. With both, the call opens while the recorder
    /// starts; a failure to open still keeps the recording.
    pub async fn join(
        &self,
        id: CalendarEventId,
        mode: JoinMode,
    ) -> Result<Option<MeetingId>, JoinError> {
        let event = self.events.event(id).await?;
        let link = event.conference_url.as_deref().filter(|_| mode.opens());
        if mode == JoinMode::JoinOnly && link.is_none() {
            return Err(JoinError::NoCallLink);
        }
        let open = async {
            match link {
                Some(url) => self.opener.open_call(url).await.map_err(JoinError::Open),
                None => Ok(()),
            }
        };
        let record = async {
            if !mode.records() {
                return Ok(None);
            }
            self.recorder
                .record(&event)
                .await
                .map(Some)
                .map_err(JoinError::Record)
        };
        let (opened, recorded) = tokio::join!(open, record);
        let meeting = recorded?;
        if let Err(error) = opened {
            log::warn!(target: "calendar", "event {id}: {error}");
            if meeting.is_none() {
                return Err(error);
            }
        }
        Ok(meeting)
    }
}

#[cfg(test)]
mod join_tests;
