//! The transcription side of a lane: the ports the live pipeline feeds.
//!
//! A lane hands its 16 kHz audio to a [`LaneTranscriber`], which delivers
//! text through a `deino_stt::SessionSink`. Two implementations:
//!
//! - [`StreamingLane`]: Parakeet Unified through `deino_stt::LiveSession`,
//!   fed continuous audio (never VAD-gated), so token times are recording
//!   times;
//! - [`UtteranceLane`]: the "Advanced" batch engines (Parakeet TDT,
//!   Whisper), which transcribe each voice span the lane's VAD closes.
//!
//! [`LaneEngines`] opens one per lane; the app implements it over the
//! engine pool (one worker per lane) and the configured engine.

mod streaming;
mod utterance;

pub use streaming::{StreamingLane, StreamingParts};
pub use utterance::{UtteranceLane, UtteranceTranscriber};

use deino_stt::{Millis, SessionSink};

use crate::lane::Lane;
use crate::vad::VoiceChange;

/// Why a lane's engine could not open.
#[derive(Debug, thiserror::Error)]
#[error("the {lane:?} transcription engine could not start")]
pub struct EngineError {
    /// The lane.
    pub lane: Lane,
    /// What went wrong (a missing model, a worker that did not start, ...).
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

/// One lane's transcriber.
pub trait LaneTranscriber: Send {
    /// Feeds the next stretch of the lane's 16 kHz audio. Never blocks for long.
    fn feed(&mut self, pcm: Vec<f32>);

    /// The lane's voice detector changed state (utterance engines cut here).
    fn voice(&mut self, _change: VoiceChange) {}

    /// Audio fed and not yet transcribed.
    fn backlog(&self) -> Millis {
        Millis::ZERO
    }

    /// Transcribes everything fed so far and delivers the last segment.
    /// Blocks until done.
    fn finish(self: Box<Self>);
}

/// Opens each lane's transcriber for a recording.
pub trait LaneEngines: Send + Sync {
    /// Opens `lane`'s transcriber, delivering text into `sink`. May load a
    /// model, so it blocks (about a second warm); call it off async threads.
    fn open(
        &self,
        lane: Lane,
        sink: Box<dyn SessionSink>,
    ) -> Result<Box<dyn LaneTranscriber>, EngineError>;
}
