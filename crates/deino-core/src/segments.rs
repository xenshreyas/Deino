//! Transcript segments and speakers: the persistence of a meeting's transcript.
//!
//! [`SegmentWriter`] stores what the transcription worker produces, in
//! batches of one transaction each; [`SegmentReader`] serves the transcript
//! in time order; [`SpeakerRepo`] keeps each meeting's voices and who they
//! belong to. [`SqliteSegmentRepo`] implements all three.
//!
//! Diarization (CAP-03) writes through two of them:
//! [`SpeakerRepo::replace_speakers`] stores the new speakers with their
//! [`SpeakerVoice`] (centroid, model, speech time), then
//! [`SegmentWriter::reattribute`] points segments at them, splitting a
//! segment where the speaker changes. Each is one transaction; running the
//! stage again replaces both.
//!
//! `transcript_segments` replaced the legacy `transcripts` table in FND-07.
//! Legacy code still reaches it through the `transcripts` compatibility view
//! (ADR 0012) until CAP-08, CAP-11 and NOT-02 port its last callers here.
//!
//! The recorder (`deino-recording`) writes through these ports, and the
//! transcript view reads through them with `crate::transcript` (TRN-01).
//! The transcript edits (TRN-02) add [`SegmentEditor`]: one segment in any
//! state, batched text edits that keep `original_text`, and the segments a
//! find and replace may change.

mod editor;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(any(test, feature = "test-support"))]
mod fake_editor;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod speaker_repo;
#[cfg(test)]
mod speaker_repo_tests;
mod voices;
#[cfg(test)]
mod voices_tests;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeSegmentRepo;
pub use editor::{SegmentEditor, SegmentText, TextEdit};
pub use model::{
    NewSegment, NewSpeaker, Page, Reattribution, Segment, SegmentPiece, Speaker, SpeakerVoice,
    WordTiming,
};
pub use repo::{SegmentReader, SegmentWriter, SqliteSegmentRepo};
pub use speaker_repo::SpeakerRepo;
