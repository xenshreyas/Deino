//! What a merge reads, plans and reports.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, MeetingStatus, Millis};
use serde::Serialize;

use crate::meetings::MeetingAudio;
use crate::notes::StoredNotes;

/// A meeting as the merge sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeCandidate {
    /// The meeting.
    pub id: MeetingId,
    /// Its title, kept on its part.
    pub title: String,
    /// When it took place: its start, else when it was saved.
    pub held_at: DateTime<Utc>,
    /// Its status; only finished meetings merge.
    pub status: MeetingStatus,
    /// True when it is in the trash.
    pub trashed: bool,
    /// Its length on the merged timeline: the recording or the transcript,
    /// whichever runs longer.
    pub length: Millis,
    /// Parts already merged into it, which move along with it.
    pub parts: u32,
    /// Its recording files.
    pub audio: MeetingAudio,
}

/// One absorbed meeting's place on the merged timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergePart {
    /// The absorbed meeting.
    pub source: MeetingId,
    /// Its part number on the merged meeting, from 1; its own earlier parts
    /// follow it.
    pub position: u32,
    /// Its title.
    pub title: String,
    /// When it took place.
    pub held_at: DateTime<Utc>,
    /// Where it starts on the merged timeline.
    pub offset: Millis,
    /// Its length.
    pub length: Millis,
    /// Its recording files.
    pub audio: MeetingAudio,
}

/// A merge, ready to apply in one transaction.
#[derive(Clone, Debug, PartialEq)]
pub struct MergePlan {
    /// The earliest meeting, which absorbs the others and keeps its ID.
    pub target: MeetingId,
    /// The absorbed meetings, in time order.
    pub parts: Vec<MergePart>,
    /// The merged timeline's length.
    pub length: Millis,
    /// The combined notes; `None` keeps the target's as they are.
    pub notes: Option<StoredNotes>,
    /// When the merge happened.
    pub merged_at: DateTime<Utc>,
}

/// What a merge did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    /// The merged meeting (the earliest of those chosen). Its AI notes now
    /// lag the transcript, so the meeting page offers to regenerate them
    /// with NOT-02's reasons.
    pub meeting_id: MeetingId,
    /// The meetings it absorbed, now deleted.
    pub absorbed: Vec<MeetingId>,
}
