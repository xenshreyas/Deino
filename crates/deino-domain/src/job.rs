//! The post-meeting pipeline's vocabulary (FND-08): where a stage is in a
//! meeting's run, and how each attempt ended.
//!
//! The runner (`deino-pipeline`) stores both in TEXT columns and sends
//! [`JobStatus`] to the UI, so they live here with the other closed sets.
//!
//! ```text
//! pending ──start──▶ running ──▶ succeeded | skipped | failed | cancelled
//!    ▲                  │
//!    └──retry, restart──┘
//! ```

db_enum! {
    /// Where a pipeline stage is in a meeting's current run. Stored in
    /// `jobs.status`.
    JobStatus {
        /// Waiting for its turn, or for its retry backoff to pass.
        Pending = "pending",
        /// An attempt is running.
        Running = "running",
        /// The stage finished its work.
        Succeeded = "succeeded",
        /// The stage had nothing to do, for example no transcript yet.
        Skipped = "skipped",
        /// The stage failed for good in this run; it can be re-run.
        Failed = "failed",
        /// The stage was cancelled; it can be re-run.
        Cancelled = "cancelled",
    }
}

impl JobStatus {
    /// True once the stage is done with this run, successfully or not.
    pub fn is_finished(self) -> bool {
        !matches!(self, Self::Pending | Self::Running)
    }

    /// True when the stage finished without a problem, so later stages may run.
    pub fn is_success(self) -> bool {
        matches!(self, Self::Succeeded | Self::Skipped)
    }
}

db_enum! {
    /// How one attempt of a pipeline stage ended. Stored in
    /// `job_runs.outcome`.
    RunOutcome {
        /// The stage finished its work.
        Succeeded = "succeeded",
        /// The stage had nothing to do.
        Skipped = "skipped",
        /// The attempt failed; the job may retry.
        Failed = "failed",
        /// The attempt was cancelled.
        Cancelled = "cancelled",
        /// The app quit or crashed during the attempt.
        Interrupted = "interrupted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn job_vocabulary_serializes_as_its_schema_names() {
        assert_names_agree(JobStatus::ALL, JobStatus::as_str);
        assert_names_agree(RunOutcome::ALL, RunOutcome::as_str);
    }

    #[test]
    fn only_pending_and_running_jobs_are_unfinished() {
        let unfinished: Vec<JobStatus> = JobStatus::ALL
            .iter()
            .copied()
            .filter(|status| !status.is_finished())
            .collect();
        let successes: Vec<JobStatus> = JobStatus::ALL
            .iter()
            .copied()
            .filter(|status| status.is_success())
            .collect();

        assert_eq!(unfinished, [JobStatus::Pending, JobStatus::Running]);
        assert_eq!(successes, [JobStatus::Succeeded, JobStatus::Skipped]);
    }
}
