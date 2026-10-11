//! Enhanced notes (NOT-02, ADR 0024): the user's rough notes and the
//! transcript become their meeting notes, Granola-style.
//!
//! The user's lines stay word for word (ink), the AI adds detail beneath
//! them and the rest of the template (grey), and every AI line cites the
//! transcript moments behind it. Each generation is a version; the newest
//! one is editable.
//!
//! | File | Role |
//! |---|---|
//! | `material.rs`, `sources.rs` | [`Material`]: transcript with speakers, notes, calendar, attendees, the series' previous notes, vocabulary, gathered by [`MaterialSources`] |
//! | `transcript.rs` | [`TranscriptLine`] (`[s12 04:31] Maya: …`), parts of a long meeting |
//! | `prompt.rs` | the `notes.enhance` prompts: one pass, a part, the merge |
//! | `wire.rs` | [`EnhancedDoc`], the shape the model answers in |
//! | `writer.rs` | [`NotesWriter`]: one pass or map-reduce over the gateway |
//! | `user_lines.rs` | the user's lines and the matcher that holds the model to them |
//! | `citations.rs` | [`SegmentIndex`]: references back to segments, invalid ones dropped |
//! | `assemble.rs` | the checks between the answer and the document |
//! | `convert.rs` | [`Draft`] → TipTap with `origin` marks and `citation` nodes |
//! | `basis.rs` | [`EnhanceBasis`] and the regenerate prompts ([`RegenerateReason`]) |
//! | `repo.rs` | [`EnhancedNotesReader`] (the stable read API), [`EnhancedNotesWriter`], [`SqliteEnhancedNotesRepo`] |
//! | `service.rs`, `history.rs` | [`EnhanceService`]: write, read, edit, restore, cite |
//! | `title.rs` | [`AutoTitler`]: the `auto_title` stage's rules |
//! | `legacy_import.rs` | [`LegacySummaryImport`]: pre-NOT-02 summaries as version 1 |
//! | `fake.rs` | in-memory ports for tests |

mod assemble;
mod basis;
mod citations;
mod convert;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod history;
mod legacy_import;
mod material;
mod model;
mod prompt;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
#[cfg(test)]
mod service_tests;
mod sources;
#[cfg(test)]
mod test_support;
mod title;
#[cfg(test)]
mod title_tests;
mod transcript;
mod user_lines;
mod wire;
mod writer;

pub use assemble::{
    Body, Checks, Draft, DraftBlock, DraftDetail, DraftKind, DraftSection, LEFTOVER_HEADING,
    assemble,
};
pub use basis::{EnhanceBasis, EventBasis};
pub use citations::{MAX_CITATIONS, SegmentIndex};
pub use convert::to_doc;
pub use error::EnhanceError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeEnhancedNotesRepo;
pub use history::MAX_CITED;
pub use legacy_import::{DROP_LEGACY_SUMMARIES_VERSION, LegacySummaryImport, summary_doc};
pub use material::{
    AttendeeInfo, Material, MeetingInfo, NoVocabulary, PreviousNotes, UserNotes, Vocabulary,
};
pub use model::{
    CitedSegment, EnhancedNotes, EnhancedNotesView, Evidence, LEGACY_MODEL, NewVersion,
    RegenerateReason, SavedEnhancedNotes, VersionInfo,
};
pub use repo::{EnhancedNotesReader, EnhancedNotesWriter, SqliteEnhancedNotesRepo, VersionEdit};
pub use service::{
    EnhancePorts, EnhanceService, KEPT_EDITS, NOTHING_TO_WRITE, Overwrite, PipelineRun,
    TemplateSource, UP_TO_DATE,
};
pub use sources::MaterialSources;
pub use title::{
    ALREADY_NAMED, AutoTitler, NO_TITLE, NOTHING_TO_NAME, TitleRun, clean_title, is_generic_title,
};
pub use transcript::{LONG_MEETING, PART_SPAN, PartLimits, TranscriptLine, clock, parts};
pub use user_lines::{LineKind, LineMatcher, UserLine, user_headings, user_lines};
pub use wire::{BlockKind, EnhancedBlock, EnhancedDetail, EnhancedDoc, EnhancedSection, Origin};
pub use writer::{MAP_CONCURRENCY, NotesWriter, Written, model_label};
