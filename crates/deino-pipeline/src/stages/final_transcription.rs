//! `final_transcription` (order 20): the transcript after the meeting
//! (CAP-08), between `finalize_audio` (10) and `diarize` (30).
//!
//! The rules are `deino_core::final_transcription::FinalTranscriptionService`:
//! a meeting recorded with live transcription off is transcribed from its
//! tracks; one with a live transcript is re-transcribed only with Settings →
//! Transcription → "High-accuracy re-pass" on. A re-pass that keeps failing
//! is skipped on its last attempt, so the live transcript still gets notes.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::{
    FinalOutcome, FinalPass, FinalSkip, FinalTranscriptionDeps, FinalTranscriptionError,
    FinalTranscriptionService, TrackError,
};
use deino_core::settings::SettingsService;
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, RetryPolicy, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "final_transcription";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 20;
/// Shown when there is no recording.
const NO_AUDIO: &str = "The meeting has no recording to transcribe.";
/// Shown when the live transcript stays.
const LIVE_KEPT: &str = "The live transcript is kept; the high-accuracy re-pass is off.";
/// Shown when the re-pass would replace the user's edits.
const EDITED: &str = "The transcript was edited, so the re-pass left it as it is.";
/// Shown when nothing was said.
const NO_SPEECH: &str = "Nothing was said in the recording.";
/// Shown when the re-pass gave up.
const REPASS_FAILED: &str = "The re-pass could not run, so the live transcript is kept.";

/// Settings → Transcription → "High-accuracy re-pass after meeting".
#[async_trait]
pub trait RepassSetting: Send + Sync {
    /// True when live transcripts are transcribed again after the meeting.
    async fn enabled(&self) -> bool;
}

#[async_trait]
impl RepassSetting for SettingsService {
    async fn enabled(&self) -> bool {
        self.get().await.transcription.high_accuracy_repass
    }
}

/// Transcribes a finished meeting's tracks.
pub struct FinalTranscriptionStage {
    service: FinalTranscriptionService,
    repass: Arc<dyn RepassSetting>,
}

impl FinalTranscriptionStage {
    /// The stage over the shared ports and the track transcriber.
    pub fn new(ports: &StagePorts) -> Self {
        let service = FinalTranscriptionService::new(FinalTranscriptionDeps {
            meetings: Arc::clone(&ports.meetings),
            segments: Arc::clone(&ports.segments),
            writer: Arc::clone(&ports.segment_writer),
            speakers: Arc::clone(&ports.speakers),
            transcriber: Arc::clone(&ports.transcriber),
            clock: Arc::clone(&ports.clock),
        });
        Self {
            service,
            repass: Arc::clone(&ports.repass),
        }
    }
}

crate::registry::register_stage!(FinalTranscriptionStage);

#[async_trait]
impl PipelineStage for FinalTranscriptionStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let pass = FinalPass {
            repass: self.repass.enabled().await,
            cancel: ctx.cancel.child_token(),
        };
        let outcome = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            outcome = self.service.run(&meeting, &pass) => outcome,
        };
        match outcome {
            Ok(FinalOutcome::Written { .. }) => Ok(StageOutcome::Done),
            Ok(FinalOutcome::Skipped(skip)) => Ok(StageOutcome::Skipped(reason(skip))),
            Err(error) => outcome_of(&error, ctx.attempt),
        }
    }
}

fn reason(skip: FinalSkip) -> &'static str {
    match skip {
        FinalSkip::NoAudio => NO_AUDIO,
        FinalSkip::LiveTranscriptKept => LIVE_KEPT,
        FinalSkip::EditedByUser => EDITED,
        FinalSkip::NoSpeech => NO_SPEECH,
    }
}

/// How a failure on attempt `attempt` ends the stage: a missing model waits
/// for the user; an engine or storage failure is retried; a re-pass that
/// cannot run is skipped (the live transcript stays) once it is out of
/// retries, or at once when the model is missing.
fn outcome_of(error: &FinalTranscriptionError, attempt: u32) -> Result<StageOutcome, StageError> {
    let last = !RetryPolicy::DEFAULT.allows_retry_after(attempt);
    match error {
        FinalTranscriptionError::Track(TrackError::Cancelled)
        | FinalTranscriptionError::Repass(TrackError::Cancelled) => Err(StageError::Cancelled),
        FinalTranscriptionError::Repass(TrackError::Unavailable(_)) => {
            Ok(StageOutcome::Skipped(REPASS_FAILED))
        }
        FinalTranscriptionError::Repass(_) if last => Ok(StageOutcome::Skipped(REPASS_FAILED)),
        FinalTranscriptionError::Track(TrackError::Unavailable(_))
        | FinalTranscriptionError::NotFound(_) => {
            Err(StageError::permanent(error.code(), error.to_string()))
        }
        _ => Err(StageError::retryable_from(error)),
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fixed::FixedRepass;

#[cfg(any(test, feature = "test-support"))]
mod fixed {
    use async_trait::async_trait;

    use super::RepassSetting;

    /// The re-pass setting fixed on or off.
    #[derive(Clone, Copy, Debug)]
    pub struct FixedRepass(pub bool);

    #[async_trait]
    impl RepassSetting for FixedRepass {
        async fn enabled(&self) -> bool {
            self.0
        }
    }
}

#[cfg(test)]
#[path = "final_transcription_tests.rs"]
mod tests;
