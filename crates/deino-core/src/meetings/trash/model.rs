//! The trash as the Trash view shows it, and the files a meeting leaves on
//! disk.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

/// How long a meeting stays in the trash before it is deleted for good
/// (Granola §3.6: "auto-purged after 30 days").
pub const TRASH_RETENTION: TimeDelta = TimeDelta::days(30);

/// A meeting in the trash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashedMeeting {
    /// The meeting's ID.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it was saved.
    pub created_at: DateTime<Utc>,
    /// When it was moved to the trash.
    pub deleted_at: DateTime<Utc>,
    /// When the purge deletes it for good.
    pub purges_at: DateTime<Utc>,
}

impl TrashedMeeting {
    /// The row for a meeting trashed at `deleted_at`.
    pub fn new(
        id: MeetingId,
        title: String,
        created_at: DateTime<Utc>,
        deleted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title,
            created_at,
            deleted_at,
            purges_at: deleted_at + TRASH_RETENTION,
        }
    }
}

/// What the trash did to the meetings it was asked about.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashOutcome {
    /// The meetings that changed; the others were missing or already there.
    pub changed: Vec<MeetingId>,
}

/// The recording files of one meeting, or of one part of a merged meeting
/// (`owner` is then the absorbed meeting, whose folders hold them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFiles {
    /// The meeting the files belong to.
    pub meeting: MeetingId,
    /// The meeting whose folders (`recordings/<id>`, `meetings/<id>`) to delete.
    pub owner: MeetingId,
    /// Absolute paths from `meetings.audio_*` (or the part's copy of them).
    pub paths: Vec<String>,
}
