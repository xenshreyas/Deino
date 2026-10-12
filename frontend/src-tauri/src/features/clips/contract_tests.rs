//! Contract tests: the JSON of the `clips` commands through the real plugin,
//! ACL and error mapping, over SQLite, with in-memory files and cutter.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::clips::{
    ClipDeps, ClipMedia, ClipService, ClipSource, FakeClipCutter, FakeClipFiles, FakeClipSources,
    FakeFinder, SqliteClipRepo,
};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-1";

/// An app with the `clips` plugin over a meeting recorded in `/r/meeting-1`.
fn clips_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
        database
    });
    let files = Arc::new(FakeClipFiles::under("/r"));
    let source = ClipSource {
        path: PathBuf::from("/r/meeting-1/audio.mp4"),
        media: ClipMedia::Audio,
    };
    let service = ClipService::new(ClipDeps {
        repo: Arc::new(SqliteClipRepo::new(database)),
        sources: Arc::new(FakeClipSources::at(source)),
        files: files.clone(),
        cutter: Arc::new(FakeClipCutter::writing_to(files)),
        finder: Arc::new(FakeFinder::default()),
        clock: Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z"))),
    });
    app.manage(Arc::new(service));
    app
}

/// `plugin:clips|<command>` with `args`.
fn call(app: &App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(&main_window(app), &format!("plugin:clips|{command}"), args)
}

#[test]
fn create_takes_a_camel_case_clip_and_returns_it_with_its_file() {
    let app = clips_app();

    let clip = call(
        &app,
        "create",
        json!({ "clip": { "meetingId": MEETING, "name": "Pricing", "start": 3000, "end": 7500 } }),
    )
    .unwrap();

    assert_eq!(clip["meetingId"], MEETING);
    assert_eq!(clip["meetingTitle"], "Standup");
    assert_eq!(
        (clip["start"].as_u64(), clip["end"].as_u64()),
        (Some(3000), Some(7500))
    );
    assert_eq!(clip["media"], "audio");
    assert_eq!(clip["path"], "/r/meeting-1/clips/Pricing.m4a");
    let listed = call(&app, "list", json!({ "meetingId": MEETING })).unwrap();
    assert_eq!(listed, json!([clip]));
}

#[test]
fn rename_reveal_and_delete_take_the_clip_id() {
    let app = clips_app();
    let clip = call(
        &app,
        "create",
        json!({ "clip": { "meetingId": MEETING, "name": "Intro", "start": 0, "end": 1000 } }),
    )
    .unwrap();
    let id = clip["id"].clone();

    let renamed = call(&app, "rename", json!({ "id": id, "name": "Outro" })).unwrap();
    call(&app, "reveal", json!({ "id": id })).unwrap();
    call(&app, "delete", json!({ "id": id })).unwrap();

    assert_eq!(renamed["name"], "Outro");
    assert_eq!(call(&app, "list_all", json!({})).unwrap(), json!([]));
}

#[test]
fn an_empty_range_is_an_invalid_error() {
    let app = clips_app();

    let error = call(
        &app,
        "create",
        json!({ "clip": { "meetingId": MEETING, "name": "Intro", "start": 10, "end": 10 } }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "invalid");
}
