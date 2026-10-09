//! Commands of the `templates` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::templates::{
    MeetingTemplateService, Template, TemplateChoice, TemplateContent, TemplatePreview,
    TemplateService,
};
use deino_domain::{AppError, MeetingId, TemplateId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The template library in Tauri state.
type Library<'a> = State<'a, Arc<TemplateService>>;
/// Per-meeting templates in Tauri state.
type Meetings<'a> = State<'a, Arc<MeetingTemplateService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        get,
        create,
        update,
        duplicate,
        delete,
        import_json,
        export_json,
        preview,
        meeting_template,
        set_meeting_template
    ]
}

/// Every template, built-in and the user's, by name.
#[tauri::command]
#[specta::specta]
async fn list(library: Library<'_>) -> Result<Vec<Template>, AppError> {
    library.list().await.into_ipc()
}

/// One template; `not_found` if there is none.
#[tauri::command]
#[specta::specta]
async fn get(library: Library<'_>, id: TemplateId) -> Result<Template, AppError> {
    library.get(&id).await.into_ipc()
}

/// Saves a new template of the user's; `invalid` names the broken rule.
#[tauri::command]
#[specta::specta]
async fn create(library: Library<'_>, content: TemplateContent) -> Result<Template, AppError> {
    library.create(content).await.into_ipc()
}

/// Replaces a user template's content; `conflict` for a built-in.
#[tauri::command]
#[specta::specta]
async fn update(
    library: Library<'_>,
    id: TemplateId,
    content: TemplateContent,
) -> Result<Template, AppError> {
    library.update(&id, content).await.into_ipc()
}

/// An editable copy of any template, named "<name> (copy)".
#[tauri::command]
#[specta::specta]
async fn duplicate(library: Library<'_>, id: TemplateId) -> Result<Template, AppError> {
    library.duplicate(&id).await.into_ipc()
}

/// Deletes a user template; `conflict` for a built-in.
#[tauri::command]
#[specta::specta]
async fn delete(library: Library<'_>, id: TemplateId) -> Result<(), AppError> {
    library.delete(&id).await.into_ipc()
}

/// Saves a template file (Deino's format or the pre-NOT-04 one) as a new template.
#[tauri::command]
#[specta::specta]
async fn import_json(library: Library<'_>, json: String) -> Result<Template, AppError> {
    library.import(&json).await.into_ipc()
}

/// The template as a shareable JSON file.
#[tauri::command]
#[specta::specta]
async fn export_json(library: Library<'_>, id: TemplateId) -> Result<String, AppError> {
    library.export(&id).await.into_ipc()
}

/// `content` (saved or not) rendered against `meeting`, without an LLM call.
#[tauri::command]
#[specta::specta]
async fn preview(
    meetings: Meetings<'_>,
    content: TemplateContent,
    meeting: MeetingId,
) -> Result<TemplatePreview, AppError> {
    meetings.preview(content, &meeting).await.into_ipc()
}

/// The template `meeting` gets, and why.
#[tauri::command]
#[specta::specta]
async fn meeting_template(
    meetings: Meetings<'_>,
    meeting: MeetingId,
) -> Result<TemplateChoice, AppError> {
    meetings.choice(&meeting).await.into_ipc()
}

/// Chooses `template` for `meeting`, or `null` to go back to automatic
/// selection; returns the meeting's new choice.
#[tauri::command]
#[specta::specta]
async fn set_meeting_template(
    meetings: Meetings<'_>,
    meeting: MeetingId,
    template: Option<TemplateId>,
) -> Result<TemplateChoice, AppError> {
    meetings.set(&meeting, template.as_ref()).await.into_ipc()
}
