//! A meeting's worker: plans the next step from the meeting's jobs, takes
//! it, and repeats until nothing can run. When the pipeline ends, it moves
//! the meeting to `ready` or `failed` through the meetings lifecycle.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_core::meetings::MeetingError;
use deino_db::DbError;
use deino_domain::{JobStatus, MeetingEvent, MeetingId, RunOutcome};

use super::Inner;
use crate::plan::{self, Ending, Step};
use crate::{Job, PipelineProgress, Settlement};

/// Processes `meeting`'s jobs until its pipeline ends or the app shuts down.
pub(super) async fn drive(inner: Arc<Inner>, meeting: MeetingId) {
    if !inner.until_open().await {
        return inner.slots.abandon(&meeting);
    }
    while !inner.shutdown.is_cancelled() {
        match inner.step(&meeting).await {
            Ok(true) => {}
            // The step gave the meeting up.
            Ok(false) => return,
            // The jobs stay as stored; the next wake or start resumes them.
            Err(error) => {
                log::error!(target: "pipeline", "stopped processing meeting {meeting}: {error}");
                break;
            }
        }
    }
    inner.slots.abandon(&meeting);
}

impl Inner {
    /// Waits until the runner is open; `false` when it shut down first.
    async fn until_open(&self) -> bool {
        let mut open = self.open.subscribe();
        tokio::select! {
            opened = open.wait_for(|open| *open) => opened.is_ok(),
            () = self.shutdown.cancelled() => false,
        }
    }

    /// Takes the meeting's next step. Returns `false` once the worker has
    /// given the meeting up.
    async fn step(&self, meeting: &MeetingId) -> Result<bool, DbError> {
        let jobs = self.deps.registry.in_order(self.deps.jobs.jobs(meeting).await?);
        match plan::next_step(&jobs, self.deps.clock.now()) {
            Step::Run(job) => self.attempt(&job).await.map(|()| true),
            Step::Retry { job, at } => self.wait_for_retry(&job, at).await.map(|()| true),
            Step::Busy => {
                self.slots.abandon(meeting);
                Ok(false)
            }
            Step::Ended(ending) => Ok(self.end(meeting, ending).await),
        }
    }

    /// Waits out `job`'s backoff. A cancel while waiting cancels the job.
    async fn wait_for_retry(&self, job: &Job, at: DateTime<Utc>) -> Result<(), DbError> {
        let cancel = self.slots.token(&job.meeting_id);
        tokio::select! {
            () = self.deps.sleeper.sleep_until(at) => Ok(()),
            () = cancel.cancelled() => self.cancel_waiting(job).await,
        }
    }

    /// Cancels a pending `job` whose wait was interrupted by the user. On
    /// shutdown the job stays pending, to resume at the next start.
    pub(super) async fn cancel_waiting(&self, job: &Job) -> Result<(), DbError> {
        if self.shutdown.is_cancelled() {
            return Ok(());
        }
        let settlement = Settlement {
            status: JobStatus::Cancelled,
            outcome: RunOutcome::Cancelled,
            attempts: job.attempts,
            not_before: None,
            message: None,
            at: self.deps.clock.now(),
        };
        if self.deps.jobs.settle(&job.key(), JobStatus::Pending, &settlement).await? {
            self.report(job, JobStatus::Cancelled);
        }
        self.slots.renew(&job.meeting_id);
        Ok(())
    }

    /// Finishes the meeting's pipeline unless more work arrived. Returns
    /// `true` when the worker should look at the jobs again.
    async fn end(&self, meeting: &MeetingId, ending: Ending) -> bool {
        if self.slots.take_rescan(meeting) {
            return true;
        }
        self.finish_meeting(meeting, ending).await;
        !self.slots.release(meeting)
    }

    /// Moves a processing meeting to `ready` or `failed` by how its pipeline ended.
    async fn finish_meeting(&self, meeting: &MeetingId, ending: Ending) {
        let event = match ending {
            Ending::Completed => MeetingEvent::Complete,
            Ending::Stopped => MeetingEvent::Fail,
            Ending::Empty => return,
        };
        match self.deps.lifecycle.transition(meeting, event).await {
            Ok(status) => log::info!(target: "pipeline", "pipeline of meeting {meeting} ended: {status}"),
            // The meeting is not processing (a re-run of a ready meeting's stage).
            Err(MeetingError::InvalidTransition(_)) => {}
            Err(error) => {
                log::warn!(target: "pipeline", "could not finish meeting {meeting}: {error}");
            }
        }
    }

    /// Reports `job`'s new status.
    pub(super) fn report(&self, job: &Job, status: JobStatus) {
        self.deps.progress.progress(&PipelineProgress {
            meeting_id: job.meeting_id.clone(),
            stage: job.stage.clone(),
            status,
        });
    }
}
