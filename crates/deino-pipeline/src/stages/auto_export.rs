//! `auto_export` (order 99): the meeting written into the user's notes
//! folder once everything else has run (EXP-02, ADR 0025).
//!
//! The rules are `deino_core::export::AutoExportService`: it skips while
//! auto-export is off, writes by the settings' templates and only ever
//! overwrites its own notes, so running the stage again is safe. A folder
//! that went away (an unplugged drive) is retried; a missing folder setting
//! waits for the user.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::export::{AutoExportOutcome, AutoExporter, ExportError};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "auto_export";
/// Where it runs: last, after every stage that changes the notes (§4.2).
const ORDER: u16 = 99;

/// Exports the meeting into the user's folder.
pub struct AutoExportStage {
    exporter: Arc<dyn AutoExporter>,
}

impl AutoExportStage {
    /// The stage over the auto-export port.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            exporter: Arc::clone(&ports.auto_export),
        }
    }
}

#[async_trait]
impl PipelineStage for AutoExportStage {
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
            outcome = self.exporter.export(&meeting) => outcome,
        };
        match outcome {
            Ok(AutoExportOutcome::Exported { .. }) => Ok(StageOutcome::Done),
            Ok(AutoExportOutcome::Skipped(skip)) => Ok(StageOutcome::Skipped(skip.reason())),
            Err(error) => Err(failure(&error)),
        }
    }
}

/// A transient failure (the database, a folder that may come back) is
/// retried; anything else waits for the user to re-run the stage.
fn failure(error: &ExportError) -> StageError {
    if error.is_retryable() {
        StageError::retryable_from(error)
    } else {
        StageError::permanent(error.code(), error.to_string())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use scripted::ScriptedAutoExporter;

#[cfg(any(test, feature = "test-support"))]
mod scripted {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::export::{AutoExportOutcome, AutoExportSkip, AutoExporter, ExportError};
    use deino_domain::MeetingId;

    /// An [`AutoExporter`] for tests: records each meeting and answers with
    /// what it was built with (by default, "auto-export is off").
    pub struct ScriptedAutoExporter {
        answer: Box<dyn Fn() -> Result<AutoExportOutcome, ExportError> + Send + Sync>,
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl Default for ScriptedAutoExporter {
        fn default() -> Self {
            Self::answering(|| Ok(AutoExportOutcome::Skipped(AutoExportSkip::Disabled)))
        }
    }

    impl ScriptedAutoExporter {
        /// An exporter that answers every meeting with `answer()`.
        pub fn answering(
            answer: impl Fn() -> Result<AutoExportOutcome, ExportError> + Send + Sync + 'static,
        ) -> Self {
            Self {
                answer: Box::new(answer),
                meetings: Mutex::default(),
            }
        }

        /// Every meeting exported so far, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl AutoExporter for ScriptedAutoExporter {
        async fn export(&self, meeting: &MeetingId) -> Result<AutoExportOutcome, ExportError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            (self.answer)()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use deino_core::export::AutoExportSkip;
    use deino_domain::{ErrorCode, FakeClock};
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    fn ctx() -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    async fn run(exporter: ScriptedAutoExporter) -> Result<StageOutcome, StageError> {
        let ports = StagePorts {
            auto_export: Arc::new(exporter),
            ..StagePorts::fakes()
        };
        AutoExportStage::new(&ports)
            .run(&ctx(), MeetingId::new("meeting-1"))
            .await
    }

    #[test]
    fn the_stage_runs_last() {
        let stage = AutoExportStage::new(&StagePorts::fakes());
        assert_eq!(
            (stage.id(), stage.order()),
            (StageId::new("auto_export"), 99)
        );
    }

    #[tokio::test]
    async fn turned_off_it_skips_with_the_reason() {
        let outcome = run(ScriptedAutoExporter::default()).await;
        assert_eq!(outcome, Ok(StageOutcome::Skipped("Auto-export is off.")));
    }

    #[tokio::test]
    async fn a_written_note_is_done_and_an_unchanged_one_is_skipped() {
        let written = run(ScriptedAutoExporter::answering(|| {
            Ok(AutoExportOutcome::Exported { files: 3 })
        }))
        .await;
        let kept = run(ScriptedAutoExporter::answering(|| {
            Ok(AutoExportOutcome::Skipped(AutoExportSkip::AlreadyExported))
        }))
        .await;

        assert_eq!(written, Ok(StageOutcome::Done));
        assert_eq!(
            kept,
            Ok(StageOutcome::Skipped(
                AutoExportSkip::AlreadyExported.reason()
            ))
        );
    }

    #[tokio::test]
    async fn a_missing_folder_is_retried_and_an_unset_one_waits_for_the_user() {
        let unplugged = run(ScriptedAutoExporter::answering(|| {
            Err(ExportError::FolderUnavailable(
                io::ErrorKind::NotFound.into(),
            ))
        }))
        .await
        .unwrap_err();
        let unset = run(ScriptedAutoExporter::answering(|| {
            Err(ExportError::NoFolder)
        }))
        .await
        .unwrap_err();

        assert!(unplugged.is_retryable());
        assert_eq!(
            unset,
            StageError::permanent(
                ErrorCode::Invalid,
                "choose an export folder in Settings → Export"
            )
        );
    }

    #[tokio::test]
    async fn a_cancelled_run_stops_before_exporting() {
        let exporter = Arc::new(ScriptedAutoExporter::default());
        let ports = StagePorts {
            auto_export: exporter.clone(),
            ..StagePorts::fakes()
        };
        let ctx = ctx();
        ctx.cancel.cancel();

        let outcome = AutoExportStage::new(&ports)
            .run(&ctx, MeetingId::new("meeting-1"))
            .await;

        assert_eq!(outcome, Err(StageError::Cancelled));
        assert!(exporter.meetings().is_empty());
    }
}
