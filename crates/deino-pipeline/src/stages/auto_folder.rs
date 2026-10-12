//! `auto_folder` (order 75, ORG-05): files a processed meeting into every
//! folder whose auto-add rules match it, through
//! `deino_core::folders::AutoFiler`.
//!
//! It runs right after `auto_tag` (70), so tag rules see the AI's tags.
//! Filing is idempotent, so a re-run never duplicates anything.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::folders::{AutoFiled, AutoFiler};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "auto_folder";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 75;
/// Shown when no folder has rules.
const NO_RULES: &str = "No folder has auto-add rules.";
/// Shown when rules exist but none matches.
const NO_MATCH: &str = "No folder's auto-add rules match this meeting.";
/// Shown when the meeting went to the trash before the stage ran.
const GONE: &str = "The meeting was deleted.";

/// Files the meeting by the folders' rules.
pub struct AutoFolderStage {
    filer: Arc<dyn AutoFiler>,
}

impl AutoFolderStage {
    /// The stage over the auto-filer.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            filer: Arc::clone(&ports.auto_filer),
        }
    }
}

crate::registry::register_stage!(AutoFolderStage);

#[async_trait]
impl PipelineStage for AutoFolderStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        if ctx.cancel.is_cancelled() {
            return Err(StageError::Cancelled);
        }
        // Storage is the only failure here, and a busy database passes.
        let filed = self
            .filer
            .auto_file(&meeting)
            .await
            .map_err(|error| StageError::retryable_from(&error))?;
        Ok(match filed {
            AutoFiled::Filed(_) => StageOutcome::Done,
            AutoFiled::NoRules => StageOutcome::Skipped(NO_RULES),
            AutoFiled::NoMatch => StageOutcome::Skipped(NO_MATCH),
            AutoFiled::MeetingGone => StageOutcome::Skipped(GONE),
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedAutoFiling;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::folders::{AutoFiled, AutoFiler, FolderError};
    use deino_domain::MeetingId;

    /// An [`AutoFiler`] for tests: records each meeting and files nothing.
    #[derive(Debug, Default)]
    pub struct RecordedAutoFiling {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedAutoFiling {
        /// Every meeting asked about, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl AutoFiler for RecordedAutoFiling {
        async fn auto_file(&self, meeting: &MeetingId) -> Result<AutoFiled, FolderError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(AutoFiled::NoRules)
        }
    }
}
