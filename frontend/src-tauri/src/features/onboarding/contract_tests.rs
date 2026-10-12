//! Contract tests: the JSON the onboarding permissions step and the sample
//! meeting send and receive.

use std::sync::Arc;

use deino_core::permissions::{
    FakePermissionGate, PermissionKind, PermissionService, PermissionState,
};
use deino_core::sample::SAMPLE_MEETING_ID;
use deino_domain::Clock;
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn open(gate: FakePermissionGate) -> (WebviewWindow<MockRuntime>, Arc<FakePermissionGate>) {
    let gate = Arc::new(gate);
    let service = PermissionService::new(gate.clone());
    let app = mock_app(
        mock_builder()
            .plugin(super::init())
            .manage(Arc::new(service)),
    );
    (main_window(&app), gate)
}

#[test]
fn permissions_lists_each_kind_with_its_state() {
    let (window, gate) = open(FakePermissionGate::default());
    gate.set(PermissionKind::Microphone, PermissionState::Granted);
    gate.set(PermissionKind::SystemAudio, PermissionState::Unknown);

    let listed = invoke(&window, "plugin:onboarding|permissions", json!({})).unwrap();

    assert_eq!(
        listed,
        json!([
            { "kind": "microphone", "state": "granted", "optional": false },
            { "kind": "system_audio", "state": "unknown", "optional": false },
            { "kind": "notifications", "state": "not_determined", "optional": false },
            { "kind": "accessibility", "state": "not_determined", "optional": true },
            { "kind": "screen_recording", "state": "not_determined", "optional": true },
        ])
    );
}

#[test]
fn request_permission_answers_with_the_new_status() {
    let (window, gate) = open(FakePermissionGate::default());
    gate.refuse(PermissionKind::Notifications, PermissionState::Denied);

    let status = invoke(
        &window,
        "plugin:onboarding|request_permission",
        json!({ "kind": "notifications" }),
    )
    .unwrap();

    assert_eq!(
        status,
        json!({ "kind": "notifications", "state": "denied", "optional": false })
    );
}

#[test]
fn open_permission_settings_returns_null_or_unavailable() {
    let (window, gate) = open(FakePermissionGate::default());
    let opened = invoke(
        &window,
        "plugin:onboarding|open_permission_settings",
        json!({ "kind": "accessibility" }),
    );
    assert_eq!(opened, Ok(Value::Null));
    assert_eq!(gate.opened(), [PermissionKind::Accessibility]);

    let (window, _) = open(FakePermissionGate::without_settings());
    let error = invoke(
        &window,
        "plugin:onboarding|open_permission_settings",
        json!({ "kind": "microphone" }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "unavailable");
}

#[test]
fn an_unknown_kind_is_rejected() {
    let (window, _) = open(FakePermissionGate::default());

    let error = invoke(
        &window,
        "plugin:onboarding|request_permission",
        json!({ "kind": "camera" }),
    );

    assert!(error.is_err());
}

#[test]
fn import_sample_returns_the_same_meeting_every_time() {
    let database = tauri::async_runtime::block_on(test_db());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z")));
    let importer = crate::composition::onboarding::sample_importer(&database, &clock);
    let app = mock_app(
        mock_builder()
            .plugin(super::init())
            .manage(Arc::new(importer)),
    );
    let window = main_window(&app);

    let first = invoke(&window, "plugin:onboarding|import_sample", json!({})).unwrap();
    let again = invoke(&window, "plugin:onboarding|import_sample", json!({})).unwrap();

    assert_eq!(first, json!(SAMPLE_MEETING_ID));
    assert_eq!(again, first);
}
