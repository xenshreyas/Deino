//! Action items: commitments from meetings, or standalone to-dos.

use chrono::{DateTime, Utc};
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, MeetingId, PersonId,
    SegmentId,
};
use serde::Serialize;

/// Something someone committed to do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    /// The item's ID.
    pub id: ActionItemId,
    /// The meeting it came from; `None` for standalone items, and after the
    /// meeting is deleted (the item outlives it).
    pub meeting_id: Option<MeetingId>,
    /// What to do, in a few words.
    pub title: String,
    /// One or two sentences of context; may be empty.
    pub description: String,
    /// Who should do it; `None` when unassigned.
    pub assignee_person_id: Option<PersonId>,
    /// When it is due.
    pub due_at: Option<DateTime<Utc>>,
    /// Whether it is still to do.
    pub status: ActionItemStatus,
    /// How urgent it is, when known.
    pub priority: Option<ActionItemPriority>,
    /// The transcript moment it was extracted from, for "jump to source".
    pub source_segment_id: Option<SegmentId>,
    /// When it was done; set exactly when `status` is `Done`.
    pub completed_at: Option<DateTime<Utc>>,
    /// Who created it.
    pub created_by: ActionItemOrigin,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it last changed; later than `created_at` once the user touched
    /// it, so regeneration (AI-05) can leave touched items alone.
    pub updated_at: DateTime<Utc>,
}

impl ActionItem {
    /// True for an item the AI extracted and the user has not touched since:
    /// still open and never edited (`updated_at` equals `created_at`, since
    /// every change through the service moves `updated_at`). Regenerating a
    /// meeting's items replaces only these.
    pub fn is_untouched_ai(&self) -> bool {
        self.created_by == ActionItemOrigin::Ai
            && self.status == ActionItemStatus::Open
            && self.updated_at == self.created_at
    }
}
