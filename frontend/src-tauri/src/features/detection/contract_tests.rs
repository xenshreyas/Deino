//! Contract tests: the JSON the prompt and Settings send to and receive from
//! the detection commands, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_detect::fake::{FakeRecorder, FakeSettings, RecordedSink};
use deino_detect::policy::SHORT_DWELL;
use deino_detect::{
    AppIdentity, DetectionConfig, DetectionPorts, DetectionService, MicClient, Pid, SignalSnapshot,
};
use deino_domain::BundleId;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewUrl, WebviewWindow};

use super::prompt::PROMPT_LABEL;
use crate::features::ipc_tests::{invoke, main_window, mock_app};
use crate::features::system;

/// A detection service over fakes, with Zoom's prompt on screen.
struct Prompted {
    app: App<MockRuntime>,
    recorder: FakeRecorder,
    settings: FakeSettings,
}

fn prompted() -> Prompted {
    let clock = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));
    let (recorder, settings) = (FakeRecorder::default(), FakeSettings::default());
    settings.0.set(DetectionConfig {
        enabled: true,
        ..DetectionConfig::default()
    });
    let service = Arc::new(DetectionService::new(
        DetectionPorts {
            sink: Arc::new(RecordedSink::default()),
            recorder: Arc::new(recorder.clone()),
            status: Arc::new(recorder.clone()),
            config: Arc::new(settings.clone()),
            ignore_list: Arc::new(settings.clone()),
        },
        clock.clone(),
    ));
    let zoom = SignalSnapshot {
        clients: vec![MicClient {
            app: zoom(),
            pid: Pid(42),
            full_duplex: true,
        }],
        microphone_running: true,
        ..SignalSnapshot::default()
    };
    service.evaluate(&zoom);
    clock.advance(SHORT_DWELL + TimeDelta::zero());
    service.evaluate(&zoom);
    let app = mock_app(
        mock_builder()
            .plugin(super::init())
            .plugin(system::init())
            .manage(service),
    );
    app.manage(settings_service());
    Prompted {
        app,
        recorder,
        settings,
    }
}

fn zoom() -> AppIdentity {
    AppIdentity {
        bundle_id: BundleId::try_from("us.zoom.xos").unwrap(),
        name: "zoom.us".to_owned(),
    }
}

fn settings_service() -> Arc<SettingsService> {
    let service = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    );
    Arc::new(tauri::async_runtime::block_on(service).unwrap())
}

fn prompt_window(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, PROMPT_LABEL, WebviewUrl::default())
        .build()
        .expect("prompt window")
}

#[test]
fn current_returns_the_prompt_with_camel_case_fields() {
    let prompted = prompted();

    let current = invoke(
        &prompt_window(&prompted.app),
        "plugin:detection|current",
        Value::Null,
    );

    assert_eq!(
        current,
        Ok(json!({
            "id": 1,
            "app": { "bundleId": "us.zoom.xos", "name": "zoom.us" },
            "kind": "meeting",
            "platform": "zoom",
            "event": null,
            "detectedAt": "2026-10-09T10:00:02.500Z",
            "autoStartAt": null
        }))
    );
}

#[test]
fn take_notes_starts_recording_once_and_then_reports_the_prompt_closed() {
    let prompted = prompted();
    let window = prompt_window(&prompted.app);

    let first = invoke(&window, "plugin:detection|take_notes", json!({ "id": 1 }));
    let again = invoke(&window, "plugin:detection|take_notes", json!({ "id": 1 }));

    assert_eq!(first, Ok(Value::Null));
    assert_eq!(
        again,
        Err(json!({ "code": "not_found", "message": "that meeting prompt has already closed" }))
    );
    assert_eq!(prompted.recorder.requests.get().len(), 1);
    assert!(prompted.recorder.requests.get()[0].open_notes);
}

#[test]
fn dismiss_and_ignore_app_close_the_prompt_and_ignore_saves_the_app() {
    let dismissed = prompted();
    let ignored = prompted();

    let dismiss = invoke(
        &prompt_window(&dismissed.app),
        "plugin:detection|dismiss",
        json!({ "id": 1 }),
    );
    let ignore = invoke(
        &prompt_window(&ignored.app),
        "plugin:detection|ignore_app",
        json!({ "id": 1 }),
    );

    assert_eq!((dismiss, ignore), (Ok(Value::Null), Ok(Value::Null)));
    assert!(dismissed.recorder.requests.get().is_empty());
    assert!(ignored.settings.0.get().ignored.contains(&zoom().bundle_id));
}

#[test]
fn settings_reports_the_saved_section_and_what_this_mac_supports() {
    let prompted = prompted();

    let overview = invoke(
        &main_window(&prompted.app),
        "plugin:detection|settings",
        Value::Null,
    )
    .unwrap();

    assert_eq!(
        overview["settings"],
        json!({ "enabled": true, "ignoredApps": [], "autoStart": { "enabled": false, "countdownSeconds": 10 } })
    );
    assert_eq!(overview["supported"], json!(cfg!(target_os = "macos")));
    assert!(overview["readsBrowserUrls"].is_boolean());
}

#[test]
fn the_prompt_window_may_call_detection_and_nothing_else() {
    let prompted = prompted();

    let other = invoke(
        &prompt_window(&prompted.app),
        "plugin:system|app_version",
        Value::Null,
    );

    let error = other.unwrap_err().to_string();
    assert!(error.contains("not allowed"), "unexpected error: {error}");
}
