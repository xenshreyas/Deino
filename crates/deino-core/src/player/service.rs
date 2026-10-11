//! [`PlayerService`]: which of a meeting's recording files the player can
//! play, and downloading one through the save dialog.

use std::path::Path;
use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use super::{DownloadOutcome, MediaFile, MediaFiles, MediaSource, MeetingMedia};
use crate::export::{SaveDialog, SaveRequest};
use crate::meetings::{MeetingAudio, MeetingLifecycleRepo, MeetingRepo};

/// A player failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlayerError {
    /// The database failed; the source carries the detail.
    #[error("could not read the meeting's recording")]
    Storage(#[from] DbError),
    /// The meeting has no such recording on disk.
    #[error("this recording is not on this Mac")]
    MediaNotFound,
    /// Writing the copy failed; the source carries the detail.
    #[error("could not save the recording there")]
    Copy(#[source] std::io::Error),
}

impl ErrorClass for PlayerError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) | Self::Copy(_) => ErrorCode::Internal,
            Self::MediaNotFound => ErrorCode::NotFound,
        }
    }
}

/// What the player service works with.
pub struct PlayerDeps {
    /// The recording paths (`meetings.audio_*`, `video_path`).
    pub lifecycle: Arc<dyn MeetingLifecycleRepo>,
    /// The meeting's title, for a download's file name.
    pub meetings: Arc<dyn MeetingRepo>,
    /// The files on disk.
    pub files: Arc<dyn MediaFiles>,
    /// Asks where to save a download.
    pub dialog: Arc<dyn SaveDialog>,
}

/// Serves a meeting's recording to the player.
pub struct PlayerService {
    deps: PlayerDeps,
}

impl PlayerService {
    /// The service over its ports.
    pub fn new(deps: PlayerDeps) -> Self {
        Self { deps }
    }

    /// The meeting's playable files: those its row names that exist inside
    /// the recordings folder. An unknown meeting has none.
    pub async fn media(&self, meeting: &MeetingId) -> Result<MeetingMedia, PlayerError> {
        let Some(state) = self.deps.lifecycle.state(meeting).await? else {
            return Ok(MeetingMedia::default());
        };
        let mut files = Vec::new();
        for source in MediaSource::ALL {
            let Some(path) = path_of(&state.audio, source) else {
                continue;
            };
            if self.deps.files.playable(Path::new(path)).await {
                files.push(MediaFile {
                    source,
                    path: path.to_owned(),
                });
            }
        }
        Ok(MeetingMedia { files })
    }

    /// Asks where to save the meeting's `source` file and copies it there.
    pub async fn download(
        &self,
        meeting: &MeetingId,
        source: MediaSource,
    ) -> Result<DownloadOutcome, PlayerError> {
        let media = self.media(meeting).await?;
        let file = media.file(source).ok_or(PlayerError::MediaNotFound)?;
        let from = Path::new(&file.path);
        let request = self.save_request(meeting, source, from).await?;
        let Some(to) = self.deps.dialog.save_path(request).await else {
            return Ok(DownloadOutcome::Cancelled);
        };
        self.deps
            .files
            .copy(from, &to)
            .await
            .map_err(PlayerError::Copy)?;
        Ok(DownloadOutcome::Saved)
    }

    /// "Standup (Me).m4a": the title, the source and the file's own type.
    async fn save_request(
        &self,
        meeting: &MeetingId,
        source: MediaSource,
        file: &Path,
    ) -> Result<SaveRequest, PlayerError> {
        let title = self
            .deps
            .meetings
            .find(meeting)
            .await?
            .map(|found| found.title);
        let title = deino_export::naming::sanitize(title.as_deref().unwrap_or("Meeting"));
        let (kind, extension) = file_type(file);
        Ok(SaveRequest {
            file_name: format!("{title} ({}).{extension}", source.label()),
            kind,
            extension,
        })
    }
}

/// The path the meeting's row holds for `source`.
fn path_of(audio: &MeetingAudio, source: MediaSource) -> Option<&str> {
    let path = match source {
        MediaSource::Mixed => &audio.audio_path,
        MediaSource::Microphone => &audio.audio_mic_path,
        MediaSource::System => &audio.audio_system_path,
        MediaSource::Video => &audio.video_path,
    };
    path.as_deref().filter(|path| !path.is_empty())
}

/// The save dialog's filter for a recording file, by its extension.
fn file_type(file: &Path) -> (&'static str, &'static str) {
    match file.extension().and_then(|extension| extension.to_str()) {
        Some("m4a") => ("MPEG-4 audio", "m4a"),
        Some("mov") => ("QuickTime movie", "mov"),
        Some("wav") => ("WAV audio", "wav"),
        _ => ("MPEG-4 media", "mp4"),
    }
}
