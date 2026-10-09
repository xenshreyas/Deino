//! Commands of the `folders` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::folders::{
    FiledMeeting, FilingService, Folder, FolderRule, FolderRuleEntry, FolderService, FolderTree,
    NewFolder, UnfiledMeeting,
};
use deino_core::meetings::Meeting;
use deino_domain::{AppError, FolderId, FolderRuleId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The tree half of the feature, as Tauri state.
type Folders<'a> = State<'a, Arc<FolderService>>;
/// The filing half of the feature, as Tauri state.
type Filing<'a> = State<'a, Arc<FilingService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        tree,
        get,
        create,
        rename,
        set_icon,
        set_description,
        move_to,
        delete,
        meetings,
        unfiled,
        folders_of,
        add_meeting,
        remove_meeting,
        rules,
        add_rule,
        remove_rule,
    ]
}

/// Every folder in display order (each followed by its subfolders), with
/// meeting counts and the size of the "Unfiled" smart folder.
#[tauri::command]
#[specta::specta]
async fn tree(folders: Folders<'_>) -> Result<FolderTree, AppError> {
    folders.tree().await.into_ipc()
}

/// The folder with `id`; fails with `not_found` if there is none.
#[tauri::command]
#[specta::specta]
async fn get(folders: Folders<'_>, id: FolderId) -> Result<Folder, AppError> {
    folders.get(&id).await.into_ipc()
}

/// Creates a folder; fails with `invalid` for a blank or overlong name and
/// `conflict` when a sibling has the name.
#[tauri::command]
#[specta::specta]
async fn create(folders: Folders<'_>, folder: NewFolder) -> Result<Folder, AppError> {
    folders.create(&folder).await.into_ipc()
}

/// Renames the folder; same rules as `create`.
#[tauri::command]
#[specta::specta]
async fn rename(folders: Folders<'_>, id: FolderId, name: String) -> Result<Folder, AppError> {
    folders.rename(&id, &name).await.into_ipc()
}

/// Sets the folder's icon (an icon name such as `briefcase`); `null`
/// restores the default.
#[tauri::command]
#[specta::specta]
async fn set_icon(
    folders: Folders<'_>,
    id: FolderId,
    icon: Option<String>,
) -> Result<Folder, AppError> {
    folders.set_icon(&id, icon.as_deref()).await.into_ipc()
}

/// Sets what the folder is for; context for chat and auto-filing.
#[tauri::command]
#[specta::specta]
async fn set_description(
    folders: Folders<'_>,
    id: FolderId,
    description: String,
) -> Result<Folder, AppError> {
    folders.set_description(&id, &description).await.into_ipc()
}

/// Moves the folder (with its subfolders and their meetings) under
/// `parentId`, or to the top level for `null`; fails with `conflict` for a
/// move into its own subtree or onto a sibling's name.
#[tauri::command]
#[specta::specta]
async fn move_to(
    folders: Folders<'_>,
    id: FolderId,
    parent_id: Option<FolderId>,
) -> Result<Folder, AppError> {
    folders.move_to(&id, parent_id.as_ref()).await.into_ipc()
}

/// Deletes the folder and its subfolders; every meeting is kept.
#[tauri::command]
#[specta::specta]
async fn delete(folders: Folders<'_>, id: FolderId) -> Result<(), AppError> {
    folders.delete(&id).await.into_ipc()
}

/// The meetings in the folder, newest first.
#[tauri::command]
#[specta::specta]
async fn meetings(filing: Filing<'_>, id: FolderId) -> Result<Vec<Meeting>, AppError> {
    filing.meetings(&id).await.into_ipc()
}

/// The meetings in no folder, newest first.
#[tauri::command]
#[specta::specta]
async fn unfiled(filing: Filing<'_>) -> Result<Vec<Meeting>, AppError> {
    filing.unfiled().await.into_ipc()
}

/// The folders the meeting sits in, by name.
#[tauri::command]
#[specta::specta]
async fn folders_of(filing: Filing<'_>, meeting_id: MeetingId) -> Result<Vec<Folder>, AppError> {
    filing.folders_of(&meeting_id).await.into_ipc()
}

/// Files the meeting into the folder. `seriesOffer` names the meeting's
/// recurring series when the folder could collect its future meetings.
#[tauri::command]
#[specta::specta]
async fn add_meeting(
    filing: Filing<'_>,
    folder_id: FolderId,
    meeting_id: MeetingId,
) -> Result<FiledMeeting, AppError> {
    filing.add_meeting(&folder_id, &meeting_id).await.into_ipc()
}

/// Takes the meeting out of the folder (and stops collecting its series).
#[tauri::command]
#[specta::specta]
async fn remove_meeting(
    filing: Filing<'_>,
    folder_id: FolderId,
    meeting_id: MeetingId,
) -> Result<UnfiledMeeting, AppError> {
    filing
        .remove_meeting(&folder_id, &meeting_id)
        .await
        .into_ipc()
}

/// The folder's auto-add rules, oldest first.
#[tauri::command]
#[specta::specta]
async fn rules(filing: Filing<'_>, folder_id: FolderId) -> Result<Vec<FolderRuleEntry>, AppError> {
    filing.rules(&folder_id).await.into_ipc()
}

/// Adds an auto-add rule (series, tag or attendee domain) to the folder.
#[tauri::command]
#[specta::specta]
async fn add_rule(
    filing: Filing<'_>,
    folder_id: FolderId,
    rule: FolderRule,
) -> Result<FolderRuleEntry, AppError> {
    filing.add_rule(&folder_id, rule).await.into_ipc()
}

/// Deletes an auto-add rule; meetings it filed stay.
#[tauri::command]
#[specta::specta]
async fn remove_rule(filing: Filing<'_>, id: FolderRuleId) -> Result<(), AppError> {
    filing.remove_rule(&id).await.into_ipc()
}
