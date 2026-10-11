//! A meeting's transcript as the transcript view reads it (TRN-01).
//!
//! [`TranscriptService::transcript`] answers the meeting page and the record
//! screen with one [`Transcript`]: the saved segments in spoken order and
//! the meeting's speakers, each with the name the view shows
//! ([`TranscriptSpeaker`]). It reads through the existing ports
//! (`segments::SegmentReader`, `segments::SpeakerRepo`,
//! `people::PersonRepo`) and owns one rule: a speaker shows the name of the
//! person it was identified as (following a merge), else its label.
//!
//! Live lines that are still being spoken never come from here: the
//! recorder sends them as `LiveSegment` events and the UI overlays them on
//! this read (ADR 0026). TRN-02 adds the segment edits to this module.

mod model;
mod service;
#[cfg(test)]
mod service_tests;

pub use model::{Transcript, TranscriptSpeaker};
pub use service::{TranscriptError, TranscriptPorts, TranscriptService};
