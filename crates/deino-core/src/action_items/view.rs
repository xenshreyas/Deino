//! [`ActionItemView`]: an action item with what its list row shows beside
//! it (the meeting, the assignee, the evidence moment), read in one query.

use chrono::{DateTime, Utc};
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, MeetingId, Millis,
    PersonId, SegmentId,
};
use serde::Serialize;

/// An action item as the Action Items page and the Notes tab show it.
///
/// Flat rather than nested so one `query_as!` reads it; the item's own
/// fields mean what they mean on [`ActionItem`](super::ActionItem).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemView {
    /// The item's ID.
    pub id: ActionItemId,
    /// The meeting it came from; `None` for standalone items.
    pub meeting_id: Option<MeetingId>,
    /// What to do.
    pub title: String,
    /// Context; may be empty.
    pub description: String,
    /// Who should do it.
    pub assignee_person_id: Option<PersonId>,
    /// When it is due: noon, local time, on the due day (ADR 0028).
    pub due_at: Option<DateTime<Utc>>,
    /// Open or done.
    pub status: ActionItemStatus,
    /// How urgent it is, when known.
    pub priority: Option<ActionItemPriority>,
    /// The transcript segment it was extracted from.
    pub source_segment_id: Option<SegmentId>,
    /// When it was done.
    pub completed_at: Option<DateTime<Utc>>,
    /// Who created it.
    pub created_by: ActionItemOrigin,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// The meeting's title.
    pub meeting_title: Option<String>,
    /// When the meeting took place (its start, else when it was saved).
    pub meeting_held_at: Option<DateTime<Utc>>,
    /// The assignee's name.
    pub assignee_name: Option<String>,
    /// True when the assignee is the user.
    pub assignee_is_me: bool,
    /// Where the evidence starts in the recording, for "open at this moment".
    pub source_start: Option<Millis>,
}
