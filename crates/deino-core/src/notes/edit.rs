//! Editing enhanced notes by asking (NOT-06): "make this more detailed",
//! "Niall is spelled Nyal", "remove the pricing section".
//!
//! A request becomes an [`EditProposal`]: the newest notes with the edit
//! applied, and the changed blocks as a word diff for the preview. A plain
//! rename is done without the AI ([`Rename`]); anything else asks the
//! `notes.edit` route, which answers with block changes ([`EditAnswer`])
//! that are checked and applied in place, so origin marks, formatting,
//! mentions and citations of everything the model did not touch stay as
//! they were. With a selection, only the selected blocks change. Accepting
//! stores a new version (ADR 0030's versions), so undo is restoring the
//! previous one. See `docs/adr/2026-10-12-edit-notes-by-asking.md`.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`EditRequest`], [`EditProposal`], the diff types, [`AcceptedEdit`] |
//! | `blocks.rs` | the document as numbered text blocks |
//! | `scope.rs` | [`Scope`]: the selection; `b3` block IDs |
//! | `rename.rs` | [`Rename`]: the deterministic rename shortcut |
//! | `wire.rs` | [`EditAnswer`], the shape the model answers in |
//! | `prompt.rs` | the `notes.edit` prompt |
//! | `plan.rs` | the prompt's blocks, and the answer checked into a plan |
//! | `rewrite.rs` | a block's new text back into marked inline content |
//! | `apply.rs` | the plan applied to the document |
//! | `words.rs`, `diff.rs` | word alignment and the preview's diff |
//! | `service.rs` | [`NoteEditService`]: propose and accept |

mod apply;
mod blocks;
mod diff;
mod error;
mod model;
mod plan;
mod prompt;
mod rename;
mod rewrite;
mod scope;
mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod test_support;
mod wire;
mod words;

pub use error::EditError;
pub use model::{
    AcceptedEdit, BlockChange, DiffBlock, DiffSegment, EditMethod, EditProposal, EditRequest,
    SegmentChange, TextKind,
};
pub use prompt::MAX_TRANSCRIPT_CHARS;
pub use rename::{MAX_NAME_WORDS, Rename};
pub use scope::Scope;
pub use service::{EditTranscript, NoteEditPorts, NoteEditService};
pub use wire::{EditAction, EditAnswer, EditChange, NewKind};
