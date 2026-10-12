//! What a clip is: a named range of a meeting's recording, and its file.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use deino_domain::{ClipId, MeetingId, Millis};
use serde::{Deserialize, Serialize};

/// What a clip's file holds.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "camelCase")]
#[sqlx(rename_all = "lowercase")]
pub enum ClipMedia {
    /// The mixed recording, as `.m4a`.
    Audio,
    /// The screen recording, as `.mp4`.
    Video,
}

impl ClipMedia {
    /// The clip file's extension, without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Audio => "m4a",
            Self::Video => "mp4",
        }
    }
}

/// A saved clip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    /// The clip.
    pub id: ClipId,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// The meeting's title, for the ⌘K list.
    pub meeting_title: String,
    /// What the user called it.
    pub name: String,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends (exclusive).
    pub end: Millis,
    /// Audio or video.
    pub media: ClipMedia,
    /// The clip's file, inside the meeting's recording folder.
    pub path: String,
    /// When it was cut.
    pub created_at: DateTime<Utc>,
}

/// A clip to cut, as the UI asks for it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewClip {
    /// The meeting whose recording is cut.
    pub meeting_id: MeetingId,
    /// What to call it.
    pub name: String,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends (exclusive).
    pub end: Millis,
}

/// The recording file a clip is cut from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipSource {
    /// The file, inside the recordings folder.
    pub path: PathBuf,
    /// Audio or video: decides the clip's container.
    pub media: ClipMedia,
}

/// One cut, for the [`super::ClipCutter`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutRequest {
    /// The recording file.
    pub source: PathBuf,
    /// The clip file to write.
    pub destination: PathBuf,
    /// Where the cut starts.
    pub start: Millis,
    /// Where it ends (exclusive).
    pub end: Millis,
    /// Audio (`.m4a`) or video (`.mp4`).
    pub media: ClipMedia,
}

/// A clip's row, as the repository writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipRow {
    /// The clip.
    pub id: ClipId,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// Its name.
    pub name: String,
    /// Where it starts.
    pub start: Millis,
    /// Where it ends (exclusive).
    pub end: Millis,
    /// Audio or video.
    pub media: ClipMedia,
    /// Its file.
    pub path: String,
    /// When it was cut.
    pub at: DateTime<Utc>,
}

/// A clip's new name and file, as the repository writes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipRename {
    /// The new name.
    pub name: String,
    /// The renamed file.
    pub path: String,
    /// When it was renamed.
    pub at: DateTime<Utc>,
}
