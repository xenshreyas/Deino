//! Speaker diarization of a finished meeting (CAP-03): the `diarize`
//! pipeline stage's rules, Tauri-free and engine-free.
//!
//! [`DiarizeService::run`] plans which track holds whom ([`plan_channels`]:
//! the microphone is "Me", the system track holds the remote voices, a
//! silent system track with several attendees means an in-person meeting on
//! a shared microphone), runs the engine behind the [`TrackDiarizer`] port
//! (implemented by `deino-diarize`), stores the voices as "Speaker N" rows
//! with their centroids, and attributes every segment by time overlap,
//! splitting segments where the speaker changes ([`attribute`]).
//!
//! `deino-pipeline`'s `DiarizeStage` runs it at order 30 (after
//! `final_transcription`, before CAP-04's `identify_speakers`).

mod assign;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod plan;
mod port;
mod service;

pub use assign::{Attributed, MIN_PIECE, attribute, voice_at};
pub use error::DiarizationError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FAKE_EMBEDDING_MODEL, FakeTrackDiarizer};
pub use model::{
    DiarizationOutcome, SpeakerHint, TrackDiarization, TrackRequest, Voice, VoiceTurn,
};
pub use plan::{ChannelPlan, PlannedTrack, REMOTE_SILENCE_MAX, plan_channels, speech};
pub use port::{EngineError, TrackDiarizer, TrackError};
pub use service::{DiarizeDeps, DiarizeService, ME_LABEL, speaker_labels};
