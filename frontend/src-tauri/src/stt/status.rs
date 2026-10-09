//! What transcription is doing right now, for `get_transcription_status`.
//!
//! The stop flow polls it until nothing is queued or processing. Jobs (the
//! live lane, the legacy chunk worker) hold a [`JobGuard`] while they run;
//! queued work is counted in units the job chooses (chunks, or seconds of
//! live backlog). Every change stamps the last-activity time.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use deino_stt::clock::MonotonicClock;

/// Transcription activity counters, shared by every transcription job.
#[derive(Debug)]
pub struct TranscriptionActivity {
    jobs: AtomicUsize,
    queued: AtomicUsize,
    /// Monotonic ms of the last change.
    last_activity_ms: AtomicU64,
    clock: Arc<dyn MonotonicClock>,
}

/// A moment's view of the activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivitySnapshot {
    /// Units of work queued.
    pub queued: usize,
    /// Whether any job is running.
    pub processing: bool,
    /// Time since the last change.
    pub idle: Duration,
}

/// Marks a job as running until dropped.
#[must_use = "the job counts as running only while the guard lives"]
pub struct JobGuard(Arc<TranscriptionActivity>);

impl Drop for JobGuard {
    fn drop(&mut self) {
        self.0.jobs.fetch_sub(1, Ordering::SeqCst);
        self.0.touch();
    }
}

impl TranscriptionActivity {
    /// Idle activity measured on `clock`.
    pub fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        let now = millis(clock.now());
        Self {
            jobs: AtomicUsize::new(0),
            queued: AtomicUsize::new(0),
            last_activity_ms: AtomicU64::new(now),
            clock,
        }
    }

    /// Starts a job; it runs until the guard drops.
    pub fn begin_job(self: &Arc<Self>) -> JobGuard {
        self.jobs.fetch_add(1, Ordering::SeqCst);
        self.touch();
        JobGuard(Arc::clone(self))
    }

    /// `units` more work queued.
    pub fn enqueue(&self, units: usize) {
        self.queued.fetch_add(units, Ordering::SeqCst);
        self.touch();
    }

    /// `units` of queued work done.
    pub fn complete(&self, units: usize) {
        let _previous = self
            .queued
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |queued| {
                Some(queued.saturating_sub(units))
            });
        self.touch();
    }

    /// Records that something happened now.
    pub fn touch(&self) {
        self.last_activity_ms
            .store(millis(self.clock.now()), Ordering::SeqCst);
    }

    /// The current state.
    pub fn snapshot(&self) -> ActivitySnapshot {
        let last = self.last_activity_ms.load(Ordering::SeqCst);
        ActivitySnapshot {
            queued: self.queued.load(Ordering::SeqCst),
            processing: self.jobs.load(Ordering::SeqCst) > 0,
            idle: Duration::from_millis(millis(self.clock.now()).saturating_sub(last)),
        }
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use deino_stt::clock::FakeMonotonic;

    use super::*;

    #[test]
    fn a_running_job_with_queued_work_reads_as_busy_until_it_finishes() {
        let clock = Arc::new(FakeMonotonic::default());
        let activity = Arc::new(TranscriptionActivity::new(clock.clone()));

        let job = activity.begin_job();
        activity.enqueue(3);
        activity.complete(1);
        clock.advance(Duration::from_secs(2));
        let busy = activity.snapshot();
        activity.complete(5);
        drop(job);
        let done = activity.snapshot();

        assert_eq!(busy.queued, 2);
        assert!(busy.processing);
        assert_eq!(busy.idle, Duration::from_secs(2));
        assert_eq!((done.queued, done.processing), (0, false));
        assert_eq!(done.idle, Duration::ZERO);
    }
}
