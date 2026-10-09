//! The runner: executes jobs, one worker per meeting.
//!
//! - **Order and serialization:** a meeting's worker runs its stages one at
//!   a time, in registry order ([`plan`](crate::plan)); [`Slots`] guarantees
//!   one worker per meeting.
//! - **Global cap:** a semaphore of [`MAX_CONCURRENT_STAGES`] permits, held
//!   for each attempt only, so a meeting waiting out a retry blocks no one.
//! - **Retries:** retryable failures are retried with backoff from the
//!   injected `Clock` ([`RetryPolicy`]), waited out through the [`Sleeper`].
//! - **Cancellation:** per meeting (the user) or for everything (shutdown).
//!   A stage gets a short grace period to stop on its own before it is
//!   abandoned. Shutdown leaves interrupted jobs pending for the next start.
//! - **Ownership:** every task runs on one `TaskTracker`; shutdown cancels
//!   and waits for all of them. Nothing is spawned untracked (§3.7).
//! - **Startup:** workers wait until the service has reconciled the queue
//!   ([`Runner::open`]), so no job of this process is mistaken for one a
//!   crash interrupted.

mod attempt;
mod slots;
mod worker;

use std::future::Future;
use std::sync::Arc;

use deino_core::meetings::MeetingLifecycleService;
use deino_domain::{Clock, MeetingId};
use tokio::sync::{Semaphore, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use self::slots::Slots;
use crate::{JobRepo, ProgressSink, Registry, RetryPolicy, Sleeper};

/// Stages that may run at the same time across all meetings. Two keep the
/// Mac responsive while a long summary does not hold up a short stage.
pub const MAX_CONCURRENT_STAGES: usize = 2;

/// What the runner works with.
pub(crate) struct RunnerDeps {
    pub(crate) jobs: Arc<dyn JobRepo>,
    pub(crate) registry: Arc<Registry>,
    pub(crate) lifecycle: Arc<MeetingLifecycleService>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) sleeper: Arc<dyn Sleeper>,
    pub(crate) progress: Arc<dyn ProgressSink>,
    pub(crate) policy: RetryPolicy,
}

/// Runs jobs on tracked tasks. Clones share the same workers.
#[derive(Clone)]
pub(crate) struct Runner {
    inner: Arc<Inner>,
}

/// The runner's state, shared with its workers.
struct Inner {
    deps: RunnerDeps,
    permits: Semaphore,
    slots: Slots,
    tracker: TaskTracker,
    shutdown: CancellationToken,
    /// `true` once the queue is reconciled and workers may start jobs.
    open: watch::Sender<bool>,
}

impl Runner {
    /// A runner that starts jobs only after [`Runner::open`].
    pub(crate) fn new(deps: RunnerDeps) -> Self {
        let shutdown = CancellationToken::new();
        Self {
            inner: Arc::new(Inner {
                deps,
                permits: Semaphore::new(MAX_CONCURRENT_STAGES),
                slots: Slots::new(shutdown.clone()),
                tracker: TaskTracker::new(),
                shutdown,
                open: watch::Sender::new(false),
            }),
        }
    }

    /// Lets workers start jobs; called once the queue is reconciled.
    pub(crate) fn open(&self) {
        self.inner.open.send_replace(true);
    }

    /// Makes sure a worker is processing `meeting`'s jobs.
    pub(crate) fn wake(&self, meeting: &MeetingId) {
        if self.inner.shutdown.is_cancelled() || !self.inner.slots.claim(meeting) {
            return;
        }
        let inner = Arc::clone(&self.inner);
        self.inner
            .tracker
            .spawn(worker::drive(inner, meeting.clone()));
    }

    /// Cancels `meeting`'s running or waiting stage. Returns `false` when
    /// nothing is running for it.
    pub(crate) fn cancel(&self, meeting: &MeetingId) -> bool {
        self.inner.slots.cancel(meeting)
    }

    /// Runs `task` on the runner's tracker, so shutdown waits for it.
    pub(crate) fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.inner.tracker.spawn(task);
    }

    /// Cancels every worker and waits until all of them have stopped.
    pub(crate) async fn shutdown(&self) {
        self.inner.shutdown.cancel();
        self.inner.tracker.close();
        self.inner.tracker.wait().await;
    }

    /// Waits until every task has finished, without stopping the runner.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) async fn until_idle(&self) {
        self.inner.tracker.close();
        self.inner.tracker.wait().await;
        self.inner.tracker.reopen();
    }
}
