//! Contract tests: the JSON of the ask commands, through the real plugin,
//! ACL and error mapping, over the seeded fixture library and a mock LLM.

use std::sync::Arc;

use deino_core::ask::{AskDeps, AskService, GatewayAskModel, SqliteAskRepo};
use deino_core::enhance::MaterialSources;
use deino_core::recipes::{RecipeService, SqliteRecipeRepo};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// The sales discovery fixture.
const DISCOVERY: &str = "meeting-fixture-sales-discovery";

/// An app with the `ask` plugin over the fixtures; the model answers every
/// question citing the first transcript line and an invented one.
fn ask_app() -> tauri::App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        database
    });
    let mock = MockProvider::new().answer_with(Route::Chat, |_| "Hi [s1] [s9999].".into());
    let gateway = Arc::new(mock_gateway(Arc::new(mock)).gateway);
    let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let recipes = Arc::new(RecipeService::new(
        Arc::new(SqliteRecipeRepo::new(database.clone())),
        clock.clone(),
    ));
    app.manage(Arc::new(AskService::new(AskDeps {
        sources: MaterialSources::sqlite(&database),
        store: Arc::new(SqliteAskRepo::new(database)),
        recipes,
        model: Arc::new(GatewayAskModel::new(gateway)),
        clock,
    })));
    app
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(&main_window(app), &format!("plugin:ask|{command}"), args)
}

/// What the UI passes for `on_event`: an IPC channel's serialized ID.
fn channel() -> Value {
    json!("__CHANNEL__:1")
}

#[test]
fn ask_returns_the_answer_with_resolved_camel_case_citations_and_saves_the_thread() {
    let app = ask_app();
    let request = json!({ "question": "Who spoke first?", "recipe": null });

    let answer = call(
        &app,
        "ask",
        json!({ "meeting": DISCOVERY, "request": request, "onEvent": channel() }),
    )
    .unwrap();

    assert_eq!(answer["role"], "assistant");
    assert_eq!(answer["parts"][0]["text"], "Hi");
    assert_eq!(answer["parts"][0]["citations"].as_array().unwrap().len(), 1);
    assert!(answer["parts"][0]["citations"][0]["segmentId"].is_string());
    assert_eq!(answer["parts"][1]["text"], ".");
    let thread = call(&app, "thread", json!({ "meeting": DISCOVERY })).unwrap();
    assert_eq!(thread[0]["parts"][0]["text"], "Who spoke first?");
    assert_eq!(thread[0]["recipe"], Value::Null);
}

#[test]
fn clearing_empties_the_thread_and_an_empty_question_is_invalid() {
    let app = ask_app();
    let empty = json!({ "question": " ", "recipe": null });

    let error = call(
        &app,
        "ask",
        json!({ "meeting": DISCOVERY, "request": empty, "onEvent": channel() }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "invalid");
    assert_eq!(
        call(&app, "clear", json!({ "meeting": DISCOVERY })).unwrap(),
        Value::Null
    );
    assert_eq!(
        call(&app, "thread", json!({ "meeting": DISCOVERY })).unwrap(),
        json!([])
    );
}
