//! Contract tests: the JSON Settings → Developer sends and receives for API
//! keys, through the real plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::api_keys::{ApiKeyDeps, ApiKeyService, Argon2Hasher, OsTokens, SqliteApiKeyRepo};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window() -> WebviewWindow<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let keys = ApiKeyService::new(ApiKeyDeps {
        repo: Arc::new(SqliteApiKeyRepo::new(database)),
        hasher: Arc::new(Argon2Hasher),
        tokens: Arc::new(OsTokens),
        clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
    });
    app.manage(Arc::new(keys));
    main_window(&app)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:api|{command}"), args)
}

#[test]
fn a_new_key_comes_back_once_with_its_token_and_lists_without_it() {
    let window = window();

    let created = call(
        &window,
        "create_key",
        json!({ "key": { "name": "Raycast", "scope": "read_write" } }),
    )
    .unwrap();
    let listed = call(&window, "list_keys", json!({})).unwrap();

    assert!(created["token"].as_str().unwrap().starts_with("deino_"));
    assert_eq!(created["key"]["scope"], "read_write");
    assert_eq!(created["key"]["createdAt"], "2026-10-12T09:00:00Z");
    assert_eq!(listed, json!([created["key"]]));
    assert!(listed[0].get("token").is_none());
    assert!(listed[0].get("secretHash").is_none());
}

#[test]
fn revoke_stamps_the_key_and_delete_removes_it() {
    let window = window();
    let created = call(
        &window,
        "create_key",
        json!({ "key": { "name": "Cron", "scope": "read" } }),
    )
    .unwrap();
    let id = created["key"]["id"].clone();

    let revoked = call(&window, "revoke_key", json!({ "id": id })).unwrap();
    call(&window, "delete_key", json!({ "id": id })).unwrap();
    let again = call(&window, "delete_key", json!({ "id": id })).unwrap_err();

    assert_eq!(revoked["revokedAt"], "2026-10-12T09:00:00Z");
    assert_eq!(call(&window, "list_keys", json!({})).unwrap(), json!([]));
    assert_eq!(again["code"], "not_found");
}

#[test]
fn a_blank_name_is_invalid() {
    let window = window();

    let error = call(
        &window,
        "create_key",
        json!({ "key": { "name": "  ", "scope": "read" } }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "invalid");
}
