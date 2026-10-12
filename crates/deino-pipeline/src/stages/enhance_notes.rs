//! `enhance_notes` (order 50): the meeting's enhanced notes (NOT-02).
//!
//! The rules are `deino_core::enhance::EnhanceService::run_for_pipeline`:
//! it writes a new version unless there is nothing to write from, the user
//! edited the notes, or nothing changed since they were written; each of
//! those is a skip with a reason. The stage honours Settings → AI →
//! "Generate notes automatically" (`ai.auto_summary`); the Notes tab's
//! Enhance button still works when it is off.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::{EnhanceError, EnhanceService, PipelineRun};
use deino_core::settings::SettingsService;
use deino_domain::{ErrorClass, MeetingId, StageId};
use deino_llm::LlmError;

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "enhance_notes";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 50;
/// Shown when automatic notes are off.
pub const AUTOMATIC_NOTES_OFF: &str =
    "Automatic notes are off in Settings → AI. Use Enhance on the Notes tab.";

/// Writes a meeting's enhanced notes for the pipeline.
#[async_trait]
pub trait NotesEnhancer: Send + Sync {
    /// Writes the notes of `meeting`, or says why it did not.
    async fn enhance(&self, meeting: &MeetingId) -> Result<PipelineRun, EnhanceError>;
}

#[async_trait]
impl NotesEnhancer for EnhanceService {
    async fn enhance(&self, meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
        self.run_for_pipeline(meeting).await
    }
}

/// Whether the post-meeting pipeline writes notes on its own.
#[async_trait]
pub trait AutomaticNotes: Send + Sync {
    /// True when notes (and titles) are written after every meeting.
    async fn enabled(&self) -> bool;
}

#[async_trait]
impl AutomaticNotes for SettingsService {
    async fn enabled(&self) -> bool {
        self.get().await.ai.auto_summary
    }
}

/// Writes the meeting's enhanced notes.
pub struct EnhanceNotesStage {
    enhancer: Arc<dyn NotesEnhancer>,
    automatic: Arc<dyn AutomaticNotes>,
}

impl EnhanceNotesStage {
    /// The stage over the enhancer and the setting.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            enhancer: Arc::clone(&ports.enhancer),
            automatic: Arc::clone(&ports.automatic),
        }
    }
}

crate::registry::register_stage!(EnhanceNotesStage);

#[async_trait]
impl PipelineStage for EnhanceNotesStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        if !self.automatic.enabled().await {
            return Ok(StageOutcome::Skipped(AUTOMATIC_NOTES_OFF));
        }
        let run = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            run = self.enhancer.enhance(&meeting) => run,
        };
        match run {
            Ok(PipelineRun::Written(_)) => Ok(StageOutcome::Done),
            Ok(PipelineRun::Skipped(reason)) => Ok(StageOutcome::Skipped(reason)),
            Err(error) => Err(stage_error(&error)),
        }
    }
}

/// An enhance failure as a stage failure: retried when it may pass (a rate
/// limit, the network, a busy database), else permanent, with the error's
/// user-safe message ("Add an Anthropic key or pick a local model…").
pub fn stage_error(error: &EnhanceError) -> StageError {
    match error {
        EnhanceError::Llm(LlmError::Cancelled) => StageError::Cancelled,
        error if error.is_retryable() => StageError::retryable_from(error),
        error => StageError::permanent(error.code(), error.to_string()),
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::{FixedAutomaticNotes, RecordedEnhancer};

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use deino_core::enhance::{EnhanceError, PipelineRun};
    use deino_domain::MeetingId;

    use super::{AutomaticNotes, NotesEnhancer};

    /// A [`NotesEnhancer`] for tests: records each meeting and writes version 1.
    #[derive(Debug, Default)]
    pub struct RecordedEnhancer {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedEnhancer {
        /// Every meeting enhanced so far, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl NotesEnhancer for RecordedEnhancer {
        async fn enhance(&self, meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(meeting.clone());
            Ok(PipelineRun::Written(1))
        }
    }

    /// [`AutomaticNotes`] fixed on or off.
    #[derive(Clone, Copy, Debug)]
    pub struct FixedAutomaticNotes(pub bool);

    #[async_trait]
    impl AutomaticNotes for FixedAutomaticNotes {
        async fn enabled(&self) -> bool {
            self.0
        }
    }
}

#[cfg(test)]
#[path = "enhance_notes_tests.rs"]
mod tests;
