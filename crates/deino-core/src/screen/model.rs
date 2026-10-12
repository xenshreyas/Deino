//! The snapshot rows.

use deino_domain::{MeetingId, Millis};
use serde::Serialize;

/// A snapshot to store: its PNG is already in the meeting's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSnapshot {
    /// The meeting.
    pub meeting: MeetingId,
    /// Where on the recording's timeline it was taken.
    pub at: Millis,
    /// The PNG's name inside the meeting's `snapshots/` folder.
    pub file_name: String,
    /// Its size in pixels.
    pub width: u32,
    /// Its size in pixels.
    pub height: u32,
    /// Its 64-bit perceptual hash, as SQLite's signed integer.
    pub phash: i64,
}

/// A stored snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// The row's id.
    pub id: i64,
    /// Where on the recording's timeline it was taken.
    pub at: Millis,
    /// The PNG's name inside the meeting's `snapshots/` folder.
    pub file_name: String,
    /// Its size in pixels.
    pub width: u32,
    /// Its size in pixels.
    pub height: u32,
    /// The text read in it (`screen_context`); `None` until it was read.
    pub text: Option<String>,
}

/// What the shared screen showed at one moment, for the notes (NOT-02):
/// a read snapshot's text and its optional description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenText {
    /// Where on the recording's timeline the snapshot was taken.
    pub at: Millis,
    /// What Apple Vision read in it; may be empty when it was described.
    pub text: String,
    /// The `vision.describe` answer, when snapshots are described.
    pub description: Option<String>,
}
