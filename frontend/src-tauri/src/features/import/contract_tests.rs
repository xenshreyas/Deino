//! Contract tests: the JSON of the `import` commands through the real
//! plugin, ACL and error mapping, over SQLite and a temporary folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::import::MediaPicker;
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::watch::{FolderPicker, WatchScheduler};
use deino_domain::Clock;
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, WebviewWindow};

use crate::composition::import::Imports;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// Dialogs that always choose the same paths.
struct Chooses {
    file: PathBuf,
    folder: PathBuf,
}

#[async_trait]
impl MediaPicker for Chooses {
    async fn pick_media(&self) -> Option<PathBuf> {
        Some(self.file.clone())
    }
}

#[async_trait]
impl FolderPicker for Chooses {
    async fn pick_folder(&self) -> Option<PathBuf> {
        Some(self.folder.clone())
    }
}

/// An app with the `import` plugin whose dialogs choose `dir/memo.m4a` and
/// `dir/inbox`.
fn import_app(dir: &Path) -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let lifecycle = Arc::new(MeetingLifecycleService::new(meetings, clock.clone()));
    let dialogs = Arc::new(Chooses {
        file: dir.join("memo.m4a"),
        folder: dir.join("inbox"),
    });
    let imports = Imports::build(
        &database,
        &clock,
        (dir.join("recordings"), lifecycle),
        (dialogs.clone(), dialogs),
    );
    let (_scheduler, nudge, _cancel) = WatchScheduler::new(imports.watch.clone());
    imports.manage(&app, nudge);
    app
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:import|{command}"), args)
}

#[test]
fn a_picked_file_is_checked_and_described_in_camel_case() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("memo.m4a"), b"audio").unwrap();
    let app = import_app(dir.path());

    let picked = call(&main_window(&app), "pick_file", json!({})).unwrap();

    assert_eq!(picked["fileName"], "memo.m4a");
    assert_eq!(picked["kind"], "audio");
    assert!(picked["path"].as_str().unwrap().ends_with("memo.m4a"));
}

#[test]
fn files_imports_cannot_take_map_to_error_codes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();
    std::fs::write(dir.path().join("fake.mp3"), b"text").unwrap();
    let app = import_app(dir.path());
    let window = main_window(&app);
    let path = |name: &str| dir.path().join(name).to_string_lossy().into_owned();

    let text = call(&window, "check", json!({ "path": path("notes.txt") })).unwrap_err();
    let missing = call(&window, "check", json!({ "path": path("gone.mov") })).unwrap_err();
    let broken = call(
        &window,
        "start",
        json!({ "path": path("fake.mp3"), "title": null }),
    )
    .unwrap_err();

    assert_eq!(text["code"], "invalid");
    assert_eq!(missing["code"], "not_found");
    assert_eq!(broken["code"], "invalid");
}

#[test]
fn watched_folders_are_added_toggled_listed_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("inbox")).unwrap();
    let app = import_app(dir.path());
    let window = main_window(&app);

    let added = call(&window, "add_watched_folder", json!({})).unwrap();
    let id = added["id"].clone();
    let again = call(&window, "add_watched_folder", json!({})).unwrap_err();
    call(
        &window,
        "set_move_to_processed",
        json!({ "id": id, "on": true }),
    )
    .unwrap();
    let listed = call(&window, "watched_folders", json!({})).unwrap();

    assert!(added["path"].as_str().unwrap().ends_with("inbox"));
    assert_eq!(added["moveToProcessed"], false);
    assert!(added["createdAt"].is_string());
    assert_eq!(again["code"], "conflict");
    assert_eq!(listed[0]["moveToProcessed"], true);
    assert_eq!(call(&window, "watch_failures", json!({})), Ok(json!([])));
    assert_eq!(
        call(&window, "remove_watched_folder", json!({ "id": id })),
        Ok(Value::Null)
    );
    assert_eq!(call(&window, "watched_folders", json!({})), Ok(json!([])));
}

#[test]
fn unknown_failures_are_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let app = import_app(dir.path());
    let window = main_window(&app);
    let id = json!({ "id": "0192a5f0-0000-7000-8000-000000000000" });

    let retry = call(&window, "retry_failure", id.clone()).unwrap_err();
    let dismiss = call(&window, "dismiss_failure", id).unwrap_err();

    assert_eq!(retry["code"], "not_found");
    assert_eq!(dismiss["code"], "not_found");
}
