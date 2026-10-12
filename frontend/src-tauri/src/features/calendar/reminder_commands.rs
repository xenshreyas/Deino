//! ORG-02's second-half commands in the `calendar` plugin: the reminder
//! window's event and dismiss, the open note the start-time trigger
//! watches, and Coming up's "Review last meeting" and "Open in Calendar".

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::ffi::OsStr;
use std::sync::Arc;

use deino_core::calendar::linking::{LastMeetingService, LinkError, RelatedMeeting};
use deino_core::calendar::ticker::{OpenNote, ReminderWindow as _};
use deino_core::calendar::{CalendarError, CalendarEvent, CalendarLink, CalendarService};
use deino_domain::{AppError, CalendarEventId, ErrorClass, ErrorCode, SystemZone};
use tauri::State;

use super::reminder::ReminderCard;
use crate::error::IntoIpc;
use crate::system_open::{OpenError, launch, open_external_url};

/// The event the reminder window is for, or `None` once it closed.
#[tauri::command]
#[specta::specta]
pub(super) async fn reminder(
    window: State<'_, Arc<ReminderCard>>,
    calendar: State<'_, Arc<CalendarService>>,
) -> Result<Option<CalendarEvent>, AppError> {
    let Some(id) = window.current() else {
        return Ok(None);
    };
    calendar.event(id).await.map(Some).into_ipc()
}

/// Closes the reminder window (dismiss, or after a button did its work).
#[tauri::command]
#[specta::specta]
pub(super) async fn dismiss_reminder(window: State<'_, Arc<ReminderCard>>) -> Result<(), AppError> {
    window.close();
    Ok(())
}

/// The UI opened the note of event `event` (`None`: it closed it). At the
/// event's start, an open note starts recording when its rules say so.
#[tauri::command]
#[specta::specta]
pub(super) async fn set_open_note(
    note: State<'_, Arc<OpenNote>>,
    event: Option<CalendarEventId>,
) -> Result<(), AppError> {
    note.set(event);
    Ok(())
}

/// "Review last meeting": the meeting before event `id` in its series, or
/// with the same people; `None` when there is none.
#[tauri::command]
#[specta::specta]
pub(super) async fn last_meeting(
    calendar: State<'_, Arc<CalendarService>>,
    history: State<'_, Arc<LastMeetingService>>,
    id: CalendarEventId,
) -> Result<Option<RelatedMeeting>, AppError> {
    let event = calendar
        .event(id)
        .await
        .map_err(LinkError::from)
        .into_ipc()?;
    history.last_meeting(&event).await.into_ipc()
}

/// "Open in Calendar": macOS Calendar for its events, the provider's web
/// calendar on the event's day for Google and Microsoft. Fails with
/// `invalid` when no link can be built.
#[tauri::command]
#[specta::specta]
pub(super) async fn open_in_calendar(
    calendar: State<'_, Arc<CalendarService>>,
    id: CalendarEventId,
) -> Result<(), AppError> {
    open_event(&calendar, id).await.into_ipc()
}

/// Why "Open in Calendar" failed.
#[derive(Debug, thiserror::Error)]
enum OpenInCalendarError {
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    #[error("this event cannot be opened in a calendar")]
    NoLink,
    #[error(transparent)]
    Open(#[from] OpenError),
}

impl ErrorClass for OpenInCalendarError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Calendar(error) => error.code(),
            Self::NoLink => ErrorCode::Invalid,
            Self::Open(error) => error.code(),
        }
    }
}

async fn open_event(
    calendar: &CalendarService,
    id: CalendarEventId,
) -> Result<(), OpenInCalendarError> {
    let link = calendar.calendar_link(id, &SystemZone).await?;
    match link.ok_or(OpenInCalendarError::NoLink)? {
        // Built in Rust from the stored event; only Calendar's own scheme.
        CalendarLink::App(url) if url.scheme() == "ical" => {
            Ok(launch(OsStr::new(url.as_str())).await?)
        }
        CalendarLink::App(_) => Err(OpenInCalendarError::NoLink),
        CalendarLink::Web(url) => Ok(open_external_url(url.as_str()).await?),
    }
}
