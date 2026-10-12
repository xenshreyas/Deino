//! Comments (TRN-06, ADR 0048): the user's private annotations on a span of
//! a transcript line or of the notes, as threads with replies that can be
//! resolved.
//!
//! A thread is anchored by [`CommentAnchor`]: the target, the transcript
//! line's [`deino_domain::SegmentId`] (stable across edits of other lines,
//! search re-indexing and re-saves), UTF-16 offsets into that line's (or the
//! notes') text, and the quoted text. The UI finds the span again from the
//! quote when an edit moved it, and shows the thread as detached when the
//! quote is gone or a re-transcription deleted the line
//! (`segment_id` becomes `None`): never lost. Threads of a meeting in the
//! trash are never listed. Single user: no authors, no notifications.
//!
//! | File | Layer | Owns |
//! |---|---|---|
//! | `model.rs` | domain | [`CommentThread`], [`Comment`], [`CommentAnchor`], [`NewThread`] |
//! | `repo.rs` | storage | [`CommentRepo`], [`SqliteCommentRepo`] |
//! | `service.rs` | rules | [`CommentService`] |

mod error;
mod model;
mod repo;
mod service;
#[cfg(test)]
mod service_tests;

pub use error::CommentError;
pub use model::{Comment, CommentAnchor, CommentTarget, CommentThread, NewThread, ThreadRow};
pub use repo::{CommentRepo, SqliteCommentRepo};
pub use service::CommentService;
