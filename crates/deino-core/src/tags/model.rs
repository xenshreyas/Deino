//! Tags and how they are applied to meetings.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, PaletteColor, TagId, TagSource};
use serde::Serialize;

/// A label the user (or the auto-tagger) puts on meetings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// The tag's ID.
    pub id: TagId,
    /// Its name; unique, ignoring case.
    pub name: String,
    /// Its colour in the UI.
    pub color: PaletteColor,
    /// When the tag applies, for the auto-tagger (ORG-06).
    pub description: String,
    /// True when the auto-tagger may apply it.
    pub auto_apply: bool,
    /// When the tag was created.
    pub created_at: DateTime<Utc>,
    /// When the tag last changed.
    pub updated_at: DateTime<Utc>,
}

/// A tag to put on a meeting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagApplication {
    /// The meeting to tag.
    pub meeting_id: MeetingId,
    /// The tag to apply.
    pub tag_id: TagId,
    /// Who is applying it.
    pub applied_by: TagSource,
    /// When.
    pub applied_at: DateTime<Utc>,
}

/// Which tag sits on which meeting: one row of `meeting_tags`, for the
/// Meetings page's tag dots and filters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTagLink {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The tag on it.
    pub tag_id: TagId,
    /// Who applied it.
    pub applied_by: TagSource,
}

/// A tag on a meeting, with who put it there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTag {
    /// The tag.
    pub tag: Tag,
    /// Who applied it; the auto-tagger skips meetings the user tagged.
    pub applied_by: TagSource,
    /// When it was applied.
    pub applied_at: DateTime<Utc>,
}
