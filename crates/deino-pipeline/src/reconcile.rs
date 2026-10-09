//! Startup reconciliation: the queue is made true before anything runs.
//!
//! After a quit or a crash the database can hold jobs that are no longer
//! true (spacedrive's "no zombie running rows" rule, Apache-2.0):
//!
//! 1. jobs of stages that are no longer registered are deleted;
//! 2. jobs left `running` were interrupted: they return to `pending`, or
//!    fail once a crash has eaten every attempt (a stage that crashes the
//!    app must not crash it forever);
//! 3. then the runner opens and every `processing` meeting and every
//!    meeting with pending work is woken; a `processing` meeting without
//!    jobs (the app stopped between the status change and the enqueue) is
//!    scheduled first. A woken meeting whose pipeline already ended is
//!    moved to `ready` or `failed`.

use deino_db::DbError;
use deino_domain::{JobStatus, MeetingId, MeetingStatus, RunOutcome};

use crate::{Job, PipelineError, PipelineService, RetryPolicy, Settlement};

/// Shown on a stage that was interrupted on its last allowed attempt.
const INTERRUPTED_TOO_OFTEN: &str = "The app quit while this step ran, too many times in a row.";

impl PipelineService {
    /// Reconciles the queue, opens the runner and resumes pending work.
    /// [`PipelineService::start`] runs it in the background at startup.
    pub async fn reconcile(&self) -> Result<(), PipelineError> {
        let pruned = self.jobs.prune(&self.registry.ids()).await?;
        let interrupted = self.recover_interrupted().await?;
        self.runner.open();
        let processing = self
            .meetings
            .list_by_status(MeetingStatus::Processing)
            .await?;
        for meeting in &processing {
            if let Err(error) = self.resume(&meeting.id).await {
                log::error!(target: "pipeline", "could not resume meeting {}: {error}", meeting.id);
            }
        }
        for meeting in self.jobs.meetings_with_work().await? {
            self.runner.wake(&meeting);
        }
        log::info!(
            target: "pipeline",
            "pipeline resumed: {pruned} retired jobs removed, {interrupted} interrupted jobs recovered, {} meetings processing",
            processing.len()
        );
        Ok(())
    }

    /// Wakes a `processing` meeting, scheduling its pipeline only when it
    /// has no jobs (the app stopped between the status change and the
    /// enqueue). Existing jobs are never reset here: a stage that failed
    /// because it kept crashing the app must not start over by itself.
    async fn resume(&self, meeting: &MeetingId) -> Result<(), DbError> {
        if self.jobs.jobs(meeting).await?.is_empty() {
            self.jobs
                .schedule(meeting, &self.registry.ids(), self.clock.now())
                .await?;
        }
        self.runner.wake(meeting);
        Ok(())
    }

    /// Settles every job left running by the last session; returns how many.
    async fn recover_interrupted(&self) -> Result<usize, DbError> {
        let running = self.jobs.running().await?;
        let now = self.clock.now();
        for job in &running {
            let settlement = interrupted(job, &RetryPolicy::DEFAULT, now);
            self.jobs
                .settle(&job.key(), JobStatus::Running, &settlement)
                .await?;
        }
        Ok(running.len())
    }
}

/// How a job a crash interrupted settles: the attempt counts, so a stage
/// that keeps crashing the app runs out of attempts.
fn interrupted(job: &Job, policy: &RetryPolicy, at: chrono::DateTime<chrono::Utc>) -> Settlement {
    let exhausted = !policy.allows_retry_after(job.attempts);
    Settlement {
        status: if exhausted {
            JobStatus::Failed
        } else {
            JobStatus::Pending
        },
        outcome: RunOutcome::Interrupted,
        attempts: job.attempts,
        not_before: None,
        message: exhausted.then(|| INTERRUPTED_TOO_OFTEN.to_owned()),
        at,
    }
}

#[cfg(test)]
mod tests {
    use deino_testkit::at;

    use super::*;
    use crate::testing::job;

    #[test]
    fn an_interrupted_job_is_retried_until_its_attempts_run_out() {
        let now = at("2026-10-08T09:00:00Z");
        let first = Job {
            attempts: 1,
            ..job("a")
        };
        let last = Job {
            attempts: 3,
            ..job("a")
        };

        let retried = interrupted(&first, &RetryPolicy::DEFAULT, now);
        let failed = interrupted(&last, &RetryPolicy::DEFAULT, now);

        assert_eq!(
            (retried.status, retried.outcome, retried.attempts),
            (JobStatus::Pending, RunOutcome::Interrupted, 1)
        );
        assert_eq!(failed.status, JobStatus::Failed);
        assert_eq!(failed.message.as_deref(), Some(INTERRUPTED_TOO_OFTEN));
    }
}
