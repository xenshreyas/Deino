//! Resume on a finished meeting (CAP-07, Granola §2.5): recording starts
//! again on the meeting's own row, folder and speakers, and the new audio
//! and segments are appended after what was recorded. The meeting moves
//! back to `recording`; stopping moves it to `processing` again, so the
//! pipeline rebuilds the audio files, speakers and notes over all of it.

use chrono::{DateTime, TimeDelta, Utc};
use deino_audio::manifest::ManifestStore;
use deino_audio::{Frames, recording_dir};
use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};

use super::{Launched, RecordingService, blocking, meeting};
use crate::error::RecordingError;
use crate::events::RecordingSnapshot;
use crate::session::{Plan, Take};

/// A finished meeting, ready to take more audio.
struct Reopened {
    plan: Plan,
    title: deino_core::meetings::MeetingTitle,
    began_at: DateTime<Utc>,
    event: Option<deino_domain::CalendarEventId>,
}

impl RecordingService {
    /// Resumes recording on the finished (`ready` or `failed`) meeting
    /// `id`; returns its ID once audio is being captured. Fails with
    /// `conflict` while another recording runs or the meeting is still
    /// being recorded or processed.
    pub async fn resume_meeting(&self, id: &MeetingId) -> Result<MeetingId, RecordingError> {
        self.claim()?;
        let resumed = self.reopen(id).await;
        self.settle(resumed)
    }

    async fn reopen(&self, id: &MeetingId) -> Result<Launched, RecordingError> {
        let Reopened {
            plan,
            title,
            began_at,
            event,
        } = self.reopened(id).await?;
        let recorded = plan.take.start().as_duration();
        let starting = RecordingSnapshot {
            calendar_event_id: event,
            ..Self::starting(&plan, &title)
        };
        self.state.set(starting.clone());
        let prepared = self.prepare(plan).await?;
        let moved = self
            .deps
            .lifecycle
            .transition(id, MeetingEvent::ResumeRecording)
            .await;
        if let Err(error) = moved {
            blocking(move || prepared.abort()).await.ok();
            return Err(error.into());
        }
        let now = self.deps.recorder.wall_clock.now();
        let clock_from = now - TimeDelta::from_std(recorded).unwrap_or(TimeDelta::zero());
        self.go(prepared, starting, (began_at, clock_from)).await
    }

    /// The plan for more audio on `id`, after checking it can take it.
    async fn reopened(&self, id: &MeetingId) -> Result<Reopened, RecordingError> {
        let state = self
            .deps
            .meetings
            .state(id)
            .await?
            .ok_or_else(|| RecordingError::NotFound(id.clone()))?;
        if !matches!(state.status, MeetingStatus::Ready | MeetingStatus::Failed) {
            return Err(RecordingError::NotResumable(state.status));
        }
        let found = self.deps.catalog.find(id).await?;
        let title = meeting::title(found.as_ref().map(|m| m.title.as_str()))?;
        let dir = recording_dir(&self.deps.recordings, id);
        let start = recorded_length(dir.clone()).await?;
        let speakers = meeting::resume_speakers(&self.deps, id).await?;
        let now = self.deps.recorder.wall_clock.now();
        Ok(Reopened {
            plan: Plan {
                meeting: id.clone(),
                speakers,
                dir,
                preferences: self.deps.preferences.current().await,
                runtime: tokio::runtime::Handle::current(),
                take: Take::Resume { start },
            },
            title,
            began_at: state.started_at.unwrap_or(now),
            event: state.calendar_event_id,
        })
    }
}

/// How long the recording in `dir` is: where the resumed audio starts.
async fn recorded_length(dir: std::path::PathBuf) -> Result<Frames, RecordingError> {
    let manifest = blocking(move || ManifestStore::new(&dir).read()).await??;
    Ok(manifest.length())
}
