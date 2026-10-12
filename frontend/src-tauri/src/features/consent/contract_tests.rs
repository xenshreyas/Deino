//! Contract tests: the JSON of `plugin:consent|<command>` through the real
//! plugin and ACL, from the main window and from the record panel's window,
//! which may call these two commands and no other settings command.

use std::sync::Arc;

use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewUrl, WebviewWindow};

use crate::features::ipc_tests::{invoke, mock_app};

/// The record panel's window label (`capabilities/record-panel.json`).
const PANEL_LABEL: &str = "record_panel";

fn app() -> (App<MockRuntime>, Arc<SettingsService>) {
    let app = mock_app(
        mock_builder()
            .plugin(super::init())
            .plugin(crate::features::settings::init()),
    );
    let service = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    );
    let settings = Arc::new(tauri::async_runtime::block_on(service).unwrap());
    app.manage(Arc::clone(&settings));
    (app, settings)
}

fn panel(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::default())
        .build()
        .expect("record panel window")
}

#[test]
fn the_panel_reads_the_reminder_and_turns_it_off() {
    let (app, settings) = app();
    let panel = panel(&app);

    let shown = invoke(&panel, "plugin:consent|reminder", json!({})).unwrap();
    let stopped = invoke(&panel, "plugin:consent|stop_reminding", json!({})).unwrap();
    let after = invoke(&panel, "plugin:consent|reminder", json!({})).unwrap();

    assert_eq!(shown, json!(true));
    assert_eq!(stopped, Value::Null);
    assert_eq!(after, json!(false));
    assert!(
        !tauri::async_runtime::block_on(settings.get())
            .consent
            .reminder
    );
}

#[test]
fn the_panel_cannot_read_or_change_other_settings() {
    let (app, _) = app();
    let panel = panel(&app);

    assert!(invoke(&panel, "plugin:settings|get", json!({})).is_err());
}
