//! Contract tests: the JSON the UI sends to and receives from each `llm`
//! command, through the real plugin, ACL and error mapping, over fakes.

use std::collections::BTreeMap;
use std::sync::Arc;

use deino_llm::{
    AdminParts, FakeHttp, LlmAdmin, MockProvider, ModelCatalog, OllamaProvider, ProviderId, Route,
    StaticCredentials, mock_gateway,
};
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window(credentials: StaticCredentials) -> WebviewWindow<MockRuntime> {
    let credentials = Arc::new(credentials);
    let tags = r#"{"models":[{"name":"gemma3:1b","size":815319791}]}"#;
    let ollama = Arc::new(OllamaProvider::new(
        Arc::new(FakeHttp::new().reply(200, tags)),
        credentials.clone(),
        credentials.clone(),
    ));
    let catalogs: BTreeMap<ProviderId, Arc<dyn ModelCatalog>> =
        BTreeMap::from([(ProviderId::Ollama, ollama.clone() as Arc<dyn ModelCatalog>)]);
    let admin = LlmAdmin::new(AdminParts {
        gateway: Arc::new(mock_gateway(Arc::new(MockProvider::new())).gateway),
        catalogs,
        credentials: credentials.clone(),
        endpoints: credentials,
        ollama,
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
    });
    let app = mock_app(mock_builder().plugin(super::init()).manage(Arc::new(admin)));
    main_window(&app)
}

#[test]
fn providers_report_masked_keys_in_camel_case() {
    let window =
        window(StaticCredentials::new().key(ProviderId::Anthropic, "sk-ant-api03-abcdef7f3c"));

    let providers = invoke(&window, "plugin:llm|providers", Value::Null).unwrap();

    assert_eq!(
        providers[0],
        json!({
            "id": "anthropic",
            "name": "Anthropic",
            "requiresKey": true,
            "maskedKey": "sk-a…7f3c",
            "endpoint": null,
        })
    );
    assert!(
        !providers.to_string().contains("abcdef"),
        "the key never crosses IPC"
    );
}

#[test]
fn models_list_a_provider_s_models() {
    let models = invoke(
        &window(StaticCredentials::new()),
        "plugin:llm|models",
        json!({ "provider": "ollama", "apiKey": null, "endpoint": null }),
    )
    .unwrap();

    assert_eq!(
        models,
        json!([{
            "id": "gemma3:1b",
            "displayName": null,
            "contextLength": null,
            "sizeBytes": 815_319_791,
            "modifiedAt": null,
            "ready": true,
        }])
    );
}

#[test]
fn routes_list_every_route_with_its_target_and_source() {
    let routes = invoke(
        &window(StaticCredentials::new()),
        "plugin:llm|routes",
        Value::Null,
    )
    .unwrap();

    assert_eq!(routes.as_array().unwrap().len(), Route::ALL.len());
    assert_eq!(
        routes[0],
        json!({
            "route": "notes.enhance",
            "tier": "smart",
            "target": { "provider": "builtin", "model": "mock-local" },
            "source": "default",
        })
    );
}

#[test]
fn set_route_pins_a_target_and_rejects_pinning_embed() {
    let window = window(StaticCredentials::new());
    let target = json!({ "provider": "openai", "model": "gpt-5" });

    let pinned = invoke(
        &window,
        "plugin:llm|set_route",
        json!({ "route": "chat", "target": target }),
    )
    .unwrap();
    let embed = invoke(
        &window,
        "plugin:llm|set_route",
        json!({ "route": "embed", "target": target }),
    );

    assert_eq!(pinned["target"], target);
    assert_eq!(pinned["source"], "custom");
    assert_eq!(
        embed,
        Err(
            json!({ "code": "invalid", "message": "Built-in does not support choosing the embedding model" })
        )
    );
}

#[test]
fn a_missing_catalog_is_an_invalid_request() {
    let error = invoke(
        &window(StaticCredentials::new()),
        "plugin:llm|models",
        json!({ "provider": "anthropic", "apiKey": "sk-typed", "endpoint": null }),
    )
    .unwrap_err();

    assert_eq!(error["code"], "invalid");
}
