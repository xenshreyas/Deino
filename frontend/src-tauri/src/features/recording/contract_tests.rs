//! Contract tests: the `recording` commands' JSON in and out, and the
//! recorder's state events, through the real plugin and ACL on Tauri's mock
//! runtime. The recorder runs over a test database, mock audio, a fake
//! track encoder and the scripted fake STT engine.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use deino_audio::{
    Bytes, FakeDiskSpace, FakeEncoderFactory, HostClock, RecorderDeps, SystemHostClock, TestSignal,
};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::vocabulary::NoLiveVocabulary;
use deino_recording::testing::ScriptedEngines;
use deino_recording::{MockCapture, Preferences, RecordingDeps, RecordingService};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager};

use super::TauriRecordingEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

/// An app with the `recording` plugin over a recorder that records into
/// `recordings`.
fn recording_app(recordings: &tempfile::TempDir) -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let host_clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
    let wall = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let deps = RecordingDeps {
        catalog: meetings.clone(),
        meetings: meetings.clone(),
        lifecycle: Arc::new(MeetingLifecycleService::new(meetings, wall.clone())),
        segments: segments.clone(),
        speakers: segments,
        people: Arc::new(SqlitePeopleRepo::new(database)),
        engines: Arc::new(ScriptedEngines),
        capture: Arc::new(MockCapture::new(
            Arc::clone(&host_clock),
            TestSignal::microphone(),
            TestSignal::system(),
        )),
        preferences: Arc::new(Preferences::default()),
        recorder: RecorderDeps {
            encoders: Arc::new(FakeEncoderFactory::default()),
            host_clock,
            wall_clock: wall,
            disk: Arc::new(FakeDiskSpace::new(Bytes::gib(100))),
        },
        events: Arc::new(TauriRecordingEvents::new(app.handle().clone())),
        vocabulary: Arc::new(NoLiveVocabulary),
        recordings: PathBuf::from(recordings.path()),
    };
    app.manage(Arc::new(RecordingService::new(deps)));
    app
}

#[test]
fn state_is_idle_with_camel_case_fields_before_a_recording() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);

    let state = invoke(&main_window(&app), "plugin:recording|state", Value::Null);

    assert_eq!(
        state,
        Ok(json!({
            "phase": "idle",
            "meetingId": null,
            "title": null,
            "startedAt": null,
            "pausedMs": 0,
            "pausedAt": null,
            "echo": null,
            "calendarEventId": null,
            "microphone": null,
            "systemAudio": true,
            "issues": [],
        }))
    );
}

#[test]
fn stop_pause_and_resume_without_a_recording_are_conflicts() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);
    let window = main_window(&app);
    let conflict = json!({ "code": "conflict", "message": "nothing is being recorded" });

    for command in ["stop", "pause", "resume"] {
        let result = invoke(&window, &format!("plugin:recording|{command}"), Value::Null);

        assert_eq!(result, Err(conflict.clone()), "{command}");
    }
}

#[test]
fn start_records_a_meeting_announces_its_state_and_stop_returns_it() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);
    let (sender, states) = mpsc::channel();
    app.listen_any("plugin:recording:recording-state", move |event| {
        sender.send(event.payload().to_owned()).ok();
    });
    let window = main_window(&app);

    let started = invoke(
        &window,
        "plugin:recording|start",
        json!({ "title": "Standup" }),
    )
    .unwrap();
    let again = invoke(&window, "plugin:recording|start", json!({ "title": null }));
    let paused = invoke(&window, "plugin:recording|pause", Value::Null).unwrap();
    let resumed = invoke(&window, "plugin:recording|resume", Value::Null).unwrap();
    let stopped = invoke(&window, "plugin:recording|stop", Value::Null);

    assert!(started.is_string());
    assert_eq!(again.unwrap_err()["code"], "conflict");
    assert_eq!(paused["phase"], "paused");
    assert_eq!(resumed["phase"], "recording");
    assert_eq!(stopped, Ok(started.clone()));
    let first: Value = serde_json::from_str(&states.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(first["snapshot"]["phase"], "starting");
    assert_eq!(first["snapshot"]["meetingId"], started);
    assert_eq!(first["snapshot"]["title"], "Standup");
}

#[test]
fn resume_meeting_of_an_unknown_meeting_is_not_found() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);

    let resumed = invoke(
        &main_window(&app),
        "plugin:recording|resume_meeting",
        json!({ "meetingId": "meeting-missing" }),
    );

    assert_eq!(resumed.unwrap_err()["code"], "not_found");
}

#[test]
fn device_switches_without_a_recording_are_conflicts() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);
    let window = main_window(&app);

    let microphone = invoke(
        &window,
        "plugin:recording|set_microphone",
        json!({ "name": "USB" }),
    );
    let system = invoke(
        &window,
        "plugin:recording|set_system_audio",
        json!({ "enabled": false }),
    );

    assert_eq!(microphone.unwrap_err()["code"], "conflict");
    assert_eq!(system.unwrap_err()["code"], "conflict");
}

#[test]
fn the_panel_reports_its_region_and_lists_microphones() {
    let recordings = tempfile::tempdir().unwrap();
    let app = recording_app(&recordings);
    let window = main_window(&app);

    let region = json!({ "region": { "x": 260.0, "y": 12.0, "width": 112.0, "height": 36.0 } });
    let reported = invoke(&window, "plugin:recording|set_panel_region", region);
    let microphones = invoke(&window, "plugin:recording|microphones", Value::Null).unwrap();

    assert_eq!(reported, Ok(json!("right")));
    assert!(microphones.is_array());
}
