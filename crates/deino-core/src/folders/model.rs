//! Folders: nested collections of meetings.

use chrono::{DateTime, Utc};
use deino_domain::{FolderId, SeriesId};
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

/// One folder in the sidebar tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeEntry {
    /// The folder.
    pub folder: Folder,
    /// How deep it sits: 0 at the top level.
    pub depth: u32,
    /// The meetings outside the trash filed directly in it.
    pub meeting_count: u32,
}

/// The whole folder tree in display order: each folder followed by its
/// subfolders (depth-first, siblings by name), so the UI renders it in one
/// pass and hides a collapsed folder's descendants by depth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderTree {
    /// Every folder, in display order.
    pub entries: Vec<FolderTreeEntry>,
    /// How many meetings sit in no folder (the "Unfiled" smart folder).
    pub unfiled_count: u32,
}

/// What filing a meeting into a folder led to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FiledMeeting {
    /// The meeting's recurring series when the folder does not collect it
    /// yet: the UI asks "Auto-add future meetings in this series?" and adds
    /// a series rule if the user agrees. `None` for one-off meetings and
    /// series the folder already collects.
    pub series_offer: Option<SeriesId>,
}

/// What taking a meeting out of a folder led to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnfiledMeeting {
    /// True when the folder also stopped collecting the meeting's series:
    /// taking one meeting of a series out removes the series rule (Granola's
    /// behaviour), so the UI can say so.
    pub series_rule_removed: bool,
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
