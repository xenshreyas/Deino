//! Contract tests: the JSON the UI sends to and receives from the folders
//! commands, through the real plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::folders::{FilingService, FolderPorts, FolderService, SqliteFolderRepo};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-09T10:00:00Z";

/// A window over a database holding one meeting, `meeting-1`.
fn window() -> WebviewWindow<MockRuntime> {
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed::meeting(&database, "meeting-1", "Standup", at(NOW)).await;
        database
    });
    let repo = Arc::new(SqliteFolderRepo::new(database));
    let ports = FolderPorts::over(repo, Arc::new(FakeClock::new(at(NOW))));
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(FolderService::new(&ports)))
        .manage(Arc::new(FilingService::new(&ports)));
    let app = mock_app(builder);
    main_window(&app)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:folders|{command}"), args)
}

fn create(window: &WebviewWindow<MockRuntime>, name: &str, parent: &Value) -> Value {
    let args = json!({ "folder": { "name": name, "parentId": parent } });
    call(window, "create", args).unwrap()
}

#[test]
fn create_returns_the_folder_with_camel_case_fields() {
    let window = window();

    let folder = create(&window, " Clients ", &Value::Null);

    assert_eq!(folder["name"], "Clients");
    assert_eq!(folder["parentId"], Value::Null);
    assert_eq!(folder["icon"], Value::Null);
    assert_eq!(folder["description"], "");
    assert_eq!(folder["createdAt"], NOW);
}

#[test]
fn the_tree_lists_entries_with_depth_and_counts_plus_the_unfiled_count() {
    let window = window();
    let work = create(&window, "Work", &Value::Null);
    create(&window, "Clients", &work["id"]);
    let filed = call(
        &window,
        "add_meeting",
        json!({ "folderId": work["id"], "meetingId": "meeting-1" }),
    );

    let tree = call(&window, "tree", Value::Null).unwrap();

    assert_eq!(filed, Ok(json!({ "seriesOffer": null })));
    let rows: Vec<(Value, Value, Value)> = tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["folder"]["name"].clone(),
                entry["depth"].clone(),
                entry["meetingCount"].clone(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (json!("Work"), json!(0), json!(1)),
            (json!("Clients"), json!(1), json!(0))
        ]
    );
    assert_eq!(tree["unfiledCount"], 0);
}

#[test]
fn rules_cross_as_tagged_unions() {
    let window = window();
    let work = create(&window, "Work", &Value::Null);
    let rule = json!({ "kind": "attendee_domain", "domain": "@Acme.com" });

    let added = call(
        &window,
        "add_rule",
        json!({ "folderId": work["id"], "rule": rule }),
    )
    .unwrap();
    let listed = call(&window, "rules", json!({ "folderId": work["id"] })).unwrap();

    assert_eq!(
        added["rule"],
        json!({ "kind": "attendee_domain", "domain": "acme.com" })
    );
    assert_eq!(added["seriesTitle"], Value::Null);
    assert_eq!(listed, json!([added.clone()]));
    let removed = call(&window, "remove_rule", json!({ "id": added["id"] }));
    assert_eq!(removed, Ok(Value::Null));
}

#[test]
fn moving_and_listing_meetings_use_folder_and_meeting_ids() {
    let window = window();
    let work = create(&window, "Work", &Value::Null);
    let home = create(&window, "Home", &Value::Null);
    call(
        &window,
        "add_meeting",
        json!({ "folderId": work["id"], "meetingId": "meeting-1" }),
    )
    .unwrap();

    let moved = call(
        &window,
        "move_to",
        json!({ "id": work["id"], "parentId": home["id"] }),
    )
    .unwrap();
    let meetings = call(&window, "meetings", json!({ "id": work["id"] })).unwrap();
    let of_meeting = call(&window, "folders_of", json!({ "meetingId": "meeting-1" })).unwrap();
    let removed = call(
        &window,
        "remove_meeting",
        json!({ "folderId": work["id"], "meetingId": "meeting-1" }),
    );

    assert_eq!(moved["parentId"], home["id"]);
    assert_eq!(meetings[0]["id"], "meeting-1");
    assert_eq!(of_meeting[0]["name"], "Work");
    assert_eq!(removed, Ok(json!({ "seriesRuleRemoved": false })));
    assert_eq!(
        call(&window, "unfiled", Value::Null).unwrap()[0]["id"],
        "meeting-1"
    );
}

#[test]
fn errors_arrive_as_app_errors_with_a_code() {
    let window = window();
    let work = create(&window, "Work", &Value::Null);

    let blank = call(
        &window,
        "create",
        json!({ "folder": { "name": " ", "parentId": null } }),
    );
    let taken = call(
        &window,
        "create",
        json!({ "folder": { "name": "work", "parentId": null } }),
    );
    let cycle = call(
        &window,
        "move_to",
        json!({ "id": work["id"], "parentId": work["id"] }),
    );
    let icon = call(
        &window,
        "set_icon",
        json!({ "id": work["id"], "icon": "Not An Icon" }),
    );

    assert_eq!(
        blank,
        Err(json!({ "code": "invalid", "message": "a folder name cannot be empty" }))
    );
    assert_eq!(
        taken,
        Err(
            json!({ "code": "conflict", "message": "a folder with that name already exists here" })
        )
    );
    assert_eq!(cycle.unwrap_err()["code"], "conflict");
    assert_eq!(icon.unwrap_err()["code"], "invalid");
}
