//! [`ImportService`]: one file in, one `processing` meeting out.
//!
//! 1. Check the file: an accepted extension, present, at most
//!    [`MAX_IMPORT_BYTES`].
//! 2. Convert its audio into `<recordings>/<meeting>/` ([`MediaIngest`]),
//!    one import at a time so two long files never convert at once.
//! 3. Create the meeting (`import`), store its files and timing, and move
//!    it to `processing`, which starts the pipeline.
//!
//! The meeting is created only once its audio exists, so a failed
//! conversion leaves nothing behind; a failure after that removes the
//! folder again.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{Clock, MeetingEvent, MeetingId, MeetingSource, MeetingStatus, Seconds};
use tokio::sync::Mutex;

use super::{
    FileImporter, ImportCandidate, ImportError, ImportRequest, ImportedMeeting, IngestedMedia,
    MAX_IMPORT_BYTES, MediaFacts, MediaIngest, MediaKind, MediaPicker,
};
use crate::meetings::{
    MeetingAudio, MeetingError, MeetingLifecycleRepo, MeetingLifecycleService, MeetingTiming,
    MeetingTitle, NewMeeting,
};

/// The title used when the file's name gives none.
pub const FALLBACK_TITLE: &str = "Imported recording";

/// What the service works through.
#[derive(Clone)]
pub struct ImportDeps {
    /// Reads and converts files.
    pub ingest: Arc<dyn MediaIngest>,
    /// Creates the meeting and stores its files and timing.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// Moves the meeting to `processing` (the pipeline starts).
    pub lifecycle: Arc<MeetingLifecycleService>,
    /// The folder each meeting's recording folder goes in.
    pub recordings: PathBuf,
    /// IDs and timestamps.
    pub clock: Arc<dyn Clock>,
}

/// Imports recordings as meetings.
pub struct ImportService {
    deps: ImportDeps,
    /// Held while a file converts: imports run one at a time.
    turn: Mutex<()>,
}

impl ImportService {
    /// The service over its ports.
    pub fn new(deps: ImportDeps) -> Self {
        Self {
            deps,
            turn: Mutex::new(()),
        }
    }

    /// Checks that `path` can be imported, without importing it.
    pub async fn check(&self, path: &Path) -> Result<(MediaKind, MediaFacts), ImportError> {
        let kind = MediaKind::of(path).ok_or(ImportError::UnsupportedFile)?;
        let facts = self.deps.ingest.facts(path).await?;
        if facts.size_bytes > MAX_IMPORT_BYTES {
            return Err(ImportError::TooLarge);
        }
        Ok((kind, facts))
    }

    /// `path` as the import dialog shows it, once it passed [`Self::check`].
    pub async fn candidate(&self, path: &Path) -> Result<ImportCandidate, ImportError> {
        let (kind, _) = self.check(path).await?;
        Ok(ImportCandidate {
            path: path_text(path),
            file_name: path.file_name().map_or_else(
                || path_text(path),
                |name| name.to_string_lossy().into_owned(),
            ),
            kind,
        })
    }

    /// Asks `picker` for a file and checks it; `None` when cancelled.
    pub async fn pick(
        &self,
        picker: &dyn MediaPicker,
    ) -> Result<Option<ImportCandidate>, ImportError> {
        match picker.pick_media().await {
            Some(path) => self.candidate(&path).await.map(Some),
            None => Ok(None),
        }
    }

