//! `final_transcription` (order 20): the transcript after the meeting
//! (CAP-08), between `finalize_audio` (10) and `diarize` (30).
//!
//! The rules are `deino_core::final_transcription::FinalTranscriptionService`:
//! a meeting recorded with live transcription off is transcribed from its
//! tracks; one with a live transcript is re-transcribed only with Settings →
//! Transcription → "High-accuracy re-pass" on. A re-pass that keeps failing
//! is skipped on its last attempt, so the live transcript still gets notes.
//!
//! With ElevenLabs turned on (CAP-14, the [`CloudChoice`] port), the cloud
//! transcribes instead. A busy or unreachable service is retried; once the
//! cloud fails for good, the run falls back to the Mac's rules above and
//! the stage is skipped with "ElevenLabs transcription failed: …", so the
//! transcript made on this Mac stays and the later stages still run.
//!
//! While it runs, the stage reports how far through the tracks it is
//! through `ctx.events` ([`StageFraction`]: a running event with a
//! fraction, once per whole percent). Only numbers, never what was said.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_core::final_transcription::{
    CloudChoice, CloudError, FinalOutcome, FinalPass, FinalSkip, FinalTranscriptionDeps,
    FinalTranscriptionError, FinalTranscriptionService, TrackError, TrackProgress,
};
use deino_core::settings::SettingsService;
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{
    PipelineStage, RetryPolicy, StageCtx, StageError, StageFraction, StageOutcome, StagePorts,
};

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

    /// True once when the user asked to re-transcribe `meeting` (the run
    /// then transcribes whatever the setting says); false otherwise.
    fn take_request(&self, _meeting: &MeetingId) -> bool {
        false
    }
}

#[async_trait]
impl RepassSetting for SettingsService {
    async fn enabled(&self) -> bool {
        self.get().await.transcription.high_accuracy_repass
    }
}

/// The re-pass setting plus "Re-transcribe" requests: a requested meeting
/// is transcribed again once, whatever the setting says.
pub struct RequestedRepass {
    setting: Arc<dyn RepassSetting>,
    requested: Mutex<HashSet<MeetingId>>,
}

impl RequestedRepass {
    /// The requests over `setting`.
    pub fn new(setting: Arc<dyn RepassSetting>) -> Self {
        let requested = Mutex::new(HashSet::new());
        Self { setting, requested }
    }

    /// Asks for `meeting` to be transcribed again on its next run.
    pub fn request(&self, meeting: &MeetingId) {
        let mut requested = self
            .requested
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        requested.insert(meeting.clone());
    }
}

#[async_trait]
impl RepassSetting for RequestedRepass {
    async fn enabled(&self) -> bool {
        self.setting.enabled().await
    }

    fn take_request(&self, meeting: &MeetingId) -> bool {
        let mut requested = self
            .requested
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        requested.remove(meeting)
    }
}

/// Transcribes a finished meeting's tracks.
pub struct FinalTranscriptionStage {
    service: FinalTranscriptionService,
    repass: Arc<dyn RepassSetting>,
    cloud: Arc<dyn CloudChoice>,
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
            originals: Arc::clone(&ports.originals),
            clock: Arc::clone(&ports.clock),
        });
        Self {
            service,
            repass: Arc::clone(&ports.repass),
            cloud: Arc::clone(&ports.cloud),
        }
    }

    /// Runs the service with `pass`, stopping when the stage is cancelled.
    async fn transcribe(
        &self,
        ctx: &StageCtx,
        meeting: &MeetingId,
        pass: &FinalPass,
    ) -> Result<Result<FinalOutcome, FinalTranscriptionError>, StageError> {
        tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => Err(StageError::Cancelled),
            outcome = self.service.run(meeting, pass) => Ok(outcome),
        }
    }

    /// The cloud failed with `error`: retry while it may pass, else run the
    /// Mac's rules and report the cloud's failure.
    async fn cloud_failed(
        &self,
        ctx: &StageCtx,
        meeting: &MeetingId,
        (pass, error): (FinalPass, &CloudError),
    ) -> Result<StageOutcome, StageError> {
        if matches!(error, CloudError::Cancelled) {
            return Err(StageError::Cancelled);
        }
        if error.is_retryable() && RetryPolicy::DEFAULT.allows_retry_after(ctx.attempt) {
            return Err(StageError::retryable(error.code(), error.status()));
        }
        log::warn!(target: "pipeline", "cloud transcription of meeting {meeting} failed; keeping the Mac's transcript: {error}");
        self.cloud.finished(meeting);
        let local = FinalPass {
            cloud: None,
            ..pass
        };
        match self.transcribe(ctx, meeting, &local).await? {
            Ok(_) => Ok(StageOutcome::Skipped(error.status())),
            Err(local) => outcome_of(&local, ctx.attempt),
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
        let requested = self.repass.take_request(&meeting);
        let pass = FinalPass {
            repass: requested || self.repass.enabled().await,
            requested,
            cloud: self.cloud.for_meeting(&meeting).await,
            cancel: ctx.cancel.child_token(),
            progress: progress(ctx, &meeting),
        };
        let outcome = self.transcribe(ctx, &meeting, &pass).await?;
        if let Err(FinalTranscriptionError::Cloud(error)) = &outcome {
            return self.cloud_failed(ctx, &meeting, (pass, error)).await;
        }
        if pass.cloud.is_some() {
            self.cloud.finished(&meeting);
        }
        match outcome {
            Ok(FinalOutcome::Written { .. }) => Ok(StageOutcome::Done),
            Ok(FinalOutcome::Skipped(skip)) => Ok(StageOutcome::Skipped(reason(skip))),
            Err(error) => outcome_of(&error, ctx.attempt),
        }
    }
}

/// The run's progress, reported as this stage's fraction of `meeting`.
fn progress(ctx: &StageCtx, meeting: &MeetingId) -> TrackProgress {
    let stage = StageId::new(ID);
    let fraction = StageFraction::new(Arc::clone(&ctx.events), meeting.clone(), stage);
    TrackProgress::new(move |done| fraction.report(done))
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
