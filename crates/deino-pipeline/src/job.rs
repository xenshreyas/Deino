//! The pipeline's records: a [`Job`] per meeting and stage, the
//! [`Settlement`] that ends an attempt, and the [`StageStatus`] the UI shows.

use chrono::{DateTime, Utc};
use deino_domain::{JobStatus, MeetingId, RunOutcome, StageId};
use serde::Serialize;

/// Names one job: a stage of a meeting's pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobKey {
    /// The meeting being processed.
    pub meeting_id: MeetingId,
    /// The stage to run for it.
    pub stage: StageId,
}

/// One stage of one meeting's current pipeline run (a `jobs` row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// The meeting being processed.
    pub meeting_id: MeetingId,
    /// The stage to run for it.
    pub stage: StageId,
    /// Where the stage is in this run.
    pub status: JobStatus,
    /// Attempts started in this run.
    pub attempts: u32,
    /// A pending retry waits until then; `None` runs as soon as it is next.
    pub not_before: Option<DateTime<Utc>>,
    /// Why the stage was skipped or how its last attempt failed (user-safe).
    pub message: Option<String>,
    /// When the job last changed.
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// The job's key.
    pub fn key(&self) -> JobKey {
        JobKey {
            meeting_id: self.meeting_id.clone(),
            stage: self.stage.clone(),
        }
    }
}

/// How an attempt (or a stop between attempts) leaves a job: its new state,
/// and the outcome recorded on the attempt's `job_runs` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    /// The job's new status.
    pub status: JobStatus,
    /// How the open attempt ended, if one is open.
    pub outcome: RunOutcome,
    /// Attempts started in this run, counting the one that just ended
    /// unless the app interrupted it.
    pub attempts: u32,
    /// When a retry may start; only for a pending job.
    pub not_before: Option<DateTime<Utc>>,
    /// Why the stage was skipped or how it failed (user-safe).
    pub message: Option<String>,
    /// When the job settles.
    pub at: DateTime<Utc>,
}

/// A stage's state in a meeting's pipeline, as the meeting page shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StageStatus {
    /// The stage.
    pub stage: StageId,
    /// Where it is in the current run.
    pub status: JobStatus,
    /// Attempts started in this run; a pending stage with attempts is
    /// waiting to retry.
    pub attempts: u32,
    /// When a pending retry starts, if it is waiting.
    pub retry_at: Option<DateTime<Utc>>,
    /// Why it was skipped or how it failed: user-safe, never meeting content.
    pub message: Option<String>,
    /// When the stage's status last changed.
    pub updated_at: DateTime<Utc>,
}

impl From<Job> for StageStatus {
    fn from(job: Job) -> Self {
        Self {
            stage: job.stage,
            status: job.status,
            attempts: job.attempts,
            retry_at: job.not_before,
            message: job.message,
            updated_at: job.updated_at,
        }
    }
}
