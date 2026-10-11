//! Commands of the `calendar` plugin: sync, Settings → Calendars, and what
//! "Coming up" lists.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::calendar::{
    CalendarEvent, CalendarService, CalendarStatus, CalendarView, TimeRange,
};
use deino_domain::{AppError, CalendarEventId, CalendarId, CalendarProvider};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, for its `tauri_specta::Builder`.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        status,
        enable,
        disable,
        refresh,
        open_access_settings,
        list_calendars,
        set_calendar_visible,
        upcoming,
        event,
        connect,
        disconnect,
    ]
}

/// Whether sync is on, macOS Calendars access, and the last sync's outcome.
#[tauri::command]
#[specta::specta]
async fn status(calendar: State<'_, Arc<CalendarService>>) -> Result<CalendarStatus, AppError> {
    Ok(calendar.status().await)
}

/// Turns sync on, asking macOS for Calendars access the first time, and
/// syncs. With access denied, sync stays off and `access` says why.
#[tauri::command]
#[specta::specta]
async fn enable(calendar: State<'_, Arc<CalendarService>>) -> Result<CalendarStatus, AppError> {
    calendar.enable().await.into_ipc()
}

/// Turns sync off; synced events stay for the meetings linked to them.
#[tauri::command]
#[specta::specta]
async fn disable(calendar: State<'_, Arc<CalendarService>>) -> Result<CalendarStatus, AppError> {
    calendar.disable().await.into_ipc()
}

/// Syncs now (when sync is on) and returns the status.
#[tauri::command]
#[specta::specta]
async fn refresh(calendar: State<'_, Arc<CalendarService>>) -> Result<CalendarStatus, AppError> {
    Ok(calendar.refresh().await)
}

/// Opens System Settings at Privacy & Security → Calendars; false when it
/// could not be opened.
#[tauri::command]
#[specta::specta]
async fn open_access_settings(calendar: State<'_, Arc<CalendarService>>) -> Result<bool, AppError> {
    Ok(calendar.open_access_settings())
}

/// Every calendar with its account, colour and visibility.
#[tauri::command]
#[specta::specta]
async fn list_calendars(
    calendar: State<'_, Arc<CalendarService>>,
) -> Result<Vec<CalendarView>, AppError> {
    calendar.calendars().await.into_ipc()
}

/// Shows or hides a calendar's events and returns the calendar.
#[tauri::command]
#[specta::specta]
async fn set_calendar_visible(
    calendar: State<'_, Arc<CalendarService>>,
    id: CalendarId,
    visible: bool,
) -> Result<CalendarView, AppError> {
    calendar.set_calendar_visible(id, visible).await.into_ipc()
}

/// The visible events overlapping `range`, by start; empty while sync is off.
#[tauri::command]
#[specta::specta]
async fn upcoming(
    calendar: State<'_, Arc<CalendarService>>,
    range: TimeRange,
) -> Result<Vec<CalendarEvent>, AppError> {
    calendar.upcoming(range).await.into_ipc()
}

/// One synced event.
#[tauri::command]
#[specta::specta]
async fn event(
    calendar: State<'_, Arc<CalendarService>>,
    id: CalendarEventId,
) -> Result<CalendarEvent, AppError> {
    calendar.event(id).await.into_ipc()
}

/// Connects a Google or Microsoft account: opens its sign-in page in the
/// browser, waits for the user, then syncs. Fails with `invalid` until its
/// client ID is set, and `permission_denied` when the user declines.
#[tauri::command]
#[specta::specta]
async fn connect(
    calendar: State<'_, Arc<CalendarService>>,
    provider: CalendarProvider,
) -> Result<CalendarStatus, AppError> {
    calendar.connect(provider).await.into_ipc()
}

/// Disconnects a Google or Microsoft account: forgets its sign-in and its
/// synced calendars.
#[tauri::command]
#[specta::specta]
async fn disconnect(
    calendar: State<'_, Arc<CalendarService>>,
    provider: CalendarProvider,
) -> Result<CalendarStatus, AppError> {
    calendar.disconnect(provider).await.into_ipc()
}
