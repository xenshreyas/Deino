//! `RecordingService`'s state machine and launch recovery, over a real
//! database, mock capture (generated voices) and the scripted fake engine.
//!
//! - one recording at a time; stop, pause and resume need one;
//! - pause time is counted and skipped;
//! - a start that fails leaves no meeting and the recorder idle;
//! - stop moves the meeting to `processing` with its timing;
//! - at launch, a meeting a crash left in `recording` is repaired and moved
//!   to `processing`, and one that never got audio moves to `failed`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/rig.rs"]
mod rig;

use std::sync::Arc;
use std::time::Duration;

use chrono::TimeDelta;
use deino_audio::TestSignal;
use deino_audio::capture::OutputRoute;
use deino_core::meetings::{MeetingTitle, NewMeeting};
use deino_domain::{Clock, MeetingId, MeetingSource, MeetingStatus, Millis};
use deino_recording::engine::{EngineError, LaneTranscriber};
use deino_recording::{
    LaneEngines, LaneRequest, Playhead, RecordingDeps, RecordingError, RecordingPhase,
    StartRequest, recover_interrupted,
};
use rig::{FastClock, FixtureCapture, Rig, rig, rig_with};

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

async fn with_status(deps: &RecordingDeps, status: MeetingStatus) -> Vec<MeetingId> {
    let meetings = deps.meetings.list_by_status(status).await.unwrap();
    meetings.into_iter().map(|meeting| meeting.id).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_one_recording_runs_and_stop_pause_resume_need_one() {
    let rig = rig(voices()).await;

    assert!(matches!(
        rig.service.stop().await,
        Err(RecordingError::NotRecording)
    ));
    assert!(matches!(
        rig.service.pause(),
        Err(RecordingError::NotRecording)
    ));
    assert!(matches!(
        rig.service.resume(),
        Err(RecordingError::NotRecording)
    ));
    let meeting = rig.service.start(StartRequest::default()).await.unwrap();
    let second = rig
        .service
        .start(StartRequest::titled(Some("Another".into())))
        .await;
    let resumed = rig.service.resume();

    assert!(matches!(second, Err(RecordingError::AlreadyRecording)));
    assert!(matches!(resumed, Err(RecordingError::NotRecording)));
    assert_eq!(rig.service.state().phase, RecordingPhase::Recording);
    assert_eq!(
        rig.service.state().title.as_deref(),
        Some("Untitled meeting")
    );
    assert_eq!(rig.service.stop().await.unwrap(), meeting);
    assert_eq!(rig.service.state().phase, RecordingPhase::Idle);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pause_time_is_counted_and_stop_hands_the_meeting_to_processing() {
    let rig = rig(voices()).await;
    let meeting = rig
        .service
        .start(StartRequest::titled(Some("Standup".into())))
        .await
        .unwrap();
    record_for(&rig, Duration::from_secs(2)).await;

    let paused = rig.service.pause().unwrap();
    rig.wall.advance(TimeDelta::seconds(3));
    let resumed = rig.service.resume().unwrap();
    rig.service.stop().await.unwrap();

    assert_eq!(paused.phase, RecordingPhase::Paused);
    assert!(paused.paused_at.is_some());
    assert_eq!(resumed.phase, RecordingPhase::Recording);
    assert_eq!(resumed.paused_ms, 3_000);
    assert_eq!(
        with_status(&rig.deps, MeetingStatus::Processing).await,
        std::slice::from_ref(&meeting)
    );
    let phases: Vec<_> = rig
        .events
        .states()
        .iter()
        .map(|state| state.phase)
        .collect();
    assert_eq!(
        phases,
        [
            RecordingPhase::Starting,
            RecordingPhase::Recording,
            RecordingPhase::Paused,
            RecordingPhase::Recording,
            RecordingPhase::Stopping,
            RecordingPhase::Idle,
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_playhead_follows_the_recording_and_stands_still_while_paused() {
    let rig = rig(voices()).await;
    assert_eq!(rig.service.playhead(), Playhead::Idle);
    let meeting = rig.service.start(StartRequest::default()).await.unwrap();
    record_for(&rig, Duration::from_secs(2)).await;

    let Playhead::Recording { meeting: first, at } = rig.service.playhead() else {
        panic!("recording");
    };
    rig.service.pause().unwrap();
    let paused = rig.service.playhead();
    rig.service.resume().unwrap();
    record_for(&rig, Duration::from_secs(1)).await;
    let Playhead::Recording { at: later, .. } = rig.service.playhead() else {
        panic!("recording again");
    };
    rig.service.stop().await.unwrap();

    assert_eq!(first, meeting);
    assert!(at >= Millis(2_000), "{at}");
    assert!(later >= Millis(at.0 + 1_000), "{at} then {later}");
    assert_eq!(
        paused,
        Playhead::Paused {
            meeting: meeting.clone()
        }
    );
    assert_eq!(rig.service.playhead(), Playhead::Idle);
}

/// Engines whose model is missing.
struct MissingModel;

impl LaneEngines for MissingModel {
    fn open(&self, request: LaneRequest) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        Err(EngineError {
            lane: request.lane,
            source: "no model downloaded".into(),
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_start_that_fails_leaves_no_meeting_and_the_recorder_idle() {
    let rig = rig_with(voices(), Arc::new(MissingModel)).await;

    let started = rig.service.start(StartRequest::default()).await;

    assert!(matches!(started, Err(RecordingError::Engine(_))));
    assert_eq!(rig.service.state().phase, RecordingPhase::Idle);
    assert_eq!(
        rig.events.states().last().map(|state| state.phase),
        Some(RecordingPhase::Idle)
    );
    assert!(
        with_status(&rig.deps, MeetingStatus::Recording)
            .await
            .is_empty()
    );
    assert!(matches!(
        rig.service.stop().await,
        Err(RecordingError::NotRecording)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn launch_recovery_repairs_interrupted_recordings_and_fails_empty_ones() {
    let rig = rig(voices()).await;
    let interrupted = rig.service.start(StartRequest::default()).await.unwrap();
    record_for(&rig, Duration::from_secs(2)).await;
    let never_recorded = MeetingId::new("meeting-without-audio");
    rig.deps
        .meetings
        .create(&lost_meeting(&never_recorded, &rig))
        .await
        .unwrap();
    // The app dies mid-recording: nothing is stopped or closed, and the
    // meeting stays in `recording`. (Paused first, so the leaked threads
    // write nothing more.)
    let Rig { service, deps, .. } = rig;
    service.pause().unwrap();
    std::mem::forget(service);

    let report = recover_interrupted(&deps).await.unwrap();

    assert_eq!(report.recovered, std::slice::from_ref(&interrupted));
    assert_eq!(report.failed, std::slice::from_ref(&never_recorded));
    assert_eq!(
        with_status(&deps, MeetingStatus::Processing).await,
        [interrupted]
    );
    assert_eq!(
        with_status(&deps, MeetingStatus::Failed).await,
        [never_recorded]
    );
}

/// A meeting left in `recording` before it got any audio.
fn lost_meeting(id: &MeetingId, rig: &Rig) -> NewMeeting {
    NewMeeting {
        id: id.clone(),
        title: MeetingTitle::parse("Lost").unwrap(),
        status: MeetingStatus::Recording,
        source: MeetingSource::Live,
        created_at: rig.wall.now(),
        started_at: None,
        folder_path: None,
    }
}
