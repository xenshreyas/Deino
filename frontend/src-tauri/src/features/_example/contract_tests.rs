//! Contract tests: every command's JSON in and out, and the typed event, through
//! the real plugin and ACL on Tauri's mock runtime. Copy these with the feature.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use deino_core::_example::{ExampleService, FakeExampleNoteRepo};
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager};

use super::TauriExampleEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

fn example_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let events = Arc::new(TauriExampleEvents::new(app.handle().clone()));
    let clock = Arc::new(FakeClock::new(at("2026-10-08T09:00:00Z")));
    let repo = Arc::new(FakeExampleNoteRepo::default());
    app.manage(Arc::new(ExampleService::new(repo, events, clock)));
    app
}

#[test]
fn add_returns_the_saved_note_and_list_includes_it() {
    let app = example_app();
    let window = main_window(&app);

    let note = invoke(
        &window,
        "plugin:example|add",
        json!({ "text": " buy milk " }),
    )
    .unwrap();
    let notes = invoke(&window, "plugin:example|list", Value::Null).unwrap();

    assert_eq!(note["text"], "buy milk");
    assert_eq!(note["createdAt"], "2026-10-08T09:00:00Z");
    assert!(
        note["id"].as_str().is_some_and(|id| id.len() == 36),
        "a hyphenated UUID"
    );
    assert_eq!(notes, json!([note]));
}

#[test]
fn add_emits_note_added_with_the_note() {
    let app = example_app();
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:example:note-added", move |event| {
        sender
            .send(event.payload().to_owned())
            .expect("the test is still listening");
    });

    let note = invoke(
        &main_window(&app),
        "plugin:example|add",
        json!({ "text": "buy milk" }),
    );

    let payload: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(payload, json!({ "note": note.unwrap() }));
}

#[test]
fn invalid_input_is_an_app_error_with_the_invalid_code() {
    let app = example_app();

    let error = invoke(
        &main_window(&app),
        "plugin:example|add",
        json!({ "text": "  " }),
    );

    assert_eq!(
        error,
        Err(json!({ "code": "invalid", "message": "a note cannot be empty" }))
    );
}
