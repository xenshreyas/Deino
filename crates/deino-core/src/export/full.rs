//! The account export (EXP-02): every meeting as Markdown and JSON in one
//! zip, recordings optionally, plus CSV tables of meetings, action items
//! and people. It runs in the background with progress, from
//! Settings → Export.
//!
//! The job is one tracked task: it assembles meetings one at a time and
//! streams them through a small channel to a blocking thread that writes
//! the zip ([`archive`](super::archive)), so memory stays flat however many
//! meetings there are. One job runs at a time; it can be cancelled, and app
//! shutdown cancels and waits for it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use deino_domain::{Clock, MeetingId};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::archive::{Archive, ArchiveEntry};
use super::{
    ExportAssembler, ExportError, FullExportState, FullExportStatus, SaveDialog, SaveRequest,
};
use crate::meetings::MeetingLifecycleRepo;

/// How many assembled meetings may wait for the zip writer.
const QUEUE: usize = 4;

/// Hears about the account export's progress.
pub trait FullExportEvents: Send + Sync {
    /// The status changed.
    fn changed(&self, status: &FullExportStatus);
}

/// What the account export needs.
pub struct FullExportDeps {
    /// Reads meetings.
    pub assembler: Arc<ExportAssembler>,
    /// Where recordings are.
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// Where progress goes.
    pub events: Arc<dyn FullExportEvents>,
    /// Dates the suggested file name.
    pub clock: Arc<dyn Clock>,
}

/// Runs account exports in the background.
pub struct FullExportService {
    inner: Arc<Inner>,
    tracker: TaskTracker,
}

struct Inner {
    deps: FullExportDeps,
    status: Mutex<FullExportStatus>,
    cancel: Mutex<Option<CancellationToken>>,
}

/// One running export.
struct Job {
    target: PathBuf,
    include_audio: bool,
    cancel: CancellationToken,
}

/// What the writer thread receives.
enum Message {
    Entry(Box<ArchiveEntry>),
    Finish,
}

impl FullExportService {
    /// The service, idle.
    pub fn new(deps: FullExportDeps) -> Self {
        let inner = Inner {
            deps,
            status: Mutex::new(FullExportStatus::idle()),
            cancel: Mutex::new(None),
        };
        Self {
            inner: Arc::new(inner),
            tracker: TaskTracker::new(),
        }
    }

    /// The latest status, for a page opened mid-export.
    pub fn status(&self) -> FullExportStatus {
        self.inner
            .status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Asks where to save the archive and starts writing it. `false` when
    /// the user cancelled the dialog; fails with `Busy` while one runs.
    pub async fn start(
        &self,
        dialog: &dyn SaveDialog,
        include_audio: bool,
    ) -> Result<bool, ExportError> {
        if self.status().state == FullExportState::Running {
            return Err(ExportError::Busy);
        }
        let date = self.inner.deps.clock.now().format("%Y-%m-%d");
        let request = SaveRequest {
            file_name: format!("Deino export {date}.zip"),
            kind: "Zip archive",
            extension: "zip",
        };
        let Some(target) = dialog.save_path(request).await else {
            return Ok(false);
        };
        let cancel = CancellationToken::new();
        *self
            .inner
            .cancel
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(cancel.clone());
        self.inner.publish(FullExportStatus::running(0, 0));
        let job = Job {
            target,
            include_audio,
            cancel,
        };
        let inner = Arc::clone(&self.inner);
        self.tracker.spawn(async move { inner.run(job).await });
        Ok(true)
    }

    /// Stops the running export, if any; its partial archive is deleted.
    pub fn cancel(&self) {
        if let Some(cancel) = self
            .inner
            .cancel
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            cancel.cancel();
        }
    }

    /// Cancels the running export and waits for it to stop (app exit).
    pub async fn shutdown(&self) {
        self.cancel();
        self.until_idle().await;
    }

    /// Waits for the running export to end; no export can start after.
    pub async fn until_idle(&self) {
        self.tracker.close();
        self.tracker.wait().await;
    }
}

impl Inner {
    fn publish(&self, status: FullExportStatus) {
        self.deps.events.changed(&status);
        *self.status.lock().unwrap_or_else(PoisonError::into_inner) = status;
    }

    async fn run(&self, job: Job) {
        let file_name = job
            .target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let status = match self.write(&job).await {
            Ok(total) => FullExportStatus::finished(total, file_name),
            Err(ExportError::Cancelled) => {
                FullExportStatus::ended(FullExportState::Cancelled, None)
            }
            Err(error) => {
                log::warn!(target: "export", "the account export failed: {error}");
                FullExportStatus::ended(FullExportState::Failed, Some(error.to_string()))
            }
        };
        self.publish(status);
    }

    /// Streams every meeting to the writer thread; returns how many.
    async fn write(&self, job: &Job) -> Result<u32, ExportError> {
        let ids = self.deps.assembler.meeting_ids().await?;
        let total = u32::try_from(ids.len()).unwrap_or(u32::MAX);
        let (sender, receiver) = mpsc::channel(QUEUE);
        let target = job.target.clone();
        let writer = tokio::task::spawn_blocking(move || write_archive(&target, receiver));
        let produced = self.produce(job, &ids, &sender).await;
        let finished = match produced {
            Ok(()) => sender.send(Message::Finish).await.is_ok(),
            Err(_) => false,
        };
        drop(sender);
        let written = writer.await.map_err(|_| ExportError::Cancelled)?;
        produced?;
        written?;
        if finished {
            Ok(total)
        } else {
            Err(ExportError::Cancelled)
        }
    }

    async fn produce(
        &self,
        job: &Job,
        ids: &[MeetingId],
        sender: &mpsc::Sender<Message>,
    ) -> Result<(), ExportError> {
        let total = u32::try_from(ids.len()).unwrap_or(u32::MAX);
        for (done, id) in (1..).zip(ids) {
            if job.cancel.is_cancelled() {
                return Err(ExportError::Cancelled);
            }
            let entry = ArchiveEntry {
                export: self.deps.assembler.assemble(id).await?,
                audio: self.audio(job, id).await?,
            };
            if sender.send(Message::Entry(Box::new(entry))).await.is_err() {
                // The writer stopped; its own error explains why.
                return Ok(());
            }
            self.publish(FullExportStatus::running(done, total));
        }
        Ok(())
    }

    async fn audio(&self, job: &Job, id: &MeetingId) -> Result<Option<PathBuf>, ExportError> {
        if !job.include_audio {
            return Ok(None);
        }
        let audio = self
            .deps
            .lifecycle
            .state(id)
            .await?
            .map(|state| state.audio);
        let Some(path) = audio.and_then(|audio| audio.audio_path.or(audio.audio_mic_path)) else {
            return Ok(None);
        };
        let exists = tokio::fs::metadata(&path)
            .await
            .is_ok_and(|metadata| metadata.is_file());
        Ok(exists.then(|| PathBuf::from(path)))
    }
}

/// The writer thread: adds entries until `Finish`; a closed channel without
/// it (a cancel or a failure upstream) discards the archive.
fn write_archive(
    target: &std::path::Path,
    mut receiver: mpsc::Receiver<Message>,
) -> Result<(), ExportError> {
    let mut archive = Archive::create(target).map_err(ExportError::Write)?;
    loop {
        match receiver.blocking_recv() {
            Some(Message::Entry(entry)) => {
                if let Err(error) = archive.add(*entry) {
                    archive.discard();
                    return Err(error);
                }
            }
            Some(Message::Finish) => return archive.finish(),
            None => {
                archive.discard();
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
#[path = "full_tests.rs"]
mod tests;
