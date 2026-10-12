//! [`RetentionService`]: the retention sweep's rules, and what the meeting
//! page asks about a meeting.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{Clock, MeetingId};

use super::{
    MeetingRetention, MeetingTrash, RecordingFiles, RetentionError, RetentionPolicy,
    RetentionStore, SweepOutcome, cutoff,
};
use crate::meetings::MeetingWork;
use crate::meetings::trash::Housekeeping;

/// What retention works through.
pub struct RetentionDeps {
    /// The rows it reads and deletes.
    pub store: Arc<dyn RetentionStore>,
    /// The periods the user chose.
    pub policy: Arc<dyn RetentionPolicy>,
    /// Deletes recording files inside the app's recordings folder.
    pub files: Arc<dyn RecordingFiles>,
    /// ORG-07's trash, for "everything".
    pub trash: Arc<dyn MeetingTrash>,
    /// The pipeline, which re-indexes a meeting whose transcript went, so
    /// the in-memory semantic index forgets it too.
    pub work: Arc<dyn MeetingWork>,
    /// The time ages are counted from.
    pub clock: Arc<dyn Clock>,
}

/// Deletes what has outlived its retention period.
pub struct RetentionService {
    deps: RetentionDeps,
}

impl RetentionService {
    /// The service over its ports.
    pub fn new(deps: RetentionDeps) -> Self {
        Self { deps }
    }

    /// What retention removed from `meeting`.
    pub async fn status(&self, meeting: &MeetingId) -> Result<MeetingRetention, RetentionError> {
        self.deps
            .store
            .status(meeting)
            .await?
            .ok_or_else(|| RetentionError::MeetingNotFound(meeting.clone()))
    }

    /// Deletes everything that has expired now: whole meetings first (to
    /// the trash), then transcripts, then recordings.
    pub async fn sweep(&self) -> Result<SweepOutcome, RetentionError> {
        let periods = self.deps.policy.retention().await;
        let now = self.deps.clock.now();
        let mut outcome = SweepOutcome::default();
        if let Some(before) = cutoff(now, periods.everything) {
            outcome.trashed = self.trash_meetings(before).await?;
        }
        if let Some(before) = cutoff(now, periods.transcript) {
            outcome.transcripts = self.delete_transcripts(before, now).await?;
        }
        if let Some(before) = cutoff(now, periods.audio) {
            outcome.recordings = self.delete_recordings(before, now).await?;
        }
        Ok(outcome)
    }

    /// Moves meetings saved at or before `before` to the trash.
    async fn trash_meetings(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, RetentionError> {
        let due = self.deps.store.meetings_due(before).await?;
        if due.is_empty() {
            return Ok(due);
        }
        self.deps
            .trash
            .trash(&due)
            .await
            .map_err(RetentionError::Trash)
    }

    /// Deletes the transcripts of meetings saved at or before `before`, then has
    /// each re-indexed from what is left (its notes).
    async fn delete_transcripts(
        &self,
        before: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, RetentionError> {
        let due = self.deps.store.transcripts_due(before).await?;
        if due.is_empty() {
            return Ok(due);
        }
        let deleted = self.deps.store.delete_transcripts(&due, now).await?;
        for meeting in &deleted {
            self.deps.work.reindex(meeting).await;
        }
        Ok(deleted)
    }

    /// Forgets the recordings of meetings saved at or before `before`, then
    /// deletes their files. The rows go first, so a crash in between leaves
    /// orphaned files, never a meeting pointing at missing audio.
    async fn delete_recordings(
        &self,
        before: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, RetentionError> {
        let files = self.deps.store.recordings_due(before).await?;
        let due: Vec<MeetingId> = files
            .iter()
            .map(|stored| stored.meeting.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if due.is_empty() {
            return Ok(due);
        }
        let forgotten = self.deps.store.forget_recordings(&due, now).await?;
        for stored in files.iter().filter(|f| forgotten.contains(&f.meeting)) {
            if let Err(error) = self.deps.files.delete_recordings(stored).await {
                // The rows are gone; a leftover file is logged, never fatal.
                log::warn!(target: "retention", "could not delete recordings of {}: {error}", stored.owner);
            }
        }
        Ok(forgotten)
    }
}

#[async_trait]
impl Housekeeping for RetentionService {
    async fn tidy(&self) {
        match self.sweep().await {
            Ok(outcome) if outcome.is_empty() => {}
            Ok(outcome) => log::info!(
                target: "retention",
                "moved {} meetings to the trash, deleted {} transcripts and {} recordings",
                outcome.trashed.len(),
                outcome.transcripts.len(),
                outcome.recordings.len(),
            ),
            Err(error) => log::warn!(target: "retention", "the retention sweep failed: {error}"),
        }
    }
}
