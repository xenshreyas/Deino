//! Fake stages for this crate's tests: stages that succeed, skip, fail a
//! few times, fail for good, hang, or wait at a gate, and that record when
//! they ran and how many ran at once.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_domain::{ErrorCode, JobStatus, MeetingId, StageId};
use deino_testkit::at;
use tokio::sync::Semaphore;

use crate::{Job, PipelineStage, StageCtx, StageError, StageOutcome};

/// What a [`FakeStage`] does when it runs.
#[derive(Clone, Debug)]
pub(crate) enum Behavior {
    /// Succeeds at once.
    Succeed,
    /// Skips, with a reason.
    Skip,
    /// Fails its first `times` calls (retryably or not), then succeeds.
    Fail { times: usize, retryable: bool },
    /// Never returns and ignores cancellation.
    Hang,
    /// Waits for cancellation, then reports it.
    UntilCancelled,
    /// Waits for a permit from the gate, then succeeds.
    Gated(Arc<Semaphore>),
}

/// Shared by several fake stages: which stage started when, and how many
/// ran at the same time.
#[derive(Debug, Default)]
pub(crate) struct Activity {
    running: AtomicUsize,
    peak: AtomicUsize,
    starts: Mutex<Vec<(String, MeetingId)>>,
}

impl Activity {
    /// The most stages that ever ran at once.
    pub(crate) fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    /// `(stage, meeting)` of every run, in start order.
    pub(crate) fn starts(&self) -> Vec<(String, MeetingId)> {
        self.starts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn enter(&self, stage: &str, meeting: &MeetingId) {
        let running = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(running, Ordering::SeqCst);
        self.starts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((stage.to_owned(), meeting.clone()));
    }

    fn leave(&self) {
        self.running.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A scripted stage.
#[derive(Debug)]
pub(crate) struct FakeStage {
    id: &'static str,
    order: u16,
    behavior: Behavior,
    calls: AtomicUsize,
    activity: Arc<Activity>,
}

impl FakeStage {
    pub(crate) fn new(id: &'static str, order: u16, behavior: Behavior) -> Self {
        Self {
            id,
            order,
            behavior,
            calls: AtomicUsize::new(0),
            activity: Arc::default(),
        }
    }

    pub(crate) fn succeeding(id: &'static str, order: u16) -> Self {
        Self::new(id, order, Behavior::Succeed)
    }

    /// Records its runs in `activity`, shared with other stages.
    pub(crate) fn sharing(mut self, activity: &Arc<Activity>) -> Self {
        self.activity = Arc::clone(activity);
        self
    }

    pub(crate) fn into_dyn(self) -> Arc<dyn PipelineStage> {
        Arc::new(self)
    }

    async fn act(&self, ctx: &StageCtx, call: usize) -> Result<StageOutcome, StageError> {
        match &self.behavior {
            Behavior::Fail { times, retryable } if call < *times => Err(failure(*retryable)),
            Behavior::Succeed | Behavior::Fail { .. } => Ok(StageOutcome::Done),
            Behavior::Skip => Ok(StageOutcome::Skipped("nothing to do")),
            Behavior::Hang => std::future::pending().await,
            Behavior::UntilCancelled => {
                ctx.cancel.cancelled().await;
                Err(StageError::Cancelled)
            }
            Behavior::Gated(gate) => {
                let permit = gate.acquire().await.expect("the gate stays open");
                permit.forget();
                Ok(StageOutcome::Done)
            }
        }
    }
}

fn failure(retryable: bool) -> StageError {
    if retryable {
        StageError::retryable(ErrorCode::Network, "the provider did not answer")
    } else {
        StageError::permanent(ErrorCode::Unavailable, "no model is configured")
    }
}

#[async_trait]
impl PipelineStage for FakeStage {
    fn id(&self) -> StageId {
        StageId::new(self.id)
    }

    fn order(&self) -> u16 {
        self.order
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.activity.enter(self.id, &meeting);
        let outcome = self.act(ctx, call).await;
        self.activity.leave();
        outcome
    }
}

/// A pending job of `meeting-a` for `stage`.
pub(crate) fn job(stage: &str) -> Job {
    Job {
        meeting_id: MeetingId::new("meeting-a"),
        stage: StageId::new(stage),
        status: JobStatus::Pending,
        attempts: 0,
        not_before: None,
        message: None,
        updated_at: at("2026-10-08T09:00:00Z"),
    }
}
