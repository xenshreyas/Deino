//! Transcript segments and speakers: the persistence of a meeting's transcript.
//!
//! [`SegmentWriter`] stores what the transcription worker produces, in
//! batches of one transaction each; [`SegmentReader`] serves the transcript
//! in time order; [`SpeakerRepo`] keeps each meeting's voices and who they
//! belong to. [`SqliteSegmentRepo`] implements all three.
//!
//! `transcript_segments` replaced the legacy `transcripts` table in FND-07.
//! Legacy code still reaches it through the `transcripts` compatibility view
//! (ADR 0012) until FND-05b, CAP-08, CAP-11 and TRN-01 port it here.
//!
//! There is no service yet: the recorder (FND-05b) and the transcript view
//! (TRN-01) add the rules that use these ports.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod speaker_repo;
#[cfg(test)]
mod speaker_repo_tests;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeSegmentRepo;
pub use model::{NewSegment, Page, Segment, Speaker, WordTiming};
pub use repo::{SegmentReader, SegmentWriter, SqliteSegmentRepo};
pub use speaker_repo::SpeakerRepo;
