//! [`ImportJobs`]: the one background import at a time behind Settings →
//! Import. The app claims the slot with [`ImportJobs::begin`], runs
//! [`ImportJobs::run`] on its runtime, and the UI reads
//! [`ImportJobs::status`] and listens to the progress events.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_domain::ImportSource;
use tokio_util::sync::CancellationToken;

use super::{ImportEvents, ImportPreview, ImportProgress, ImportRunner, Importer, ImporterError};

/// The running job, if any, and the last progress.
struct Slot {
    progress: ImportProgress,
    cancel: Option<CancellationToken>,
}

/// Remembers the latest progress and passes it on.
struct Relay<'a> {
    jobs: &'a ImportJobs,
}

impl ImportEvents for Relay<'_> {
    fn progress(&self, progress: &ImportProgress) {
        self.jobs.slot().progress = progress.clone();
        self.jobs.events.progress(progress);
    }
}

/// Runs imports one at a time, cancellably.
pub struct ImportJobs {
    runner: ImportRunner,
    events: Arc<dyn ImportEvents>,
    slot: Mutex<Slot>,
}

impl ImportJobs {
    /// Jobs over `runner`, announcing progress to `events`.
    pub fn new(runner: ImportRunner, events: Arc<dyn ImportEvents>) -> Self {
        Self {
            runner,
            events,
            slot: Mutex::new(Slot {
                progress: ImportProgress::idle(),
                cancel: None,
            }),
        }
    }

    /// Counts what importing `source` would add.
    pub async fn preview(&self, source: &dyn Importer) -> Result<ImportPreview, ImporterError> {
        self.runner.preview(source).await
    }

    /// Claims the job slot for `source`; fails with `Busy` while another
    /// import runs. The returned token cancels the job.
    pub fn begin(&self, source: ImportSource) -> Result<CancellationToken, ImporterError> {
        let mut slot = self.slot();
        if slot.cancel.is_some() {
            return Err(ImporterError::Busy);
        }
        let token = CancellationToken::new();
        slot.cancel = Some(token.clone());
        slot.progress = ImportProgress::listing(source);
        Ok(token)
    }

    /// Runs the job claimed with [`ImportJobs::begin`] to its end, then
    /// frees the slot.
    pub async fn run(&self, source: &dyn Importer, cancel: CancellationToken) -> ImportProgress {
        let progress = self.runner.run(source, &cancel, &Relay { jobs: self }).await;
        self.slot().cancel = None;
        progress
    }

    /// Asks the running job to stop after the meeting it is on; false when
    /// nothing runs.
    pub fn cancel(&self) -> bool {
        match &self.slot().cancel {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// The latest progress.
    pub fn status(&self) -> ImportProgress {
        self.slot().progress.clone()
    }

    fn slot(&self) -> MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
