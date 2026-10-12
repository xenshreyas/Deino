//! `finalize_audio` (order 10): the recording's three playable files.
//!
//! A live recording leaves fragmented per-track segments in
//! `<recordings>/<meeting id>/` (FND-05, ADR 0014). This stage repairs them
//! if the app died mid-meeting (`recover_recording` is a no-op for a clean
//! stop), writes the flat `mic.m4a`, `system.m4a` and the mixed `audio.mp4`
//! (`finalize_recording`, idempotent, temporary names then rename), and
//! stores their paths on the meeting, where `diarize` (order 30) and the
//! player find them. Meetings without a live recording (imports, legacy
//! meetings) are skipped.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_audio::{
    FinalizeError, FinalizedAudio, finalize_recording, recording_dir, recover_recording,
};
use deino_core::meetings::{MeetingAudio, MeetingLifecycleRepo};
use deino_domain::{ErrorClass, ErrorCode, MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "finalize_audio";
/// Where it runs: first (PARITY_PLAN §4.2).
const ORDER: u16 = 10;
/// Shown for meetings recorded before FND-05b, or imported.
const NO_RECORDING: &str = "The meeting has no live recording to finalize.";
/// Shown on platforms without native audio encoding.
const UNSUPPORTED: &str = "Finalizing audio needs macOS.";
/// The manifest that marks a live recording's folder.
const MANIFEST: &str = "recording.json";

/// Writes a live recording's playable files and records their paths.
pub struct FinalizeAudioStage {
    meetings: Arc<dyn MeetingLifecycleRepo>,
    recordings: PathBuf,
}

impl FinalizeAudioStage {
    /// The stage over the meetings port and the recordings folder.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            meetings: Arc::clone(&ports.meetings),
            recordings: ports.recordings.clone(),
        }
    }

    /// Stores the finalized files' paths, keeping any video path.
    async fn store(
        &self,
        meeting: &MeetingId,
        files: &FinalizedAudio,
        ctx: &StageCtx,
    ) -> Result<(), StageError> {
        let state = self
            .meetings
            .state(meeting)
            .await
            .map_err(|e| StageError::retryable_from(&e))?;
        let audio = MeetingAudio {
            audio_path: Some(path_text(&files.mixed)),
            audio_mic_path: Some(path_text(&files.microphone)),
            audio_system_path: Some(path_text(&files.system)),
            video_path: state.and_then(|state| state.audio.video_path),
        };
        self.meetings
            .set_audio(meeting, &audio, ctx.clock.now())
            .await
            .map_err(|e| StageError::retryable_from(&e))?;
        Ok(())
    }
}

crate::registry::register_stage!(FinalizeAudioStage);

#[async_trait]
impl PipelineStage for FinalizeAudioStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let dir = recording_dir(&self.recordings, &meeting);
        if !dir.join(MANIFEST).exists() {
            return Ok(StageOutcome::Skipped(NO_RECORDING));
        }
        if ctx.cancel.is_cancelled() {
            return Err(StageError::Cancelled);
        }
        let files = tokio::task::spawn_blocking(move || finalize(&dir))
            .await
            .map_err(|_| {
                StageError::retryable(
                    ErrorCode::Internal,
                    "finalizing the audio stopped unexpectedly",
                )
            })?;
        match files {
            Ok(files) => {
                self.store(&meeting, &files, ctx).await?;
                Ok(StageOutcome::Done)
            }
            Err(Failure::Unsupported) => Ok(StageOutcome::Skipped(UNSUPPORTED)),
            Err(Failure::Failed(error)) => Err(error),
        }
    }
}

/// Why finalize did not produce files.
enum Failure {
    Unsupported,
    Failed(StageError),
}

/// Repairs (if needed) and finalizes the recording in `dir`. Blocking.
fn finalize(dir: &Path) -> Result<FinalizedAudio, Failure> {
    recover_recording(dir)
        .map_err(|error| Failure::Failed(StageError::permanent(error.code(), error.to_string())))?;
    finalize_recording(dir).map_err(|error| match error {
        FinalizeError::Unsupported => Failure::Unsupported,
        error => Failure::Failed(StageError::retryable(
            ErrorCode::Internal,
            error.to_string(),
        )),
    })
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "finalize_audio_tests.rs"]
mod tests;
