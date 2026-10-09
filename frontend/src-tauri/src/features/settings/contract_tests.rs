//! Contract tests: the JSON the UI sends to and receives from the settings
//! commands, and the typed event, through the real plugin and ACL.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use deino_core::settings::{FakeSettingsStore, SettingsService};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager};

use super::TauriSettingsEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

fn settings_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let events = Arc::new(TauriSettingsEvents::new(app.handle().clone()));
    let store = Arc::new(FakeSettingsStore::default());
    let service = SettingsService::load(store, events);
    app.manage(Arc::new(tauri::async_runtime::block_on(service).unwrap()));
    app
}

#[test]
fn get_returns_every_section_with_camel_case_fields() {
    let app = settings_app();

    let settings = invoke(&main_window(&app), "plugin:settings|get", Value::Null).unwrap();

    assert_eq!(settings["version"], 1);
    assert_eq!(settings["ui"]["theme"], "system");
    assert_eq!(settings["ai"]["provider"], "builtin-ai");
    assert_eq!(settings["recording"]["autoSave"], true);
    assert_eq!(settings["transcription"]["engine"], "parakeet_unified");
}

#[test]
fn patch_returns_the_merged_settings_and_emits_them() {
    let app = settings_app();
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:settings:settings-changed", move |event| {
        sender
            .send(event.payload().to_owned())
            .expect("the test is still listening");
    });

    let patch = json!({ "patch": { "ui": { "theme": "dark" }, "ai": { "autoSummary": true } } });
    let settings = invoke(&main_window(&app), "plugin:settings|patch", patch).unwrap();

    assert_eq!(settings["ui"]["theme"], "dark");
    assert_eq!(settings["ai"]["autoSummary"], true);
    assert_eq!(settings["ui"]["showConfidenceIndicator"], true);
    let payload: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(payload, json!({ "settings": settings }));
}

#[test]
fn a_bad_patch_is_an_invalid_app_error_and_changes_nothing() {
    let app = settings_app();
    let window = main_window(&app);

    let unknown = invoke(
        &window,
        "plugin:settings|patch",
        json!({ "patch": { "ui": { "colour": "red" } } }),
    );
    let settings = invoke(&window, "plugin:settings|get", Value::Null).unwrap();

    assert_eq!(
        unknown,
        Err(json!({ "code": "invalid", "message": "there is no setting named ui.colour" }))
    );
    assert_eq!(settings["ui"]["theme"], "system");
}
