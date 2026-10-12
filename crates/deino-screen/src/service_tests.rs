use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use deino_domain::{BundleId, Millis};

use super::*;
use crate::capture::VIDEO_FILE;
use crate::fake::{FakeEncoders, FakeScreen, MemoryStore, ScriptedClock};
use crate::frame::{Frame, FrameRate};
use crate::ports::Position;
use crate::sampler::SnapshotPolicy;
use crate::status::CaptureState;
use crate::pick::CaptureTarget;
use crate::target::WindowInfo;

fn zoom_window() -> WindowInfo {
    WindowInfo {
        id: 42,
        bundle_id: BundleId::try_from("us.zoom.xos").ok(),
        width: 1_440.0,
        height: 900.0,
        layer: 0,
        on_screen: true,
        title: None,
        app_name: None,
    }
}

fn plan(folder: PathBuf) -> CapturePlan {
    CapturePlan {
        meeting: MeetingId::new("meeting"),
        folder,
        target: CaptureTarget::MeetingApp(BundleId::try_from("us.zoom.xos").unwrap()),
        video: Some(FrameRate::DEFAULT),
        snapshots: Some(SnapshotPolicy::default()),
    }
}

/// Waits (by polling, never sleeping) until `done` holds, for up to 5 s.
fn eventually(done: impl Fn() -> bool) {
    let (_keep, timer) = channel::<()>();
    for _ in 0..500 {
        if done() {
            return;
        }
        timer.recv_timeout(Duration::from_millis(10)).ok();
    }
    panic!("timed out");
}

#[test]
fn a_capture_runs_on_its_thread_until_the_recording_ends() {
    let folder = std::env::temp_dir().join(format!("deino-screen-service-{}", std::process::id()));
    std::fs::remove_dir_all(&folder).ok();
    let screen = FakeScreen::granted(vec![zoom_window()]);
    let encoders = FakeEncoders::new();
    let store = Arc::new(MemoryStore::default());
    screen.push_frames([Frame::solid(64, 40, [0, 0, 0, 255]).unwrap()]);
    let capture = ScreenCapture::new(ScreenCaptureParts {
        platform: Some(screen.platform(&encoders)),
        clock: ScriptedClock::new([Position::At(Millis(0)), Position::At(Millis(500))]),
        store: store.clone(),
    });
    assert!(capture.permitted());

    capture.start(plan(folder.clone()));

    eventually(|| folder.join(VIDEO_FILE).exists());
    // A 1440 × 900 pt window on a 2× display, fitted into 1920 px.
    assert_eq!(screen.opened(), [(42, (1_920, 1_200))]);
    assert_eq!(store.records().len(), 1);
    eventually(|| capture.status().state == CaptureState::Off);
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn turning_a_meeting_off_stops_its_capture_only() {
    let folder = std::env::temp_dir().join(format!("deino-screen-off-{}", std::process::id()));
    let screen = FakeScreen::granted(vec![zoom_window()]);
    let positions = std::iter::repeat_n(Position::Paused, 100_000);
    let capture = ScreenCapture::new(ScreenCaptureParts {
        platform: Some(screen.platform(&FakeEncoders::new())),
        clock: ScriptedClock::new(positions),
        store: Arc::new(MemoryStore::default()),
    });
    capture.start(plan(folder.clone()));

    assert!(!capture.turn_off(&MeetingId::new("other")));
    assert!(capture.turn_off(&MeetingId::new("meeting")));

    assert_eq!(capture.status().state, CaptureState::Off);
    assert!(!capture.turn_off(&MeetingId::new("meeting")));
    std::fs::remove_dir_all(&folder).ok();
}

#[test]
fn a_missing_window_or_platform_captures_nothing() {
    let screen = FakeScreen::granted(vec![]);
    let capture = ScreenCapture::new(ScreenCaptureParts {
        platform: Some(screen.platform(&FakeEncoders::new())),
        clock: ScriptedClock::new([]),
        store: Arc::new(MemoryStore::default()),
    });
    capture.start(plan(PathBuf::from("/nonexistent")));
    eventually(|| capture.status().state == CaptureState::Off);
    assert!(screen.opened().is_empty());

    let nowhere = ScreenCapture::new(ScreenCaptureParts {
        platform: None,
        clock: ScriptedClock::new([]),
        store: Arc::new(MemoryStore::default()),
    });
    assert!(!nowhere.supported());
    assert!(!nowhere.permitted());
    nowhere.start(plan(PathBuf::from("/nonexistent")));
    assert_eq!(nowhere.status().state, CaptureState::Off);
}
