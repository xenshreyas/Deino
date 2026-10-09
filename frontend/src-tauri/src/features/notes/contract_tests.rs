//! Contract tests: every notes command's JSON in and out, and the typed
//! event, through the real plugin and ACL on Tauri's mock runtime.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use deino_core::meetings::{FakeMeetingRepo, MeetingLifecycleRepo, MeetingTitle, NewMeeting};
use deino_core::notes::{FakeNotesRepo, MemoryNoteImages, NotePorts, NoteService};
use deino_domain::{MeetingId, MeetingSource, MeetingStatus};
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager, WebviewWindow};

use super::{DroppedFiles, TauriNotesEvents};
use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-08T09:00:00Z";
/// "\x89PNG\r\n\x1a\nimage" in base64.
const PNG_BASE64: &str = "iVBORw0KGgppbWFnZQ==";
/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

fn notes_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let meetings = FakeMeetingRepo::default();
    let standup = NewMeeting {
        id: MeetingId::new("meeting-1"),
        title: MeetingTitle::parse("Standup").unwrap(),
        status: MeetingStatus::Ready,
        source: MeetingSource::Live,
        created_at: at(NOW),
        started_at: None,
        folder_path: None,
    };
    tauri::async_runtime::block_on(meetings.create(&standup)).unwrap();
    let ports = NotePorts {
        notes: Arc::new(FakeNotesRepo::default()),
        meetings: Arc::new(meetings),
        images: Arc::new(MemoryNoteImages::default()),
        events: Arc::new(TauriNotesEvents::new(app.handle().clone())),
    };
    let clock = Arc::new(FakeClock::new(at(NOW)));
    app.manage(Arc::new(NoteService::new(ports, clock)));
    app.manage(Arc::new(DroppedFiles::default()));
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:notes|{command}"), args)
}

#[test]
fn get_returns_an_empty_document_and_the_meeting_folder() {
    let app = notes_app();

    let notes = call(
        &main_window(&app),
        "get",
        json!({ "meetingId": "meeting-1" }),
    );

    assert_eq!(
        notes,
        Ok(json!({
            "meetingId": "meeting-1",
            "doc": { "type": "doc", "content": [{ "type": "paragraph", "content": [] }] },
            "updatedAt": null,
            "folder": "/meetings/meeting-1",
        }))
    );
}

#[test]
fn save_takes_editor_json_and_announces_the_save() {
    let app = notes_app();
    let window = main_window(&app);
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:notes:notes-saved", move |event| {
        sender.send(event.payload().to_owned()).unwrap();
    });
    let doc = json!({ "type": "doc", "content": [
        { "type": "taskList", "content": [{ "type": "taskItem", "attrs": { "checked": false },
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "send deck" }] }] }] }
    ] });

    let saved = call(
        &window,
        "save",
        json!({ "meetingId": "meeting-1", "doc": doc }),
    )
    .unwrap();
    let event: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    let opened = call(&window, "get", json!({ "meetingId": "meeting-1" })).unwrap();

    assert_eq!(
        saved,
        json!({ "meetingId": "meeting-1", "updatedAt": "2026-10-08T09:00:00Z" })
    );
    assert_eq!(event, json!({ "saved": saved }));
    let item = &opened["doc"]["content"][0]["content"][0];
    assert_eq!(
        item["content"][0]["content"][0],
        json!({ "type": "text", "text": "send deck", "marks": [] })
    );
}

#[test]
fn bad_input_arrives_as_invalid_or_not_found() {
    let app = notes_app();
    let window = main_window(&app);
    let empty_text = json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "" }] }] });

    let invalid = call(
        &window,
        "save",
        json!({ "meetingId": "meeting-1", "doc": empty_text }),
    )
    .unwrap_err();
    let missing = call(&window, "get", json!({ "meetingId": "meeting-x" })).unwrap_err();
    let blank_title = call(&window, "create_scheduled", json!({ "title": " " })).unwrap_err();

    assert_eq!(invalid["code"], "invalid");
    assert_eq!(
        missing,
        json!({ "code": "not_found", "message": "meeting meeting-x was not found" })
    );
    assert_eq!(blank_title["code"], "invalid");
}

#[test]
fn images_arrive_as_base64_or_as_a_dropped_path() {
    let app = notes_app();
    let window = main_window(&app);

    let stored = call(
        &window,
        "save_image",
        json!({ "meetingId": "meeting-1", "base64": PNG_BASE64 }),
    )
    .unwrap();
    let garbage = call(
        &window,
        "save_image",
        json!({ "meetingId": "meeting-1", "base64": "%%%" }),
    )
    .unwrap_err();
    let undropped = call(
        &window,
        "attach_dropped_image",
        json!({ "meetingId": "meeting-1", "path": "/etc/hosts" }),
    )
    .unwrap_err();

    let src = stored["src"].as_str().unwrap();
    assert!(
        src.starts_with("images/") && src.len() == "images/".len() + 36 + 4,
        "{src}"
    );
    assert_eq!(garbage["code"], "invalid");
    assert_eq!(
        undropped,
        json!({ "code": "invalid", "message": "that file was not dropped on the window" })
    );
}

#[test]
fn scheduled_meetings_and_markdown_conversion() {
    let app = notes_app();
    let window = main_window(&app);

    let meeting = call(
        &window,
        "create_scheduled",
        json!({ "title": "Board prep" }),
    )
    .unwrap();
    let doc = call(
        &window,
        "markdown_to_doc",
        json!({ "markdown": "# Plan\n\n- [x] **ship**" }),
    )
    .unwrap();
    let markdown = call(&window, "doc_to_markdown", json!({ "doc": doc.clone() })).unwrap();

    assert_eq!(meeting["title"], "Board prep");
    assert_eq!(doc["content"][1]["type"], "taskList");
    assert_eq!(markdown, json!("# Plan\n\n- [x] **ship**\n"));
}
