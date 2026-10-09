//! Contract tests: what the UI can do with keys, and that it can never read
//! one back, through the real plugin and ACL.

use std::sync::Arc;

use deino_secrets::{InMemorySecretStore, SecretName, SecretStore};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window_with(store: Arc<InMemorySecretStore>) -> WebviewWindow<MockRuntime> {
    let secrets: Arc<dyn SecretStore> = store;
    let app = mock_app(mock_builder().plugin(super::init()).manage(secrets));
    main_window(&app)
}

fn status(window: &WebviewWindow<MockRuntime>, name: &str) -> Result<Value, Value> {
    invoke(window, "plugin:secrets|status", json!({ "name": name }))
}

#[test]
fn a_key_is_saved_reported_and_forgotten_but_never_returned() {
    let store = Arc::new(InMemorySecretStore::default());
    let window = window_with(store.clone());

    let saved = invoke(
        &window,
        "plugin:secrets|set",
        json!({ "name": "openai_api_key", "value": " sk-123 " }),
    );
    let stored = status(&window, "openai_api_key");
    let deleted = invoke(
        &window,
        "plugin:secrets|delete",
        json!({ "name": "openai_api_key" }),
    );

    assert_eq!(saved, Ok(Value::Null));
    assert_eq!(stored, Ok(json!("stored")));
    assert_eq!(deleted, Ok(Value::Null));
    assert_eq!(status(&window, "openai_api_key"), Ok(json!("missing")));
    assert_eq!(store.stored(SecretName::OpenAiApiKey), None);
}

#[test]
fn there_is_no_command_that_reads_a_key() {
    let window = window_with(Arc::new(InMemorySecretStore::default()));

    let read = invoke(
        &window,
        "plugin:secrets|get",
        json!({ "name": "openai_api_key" }),
    );

    assert!(read.is_err(), "the UI must never read a key back");
}

#[test]
fn a_key_macos_refuses_to_hand_over_is_locked() {
    let store = Arc::new(InMemorySecretStore::default());
    let window = window_with(store.clone());
    let save = json!({ "name": "anthropic_api_key", "value": "sk-ant" });
    invoke(&window, "plugin:secrets|set", save).unwrap();

    store.deny(SecretName::AnthropicApiKey);

    assert_eq!(status(&window, "anthropic_api_key"), Ok(json!("locked")));
}

#[test]
fn blank_keys_and_unknown_names_are_rejected() {
    let window = window_with(Arc::new(InMemorySecretStore::default()));

    let blank = invoke(
        &window,
        "plugin:secrets|set",
        json!({ "name": "groq_api_key", "value": "  " }),
    );
    let unknown = status(&window, "github_token");

    assert_eq!(
        blank,
        Err(json!({ "code": "invalid", "message": "a key cannot be empty" }))
    );
    assert!(unknown.is_err());
}
