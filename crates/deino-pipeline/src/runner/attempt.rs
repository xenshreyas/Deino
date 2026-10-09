//! One attempt of a stage: claim a permit and the job, run the stage under
//! its cancellation token, and settle the job by the result.

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_db::DbError;
use deino_domain::{JobStatus, MeetingId, RunOutcome};
use tokio::sync::SemaphorePermit;
use tokio_util::sync::CancellationToken;

use super::Inner;
use crate::{Job, PipelineStage, RetryPolicy, Settlement, StageCtx, StageError, StageOutcome};

/// How long a cancelled stage may take to stop on its own (and clean up)
/// before the runner abandons it.
const CANCEL_GRACE: TimeDelta = TimeDelta::seconds(2);

impl Inner {
    /// Runs one attempt of the pending `job` and stores how it ended.
    pub(super) async fn attempt(&self, job: &Job) -> Result<(), DbError> {
        let cancel = self.slots.token(&job.meeting_id);
        let Some(_permit) = self.permit(&cancel).await else {
            return self.cancel_waiting(job).await;
        };
        let Some(stage) = self.deps.registry.find(&job.stage).cloned() else {
            return Ok(());
        };
        let Some(attempt) = self.deps.jobs.start(&job.key(), self.deps.clock.now()).await? else {
            return Ok(());
        };
        self.report(job, JobStatus::Running);
        let ctx = StageCtx {
            attempt,
            cancel: cancel.child_token(),
            clock: Arc::clone(&self.deps.clock),
            events: Arc::clone(&self.deps.progress),
        };
        let result = self.run_stage(&stage, &ctx, job.meeting_id.clone()).await;
        let settlement = settle(result, &self.settle_context(attempt));
        log_attempt(job, attempt, &settlement);
        if self.deps.jobs.settle(&job.key(), JobStatus::Running, &settlement).await? {
            self.report(job, settlement.status);
        }
        if settlement.status == JobStatus::Cancelled {
            self.slots.renew(&job.meeting_id);
        }
        Ok(())
    }

    /// One of the global permits, or `None` when `cancel` fires first.
    async fn permit(&self, cancel: &CancellationToken) -> Option<SemaphorePermit<'_>> {
        tokio::select! {
            permit = self.permits.acquire() => permit.ok(),
            () = cancel.cancelled() => None,
        }
    }

    /// Runs `stage`; once cancelled, gives it [`CANCEL_GRACE`] to stop.
    async fn run_stage(
        &self,
        stage: &Arc<dyn PipelineStage>,
        ctx: &StageCtx,
        meeting: MeetingId,
    ) -> Result<StageOutcome, StageError> {
        let run = stage.run(ctx, meeting);
        tokio::pin!(run);
        tokio::select! {
            result = &mut run => return result,
            () = ctx.cancel.cancelled() => {}
        }
        let deadline = self.deps.clock.now() + CANCEL_GRACE;
        tokio::select! {
            result = run => result,
            () = self.deps.sleeper.sleep_until(deadline) => Err(StageError::Cancelled),
        }
    }

    fn settle_context(&self, attempt: u32) -> SettleContext {
        SettleContext {
            attempt,
            policy: self.deps.policy,
            now: self.deps.clock.now(),
            shutting_down: self.shutdown.is_cancelled(),
        }
    }
}

/// What, besides the result, decides how an attempt settles.
#[derive(Clone, Copy, Debug)]
struct SettleContext {
    /// The attempt that ended, from 1.
    attempt: u32,
    policy: RetryPolicy,
    now: DateTime<Utc>,
    /// The app is quitting, so a cancellation is an interruption.
    shutting_down: bool,
}

