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

use std::fmt;

use deino_stt::{Millis, SessionSink, StatusHandle};

use crate::lane::Lane;
use crate::vad::VoiceChange;

/// Why a lane's engine could not open. When the cause is a [`ModelMissing`],
/// the message is that one, so the user learns which model to download.
#[derive(Debug, thiserror::Error)]
pub struct EngineError {
    /// The lane.
    pub lane: Lane,
    /// What went wrong (a missing model, a worker that did not start, ...).
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

impl EngineError {
    /// The lane's engine could not open because its model is not downloaded.
    pub fn model_missing(lane: Lane, missing: ModelMissing) -> Self {
        Self {
            lane,
            source: Box::new(missing),
        }
    }

    /// The missing model, when that is why the engine could not open.
    pub fn missing_model(&self) -> Option<&ModelMissing> {
        self.source.downcast_ref::<ModelMissing>()
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.missing_model() {
            Some(missing) => missing.fmt(formatter),
            None => write!(
                formatter,
                "the {:?} transcription engine could not start",
                self.lane
            ),
        }
    }
}

/// A lane's engine has no model file on this Mac. The message names the
/// model and what to do; it is shown to the user as is.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "The transcription model ({model}) isn't downloaded. Download it in Settings → Transcription{}.",
    alternative.as_ref().map(|name| format!(" or switch to {name}")).unwrap_or_default()
)]
pub struct ModelMissing {
    /// The model's display name, such as "Parakeet TDT v3".
    pub model: String,
    /// An installed engine to switch to instead, such as "Parakeet Unified".
    pub alternative: Option<String>,
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

    /// A handle on the engine's stream status, for recording health
    /// (`None` for engines that do not stream).
    fn status(&self) -> Option<StatusHandle> {
        None
    }

    /// Transcribes everything fed so far and delivers the last segment.
    /// Blocks until done.
    fn finish(self: Box<Self>);
}

/// What a lane's transcriber is opened for.
pub struct LaneRequest {
    /// The lane.
    pub lane: Lane,
    /// Where its text goes.
    pub sink: Box<dyn SessionSink>,
    /// Where its audio starts on the recording's timeline: zero, or the
    /// recorded length when a finished meeting resumes (CAP-07).
    pub start: Millis,
}

/// Opens each lane's transcriber for a recording.
pub trait LaneEngines: Send + Sync {
    /// Opens the requested lane's transcriber. May load a model, so it
    /// blocks (about a second warm); call it off async threads.
    fn open(&self, request: LaneRequest) -> Result<Box<dyn LaneTranscriber>, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_model_names_the_model_and_the_way_out() {
        let error = EngineError::model_missing(
            Lane::Me,
            ModelMissing {
                model: "Parakeet TDT v3".into(),
                alternative: Some("Parakeet Unified".into()),
            },
        );

        assert_eq!(
            error.to_string(),
            "The transcription model (Parakeet TDT v3) isn't downloaded. Download it in \
             Settings → Transcription or switch to Parakeet Unified."
        );
        assert!(error.missing_model().is_some());
    }

    #[test]
    fn without_an_alternative_the_message_only_offers_the_download() {
        let missing = ModelMissing {
            model: "Parakeet Unified".into(),
            alternative: None,
        };

        assert_eq!(
            missing.to_string(),
            "The transcription model (Parakeet Unified) isn't downloaded. Download it in \
             Settings → Transcription."
        );
    }

    #[test]
    fn any_other_failure_keeps_the_lane_message() {
        let error = EngineError {
            lane: Lane::Them,
            source: Box::new(std::io::Error::other("worker died")),
        };

        assert_eq!(
            error.to_string(),
            "the Them transcription engine could not start"
        );
        assert!(error.missing_model().is_none());
    }
}
