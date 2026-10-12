//! `screen_video`: skips meetings without a screen recording, waits while
//! the capture closes it, adopts a crash's part file, and stores the path.

use std::sync::Arc;

use deino_core::meetings::{FakeMeetingRepo, MeetingAudio, MeetingTitle, NewMeeting};
use deino_domain::{FakeClock, MeetingSource, MeetingStatus};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;

fn ctx(now: DateTime<Utc>) -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(now)),
        events: Arc::new(RecordedProgress::default()),
    }
}

async fn stage(recordings: &Path) -> (ScreenVideoStage, Arc<FakeMeetingRepo>) {
    let meetings = Arc::new(FakeMeetingRepo::default());
    let now = at("2026-10-12T09:00:00Z");
    meetings
        .create(&NewMeeting {
            id: MeetingId::new("meeting-a"),
            title: MeetingTitle::parse("Design review").unwrap(),
            status: MeetingStatus::Processing,
            source: MeetingSource::Live,
            created_at: now,
            started_at: Some(now),
            folder_path: None,
        })
        .await
        .unwrap();
    let mixed = MeetingAudio {
        audio_path: Some("/recordings/meeting-a/audio.mp4".into()),
        ..MeetingAudio::default()
    };
    meetings
        .set_audio(&MeetingId::new("meeting-a"), &mixed, now)
        .await
        .unwrap();
    let ports = StagePorts {
        meetings: meetings.clone(),
        recordings: recordings.to_path_buf(),
        ..StagePorts::fakes()
    };
    (ScreenVideoStage::new(&ports), meetings)
}

/// When the file at `path` last changed.
fn modified(path: &Path) -> DateTime<Utc> {
    DateTime::<Utc>::from(std::fs::metadata(path).unwrap().modified().unwrap())
}

/// The meeting's folder under `recordings`, created.
fn folder(recordings: &Path) -> PathBuf {
    let dir = recordings.join("meeting-a");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_files_decide_what_happens() {
    let fresh = Some(TimeDelta::seconds(2));
    let old = Some(TimeDelta::seconds(25));
    assert_eq!(VideoFiles::judge(false, None), VideoFiles::None);
    assert_eq!(VideoFiles::judge(false, fresh), VideoFiles::Closing);
    assert_eq!(VideoFiles::judge(false, old), VideoFiles::Abandoned);
    assert_eq!(VideoFiles::judge(true, fresh), VideoFiles::Ready);
}

#[tokio::test]
async fn a_meeting_without_a_screen_recording_is_skipped() {
    let recordings = tempfile::tempdir().unwrap();
    let (stage, _) = stage(recordings.path()).await;

    let outcome = stage
        .run(
            &ctx(at("2026-10-12T10:00:00Z")),
            MeetingId::new("meeting-a"),
        )
        .await;

    assert_eq!(outcome, Ok(StageOutcome::Skipped(NO_VIDEO)));
}

#[tokio::test]
async fn a_video_still_being_closed_is_retried() {
    let recordings = tempfile::tempdir().unwrap();
    let part = folder(recordings.path()).join(VIDEO_PART);
    std::fs::write(&part, b"").unwrap();
    let (stage, _) = stage(recordings.path()).await;
    let soon = modified(&part) + TimeDelta::seconds(2);

    let outcome = stage.run(&ctx(soon), MeetingId::new("meeting-a")).await;

    assert!(matches!(outcome, Err(StageError::Retryable(_))));
}

#[tokio::test]
async fn a_crash_leftover_is_adopted_and_stored_beside_the_audio() {
    let recordings = tempfile::tempdir().unwrap();
    let dir = folder(recordings.path());
    std::fs::write(dir.join(VIDEO_PART), b"").unwrap();
    let (stage, meetings) = stage(recordings.path()).await;
    let later = modified(&dir.join(VIDEO_PART)) + TimeDelta::minutes(5);

    let outcome = stage.run(&ctx(later), MeetingId::new("meeting-a")).await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    assert!(dir.join(VIDEO_FILE).exists() && !dir.join(VIDEO_PART).exists());
    let audio = meetings
        .state(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .unwrap()
        .audio;
    assert_eq!(
        audio.video_path,
        Some(dir.join(VIDEO_FILE).to_string_lossy().into_owned())
    );
    assert_eq!(
        audio.audio_path.as_deref(),
        Some("/recordings/meeting-a/audio.mp4")
    );
}

#[test]
fn the_stage_is_screen_video_at_order_fifteen() {
    let stage = ScreenVideoStage::new(&StagePorts::fakes());
    assert_eq!(stage.id(), StageId::new("screen_video"));
    assert_eq!(stage.order(), 15);
}
