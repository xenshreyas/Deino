//! A running recording's controls and watchers (CAP-10) and Resume on a
//! finished meeting (CAP-07), over a real database, mock capture and the
//! scripted fake engine.
//!
//! - Resume appends audio and segments after the recorded length, on the
//!   meeting's own speakers, and the meeting goes through `recording` and
//!   `processing` again; it is refused while the meeting is not finished;
//! - the microphone switch and the system-audio toggle show in the state;
//! - levels and lane activity are readable while recording;
//! - a full disk shows as a health issue within seconds.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/rig.rs"]
mod rig;

use std::sync::Arc;
use std::time::Duration;

use deino_audio::capture::OutputRoute;
use deino_audio::manifest::ManifestStore;
use deino_audio::{Bytes, Frames, TestSignal, Track, recording_dir};

use deino_core::segments::{Page, Segment, SegmentReader, SpeakerRepo, SqliteSegmentRepo};
use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};
use deino_recording::{
    HealthIssue, IssueKind, Playhead, RecordingError, RecordingPhase, RecordingSnapshot,
    StartRequest,
};
use rig::{FastClock, FixtureCapture, Rig, rig};

/// Generated voices on headphones, at 4× real time.
fn voices() -> FixtureCapture {
    FixtureCapture {
        clock: Arc::new(FastClock::new(4)),
        microphone: TestSignal::microphone(),
        system: TestSignal::system(),
        route: OutputRoute::Headphones,
    }
}

/// Lets `audio` of recording time pass.
async fn record_for(rig: &Rig, audio: Duration) {
    let clock = Arc::clone(&rig.clock);
    tokio::task::spawn_blocking(move || clock.wait_for(audio))
        .await
        .unwrap();
}

async fn segments(rig: &Rig, meeting: &MeetingId) -> Vec<Segment> {
    SqliteSegmentRepo::new(rig.database.clone())
        .list(meeting, Page::ALL)
        .await
        .unwrap()
}

/// Records a few seconds of a new meeting and stops; returns it.
async fn recorded_meeting(rig: &Rig) -> MeetingId {
    let request = StartRequest::titled(Some("Weekly sync".into()));
    let meeting = rig.service.start(request).await.unwrap();
    record_for(rig, Duration::from_secs(3)).await;
    rig.service.stop().await.unwrap();
    meeting
}

/// Waits (up to five seconds) for the recorder's state to satisfy `ready`.
async fn state_where(rig: &Rig, ready: impl Fn(&RecordingSnapshot) -> bool) -> RecordingSnapshot {
    for _ in 0..100 {
        let state = rig.service.state();
        if ready(&state) {
            return state;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    rig.service.state()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_finished_meeting_resumes_and_appends_after_its_recorded_length() {
    let rig = rig(voices()).await;
    let meeting = recorded_meeting(&rig).await;
    let before = segments(&rig, &meeting).await;
    let dir = recording_dir(&rig.deps.recordings, &meeting);
    let first_take = ManifestStore::new(&dir).read().unwrap().length();
    rig.deps
        .lifecycle
        .transition(&meeting, MeetingEvent::Complete)
        .await
        .unwrap();

    assert_eq!(rig.service.resume_meeting(&meeting).await.unwrap(), meeting);
    let resumed = rig.service.state();
    record_for(&rig, Duration::from_secs(3)).await;
    let playhead = rig.service.playhead();
    rig.service.stop().await.unwrap();

    assert_eq!(resumed.phase, RecordingPhase::Recording);
    let first_ms = u32::try_from(first_take.as_duration().as_millis()).unwrap();
    let Playhead::Recording { at, .. } = playhead else {
        panic!("not recording: {playhead:?}");
    };
    assert!(
        at.0 >= first_ms + 1_000,
        "the playhead continues the timeline: {at:?}"
    );
    assert_eq!(resumed.title.as_deref(), Some("Weekly sync"));
    let manifest = ManifestStore::new(&dir).read().unwrap();
    for track in Track::ALL {
        let segments = &manifest.track(track).unwrap().segments;
        assert_eq!(segments.len(), 2, "{track}");
        assert_eq!(segments[1].first_frame, first_take);
    }
    assert!(manifest.length() > first_take + Frames(48_000));
    let after = segments(&rig, &meeting).await;
    let appended: Vec<&Segment> = after.iter().filter(|s| !before.contains(s)).collect();
    assert!(!appended.is_empty(), "the new audio is transcribed");
    assert!(appended.iter().all(|s| s.start.0 + 500 >= first_ms));
    let state = rig.deps.meetings.state(&meeting).await.unwrap().unwrap();
    assert_eq!(state.status, MeetingStatus::Processing);
    assert!(state.duration.unwrap().0 >= 5, "{:?}", state.duration);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resumed_segments_go_to_the_meetings_own_speakers() {
    let rig = rig(voices()).await;
    let meeting = recorded_meeting(&rig).await;
    let repo = SqliteSegmentRepo::new(rig.database.clone());
    let speakers = repo.speakers(&meeting).await.unwrap();
    rig.deps
        .lifecycle
        .transition(&meeting, MeetingEvent::Complete)
        .await
        .unwrap();

    rig.service.resume_meeting(&meeting).await.unwrap();
    record_for(&rig, Duration::from_secs(2)).await;
    rig.service.stop().await.unwrap();

    assert_eq!(repo.speakers(&meeting).await.unwrap(), speakers);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_a_finished_meeting_can_resume() {
    let rig = rig(voices()).await;
    let meeting = recorded_meeting(&rig).await;

    let processing = rig.service.resume_meeting(&meeting).await;
    let missing = rig
        .service
        .resume_meeting(&MeetingId::new("meeting-missing"))
        .await;

    assert!(matches!(
        processing,
        Err(RecordingError::NotResumable(MeetingStatus::Processing))
    ));
    assert!(matches!(missing, Err(RecordingError::NotFound(_))));
    assert_eq!(rig.service.state().phase, RecordingPhase::Idle);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn switching_devices_mid_recording_shows_in_the_state() {
    let rig = rig(voices()).await;
    assert!(matches!(
        rig.service.set_system_audio(false),
        Err(RecordingError::NotRecording)
    ));
    rig.service.start(StartRequest::default()).await.unwrap();

    let without_system = rig.service.set_system_audio(false).unwrap();
    let with_usb = rig
        .service
        .set_microphone(Some("USB Microphone".into()))
        .unwrap();
    record_for(&rig, Duration::from_secs(1)).await;

    assert!(!without_system.system_audio);
    assert_eq!(with_usb.microphone.as_deref(), Some("USB Microphone"));
    assert!(rig.service.levels().is_some());
    assert!(rig.service.activity().is_some());
    rig.service.stop().await.unwrap();
    assert!(rig.service.levels().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_full_disk_shows_as_a_health_issue() {
    let rig = rig(voices()).await;
    rig.service.start(StartRequest::default()).await.unwrap();
    record_for(&rig, Duration::from_secs(1)).await;

    rig.disk.set_free(Bytes(1024 * 1024));
    let state = state_where(&rig, |state| !state.issues.is_empty()).await;
    rig.service.stop().await.unwrap();

    let full = HealthIssue {
        kind: IssueKind::DiskFull,
        channel: None,
    };
    assert_eq!(state.issues, [full]);
    assert!(rig.service.state().issues.is_empty(), "idle again");
}
