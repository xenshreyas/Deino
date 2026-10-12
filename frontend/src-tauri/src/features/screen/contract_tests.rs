//! Contract tests: the JSON of `plugin:screen|*` through the real plugin
//! and ACL, over a fake Mac. `open_screen_recording_settings` is never
//! invoked here: it would open System Settings on the test machine, and
//! nothing here can show the Screen Recording prompt.

use std::sync::Arc;

use deino_core::screen::{CaptureControl, FakeScreenRepo, ScreenService};
use deino_domain::{FakeClock, MeetingId};
use deino_screen::fake::{FakeEncoders, FakeScreen, MemoryStore, ScriptedClock};
use deino_screen::{ScreenCapture, ScreenCaptureParts};
use deino_testkit::at;
use serde_json::json;
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// Screen capture over a fake Mac with no windows.
fn capture(screen: &Arc<FakeScreen>) -> Arc<ScreenCapture> {
    Arc::new(ScreenCapture::new(ScreenCaptureParts {
        platform: Some(screen.platform(&FakeEncoders::new())),
        clock: ScriptedClock::new([]),
        store: Arc::new(MemoryStore::default()),
    }))
}

/// Nothing is being captured.
struct Idle;

impl CaptureControl for Idle {
    fn turn_off(&self, _meeting: &MeetingId) -> bool {
        false
    }
}

#[test]
fn status_reports_the_permission_and_the_capture() {
    let app = mock_app(mock_builder().plugin(super::init()));
    let screen = FakeScreen::granted(vec![]);
    screen.set_granted(false);
    app.manage(capture(&screen));
    let window = main_window(&app);

    let status = invoke(&window, "plugin:screen|status", json!({})).unwrap();

    assert_eq!(
        status,
        json!({
            "supported": true,
            "permitted": false,
            "capture": { "meeting": null, "state": "off", "video": false, "snapshots": 0 }
        })
    );
    screen.set_granted(true);
    let status = invoke(&window, "plugin:screen|status", json!({})).unwrap();
    assert_eq!(status["permitted"], json!(true));
}

#[test]
fn a_meeting_turned_off_stays_off_and_has_no_snapshots() {
    let app = mock_app(mock_builder().plugin(super::init()));
    let repo = Arc::new(FakeScreenRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    app.manage(Arc::new(ScreenService::new(repo, Arc::new(Idle), clock)));
    let window = main_window(&app);
    let meeting = json!({ "meetingId": "meeting-a" });

    let stopped = invoke(
        &window,
        "plugin:screen|turn_off_for_meeting",
        meeting.clone(),
    )
    .unwrap();
    let snapshots = invoke(&window, "plugin:screen|snapshots", meeting).unwrap();

    assert_eq!(stopped, json!(false), "nothing was being captured");
    assert_eq!(snapshots, json!([]));
}
