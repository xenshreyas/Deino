//! `automations` (order 95): the user's automations (AUT-01).
//!
//! It runs after insights (85), whose answers "generate insight" steps
//! reuse, and after people and companies are updated (90), so a step's
//! payload sees the meeting as finished; before the follow-up draft (98)
//! and the auto-export (99). The rules are
//! `deino_core::automations::AutomationEngine` behind the
//! `AutomationRunner` port: every automation that runs on its own and
//! matches, plus the runs the user asked for or a tag armed. Steps that
//! already finished are not run again when the stage retries.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::automations::{AutomationError, AutomationRunner, EngineOutcome};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, RetryPolicy, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "automations";
/// Where it runs (§4.2).
const ORDER: u16 = 95;
/// Shown when no automation applies to the meeting.
const NOTHING_TO_RUN: &str = "No automation applies to this meeting.";

/// Runs a meeting's automations.
pub struct AutomationsStage {
    runner: Arc<dyn AutomationRunner>,
}

impl AutomationsStage {
    /// The stage over the shared ports.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            runner: Arc::clone(&ports.automations),
        }
    }
}

#[async_trait]
impl PipelineStage for AutomationsStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let final_attempt = !RetryPolicy::DEFAULT.allows_retry_after(ctx.attempt);
        let outcome = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            outcome = self.runner.run_meeting(&meeting, final_attempt) => outcome,
        };
        match outcome {
            Ok(EngineOutcome::NothingToRun) => Ok(StageOutcome::Skipped(NOTHING_TO_RUN)),
            // A failed step is the run's own result, shown in its log; the
            // stage itself did its job.
            Ok(EngineOutcome::Ran { .. }) => Ok(StageOutcome::Done),
            Err(error) => Err(failure(&error)),
        }
    }
}

/// Busy storage and rate-limited insights are retried; anything else waits
/// for the user.
fn failure(error: &AutomationError) -> StageError {
    if error.is_retryable() {
        StageError::retryable_from(error)
    } else {
        StageError::permanent(error.code(), error.to_string())
    }
}

/// A runner that records the meetings it ran on and answers as scripted.
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct RecordedAutomations {
    ran: std::sync::Mutex<Vec<(MeetingId, bool)>>,
    fail_with: std::sync::Mutex<Option<fn() -> AutomationError>>,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordedAutomations {
    /// Fails every run with the error `make` builds.
    pub fn failing(make: fn() -> AutomationError) -> Self {
        Self {
            fail_with: std::sync::Mutex::new(Some(make)),
            ..Self::default()
        }
    }

    /// The meetings run on, with whether it was the last attempt, in order.
    pub fn ran(&self) -> Vec<(MeetingId, bool)> {
        self.ran
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl AutomationRunner for RecordedAutomations {
    async fn run_meeting(
        &self,
        meeting: &MeetingId,
        final_attempt: bool,
    ) -> Result<EngineOutcome, AutomationError> {
        self.ran
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((meeting.clone(), final_attempt));
        let failure = *self
            .fail_with
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        failure.map_or(Ok(EngineOutcome::Ran { runs: 1, failed: 0 }), |make| {
            Err(make())
        })
    }
}

#[cfg(test)]
mod tests {
    use deino_db::DbError;
    use deino_domain::FakeClock;
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    fn ctx(attempt: u32) -> StageCtx {
        StageCtx {
            attempt,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    fn stage(runner: RecordedAutomations) -> (AutomationsStage, Arc<RecordedAutomations>) {
        let runner = Arc::new(runner);
        let ports = StagePorts {
            automations: runner.clone(),
            ..StagePorts::fakes()
        };
        (AutomationsStage::new(&ports), runner)
    }

    fn meeting() -> MeetingId {
        MeetingId::new("meeting-a")
    }

    #[tokio::test]
    async fn the_stage_runs_at_95_and_tells_the_engine_which_attempt_is_last() {
        let (stage, runner) = stage(RecordedAutomations::default());

        assert_eq!(stage.run(&ctx(1), meeting()).await, Ok(StageOutcome::Done));
        assert_eq!(stage.run(&ctx(3), meeting()).await, Ok(StageOutcome::Done));
        assert_eq!(runner.ran(), [(meeting(), false), (meeting(), true)]);
        assert_eq!(
            (stage.id(), stage.order()),
            (StageId::new("automations"), 95)
        );
    }

    #[tokio::test]
    async fn busy_storage_is_retried_and_a_missing_automation_is_not() {
        let busy = || AutomationError::Storage(DbError::Query(sqlx::Error::PoolTimedOut));
        let gone = || AutomationError::NoMeeting;
        let (retried, _) = stage(RecordedAutomations::failing(busy));
        let (waiting, _) = stage(RecordedAutomations::failing(gone));

        assert!(
            retried
                .run(&ctx(1), meeting())
                .await
                .unwrap_err()
                .is_retryable()
        );
        assert!(matches!(
            waiting.run(&ctx(1), meeting()).await,
            Err(StageError::Permanent(_))
        ));
    }

    #[tokio::test]
    async fn a_cancelled_pipeline_stops_the_stage() {
        let (stage, _) = stage(RecordedAutomations::default());
        let ctx = ctx(1);
        ctx.cancel.cancel();

        assert_eq!(stage.run(&ctx, meeting()).await, Err(StageError::Cancelled));
    }
}
