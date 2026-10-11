//! The enhanced-notes feature's types: stored versions, what the notes view
//! receives, and transcript evidence.

use chrono::{DateTime, Utc};
use deino_doc::Doc;
use deino_domain::{MeetingId, Millis, SegmentId, TemplateId};
use serde::Serialize;

use super::EnhanceBasis;

/// The `model` of a version imported from a legacy summary.
pub const LEGACY_MODEL: &str = "legacy";

/// One version of a meeting's enhanced notes, as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct EnhancedNotes {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// 1 for the first generation, one more per regeneration.
    pub version: u32,
    /// The notes, every run carrying its origin, AI blocks their citations.
    pub doc: Doc,
    /// [`deino_doc::Flavor::Readable`] Markdown of `doc`.
    pub markdown: String,
    /// The template they were written with.
    pub template_id: Option<TemplateId>,
    /// `provider:model` that wrote them, or [`LEGACY_MODEL`].
    pub model: String,
    /// The inputs they were written from; `None` for imported summaries.
    pub basis: Option<EnhanceBasis>,
    /// True once the user edited this version.
    pub is_user_edited: bool,
    /// When the version was written.
    pub created_at: DateTime<Utc>,
    /// When it last changed (an edit).
    pub updated_at: DateTime<Utc>,
}

/// A version to store; the repository numbers it.
#[derive(Clone, Debug, PartialEq)]
pub struct NewVersion {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The notes.
    pub doc: Doc,
    /// Readable Markdown of `doc`.
    pub markdown: String,
    /// The template used.
    pub template_id: Option<TemplateId>,
    /// `provider:model`.
    pub model: String,
    /// The inputs used.
    pub basis: Option<EnhanceBasis>,
    /// True when the content carries edits (a restored edited version).
    pub is_user_edited: bool,
    /// When it is written.
    pub at: DateTime<Utc>,
}

/// One entry of the version history menu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    /// The version's number.
    pub version: u32,
    /// When it was written.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// The template it was written with, if it still exists.
    pub template_id: Option<TemplateId>,
    /// That template's name.
    pub template_name: Option<String>,
    /// `provider:model`, or `legacy`.
    pub model: String,
    /// True once the user edited it.
    pub is_user_edited: bool,
}

/// What the Enhanced view shows: one version and, for the newest, why it
/// may be worth regenerating.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedNotesView {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The version shown.
    pub version: VersionInfo,
    /// True when it is the newest version, the only editable one.
    pub is_latest: bool,
    /// The notes.
    pub doc: Doc,
    /// What changed since the newest version was written; empty for older ones.
    pub reasons: Vec<RegenerateReason>,
}

/// Why the notes may be out of date (Granola's regenerate prompts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RegenerateReason {
    /// The meeting was linked to another calendar event, or unlinked (ORG-03).
    EventChanged,
    /// Someone was added to or removed from the meeting.
    AttendeesChanged,
    /// Transcript lines were edited or deleted.
    TranscriptEdited,
    /// The user edited their own notes.
    NotesEdited,
    /// More of the meeting was transcribed.
    AudioAppended,
}

/// What an edit of the newest version reports back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedEnhancedNotes {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The version edited.
    pub version: u32,
    /// When it was saved.
    pub updated_at: DateTime<Utc>,
}

/// A transcript moment backing a statement, resolved from a reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// The segment.
    pub segment_id: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
}

/// A cited transcript excerpt, for the citation popover (NOT-03).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CitedSegment {
    /// The segment.
    pub segment_id: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends.
    pub end: Millis,
    /// Who said it, by name when known.
    pub speaker: String,
    /// What was said.
    pub text: String,
}