/// How a job settles after its attempt returned `result`.
fn settle(result: Result<StageOutcome, StageError>, context: &SettleContext) -> Settlement {
    let settled = |status, outcome, message: Option<String>| Settlement {
        status,
        outcome,
        attempts: context.attempt,
        not_before: None,
        message,
        at: context.now,
    };
    match result {
        Ok(StageOutcome::Done) => settled(JobStatus::Succeeded, RunOutcome::Succeeded, None),
        Ok(StageOutcome::Skipped(reason)) => {
            settled(JobStatus::Skipped, RunOutcome::Skipped, Some(reason.to_owned()))
        }
        Err(StageError::Cancelled) if context.shutting_down => Settlement {
            // Quitting is not the stage's fault: the attempt does not count.
            attempts: context.attempt.saturating_sub(1),
            ..settled(JobStatus::Pending, RunOutcome::Interrupted, None)
        },
        Err(StageError::Cancelled) => settled(JobStatus::Cancelled, RunOutcome::Cancelled, None),
        Err(error) if error.is_retryable() && context.policy.allows_retry_after(context.attempt) => {
            Settlement {
                not_before: Some(context.now + context.policy.delay_after(context.attempt)),
                ..settled(JobStatus::Pending, RunOutcome::Failed, Some(error.to_string()))
            }
        }
        Err(error) => settled(JobStatus::Failed, RunOutcome::Failed, Some(error.to_string())),
    }
}

/// Logs how an attempt ended: IDs, the attempt and the user-safe message only.
fn log_attempt(job: &Job, attempt: u32, settlement: &Settlement) {
    let (meeting, stage, outcome) = (&job.meeting_id, &job.stage, settlement.outcome);
    match &settlement.message {
        Some(message) if outcome == RunOutcome::Failed => log::warn!(
            target: "pipeline",
            "{stage} attempt {attempt} for meeting {meeting} failed: {message}"
        ),
        _ => log::info!(
            target: "pipeline",
            "{stage} attempt {attempt} for meeting {meeting}: {outcome}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::ErrorCode;
    use deino_testkit::at;

    use super::*;

    const NOW: &str = "2026-10-08T09:00:00Z";

    fn context(attempt: u32) -> SettleContext {
        SettleContext {
            attempt,
            policy: RetryPolicy::DEFAULT,
            now: at(NOW),
            shutting_down: false,
        }
    }

    fn offline() -> StageError {
        StageError::retryable(ErrorCode::Network, "the provider did not answer")
    }

    #[test]
    fn a_retryable_failure_waits_out_the_backoff_as_pending() {
        let settlement = settle(Err(offline()), &context(1));

        assert_eq!(settlement.status, JobStatus::Pending);
        assert_eq!(settlement.outcome, RunOutcome::Failed);
        assert_eq!(settlement.not_before, Some(at(NOW) + TimeDelta::seconds(10)));
        assert_eq!(settlement.message.as_deref(), Some("the provider did not answer"));
    }

    #[test]
    fn the_last_attempt_and_permanent_failures_fail_the_job() {
        let exhausted = settle(Err(offline()), &context(3));
        let permanent = settle(
            Err(StageError::permanent(ErrorCode::Invalid, "no model")),
            &context(1),
        );

        assert_eq!((exhausted.status, exhausted.not_before), (JobStatus::Failed, None));
        assert_eq!(permanent.status, JobStatus::Failed);
        assert_eq!(permanent.message.as_deref(), Some("no model"));
    }

    #[test]
    fn quitting_mid_attempt_leaves_the_job_pending_without_counting_it() {
        let quitting = SettleContext {
            shutting_down: true,
            ..context(2)
        };

        let settlement = settle(Err(StageError::Cancelled), &quitting);

        assert_eq!(settlement.status, JobStatus::Pending);
        assert_eq!(settlement.outcome, RunOutcome::Interrupted);
        assert_eq!(settlement.attempts, 1);
    }

    #[test]
    fn success_skip_and_user_cancel_settle_as_themselves() {
        let done = settle(Ok(StageOutcome::Done), &context(1));
        let skipped = settle(Ok(StageOutcome::Skipped("no transcript")), &context(1));
        let cancelled = settle(Err(StageError::Cancelled), &context(1));

        assert_eq!(done.status, JobStatus::Succeeded);
        assert_eq!(
            (skipped.status, skipped.message.as_deref()),
            (JobStatus::Skipped, Some("no transcript"))
        );
        assert_eq!(cancelled.status, JobStatus::Cancelled);
    }
}
