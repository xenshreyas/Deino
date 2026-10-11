//! A meeting's transcript as the transcript view reads it (TRN-01).
//!
//! [`TranscriptService::transcript`] answers the meeting page and the record
//! screen with one [`Transcript`]: the saved segments in spoken order, each
//! with its `speaker_id`, read through `segments::SegmentReader`. Who a
//! speaker is (name, colour, suggestions) is the participants' business
//! (`speakers`, CAP-04); the view joins the two by speaker ID.
//!
//! Live lines that are still being spoken never come from here: the
//! recorder sends them as `LiveSegment` events and the UI overlays them on
//! this read (ADR 0026, ADR 0030). TRN-02 adds the segment edits here.

mod model;
mod service;
#[cfg(test)]
mod service_tests;

pub use model::Transcript;
pub use service::{TranscriptError, TranscriptService};