    /// Imports `request` and returns the new meeting, now `processing`.
    pub async fn import(&self, request: &ImportRequest) -> Result<ImportedMeeting, ImportError> {
        let (kind, facts) = self.check(&request.source).await?;
        let title = title_of(request)?;
        let _turn = self.turn.lock().await;
        let meeting = new_meeting_id(self.deps.clock.as_ref());
        let folder = self.deps.recordings.join(meeting.as_str());
        let media = self
            .deps
            .ingest
            .ingest(&request.source, kind, &folder)
            .await?;
        let draft = Draft {
            meeting: &meeting,
            title: &title,
            folder: &folder,
            facts,
            media: &media,
        };
        if let Err(error) = self.create(&draft).await {
            log::warn!(target: "import", "could not create imported meeting {meeting}: {error}");
            self.deps.ingest.discard(&folder).await;
            return Err(error);
        }
        log::info!(target: "import", "imported meeting {meeting} ({}s)", media.duration.as_secs());
        Ok(ImportedMeeting {
            meeting_id: meeting,
            title: title.as_str().to_owned(),
            duration: seconds(media.duration),
            has_video: media.video.is_some(),
        })
    }

    /// Creates the meeting row, its files and timing, then starts processing.
    async fn create(&self, draft: &Draft<'_>) -> Result<(), ImportError> {
        let now = self.deps.clock.now();
        let timing = timing(draft.facts.modified, draft.media.duration, now);
        self.deps
            .meetings
            .create(&NewMeeting {
                id: draft.meeting.clone(),
                title: draft.title.clone(),
                status: MeetingStatus::Recording,
                source: MeetingSource::Import,
                created_at: now,
                started_at: timing.started_at,
                folder_path: Some(path_text(draft.folder)),
            })
            .await?;
        self.deps
            .meetings
            .set_audio(draft.meeting, &audio_of(draft.media), now)
            .await?;
        self.deps
            .meetings
            .set_timing(draft.meeting, &timing, now)
            .await?;
        self.deps
            .lifecycle
            .transition(draft.meeting, MeetingEvent::StopRecording)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl FileImporter for ImportService {
    async fn import(&self, request: &ImportRequest) -> Result<ImportedMeeting, ImportError> {
        ImportService::import(self, request).await
    }
}

/// The parts of a meeting being created.
struct Draft<'a> {
    meeting: &'a MeetingId,
    title: &'a MeetingTitle,
    folder: &'a Path,
    facts: MediaFacts,
    media: &'a IngestedMedia,
}

/// A new meeting ID: `meeting-<UUIDv7>`, like a live recording's.
fn new_meeting_id(clock: &dyn Clock) -> MeetingId {
    MeetingId::new(format!(
        "meeting-{}",
        deino_domain::SegmentId::generate(clock)
    ))
}

/// The requested title, else the file's name, else [`FALLBACK_TITLE`].
fn title_of(request: &ImportRequest) -> Result<MeetingTitle, MeetingError> {
    let stem = request.source.file_stem().and_then(|stem| stem.to_str());
    let named = [request.title.as_deref(), stem]
        .into_iter()
        .flatten()
        .find_map(|candidate| MeetingTitle::parse(candidate).ok());
    named.map_or_else(|| MeetingTitle::parse(FALLBACK_TITLE), Ok)
}

/// When the recording ran: it ended when the file was last written (or now,
/// for a clock that disagrees) and lasted `duration`.
fn timing(
    modified: Option<DateTime<Utc>>,
    duration: std::time::Duration,
    now: DateTime<Utc>,
) -> MeetingTiming {
    let ended_at = modified.filter(|modified| *modified <= now).unwrap_or(now);
    let length = TimeDelta::from_std(duration).unwrap_or(TimeDelta::zero());
    MeetingTiming {
        started_at: Some(ended_at - length),
        ended_at: Some(ended_at),
        duration: Some(seconds(duration)),
    }
}

/// The meeting's files: the converted audio as its mixed track, and the
/// video when one was kept.
fn audio_of(media: &IngestedMedia) -> MeetingAudio {
    MeetingAudio {
        audio_path: Some(path_text(&media.audio)),
        audio_mic_path: None,
        audio_system_path: None,
        video_path: media.video.as_deref().map(path_text),
    }
}

/// A length in whole seconds.
fn seconds(duration: std::time::Duration) -> Seconds {
    Seconds(u32::try_from(duration.as_secs()).unwrap_or(u32::MAX))
}

/// A path as the meetings table stores it.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
