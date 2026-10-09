//! Folders: nested collections of meetings.

use chrono::{DateTime, Utc};
use deino_domain::FolderId;
use serde::Serialize;

/// A folder of meetings. Folders nest to any depth; a meeting can sit in
/// many folders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// The folder's ID.
    pub id: FolderId,
    /// The folder it sits in; `None` at the top level.
    pub parent_id: Option<FolderId>,
    /// Its name; unique among its siblings, ignoring case.
    pub name: String,
    /// An icon name from the UI's icon set; `None` shows the default.
    pub icon: Option<String>,
    /// What belongs here, as context for chat and auto-filing.
    pub description: String,
    /// When the folder was created.
    pub created_at: DateTime<Utc>,
    /// When the folder last changed.
    pub updated_at: DateTime<Utc>,
}

/// What happened when a folder was asked to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderMove {
    /// The folder now sits under the new parent.
    Moved,
    /// No folder has that ID.
    NotFound,
    /// The new parent is the folder itself or one of its descendants, which
    /// would make a cycle; nothing changed.
    WouldCycle,
}
