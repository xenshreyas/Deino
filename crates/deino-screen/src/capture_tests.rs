use std::path::PathBuf;

use deino_domain::{BundleId, MeetingId};

use super::*;
use crate::fake::{FakeEncoders, FakeScreen, MemoryStore, ScriptedClock};
use crate::pick::CaptureTarget;
use crate::sampler::SnapshotPolicy;
use crate::status::CaptureStatus;
use crate::target::WindowInfo;

/// A fresh recording folder under the temp dir.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("deino-screen-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&folder).ok();
    folder
}

fn window(on_screen: bool) -> WindowInfo {
    WindowInfo {
        id: 7,
        bundle_id: BundleId::try_from("us.zoom.xos").ok(),
        width: 1_280.0,
        height: 800.0,
        layer: 0,
        on_screen,
        title: None,
        app_name: None,
    }
}

struct Rig {
    screen: Arc<FakeScreen>,
    encoders: Arc<FakeEncoders>,
    store: Arc<MemoryStore>,
    status: Arc<StatusCell>,
    folder: PathBuf,
}

impl Rig {
    fn new(name: &str) -> Self {
        Self {
            screen: FakeScreen::granted(vec![window(true)]),
            encoders: FakeEncoders::new(),
            store: Arc::default(),
            status: Arc::default(),
            folder: folder(name),
        }
    }

    fn capture(&self, positions: impl IntoIterator<Item = Position>, video: bool) -> Capture {
        let plan = CapturePlan {
            meeting: MeetingId::new("meeting"),
            folder: self.folder.clone(),
            target: CaptureTarget::MeetingApp(BundleId::try_from("us.zoom.xos").unwrap()),
            video: video.then_some(FrameRate::DEFAULT),
            snapshots: Some(SnapshotPolicy::default()),
        };
        Capture::new(CaptureParts {
            plan,
            window: Some(7),
            clock: ScriptedClock::new(positions),
            windows: self.screen.clone(),
            encoders: self.encoders.clone(),
            store: self.store.clone(),
            status: self.status.clone(),
        })
    }
}

fn blank() -> Frame {
    Frame::solid(64, 40, [0, 0, 0, 255]).unwrap()
}

fn bright() -> Frame {
    Frame::new(
        64,
        40,
        (0..64 * 40 * 4)
            .map(|i| if i % 512 < 256 { 255 } else { 0 })
            .collect(),
    )
    .unwrap()
}

#[test]
fn frames_go_to_the_video_and_changed_ones_become_snapshots() {
    let rig = Rig::new("record");
    let at = |ms| Position::At(Millis(ms));
    let mut capture = rig.capture([at(0), at(1_000), at(10_000), at(20_000), at(21_000)], true);

    for frame in [blank(), blank(), blank(), bright(), bright()] {
        assert_eq!(capture.step(Some(frame)), Step::Continue);
    }
    assert_eq!(capture.step(None), Step::Done);

    let video = &rig.encoders.videos()[0];
    assert_eq!(video.frames, [0, 1_000, 10_000, 20_000, 21_000].map(Millis));
    assert_eq!(video.finished_at, Some(Millis(21_000)));
    assert_eq!(video.size, (64, 40));
    assert!(rig.folder.join(VIDEO_FILE).exists(), "renamed when closed");
    assert!(!rig.folder.join(VIDEO_PART).exists());

    // 0 s kept (first), 10 s unchanged, 20 s kept (new picture); 1 s and
    // 21 s are not due.
    let kept: Vec<Millis> = rig.store.records().iter().map(|row| row.at).collect();
    assert_eq!(kept, [Millis(0), Millis(20_000)]);
    let first = &rig.store.records()[0];
    assert_eq!(first.file_name, "0000000000.png");
    assert!(rig.folder.join(SNAPSHOTS).join(&first.file_name).exists());
    assert_eq!(rig.status.get(), CaptureStatus::default(), "ended");
    std::fs::remove_dir_all(&rig.folder).unwrap();
}

#[test]
fn a_hidden_window_pauses_the_capture_until_it_is_back() {
    let rig = Rig::new("hidden");
    let at = |ms| Position::At(Millis(ms));
    let mut capture = rig.capture([at(0), at(2_000), at(3_000), at(4_000)], true);

    capture.step(Some(blank()));
    rig.screen.set_windows(vec![window(false)]);
    capture.step(Some(blank()));
    assert_eq!(rig.status.get().state, CaptureState::Hidden);
    capture.step(Some(blank()));
    rig.screen.set_windows(vec![window(true)]);
    capture.step(Some(blank()));

    assert_eq!(rig.encoders.videos()[0].frames, [Millis(0), Millis(4_000)]);
    assert_eq!(rig.status.get().state, CaptureState::Capturing);
    capture.finish();
    std::fs::remove_dir_all(&rig.folder).unwrap();
}

#[test]
fn a_paused_recording_drops_frames() {
    let rig = Rig::new("paused");
    let mut capture = rig.capture([Position::Paused, Position::At(Millis(500))], true);

    capture.step(Some(blank()));
    assert_eq!(rig.status.get().state, CaptureState::Paused);
    capture.step(Some(blank()));

    assert_eq!(rig.encoders.videos()[0].frames, [Millis(500)]);
    capture.finish();
    std::fs::remove_dir_all(&rig.folder).unwrap();
}

#[test]
fn snapshots_carry_on_when_the_video_cannot_be_written() {
    let rig = Rig::new("novideo");
    rig.encoders.fail_open();
    let mut capture = rig.capture([Position::At(Millis(0))], true);

    capture.step(Some(blank()));

    assert!(!rig.status.get().video);
    assert_eq!(rig.store.records().len(), 1);
    capture.finish();
    assert!(!rig.folder.join(VIDEO_FILE).exists());
    std::fs::remove_dir_all(&rig.folder).unwrap();
}

#[test]
fn snapshots_alone_write_no_video() {
    let rig = Rig::new("stills");
    let mut capture = rig.capture([Position::At(Millis(0))], false);

    capture.step(Some(blank()));
    capture.finish();

    assert!(rig.encoders.videos().is_empty());
    assert_eq!(rig.store.records().len(), 1);
    std::fs::remove_dir_all(&rig.folder).unwrap();
}
