//! Commands of the `automations` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::automations::{
    Automation, AutomationInput, AutomationService, MatchingMeeting, MeetingAutomation, StepInfo,
};
use deino_domain::{AppError, AutomationId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The service in Tauri state.
type Automations<'a> = State<'a, Arc<AutomationService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        catalog,
        list,
        get,
        create,
        update,
        delete,
        duplicate,
        for_meeting,
        matching,
        run,
        test_latest
    ]
}

/// Every kind of step, ready or "WIP", in picker order.
#[tauri::command]
#[specta::specta]
async fn catalog(automations: Automations<'_>) -> Result<Vec<StepInfo>, AppError> {
    Ok(automations.catalog())
}

/// Every automation, oldest first.
#[tauri::command]
#[specta::specta]
async fn list(automations: Automations<'_>) -> Result<Vec<Automation>, AppError> {
    automations.list().await.into_ipc()
}

/// One automation; `not_found` when it does not exist.
#[tauri::command]
#[specta::specta]
async fn get(automations: Automations<'_>, id: AutomationId) -> Result<Automation, AppError> {
    automations.get(&id).await.into_ipc()
}

/// Creates an automation; `invalid` or `conflict` (name taken) when it cannot.
#[tauri::command]
#[specta::specta]
async fn create(
    automations: Automations<'_>,
    input: AutomationInput,
) -> Result<Automation, AppError> {
    automations.create(input).await.into_ipc()
}

/// Replaces an automation's settings, conditions and steps.
#[tauri::command]
#[specta::specta]
async fn update(
    automations: Automations<'_>,
    id: AutomationId,
    input: AutomationInput,
) -> Result<Automation, AppError> {
    automations.update(&id, input).await.into_ipc()
}

/// Deletes an automation and its runs.
#[tauri::command]
#[specta::specta]
async fn delete(automations: Automations<'_>, id: AutomationId) -> Result<(), AppError> {
    automations.delete(&id).await.into_ipc()
}

/// A switched-off copy named "… (copy)".
#[tauri::command]
#[specta::specta]
async fn duplicate(automations: Automations<'_>, id: AutomationId) -> Result<Automation, AppError> {
    automations.duplicate(&id).await.into_ipc()
}

/// A meeting's ⋯ → Automations menu: every automation, whether it matches
/// and its run there.
#[tauri::command]
#[specta::specta]
async fn for_meeting(
    automations: Automations<'_>,
    meeting_id: MeetingId,
) -> Result<Vec<MeetingAutomation>, AppError> {
    automations.for_meeting(&meeting_id).await.into_ipc()
}

/// The finished meetings an automation matches, with its runs there.
#[tauri::command]
#[specta::specta]
async fn matching(
    automations: Automations<'_>,
    id: AutomationId,
) -> Result<Vec<MatchingMeeting>, AppError> {
    automations.matching(&id).await.into_ipc()
}

/// Runs (or reruns) an automation on the picked meetings.
#[tauri::command]
#[specta::specta]
async fn run(
    automations: Automations<'_>,
    id: AutomationId,
    meeting_ids: Vec<MeetingId>,
) -> Result<(), AppError> {
    automations.run(&id, &meeting_ids).await.into_ipc()
}

/// Runs an automation on the newest finished meeting; returns that meeting.
#[tauri::command]
#[specta::specta]
async fn test_latest(
    automations: Automations<'_>,
    id: AutomationId,
) -> Result<MeetingId, AppError> {
    automations.test_on_latest(&id).await.into_ipc()
}
