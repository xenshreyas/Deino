//! What an edit request carries, and the proposal the user accepts or
//! rejects: the edited document and the changed blocks as a word diff.

use deino_doc::Doc;
use deino_domain::MeetingId;
use serde::{Deserialize, Serialize};

/// An edit of the newest enhanced notes, asked for in words.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EditRequest {
    /// The meeting whose notes change.
    pub meeting_id: MeetingId,
    /// The version the user is looking at; it must be the newest.
    pub version: u32,
    /// What the user asked, in their words.
    pub instruction: String,
    /// The selected text blocks, by position among the document's
    /// paragraphs, headings and code blocks in reading order (from 0);
    /// empty for the whole notes.
    #[serde(default)]
    pub selection: Vec<u32>,
}

/// How a proposal was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum EditMethod {
    /// A plain rename, done without the AI.
    Rename,
    /// The `notes.edit` route wrote it.
    Ai,
}

/// A proposed edit: the whole new document and what changed in it.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EditProposal {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The version the edit applies to; accepting fails once it is not
    /// the newest.
    pub base_version: u32,
    /// How the proposal was made.
    pub method: EditMethod,
    /// For a rename, how many places changed.
    pub occurrences: Option<u32>,
    /// The notes with the edit applied.
    pub doc: Doc,
    /// The changed blocks, in reading order.
    pub changes: Vec<DiffBlock>,
}

/// What became of one block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BlockChange {
    /// Its text changed.
    Changed,
    /// It is new.
    Added,
    /// It is gone.
    Removed,
}

/// How a text block reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TextKind {
    /// A heading.
    Heading,
    /// A paragraph.
    Paragraph,
    /// A bullet.
    Bullet,
    /// A numbered item.
    Numbered,
    /// A checklist item.
    Todo,
    /// A paragraph inside a quote.
    Quote,
    /// A code block.
    Code,
}

/// One changed block of the preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffBlock {
    /// Changed, added or removed.
    pub change: BlockChange,
    /// How the block reads.
    pub kind: TextKind,
    /// The heading it sits under, for context.
    pub section: Option<String>,
    /// Its text, word by word: kept, added and removed runs.
    pub segments: Vec<DiffSegment>,
}

/// A run of a block's text in the preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffSegment {
    /// Kept, added or removed.
    pub change: SegmentChange,
    /// The run's text.
    pub text: String,
}

/// What happened to a run of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SegmentChange {
    /// In both versions.
    Same,
    /// Only in the edited version.
    Added,
    /// Only in the current version.
    Removed,
}

/// An accepted edit, stored as a new version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedEdit {
    /// The new newest version.
    pub version: u32,
    /// The version it was made from; restoring it undoes the edit.
    pub previous_version: u32,
}
