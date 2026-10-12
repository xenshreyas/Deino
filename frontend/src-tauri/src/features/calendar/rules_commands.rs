//! ORG-02's commands in the `calendar` plugin: Coming up with each event's
//! auto-record decision, the record toggle, and Join (Home and the
//! reminder window share it).

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::calendar::TimeRange;
use deino_core::calendar::join::{JoinMode, JoinService};
use deino_core::calendar::rules::{AutoRecordService, PlannedEvent, RecordChoice, RecordDecision};
use deino_domain::{AppError, CalendarEventId, MeetingId};
use tauri::State;

use crate::error::IntoIpc;

/// The visible events overlapping `range`, by start, each with whether it
/// records on its own and why.
#[tauri::command]
#[specta::specta]
pub(super) async fn record_plan(
    rules: State<'_, Arc<AutoRecordService>>,
    range: TimeRange,
) -> Result<Vec<PlannedEvent>, AppError> {
    rules.plan(range).await.into_ipc()
}

/// The record toggle: always, never, or back to the rules, for this event
/// or all occurrences of its series. Returns the event's new decision.
#[tauri::command]
#[specta::specta]
pub(super) async fn choose_record(
    rules: State<'_, Arc<AutoRecordService>>,
    id: CalendarEventId,
    choice: RecordChoice,
) -> Result<RecordDecision, AppError> {
    rules.choose(id, choice).await.into_ipc()
}

/// Opens the event's call and/or records it, as `mode` says; returns the
/// meeting being recorded, if one started. Fails with `invalid` when Join
/// only meets an event without a call link, and `unavailable` when the
/// recorder cannot start.
#[tauri::command]
#[specta::specta]
pub(super) async fn join(
    joins: State<'_, Arc<JoinService>>,
    id: CalendarEventId,
    mode: JoinMode,
) -> Result<Option<MeetingId>, AppError> {
    joins.join(id, mode).await.into_ipc()
}
