//! `finalize_audio`: skips meetings without a live recording; on macOS,
//! turns a real two-track recording into its three files and stores them.

use std::sync::Arc;

use deino_core::meetings::{FakeMeetingRepo, MeetingTitle, NewMeeting};
use deino_domain::{FakeClock, MeetingSource, MeetingStatus};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;

const AT: &str = "2026-10-09T09:00:00Z";

fn ctx() -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at(AT))),
        events: Arc::new(RecordedProgress::default()),
    }
}

async fn stage(recordings: &Path) -> (FinalizeAudioStage, Arc<FakeMeetingRepo>) {
    let meetings = Arc::new(FakeMeetingRepo::default());
    meetings
        .create(&NewMeeting {
            id: MeetingId::new("meeting-a"),
            title: MeetingTitle::parse("Standup").unwrap(),
            status: MeetingStatus::Processing,
            source: MeetingSource::Live,
            created_at: at(AT),
            started_at: Some(at(AT)),
            folder_path: None,
        })
        .await
        .unwrap();
    let ports = StagePorts {
        meetings: meetings.clone(),
        recordings: recordings.to_path_buf(),
        ..StagePorts::fakes()
    };
    (FinalizeAudioStage::new(&ports), meetings)
}

#[tokio::test]
async fn a_meeting_without_a_live_recording_is_skipped() {
    let recordings = tempfile::tempdir().unwrap();
    let (stage, _) = stage(recordings.path()).await;

    let outcome = stage.run(&ctx(), MeetingId::new("meeting-a")).await;

    assert_eq!(outcome, Ok(StageOutcome::Skipped(NO_RECORDING)));
}

#[test]
fn the_stage_is_finalize_audio_at_order_ten() {
    let ports = StagePorts::fakes();
    let stage = FinalizeAudioStage::new(&ports);
    assert_eq!(
        (stage.id(), stage.order()),
        (StageId::new("finalize_audio"), 10)
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn a_recording_becomes_three_files_on_the_meeting() {
    use deino_audio::{
        Bytes, CmafEncoderFactory, FakeDiskSpace, FakeHostClock, HostClock, HostTime, Recorder,
        RecorderDeps,
    };

    let recordings = tempfile::tempdir().unwrap();
    let meeting = MeetingId::new("meeting-a");
    let dir = recording_dir(recordings.path(), &meeting);
    let clock = Arc::new(FakeHostClock::new(HostTime(1_000_000_000)));
    let deps = RecorderDeps {
        encoders: Arc::new(CmafEncoderFactory),
        host_clock: clock.clone(),
        wall_clock: Arc::new(FakeClock::new(at(AT))),
        disk: Arc::new(FakeDiskSpace::new(Bytes::gib(100))),
    };
    let (recorder, mut inputs) = Recorder::start(&dir, &deps).unwrap();
    for _ in 0..100 {
        inputs.microphone.push(&[0.1; 480], clock.now());
        inputs.system.push(&[0.2; 480], clock.now());
        clock.advance(std::time::Duration::from_millis(10));
    }
    recorder.stop().unwrap();
    let (stage, meetings) = stage(recordings.path()).await;

    let outcome = stage.run(&ctx(), meeting.clone()).await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    let audio = meetings.state(&meeting).await.unwrap().unwrap().audio;
    for path in [
        audio.audio_path,
        audio.audio_mic_path,
        audio.audio_system_path,
    ] {
        assert!(Path::new(&path.unwrap()).exists());
    }
}
