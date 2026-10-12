//! Watched folders, the files they see, and the failures they keep.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use deino_domain::{ImportFailure, MeetingId, WatchedFileId, WatchedFolderId};
use serde::Serialize;

/// A folder Deino imports new recordings from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFolder {
    /// Its ID.
    pub id: WatchedFolderId,
    /// Its absolute path.
    pub path: String,
    /// Whether an imported file moves into the folder's `Processed/`.
    pub move_to_processed: bool,
    /// When it was added.
    pub created_at: DateTime<Utc>,
}

/// One media file as a scan saw it. Its path, size and modification time
/// together are its fingerprint in the ledger.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sighting {
    /// Its absolute path.
    pub path: PathBuf,
    /// Its size in bytes.
    pub size_bytes: u64,
    /// When it last changed, in milliseconds since the Unix epoch.
    pub modified_ms: i64,
}

/// A file a watched folder handled: imported, failed, or already there when
/// the folder was added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandledFile {
    /// The ledger row's ID.
    pub id: WatchedFileId,
    /// The folder that saw it.
    pub folder_id: WatchedFolderId,
    /// The file.
    pub sighting: Sighting,
    /// The meeting it became, if it was imported.
    pub meeting_id: Option<MeetingId>,
    /// Why it could not be imported, if it failed.
    pub failure: Option<ImportFailure>,
    /// When it was handled.
    pub handled_at: DateTime<Utc>,
}

/// A file that could not be imported, as Settings → Watched folders lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WatchFailure {
    /// The ledger row's ID (retry and dismiss take it).
    pub id: WatchedFileId,
    /// The folder that saw it.
    pub folder_id: WatchedFolderId,
    /// The file's absolute path.
    pub path: String,
    /// Why it failed.
    pub failure: ImportFailure,
    /// When it failed.
    pub failed_at: DateTime<Utc>,
}
