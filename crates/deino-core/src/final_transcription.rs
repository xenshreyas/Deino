//! Transcription after the meeting (CAP-08): the `final_transcription`
//! stage's rules (order 20, after `finalize_audio` and before `diarize`).
//!
//! - **Live transcription off:** the recorder saved audio only, so the stage
//!   transcribes the microphone and system tracks once the meeting ends.
//! - **High-accuracy re-pass** (off by default): the batch engine hears each
//!   utterance whole, with context on both sides, which the low-latency live
//!   window cannot; the new transcript replaces the live one, keeping the
//!   speakers where they overlap.
//!
//! [`FinalTranscriptionService`] holds the rules over the
//! [`TrackTranscriber`] port, which `deino-recording`'s `OfflineTranscriber`
//! implements (decode, VAD utterances of at most 30 s, the `deino-stt` batch
//! engine); [`carried_speaker`] is the speaker carry-over.

mod carry;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod port;
mod service;

pub use carry::{ChannelSpeakers, carried_speaker};
#[cfg(any(test, feature = "test-support"))]
pub use fake::ScriptedTracks;
pub use port::{EngineError, HeardSpan, TrackError, TrackRequest, TrackTranscriber};
pub use service::{
    FinalOutcome, FinalPass, FinalSkip, FinalTranscriptionDeps, FinalTranscriptionError,
    FinalTranscriptionService,
};
