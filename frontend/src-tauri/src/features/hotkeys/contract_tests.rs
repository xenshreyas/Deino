//! Contract tests: the `hotkeys` commands' JSON through the real plugin,
//! ACL and error mapping, over a fake registrar.

use std::sync::Arc;

use deino_core::hotkeys::{FakeRegistrar, HotkeyService};
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window(registrar: FakeRegistrar) -> WebviewWindow<MockRuntime> {
    let settings = block_on(SettingsService::load(
        Arc::new(FakeSettingsStore::with(None)),
        Arc::new(RecordedSettingsEvents::default()),
    ))
    .unwrap();
    let service = Arc::new(HotkeyService::new(Arc::new(registrar), Arc::new(settings)));
    block_on(service.start());
    let builder = mock_builder().plugin(super::init()).manage(service);
    main_window(&mock_app(builder))
}

fn record_hotkey(list: &Value) -> Value {
    list.as_array().unwrap()[0].clone()
}

#[test]
fn list_returns_each_hotkey_with_its_default_and_state() {
    let window = window(FakeRegistrar::default());
    let list = invoke(&window, "plugin:hotkeys|list", json!({})).unwrap();
    assert_eq!(
        record_hotkey(&list),
        json!({
            "action": "record",
            "label": "Start or stop recording",
            "accelerator": "Ctrl+Alt+Cmd+R",
            "defaultAccelerator": "Ctrl+Alt+Cmd+R",
            "state": { "kind": "active" },
        })
    );
    assert_eq!(list.as_array().unwrap().len(), 3);
}

#[test]
fn set_saves_a_new_combination_and_turns_one_off() {
    let window = window(FakeRegistrar::default());
    let list = invoke(
        &window,
        "plugin:hotkeys|set",
        json!({ "action": "record", "accelerator": "cmd+shift+f9" }),
    )
    .unwrap();
    assert_eq!(record_hotkey(&list)["accelerator"], "Shift+Cmd+F9");

    let list = invoke(
        &window,
        "plugin:hotkeys|set",
        json!({ "action": "record", "accelerator": null }),
    )
    .unwrap();
    assert_eq!(record_hotkey(&list)["state"], json!({ "kind": "off" }));
}

#[test]
fn set_maps_conflicts_and_invalid_combinations() {
    let registrar = FakeRegistrar::default();
    registrar.hold_elsewhere("Ctrl+Alt+F1");
    let window = window(registrar);
    let set = |accelerator: &str| {
        invoke(
            &window,
            "plugin:hotkeys|set",
            json!({ "action": "ask", "accelerator": accelerator }),
        )
        .unwrap_err()["code"]
            .clone()
    };
    assert_eq!(set("Cmd+R"), "invalid");
    assert_eq!(set("Ctrl+Alt+Cmd+N"), "conflict");
    assert_eq!(set("Ctrl+Alt+F1"), "conflict");
}
