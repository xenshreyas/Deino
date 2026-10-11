//! The backfill: embeds every finished meeting, newest first, on one
//! tracked background task, reporting progress as it goes.
//!
//! It is safe to run again at any time: unchanged chunks keep their
//! vectors, so a second run only reads. It stops early when the model goes
//! missing, and when the app quits (between meetings or mid-meeting; the
//! interrupted meeting is embedded again next time).

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::{MeetingSourceReader, SemanticIndexer};
use crate::search::SearchError;

/// Where the backfill is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackfillProgress {
    /// Whether it is running.
    pub running: bool,
    /// Meetings handled so far in this run.
    pub done: u32,
    /// Meetings this run handles.
    pub total: u32,
    /// Meetings that could not be embedded in this run.
    pub failed: u32,
}

/// Who hears about the backfill's progress (the app forwards it to the UI).
pub trait IndexEvents: Send + Sync {
    /// The backfill moved on.
    fn backfill(&self, progress: &BackfillProgress);
}

/// Runs the backfill.
pub struct Backfill {
    indexer: Arc<SemanticIndexer>,
    sources: Arc<dyn MeetingSourceReader>,
    events: Arc<dyn IndexEvents>,
    progress: Arc<Mutex<BackfillProgress>>,
    tracker: TaskTracker,
    cancel: CancellationToken,
}

impl Backfill {
    /// A backfill over the indexer, reading the work list from `sources`.
    pub fn new(
        indexer: Arc<SemanticIndexer>,
        sources: Arc<dyn MeetingSourceReader>,
        events: Arc<dyn IndexEvents>,
    ) -> Self {
        Self {
            indexer,
            sources,
            events,
            progress: Arc::new(Mutex::new(BackfillProgress::default())),
            tracker: TaskTracker::new(),
            cancel: CancellationToken::new(),
        }
    }

    /// Where the current (or last) run is.
    pub fn progress(&self) -> BackfillProgress {
        *lock(&self.progress)
    }

    /// Starts a run unless one is running; returns whether it started.
    pub fn start(&self) -> bool {
        {
            let mut progress = lock(&self.progress);
            if progress.running || self.cancel.is_cancelled() {
                return false;
            }
            *progress = BackfillProgress {
                running: true,
                ..BackfillProgress::default()
            };
        }
        let run = Run {
            indexer: Arc::clone(&self.indexer),
            sources: Arc::clone(&self.sources),
            events: Arc::clone(&self.events),
            progress: Arc::clone(&self.progress),
        };
        let cancel = self.cancel.clone();
        self.tracker.spawn(async move {
            tokio::select! {
                () = cancel.cancelled() => {}
                () = run.all() => {}
            }
        });
        true
    }

    /// Stops a run and waits for its task to end. Call it before the app
    /// exits; no run starts afterwards.
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        self.tracker.close();
        self.tracker.wait().await;
    }

    /// Waits until the current run has finished (tests).
    #[cfg(any(test, feature = "test-support"))]
    pub async fn until_idle(&self) {
        self.tracker.close();
        self.tracker.wait().await;
        self.tracker.reopen();
    }
}

/// One run's state, owned by its task.
struct Run {
    indexer: Arc<SemanticIndexer>,
    sources: Arc<dyn MeetingSourceReader>,
    events: Arc<dyn IndexEvents>,
    progress: Arc<Mutex<BackfillProgress>>,
}

impl Run {
    async fn all(self) {
        let meetings = match self.sources.to_index().await {
            Ok(meetings) => meetings,
            Err(error) => {
                log::error!(target: "search", "the backfill could not list meetings: {error}");
                return self.finish();
            }
        };
        self.update(|progress| progress.total = u32::try_from(meetings.len()).unwrap_or(u32::MAX));
        for meeting in &meetings {
            match self.indexer.index_meeting(meeting).await {
                Ok(_) => self.update(|progress| progress.done += 1),
                Err(SearchError::ModelMissing) => break,
                Err(error) => {
                    log::warn!(target: "search", "could not embed meeting {meeting}: {error}");
                    self.update(|progress| {
                        progress.done += 1;
                        progress.failed += 1;
                    });
                }
            }
        }
        self.finish();
    }

    fn update(&self, change: impl FnOnce(&mut BackfillProgress)) {
        let progress = {
            let mut progress = lock(&self.progress);
            change(&mut progress);
            *progress
        };
        self.events.backfill(&progress);
    }

    fn finish(&self) {
        self.update(|progress| progress.running = false);
        let progress = *lock(&self.progress);
        log::info!(
            target: "search",
            "backfill finished: {} of {} meetings, {} failed",
            progress.done,
            progress.total,
            progress.failed
        );
    }
}

fn lock(progress: &Mutex<BackfillProgress>) -> std::sync::MutexGuard<'_, BackfillProgress> {
    progress.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "backfill_tests.rs"]
mod tests;
