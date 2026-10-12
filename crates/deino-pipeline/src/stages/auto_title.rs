//! `auto_title` (order 65): names a meeting whose title is a placeholder
//! (NOT-02), from its calendar event or its enhanced notes.
//!
//! The rules are `deino_core::enhance::AutoTitler`: a title the user or the
//! calendar gave is kept. Like `enhance_notes`, the stage honours
//! "Generate notes automatically".

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::{AutoTitler, EnhanceError, TitleRun};
use deino_domain::{MeetingId, StageId};

use super::enhance_notes::{AUTOMATIC_NOTES_OFF, AutomaticNotes, stage_error};
use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "auto_title";
/// Where it runs: after notes and action items (PARITY_PLAN §4.2).
const ORDER: u16 = 65;

/// Names meetings for the pipeline.
#[async_trait]
pub trait MeetingTitler: Send + Sync {
    /// Names `meeting` if its title is a placeholder.
    async fn title(&self, meeting: &MeetingId) -> Result<TitleRun, EnhanceError>;
}

#[async_trait]
impl MeetingTitler for AutoTitler {
    async fn title(&self, meeting: &MeetingId) -> Result<TitleRun, EnhanceError> {
        self.run(meeting).await
    }
}

/// Names the meeting when its title is a placeholder.
pub struct AutoTitleStage {
    titler: Arc<dyn MeetingTitler>,
    automatic: Arc<dyn AutomaticNotes>,
}

impl AutoTitleStage {
    /// The stage over the titler and the setting.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            titler: Arc::clone(&ports.titler),
            automatic: Arc::clone(&ports.automatic),
        }
    }
}

crate::registry::register_stage!(AutoTitleStage);

#[async_trait]
impl PipelineStage for AutoTitleStage {
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
            run = self.titler.title(&meeting) => run,
        };
        match run {
            Ok(TitleRun::Renamed(_)) => Ok(StageOutcome::Done),
            Ok(TitleRun::Skipped(reason)) => Ok(StageOutcome::Skipped(reason)),
            Err(error) => Err(stage_error(&error)),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedTitler;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use deino_core::enhance::{EnhanceError, TitleRun};
    use deino_domain::MeetingId;

    use super::MeetingTitler;

    /// A [`MeetingTitler`] for tests: records each meeting and keeps its title.
    #[derive(Debug, Default)]
    pub struct RecordedTitler {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedTitler {
        /// Every meeting asked about so far, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl MeetingTitler for RecordedTitler {
        async fn title(&self, meeting: &MeetingId) -> Result<TitleRun, EnhanceError> {
            self.meetings
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(meeting.clone());
            Ok(TitleRun::Skipped("named"))
        }
    }
}
