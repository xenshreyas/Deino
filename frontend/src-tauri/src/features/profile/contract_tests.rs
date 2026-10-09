//! Contract tests: the JSON the profile page sends and receives.

use std::sync::Arc;

use deino_core::people::FakePeopleRepo;
use deino_core::profile::ProfileService;
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window() -> WebviewWindow<MockRuntime> {
    let settings = block_on(SettingsService::load(
        Arc::new(FakeSettingsStore::with(None)),
        Arc::new(RecordedSettingsEvents::default()),
    ))
    .unwrap();
    let people = Arc::new(FakePeopleRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    let service = ProfileService::new(Arc::new(settings), people.clone(), people, clock);
    main_window(&mock_app(
        mock_builder()
            .plugin(super::init())
            .manage(Arc::new(service)),
    ))
}

fn input(emails: &Value) -> Value {
    json!({ "input": {
        "name": " Shreyas ",
        "emails": emails,
        "role": "CTO",
        "company": "Deino",
        "focusAreas": ["Hiring", ""],
        "writingStyle": "",
        "context": ""
    }})
}

#[test]
fn update_returns_the_normalised_profile() {
    let saved = invoke(
        &window(),
        "plugin:profile|update",
        input(&json!(["Shreyas@Deino.dev"])),
    );

    assert_eq!(
        saved,
        Ok(json!({
            "name": "Shreyas",
            "emails": ["shreyas@deino.dev"],
            "role": "CTO",
            "company": "Deino",
            "focusAreas": ["Hiring"],
            "writingStyle": "",
            "context": ""
        }))
    );
}

#[test]
fn a_bad_address_is_invalid_and_named_by_position() {
    let error = invoke(
        &window(),
        "plugin:profile|update",
        input(&json!(["ok@deino.dev", "nope"])),
    );

    assert_eq!(
        error,
        Err(json!({ "code": "invalid", "message": "email address 2 is not valid" }))
    );
}

#[test]
fn context_is_null_until_the_profile_says_something() {
    let window = window();

    let empty = invoke(&window, "plugin:profile|context", Value::Null);
    invoke(&window, "plugin:profile|update", input(&json!([]))).unwrap();
    let filled = invoke(&window, "plugin:profile|context", Value::Null).unwrap();

    assert_eq!(empty, Ok(Value::Null));
    assert!(filled.as_str().unwrap().contains("Role: CTO at Deino"));
}
