//! What a comment is: a thread anchored to a span of a transcript line or
//! of the notes, and the comments in it.

use chrono::{DateTime, Utc};
use deino_domain::{CommentId, CommentThreadId, MeetingId, SegmentId};
use serde::{Deserialize, Serialize};

/// What a thread annotates.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "camelCase")]
#[sqlx(rename_all = "lowercase")]
pub enum CommentTarget {
    /// A transcript line.
    Transcript,
    /// "My notes" (the notepad).
    Notes,
    /// The enhanced notes.
    Enhanced,
}

/// Where a thread is anchored. Offsets count UTF-16 code units, as the
/// webview does, in the line's text (transcript) or the notes' text.
///
/// The UI finds the span again by `quote`: at the offsets if the text there
/// is unchanged, else at the nearest place the quote still appears; when it
/// appears nowhere (an edit removed it) or the line is gone
/// (`segment_id: None` on a transcript thread), the thread is detached but
/// kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommentAnchor {
    /// What it annotates.
    pub target: CommentTarget,
    /// The transcript line, for a transcript thread; `None` once the line
    /// was deleted by a re-transcription.
    pub segment_id: Option<SegmentId>,
    /// Where the span starts.
    pub start: u32,
    /// Where it ends (exclusive).
    pub end: u32,
    /// The span's text when it was commented.
    pub quote: String,
}

/// One comment of a thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    /// The comment.
    pub id: CommentId,
    /// Its thread.
    pub thread_id: CommentThreadId,
    /// What the user wrote.
    pub body: String,
    /// When it was written.
    pub created_at: DateTime<Utc>,
    /// When it was last edited (its creation if never).
    pub updated_at: DateTime<Utc>,
}

/// A thread and its comments, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommentThread {
    /// The thread.
    pub id: CommentThreadId,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// The span it annotates.
    pub anchor: CommentAnchor,
    /// When the user resolved it, if they did.
    pub resolved_at: Option<DateTime<Utc>>,
    /// When it was started.
    pub created_at: DateTime<Utc>,
    /// Its comments, the first one starting the thread.
    pub comments: Vec<Comment>,
}

/// A thread to start, as the UI asks for it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewThread {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The span.
    pub anchor: CommentAnchor,
    /// The first comment.
    pub body: String,
}

/// A thread's row and its first comment, as the repository writes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadRow {
    /// The thread's ID.
    pub id: CommentThreadId,
    /// Its meeting.
    pub meeting_id: MeetingId,
    /// Its span.
    pub anchor: CommentAnchor,
    /// The first comment.
    pub first: Comment,
}
