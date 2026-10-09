//! Contract tests: the JSON of the tags commands and the `MeetingTagged`
//! event, through the real plugin, ACL and error mapping, over SQLite.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use deino_core::tags::{SqliteTagRepo, TagPorts, TagService};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Listener, Manager, WebviewWindow};

use super::TauriTagEvents;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);
const NOW: &str = "2026-10-09T10:00:00Z";

/// An app with the `tags` plugin over a database holding `meeting-1`.
fn tags_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "meeting-1", "Standup", at(NOW)).await;
        database
    });
    let repo = Arc::new(SqliteTagRepo::new(database));
    let service = TagService::new(TagPorts {
        tags: repo.clone(),
        applied: repo,
        events: Arc::new(TauriTagEvents::new(app.handle().clone())),
        clock: Arc::new(FakeClock::new(at(NOW))),
    });
    app.manage(Arc::new(service));
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:tags|{command}"), args)
}

fn create(window: &WebviewWindow<MockRuntime>, name: &str) -> Value {
    let tag = json!({ "name": name, "color": null, "description": "Customer calls" });
    call(window, "create", json!({ "tag": tag })).unwrap()
}

#[test]
fn create_returns_the_tag_with_its_palette_slot_and_camel_case_fields() {
    let app = tags_app();

    let tag = create(&main_window(&app), " Customer ");

    assert_eq!(tag["name"], "Customer");
    assert_eq!(tag["color"], 1);
    assert_eq!(tag["description"], "Customer calls");
    assert_eq!(tag["autoApply"], true);
    assert_eq!(tag["createdAt"], NOW);
}

#[test]
fn update_takes_every_editable_field() {
    let app = tags_app();
    let window = main_window(&app);
    let tag = create(&window, "Customer");
    let update = json!({ "name": "Customers", "color": 4, "description": "", "autoApply": false });

    let updated = call(
        &window,
        "update",
        json!({ "id": tag["id"], "update": update }),
    )
    .unwrap();

    assert_eq!(
        (updated["name"].clone(), updated["color"].clone()),
        (json!("Customers"), json!(4))
    );
    assert_eq!(updated["autoApply"], false);
    assert_eq!(call(&window, "list", Value::Null), Ok(json!([updated])));
}

#[test]
fn applying_by_hand_lists_the_tag_as_the_users_and_emits_meeting_tagged() {
    let app = tags_app();
    let window = main_window(&app);
    let tag = create(&window, "Customer");
    let (sender, received) = mpsc::channel();
    app.listen_any("plugin:tags:meeting-tagged", move |event| {
        sender
            .send(event.payload().to_owned())
            .expect("the test is still listening");
    });

    let applied = call(
        &window,
        "apply",
        json!({ "meetingId": "meeting-1", "tagId": tag["id"] }),
    );
    let on_meeting = call(&window, "tags_of", json!({ "meetingId": "meeting-1" })).unwrap();
    let links = call(&window, "links", Value::Null).unwrap();

    assert_eq!(applied, Ok(Value::Null));
    assert_eq!(on_meeting[0]["appliedBy"], "user");
    assert_eq!(on_meeting[0]["tag"]["id"], tag["id"]);
    assert_eq!(
        links,
        json!([{ "meetingId": "meeting-1", "tagId": tag["id"], "appliedBy": "user" }])
    );
    let payload: Value =
        serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
    assert_eq!(
        payload,
        json!({ "meetingId": "meeting-1", "tagId": tag["id"], "appliedBy": "user" })
    );
    let removed = call(
        &window,
        "remove",
        json!({ "meetingId": "meeting-1", "tagId": tag["id"] }),
    );
    assert_eq!(removed, Ok(Value::Null));
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let app = tags_app();
    let window = main_window(&app);
    let tag = create(&window, "Customer");

    let taken = call(
        &window,
        "create",
        json!({ "tag": { "name": "customer", "color": null, "description": "" } }),
    );
    let colour = call(
        &window,
        "create",
        json!({ "tag": { "name": "Bad", "color": 9, "description": "" } }),
    );
    let no_meeting = call(
        &window,
        "apply",
        json!({ "meetingId": "meeting-x", "tagId": tag["id"] }),
    );

    assert_eq!(
        taken,
        Err(json!({ "code": "conflict", "message": "a tag with that name already exists" }))
    );
    assert!(colour.is_err());
    assert_eq!(
        no_meeting,
        Err(json!({ "code": "not_found", "message": "meeting meeting-x was not found" }))
    );
    let deleted = call(&window, "delete", json!({ "id": tag["id"] }));
    assert_eq!(deleted, Ok(Value::Null));
}
