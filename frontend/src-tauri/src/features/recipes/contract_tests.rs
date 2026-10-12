//! Contract tests: the JSON of the recipes commands, through the real
//! plugin, ACL and error mapping.

use std::sync::Arc;

use deino_core::recipes::{LIBRARY, RecipeService, SqliteRecipeRepo};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn recipes_app() -> tauri::App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let service = RecipeService::new(
        Arc::new(SqliteRecipeRepo::new(database)),
        Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
    );
    app.manage(Arc::new(service));
    app
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &main_window(app),
        &format!("plugin:recipes|{command}"),
        args,
    )
}

fn draft(name: &str) -> Value {
    json!({ "draft": { "name": name, "description": "", "prompt": "Tell me the plan." } })
}

#[test]
fn list_returns_the_library_in_camel_case() {
    let app = recipes_app();

    let list = call(&app, "list", json!({})).unwrap();

    let all = list.as_array().unwrap();
    assert_eq!(all.len(), LIBRARY.len());
    assert_eq!(all[0]["slug"], "coach-me");
    assert_eq!(all[0]["builtIn"], true);
    assert_eq!(all[0]["kind"], "single");
    assert_eq!(all[0]["createdAt"], "2026-10-12T09:00:00Z");
}

#[test]
fn a_users_recipe_is_created_edited_shared_imported_and_removed() {
    let app = recipes_app();

    let created = call(&app, "create", draft("Plan")).unwrap();
    let id = created["id"].clone();
    let edited = call(
        &app,
        "update",
        json!({ "id": id, "draft": draft("Roadmap")["draft"] }),
    );
    let link = call(&app, "share_link", json!({ "id": id })).unwrap();
    let imported = call(&app, "import_shared", json!({ "text": link })).unwrap();

    assert_eq!(edited.unwrap()["slug"], "roadmap");
    assert!(link.as_str().unwrap().starts_with("deino://recipe?data="));
    assert_eq!(imported[0]["slug"], "roadmap-2");
    let file = call(&app, "export_json", json!({ "ids": [] })).unwrap();
    assert!(file.as_str().unwrap().contains("\"deino.recipes\""));
    assert_eq!(
        call(&app, "remove", json!({ "id": id })).unwrap(),
        Value::Null
    );
}

#[test]
fn built_ins_are_read_only_and_bad_imports_are_invalid() {
    let app = recipes_app();
    let list = call(&app, "list", json!({})).unwrap();
    let builtin = list[0]["id"].clone();

    let error = call(
        &app,
        "update",
        json!({ "id": builtin, "draft": draft("x")["draft"] }),
    )
    .unwrap_err();
    let remix = call(&app, "remix", json!({ "id": builtin })).unwrap();
    let bad = call(&app, "import_shared", json!({ "text": "hello" })).unwrap_err();

    assert_eq!(error["code"], "permission_denied");
    assert_eq!(remix["slug"], "coach-me-remix");
    assert_eq!(bad["code"], "invalid");
}
