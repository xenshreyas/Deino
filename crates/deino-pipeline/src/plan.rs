//! What a meeting's pipeline does next, decided from its jobs alone.
//!
//! Stages run in registry order and each waits for the one before it to
//! succeed or skip. A failed or cancelled stage stops the meeting's
//! pipeline: the stages after it stay pending until the user re-runs it.

use chrono::{DateTime, Utc};
use deino_domain::JobStatus;

use crate::Job;

/// The next step for one meeting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Start an attempt of this pending job.
    Run(Job),
    /// The next job is a retry that may start at `at`.
    Retry {
        /// The pending job.
        job: Job,
        /// When its backoff ends.
        at: DateTime<Utc>,
    },
    /// Another attempt of the next job is still running.
    Busy,
    /// Nothing can run; the pipeline ended this way.
    Ended(Ending),
}

/// How a meeting's pipeline ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// Every stage succeeded or skipped.
    Completed,
    /// A stage failed or was cancelled.
    Stopped,
    /// The meeting has no jobs.
    Empty,
}

/// The first job, in stage order, that has not succeeded or skipped.
pub(crate) fn next_unfinished(jobs: &[Job]) -> Option<&Job> {
    jobs.iter().find(|job| !job.status.is_success())
}

/// True when the meeting already has work queued or running, so scheduling
/// it again would only restart what is under way.
pub(crate) fn is_scheduled(jobs: &[Job]) -> bool {
    next_unfinished(jobs).is_some_and(|job| !job.status.is_finished())
}

/// The next step for a meeting whose jobs, in stage order, are `jobs`.
pub(crate) fn next_step(jobs: &[Job], now: DateTime<Utc>) -> Step {
    let Some(next) = next_unfinished(jobs) else {
        let ending = if jobs.is_empty() {
            Ending::Empty
        } else {
            Ending::Completed
        };
        return Step::Ended(ending);
    };
    match (next.status, next.not_before) {
        (JobStatus::Pending, Some(at)) if at > now => Step::Retry {
            job: next.clone(),
            at,
        },
        (JobStatus::Pending, _) => Step::Run(next.clone()),
        (JobStatus::Running, _) => Step::Busy,
        _ => Step::Ended(Ending::Stopped),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use deino_testkit::at;

    use super::*;
    use crate::testing::job;

    const NOW: &str = "2026-10-08T09:00:00Z";

    fn with(stage: &str, status: JobStatus) -> Job {
        Job {
            status,
            ..job(stage)
        }
    }

    #[test]
    fn the_first_pending_job_after_successes_runs() {
        let jobs = [
            with("a", JobStatus::Succeeded),
            with("b", JobStatus::Skipped),
            with("c", JobStatus::Pending),
            with("d", JobStatus::Pending),
        ];

        assert_eq!(next_step(&jobs, at(NOW)), Step::Run(jobs[2].clone()));
        assert!(is_scheduled(&jobs));
    }

    #[test]
    fn a_retry_waits_for_its_time_and_then_runs() {
        let retry_at = at(NOW) + TimeDelta::seconds(10);
        let jobs = [Job {
            not_before: Some(retry_at),
            ..with("a", JobStatus::Pending)
        }];

        assert_eq!(
            next_step(&jobs, at(NOW)),
            Step::Retry {
                job: jobs[0].clone(),
                at: retry_at
            }
        );
        assert_eq!(next_step(&jobs, retry_at), Step::Run(jobs[0].clone()));
    }

    #[test]
    fn a_failed_or_cancelled_stage_stops_the_stages_after_it() {
        for status in [JobStatus::Failed, JobStatus::Cancelled] {
            let jobs = [with("a", status), with("b", JobStatus::Pending)];

            assert_eq!(next_step(&jobs, at(NOW)), Step::Ended(Ending::Stopped));
            assert!(!is_scheduled(&jobs));
        }
    }

    #[test]
    fn a_running_stage_keeps_the_pipeline_busy() {
        let jobs = [with("a", JobStatus::Running), with("b", JobStatus::Pending)];

        assert_eq!(next_step(&jobs, at(NOW)), Step::Busy);
        assert!(is_scheduled(&jobs));
    }

    #[test]
    fn all_stages_done_completes_and_no_jobs_is_empty() {
        let done = [with("a", JobStatus::Succeeded), with("b", JobStatus::Skipped)];

        assert_eq!(next_step(&done, at(NOW)), Step::Ended(Ending::Completed));
        assert_eq!(next_step(&[], at(NOW)), Step::Ended(Ending::Empty));
        assert!(!is_scheduled(&done));
    }
}
