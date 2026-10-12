//! `vocab_fixups` (order 40): the transcript corrected with the user's
//! vocabulary and the known names (TRN-05).
//!
//! It runs after the final transcript exists (`final_transcription`, 20) and
//! diarization has split it by speaker (30, 35), so every correction keeps
//! the original of the segment it changed; and before `enhance_notes` (50),
//! so the notes are written from the corrected transcript. The rules are
//! `deino_core::vocabulary::FixupService`; running it again changes nothing.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::vocabulary::{FixupOutcome, FixupService, VocabError};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "vocab_fixups";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 40;
/// Shown when there is nothing to correct with.
const EMPTY_VOCABULARY: &str = "There is no vocabulary to correct the transcript with yet.";

/// Corrects a meeting's transcript.
#[async_trait]
pub trait VocabFixer: Send + Sync {
    /// Corrects `meeting`'s transcript and says what changed.
    async fn fix(&self, meeting: &MeetingId) -> Result<FixupOutcome, VocabError>;
}

#[async_trait]
impl VocabFixer for FixupService {
    async fn fix(&self, meeting: &MeetingId) -> Result<FixupOutcome, VocabError> {
        self.run(meeting).await
    }
}

/// Corrects the transcript with the vocabulary.
pub struct VocabFixupsStage {
    fixer: Arc<dyn VocabFixer>,
}

impl VocabFixupsStage {
    /// The stage over the vocabulary's fixer.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            fixer: Arc::clone(&ports.vocab_fixer),
        }
    }
}

crate::registry::register_stage!(VocabFixupsStage);

#[async_trait]
impl PipelineStage for VocabFixupsStage {
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
            outcome = self.fixer.fix(&meeting) => outcome,
        };
        match outcome {
            Ok(outcome) if outcome.empty_vocabulary => Ok(StageOutcome::Skipped(EMPTY_VOCABULARY)),
            Ok(outcome) => {
                log::info!(target: "pipeline", "vocabulary corrected {} of {} segments of {meeting}", outcome.corrected, outcome.segments);
                Ok(StageOutcome::Done)
            }
            Err(error) => Err(StageError::retryable_from(&error)),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedFixer;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use deino_core::vocabulary::{FixupOutcome, VocabError};
    use deino_domain::MeetingId;

    use super::VocabFixer;

    /// A [`VocabFixer`] for tests: records each meeting and corrects nothing.
    #[derive(Debug, Default)]
    pub struct RecordedFixer {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedFixer {
        /// Every meeting fixed so far, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl VocabFixer for RecordedFixer {
        async fn fix(&self, meeting: &MeetingId) -> Result<FixupOutcome, VocabError> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(meeting.clone());
            Ok(FixupOutcome::default())
        }
    }
}

#[cfg(test)]
#[path = "vocab_fixups_tests.rs"]
mod tests;
