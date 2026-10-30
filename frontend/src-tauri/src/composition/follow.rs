//! Wiring for following a recording while it is made
//! (`deino_recording::FollowAlong`): passes over the tracks as recorded so
//! far, through the `final_transcription` stage's transcriber and the
//! `diarize` stage's engines, so that when the meeting ends the stages
//! redo only its last moments. Runs while the live transcript is off (the
//! engine is otherwise busy with the live lanes), for the app's lifetime.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::settings::SettingsService;
use deino_domain::Clock;
use deino_recording::{FollowAlong, FollowDeps, FollowWanted, Recorder, RecordingService};
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;

use super::attach::Phase;
use super::diarize::SpeakerEngines;
use super::transcript::FollowTranscriber;
use crate::paths::AppPaths;

super::attach::attach!(
    "follow-along",
    25,
    Phase::Background(|cx| {
        attach_follow(cx.app, cx.clock);
    })
);

/// Starts following recordings; needs the recorder (`recording`, 20) and
/// the pipeline's stages to be managed first.
fn attach_follow(app: &AppHandle, clock: &Arc<dyn Clock>) {
    let parts = (
        app.try_state::<Arc<RecordingService>>(),
        app.try_state::<FollowTranscriber>(),
        app.try_state::<SpeakerEngines>(),
        app.try_state::<Arc<SettingsService>>(),
        AppPaths::resolve(app).ok(),
    );
    let (Some(recorder), Some(follow), Some(speakers), Some(settings), Some(paths)) = parts else {
        return log::warn!(target: "pipeline", "the recorder or the stages are not ready; recordings are transcribed only after the meeting");
    };
    let memory = follow.memory.clone();
    let (scans, lines) = (Arc::clone(&speakers.scans), Arc::clone(&speakers.lines));
    let forget = move || {
        memory.forget();
        scans.clear();
        lines.clear();
    };
    let follower = FollowAlong::new(FollowDeps {
        recorder: Arc::clone(&*recorder) as Arc<dyn Recorder>,
        recordings: paths.recordings(),
        transcriber: follow.transcriber.clone(),
        memory: follow.memory.clone(),
        aligner: Arc::clone(&speakers.aligner),
        diarizer: Arc::clone(&speakers.diarizer),
        wanted: Arc::new(LiveTranscriptOff(Arc::clone(&settings))),
        forget: Arc::new(forget),
        clock: Arc::clone(clock),
    });
    // Owned for the app's lifetime.
    tauri::async_runtime::spawn(follower.run(CancellationToken::new()));
}

/// Follow when Settings → Transcription → live transcription is off.
struct LiveTranscriptOff(Arc<SettingsService>);

#[async_trait]
impl FollowWanted for LiveTranscriptOff {
    async fn wanted(&self) -> bool {
        !self.0.get().await.transcription.live_transcription
    }
}
