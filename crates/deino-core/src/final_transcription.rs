//! Transcription after the meeting (CAP-08): the `final_transcription`
//! stage's rules (order 20, after `finalize_audio` and before `diarize`).
//!
//! - **Live transcription off:** the recorder saved audio only, so the stage
//!   transcribes the microphone and system tracks once the meeting ends.
//! - **High-accuracy re-pass** (off by default): the batch engine hears each
//!   utterance whole, with context on both sides, which the low-latency live
//!   window cannot; the new transcript replaces the live one, keeping the
//!   speakers where they overlap.
//! - **Cloud transcription** (CAP-14, off by default): with ElevenLabs
//!   turned on and the user's key saved, the tracks go to the
//!   [`CloudTranscriber`] instead, and its transcript replaces the live one;
//!   the replaced one is kept ([`OriginalTranscripts`]) and can be restored
//!   ([`OriginalTranscriptService`]). A failure writes nothing, so the
//!   stage falls back to the Mac.
//!
//! [`FinalTranscriptionService`] holds the rules over the
//! [`TrackTranscriber`] port, which `deino-recording`'s `OfflineTranscriber`
//! implements (decode, VAD utterances of at most 30 s, the `deino-stt` batch
//! engine); [`carried_speaker`] is the speaker carry-over. Each track
//! reports how far it has got through [`TrackProgress`], and the service
//! turns that into the whole run's fraction ([`FinalPass::progress`]).

mod carry;
mod cloud;
mod cloud_run;
mod echo_text;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod originals;
pub mod overlap;
mod port;
mod progress;
mod restore;
mod service;
mod voices;

pub use carry::{ChannelSpeakers, carried_segment, carried_speaker};
pub use cloud::{
    CLOUD_FAILED, CloudChoice, CloudError, CloudPass, CloudRequest, CloudSpan, CloudTranscriber,
    LocalOnly,
};
pub use echo_text::{
    MERGE, MIN_RUN, RISK_MARGIN, SEGMENT_WINDOW, Spoken, WORD_WINDOW, without_echo,
};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{SCRIPTED_PROGRESS, ScriptedCloud, ScriptedTracks};
#[cfg(any(test, feature = "test-support"))]
pub use originals::FakeOriginalTranscripts;
pub use originals::{KeptLine, OriginalTranscripts, SqliteOriginalTranscripts};
pub use port::{EngineError, HeardSpan, TrackError, TrackRequest, TrackTranscriber};
pub use progress::{TrackProgress, fraction_of};
pub use restore::{OriginalTranscriptDeps, OriginalTranscriptService};
pub use service::{
    FinalOutcome, FinalPass, FinalSkip, FinalTranscriptionDeps, FinalTranscriptionError,
    FinalTranscriptionService,
};
pub use voices::CloudVoices;
