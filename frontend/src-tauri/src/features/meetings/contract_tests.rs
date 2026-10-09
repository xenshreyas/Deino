//! Contract tests: the JSON the UI sends to and receives from each meetings
//! command, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use deino_core::meetings::{FakeMeetingRepo, Meeting, MeetingService};
use deino_domain::MeetingId;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const SAVED_AT: &str = "2026-10-08T09:00:00Z";

fn window_with_standup() -> WebviewWindow<MockRuntime> {
    let standup = Meeting {
        id: MeetingId::new("meeting-1"),
        title: "Standup".into(),
        created_at: at(SAVED_AT),
        updated_at: at(SAVED_AT),
    };
    let repo = Arc::new(FakeMeetingRepo::with([standup]));
    let clock = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));
    let service = Arc::new(MeetingService::new(repo, clock));
    let app = mock_app(mock_builder().plugin(super::init()).manage(service));
    main_window(&app)
}

#[test]
fn list_returns_meetings_with_camel_case_fields() {
    let meetings = invoke(&window_with_standup(), "plugin:meetings|list", Value::Null);

    assert_eq!(
        meetings,
        Ok(json!([{
            "id": "meeting-1",
            "title": "Standup",
            "createdAt": "2026-10-08T09:00:00Z",
            "updatedAt": "2026-10-08T09:00:00Z",
        }]))
    );
}

#[test]
fn rename_takes_id_and_title_and_returns_the_renamed_meeting() {
    let args = json!({ "id": "meeting-1", "title": " Planning " });

    let renamed = invoke(&window_with_standup(), "plugin:meetings|rename", args).unwrap();

    assert_eq!(renamed["title"], "Planning");
    assert_eq!(renamed["updatedAt"], "2026-10-09T10:00:00Z");
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let window = window_with_standup();

    let blank = invoke(
        &window,
        "plugin:meetings|rename",
        json!({ "id": "meeting-1", "title": "" }),
    );
    let missing = invoke(&window, "plugin:meetings|get", json!({ "id": "meeting-x" }));

    assert_eq!(
        blank,
        Err(json!({ "code": "invalid", "message": "a meeting title cannot be empty" }))
    );
    assert_eq!(
        missing,
        Err(json!({ "code": "not_found", "message": "meeting meeting-x was not found" }))
    );
}
