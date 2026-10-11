//! Launch recovery: meetings the app never finished recording.
//!
//! A crash, `kill -9` or power loss leaves the meeting in `recording` and
//! its tracks open. At launch, for each such meeting, the recording folder
//! is repaired (`deino_audio::recover_recording`: torn tails cut, `.part`
//! files renamed, the manifest marked `recovered`), its timing is set from
//! the durable audio, and it moves to `processing`, so the pipeline
//! finalizes the audio and the summary runs as after a normal stop. Its
//! transcript is whatever the router had saved (drafts included), at most
//! a few seconds short. A meeting without a recording folder (it never got
//! audio) moves to `failed`.

use chrono::TimeDelta;
use deino_audio::{Frames, RecordingManifest, Recovery, Track, recover_recording};
use deino_core::meetings::{MeetingLifecycleRepo, MeetingTiming};
use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};

use crate::deps::RecordingDeps;
use crate::error::RecordingError;
use crate::service::meeting::seconds;

/// What launch recovery did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Meetings repaired and moved to `processing`.
    pub recovered: Vec<MeetingId>,
    /// Meetings without audio, moved to `failed`.
    pub failed: Vec<MeetingId>,
}

/// Recovers every meeting left in `recording`. Call it once at launch,
/// before any recording can start.
pub async fn recover_interrupted(deps: &RecordingDeps) -> Result<RecoveryReport, RecordingError> {
    let mut report = RecoveryReport::default();
    for meeting in deps
        .meetings
        .list_by_status(MeetingStatus::Recording)
        .await?
    {
        match recover_one(deps, &meeting.id).await {
            Ok(true) => report.recovered.push(meeting.id),
            Ok(false) => report.failed.push(meeting.id),
            Err(error) => {
                log::error!(target: "recording", "could not recover meeting {}: {error}", meeting.id);
            }
        }
    }
    Ok(report)
}

/// Repairs one meeting; `false` when it had no recording to repair.
async fn recover_one(deps: &RecordingDeps, meeting: &MeetingId) -> Result<bool, RecordingError> {
    let dir = deino_audio::recording_dir(&deps.recordings, meeting);
    if !dir.exists() {
        deps.lifecycle
            .transition(meeting, MeetingEvent::Fail)
            .await?;
        return Ok(false);
    }
    let recovery = tokio::task::spawn_blocking(move || recover_recording(&dir))
        .await
        .map_err(|error| RecordingError::Thread(std::io::Error::other(error)))??;
    let manifest = match recovery {
        Recovery::NotNeeded(manifest) | Recovery::Recovered { manifest, .. } => manifest,
    };
    set_timing(deps.meetings.as_ref(), meeting, &manifest).await?;
    deps.lifecycle
        .transition(meeting, MeetingEvent::StopRecording)
        .await?;
    log::info!(target: "recording", "recovered meeting {meeting} ({} s of audio)", seconds(length(&manifest)).0);
    Ok(true)
}

/// Timing from the manifest: its start, and the durable audio's length.
async fn set_timing(
    meetings: &dyn MeetingLifecycleRepo,
    meeting: &MeetingId,
    manifest: &RecordingManifest,
) -> Result<(), RecordingError> {
    let length = length(manifest);
    let duration = TimeDelta::from_std(length.as_duration()).unwrap_or_default();
    let ended_at = manifest.started_at + duration;
    let timing = MeetingTiming {
        started_at: Some(manifest.started_at),
        ended_at: Some(ended_at),
        duration: Some(seconds(length)),
    };
    meetings.set_timing(meeting, &timing, ended_at).await?;
    Ok(())
}

/// The longer track's durable length.
fn length(manifest: &RecordingManifest) -> Frames {
    Track::ALL
        .iter()
        .filter_map(|track| manifest.track(*track))
        .map(deino_audio::manifest::TrackManifest::durable_end)
        .max()
        .unwrap_or_default()
}
