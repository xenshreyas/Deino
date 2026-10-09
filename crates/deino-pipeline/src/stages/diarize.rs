//! `diarize` (order 30): who said what, on the Mac (CAP-03, ADR 0020).
//!
//! The rules are `deino_core::diarization::DiarizeService`: plan the
//! channels, diarize the planned track through the `TrackDiarizer` port
//! (`deino-diarize`'s ONNX engine in the app), store "Speaker N" rows with
//! their voices and attribute every segment. The service replaces its
//! previous result, so a retry or a re-run is safe.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::diarization::{DiarizationError, DiarizeDeps, DiarizeService, TrackError};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, RetryPolicy, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "diarize";
/// Where it runs: after transcription, before `identify_speakers` (§4.2).
const ORDER: u16 = 30;
/// Shown when the meeting has no recorded track to diarize.
const NO_RECORDING: &str = "The meeting's recording is not available to tell speakers apart.";
/// Shown when the speaker models could not be downloaded on any attempt:
/// notes must not wait for speakers, so later stages still run.
const NO_MODELS: &str =
    "The speaker models could not be downloaded; re-run this step to tell speakers apart.";
/// Shown when there was no transcript to attribute.
const NO_TRANSCRIPT: &str = "The meeting has no transcript to attribute to speakers.";

/// Diarizes a meeting's recording and attributes its transcript.
pub struct DiarizeStage {
    service: Arc<DiarizeService>,
}

impl DiarizeStage {
    /// The stage over the shared ports and the diarization ones.
    pub fn new(ports: &StagePorts) -> Self {
        let service = DiarizeService::new(DiarizeDeps {
            meetings: Arc::clone(&ports.meetings),
            attendees: Arc::clone(&ports.attendees),
            segments: Arc::clone(&ports.segments),
            writer: Arc::clone(&ports.segment_writer),
            speakers: Arc::clone(&ports.speakers),
            diarizer: Arc::clone(&ports.diarizer),
            clock: Arc::clone(&ports.clock),
        });
        Self {
            service: Arc::new(service),
        }
    }
}

#[async_trait]
impl PipelineStage for DiarizeStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let outcome = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            outcome = self.service.run(&meeting) => outcome,
        };
        match outcome {
            Ok(outcome) if outcome.segments == 0 => Ok(StageOutcome::Skipped(NO_TRANSCRIPT)),
            Ok(_) => Ok(StageOutcome::Done),
            Err(error) => outcome_of(&error, ctx.attempt),
        }
    }
}

/// How a diarization failure on attempt `attempt` ends the stage: a missing
/// recording is nothing to do; missing models (a download that failed) are
/// retried, then skipped on the last attempt so notes are never blocked;
/// storage errors pass on their own; an engine failure repeats until
/// something changes.
fn outcome_of(error: &DiarizationError, attempt: u32) -> Result<StageOutcome, StageError> {
    match error {
        DiarizationError::AudioMissing(_) => Ok(StageOutcome::Skipped(NO_RECORDING)),
        DiarizationError::Track(TrackError::Cancelled) => Err(StageError::Cancelled),
        DiarizationError::Track(TrackError::ModelsUnavailable(_))
            if !RetryPolicy::DEFAULT.allows_retry_after(attempt) =>
        {
            Ok(StageOutcome::Skipped(NO_MODELS))
        }
        DiarizationError::Track(TrackError::ModelsUnavailable(_))
        | DiarizationError::Storage(_) => Err(StageError::retryable_from(error)),
        _ => Err(StageError::permanent(error.code(), error.to_string())),
    }
}

#[cfg(test)]
#[path = "diarize_tests.rs"]
mod tests;
