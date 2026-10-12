//! Contract tests: the JSON of the `comments` commands through the real
//! plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::comments::{CommentService, SqliteCommentRepo};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-1";

/// An app with the `comments` plugin over one meeting.
fn comments_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
        database
    });
    let clock = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
    let service = CommentService::new(Arc::new(SqliteCommentRepo::new(database)), clock);
    app.manage(Arc::new(service));
    app
}

/// `plugin:comments|<command>` with `args`.
fn call(app: &App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &main_window(app),
        &format!("plugin:comments|{command}"),
        args,
    )
}

/// A notes thread on "the plan".
fn notes_thread() -> Value {
    json!({ "thread": {
        "meetingId": MEETING,
        "anchor": { "target": "notes", "segmentId": null, "start": 4, "end": 12, "quote": "the plan" },
        "body": "Is this final?",
    } })
}

#[test]
fn start_takes_a_camel_case_anchor_and_returns_the_thread() {
    let app = comments_app();

    let thread = call(&app, "start", notes_thread()).unwrap();

    assert_eq!(thread["meetingId"], MEETING);
    assert_eq!(thread["anchor"]["target"], "notes");
    assert_eq!(thread["anchor"]["quote"], "the plan");
    assert_eq!(thread["resolvedAt"], Value::Null);
    assert_eq!(thread["comments"][0]["body"], "Is this final?");
    assert_eq!(
        call(&app, "list", json!({ "meetingId": MEETING })).unwrap(),
        json!([thread])
    );
}

#[test]
fn reply_edit_resolve_and_delete_take_their_ids() {
    let app = comments_app();
    let thread = call(&app, "start", notes_thread()).unwrap();
    let thread_id = thread["id"].clone();

    let reply = call(
        &app,
        "reply",
        json!({ "threadId": thread_id, "body": "Yes" }),
    )
    .unwrap();
    call(
        &app,
        "edit",
        json!({ "id": reply["id"], "body": "Yes, signed off" }),
    )
    .unwrap();
    call(
        &app,
        "set_resolved",
        json!({ "threadId": thread_id, "resolved": true }),
    )
    .unwrap();
    let listed = call(&app, "list", json!({ "meetingId": MEETING })).unwrap();
    call(&app, "delete", json!({ "id": reply["id"] })).unwrap();

    assert_eq!(listed[0]["comments"][1]["body"], "Yes, signed off");
    assert!(listed[0]["resolvedAt"].is_string());
}

#[test]
fn an_empty_comment_is_an_invalid_error() {
    let app = comments_app();
    let mut args = notes_thread();
    args["thread"]["body"] = json!("   ");

    let error = call(&app, "start", args).unwrap_err();

    assert_eq!(error["code"], "invalid");
}
