//! A meeting's transcript as the transcript view reads and the user edits it.
//!
//! [`TranscriptService::transcript`] answers the meeting page and the record
//! screen with one [`Transcript`]: the saved segments in spoken order, each
//! with its `speaker_id`, read through `segments::SegmentReader` (TRN-01).
//! Who a speaker is (name, colour, suggestions) is the participants'
//! business (`speakers`, CAP-04); the view joins the two by speaker ID.
//!
//! Live lines that are still being spoken never come from here: the
//! recorder sends them as `LiveSegment` events and the UI overlays them on
//! this read (ADR 0026, ADR 0033).
//!
//! [`TranscriptEditService`] holds the user's changes (TRN-02): edit a line
//! (keeping the engine's `original_text`), delete and restore one, and find
//! and replace across one meeting or all of them with a preview first.
//! Each change re-indexes its meetings through the [`TranscriptIndexer`]
//! port, which `search::ReindexQueue` implements.

mod edit;
mod edit_model;
#[cfg(test)]
mod edit_tests;
mod model;
mod pattern;
#[cfg(test)]
mod reach_tests;
mod replace_plan;
mod service;
#[cfg(test)]
mod service_tests;

pub use edit::{
    MAX_FIND_CHARS, MAX_LINE_CHARS, TranscriptEditDeps, TranscriptEditService, TranscriptIndexer,
};
pub use edit_model::{
    MeetingMatches, ReplaceOutcome, ReplacePreview, ReplaceRequest, ReplaceScope,
};
pub use model::Transcript;
pub use service::{TranscriptError, TranscriptService};
