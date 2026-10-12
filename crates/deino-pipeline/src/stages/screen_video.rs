//! `screen_video` (order 15): the screen recording, with sound, on the
//! meeting (CAP-09).
//!
//! During a recording the screen capture writes a silent
//! `<recordings>/<meeting>/video.part.mp4` and renames it `video.mp4` when
//! the recording stops (`deino-screen`). Once `finalize_audio` (order 10)
//! has written the mixed `audio.mp4`, this stage adds that audio track to
//! the video without re-encoding (`deino_audio::mux`), so a clip cut from
//! it (TRN-04) or a downloaded video has sound, and stores `video_path`,
//! which the player plays (TRN-03). It is the only writer of a live
//! recording's `video_path`, after `finalize_audio`, so the two never race.
//!
//! - No video: skipped.
//! - Only `video.part.mp4`: the capture is still closing it, so the stage
//!   retries; once the file has not changed for [`STALE_AFTER`] (a crash
//!   left it), it is adopted as is: its fragments play up to the crash.
//! - A video that already has sound (run before, or an imported one) is
//!   kept; only the path is stored.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_audio::mux::{MuxError, MuxRequest, add_audio, has_audio};
use deino_audio::recording_dir;
use deino_core::meetings::MeetingLifecycleRepo;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, StageId};
use deino_screen::{VIDEO_FILE, VIDEO_PART};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "screen_video";
/// Where it runs: right after `finalize_audio`, before anything reads files.
const ORDER: u16 = 15;
/// Shown for meetings without a screen recording.
const NO_VIDEO: &str = "The meeting has no screen recording.";
/// A part file unchanged this long was left by a crash, not being closed.
pub const STALE_AFTER: TimeDelta = TimeDelta::seconds(20);
/// The mixed audio `finalize_audio` writes.
const MIXED_AUDIO: &str = "audio.mp4";
/// The muxed file before it replaces the video.
const MUXING: &str = "video.muxing.mp4";

/// What the files in the recording folder call for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoFiles {
    /// No screen recording.
    None,
    /// The capture is still closing `video.part.mp4`.
    Closing,
    /// A crash left `video.part.mp4`; adopt it.
    Abandoned,
    /// `video.mp4` is ready.
    Ready,
}

impl VideoFiles {
    /// What `video.mp4` existing and `video.part.mp4` last changed `part_age`
    /// ago (`None`: no part file) call for.
    pub fn judge(video: bool, part_age: Option<TimeDelta>) -> Self {
        match (video, part_age) {
            (true, _) => Self::Ready,
            (false, None) => Self::None,
            (false, Some(age)) if age < STALE_AFTER => Self::Closing,
            (false, Some(_)) => Self::Abandoned,
        }
    }
}

/// Gives the screen recording its sound and stores its path.
pub struct ScreenVideoStage {
    meetings: Arc<dyn MeetingLifecycleRepo>,
    recordings: PathBuf,
}

impl ScreenVideoStage {
    /// The stage over the meetings port and the recordings folder.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            meetings: Arc::clone(&ports.meetings),
            recordings: ports.recordings.clone(),
        }
    }

    /// Stores `video` as the meeting's `video_path`, keeping its audio paths.
    async fn store(
        &self,
        meeting: &MeetingId,
        video: &Path,
        ctx: &StageCtx,
    ) -> Result<(), StageError> {
        let state = self
            .meetings
            .state(meeting)
            .await
            .map_err(|e| StageError::retryable_from(&e))?;
        let mut audio = state.map(|state| state.audio).unwrap_or_default();
        audio.video_path = Some(video.to_string_lossy().into_owned());
        self.meetings
            .set_audio(meeting, &audio, ctx.clock.now())
            .await
            .map_err(|e| StageError::retryable_from(&e))?;
        Ok(())
    }
}

crate::registry::register_stage!(ScreenVideoStage);

#[async_trait]
impl PipelineStage for ScreenVideoStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let dir = recording_dir(&self.recordings, &meeting);
        let video = dir.join(VIDEO_FILE);
        match VideoFiles::judge(
            video.exists(),
            part_age(&dir.join(VIDEO_PART), ctx.clock.now()),
        ) {
            VideoFiles::None => return Ok(StageOutcome::Skipped(NO_VIDEO)),
            VideoFiles::Closing => {
                return Err(StageError::retryable(
                    ErrorCode::Unavailable,
                    "the screen recording is still being closed",
                ));
            }
            VideoFiles::Abandoned => adopt(&dir)?,
            VideoFiles::Ready => {}
        }
        if ctx.cancel.is_cancelled() {
            return Err(StageError::Cancelled);
        }
        let folder = dir.clone();
        tokio::task::spawn_blocking(move || give_sound(&folder))
            .await
            .map_err(|_| {
                StageError::retryable(ErrorCode::Internal, "adding sound stopped unexpectedly")
            })??;
        self.store(&meeting, &video, ctx).await?;
        Ok(StageOutcome::Done)
    }
}

/// How long ago the file at `path` last changed; `None` when it is missing.
fn part_age(path: &Path, now: DateTime<Utc>) -> Option<TimeDelta> {
    let modified: SystemTime = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()?;
    Some(now.signed_duration_since(DateTime::<Utc>::from(modified)))
}

/// Renames a crash's `video.part.mp4` to `video.mp4`.
fn adopt(dir: &Path) -> Result<(), StageError> {
    std::fs::rename(dir.join(VIDEO_PART), dir.join(VIDEO_FILE))
        .map_err(|error| StageError::retryable(ErrorCode::Internal, error.to_string()))
}

/// Adds `audio.mp4` to `video.mp4` when it has no sound yet. Blocking.
fn give_sound(dir: &Path) -> Result<(), StageError> {
    let (video, audio) = (dir.join(VIDEO_FILE), dir.join(MIXED_AUDIO));
    if !audio.exists() {
        return Ok(());
    }
    match has_audio(&video) {
        Ok(false) => {}
        // Already has sound, or no AVFoundation here: the video stays as is.
        Ok(true) | Err(MuxError::Unsupported) => return Ok(()),
        Err(error) => return Err(failed(&error)),
    }
    let muxing = dir.join(MUXING);
    add_audio(&MuxRequest {
        video: &video,
        audio: &audio,
        destination: &muxing,
    })
    .map_err(|error| failed(&error))?;
    std::fs::rename(&muxing, &video)
        .map_err(|error| StageError::retryable(ErrorCode::Internal, error.to_string()))
}

/// A mux failure as a stage error: retried unless it can never work.
fn failed(error: &MuxError) -> StageError {
    match error.code() {
        ErrorCode::Internal => StageError::retryable(error.code(), error.to_string()),
        code => StageError::permanent(code, error.to_string()),
    }
}

#[cfg(test)]
#[path = "screen_video_tests.rs"]
mod tests;
