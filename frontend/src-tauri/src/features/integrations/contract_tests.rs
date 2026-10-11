//! Contract tests: the JSON of the integrations commands through the real
//! plugin, ACL and error mapping, over SQLite, an in-memory Keychain and the
//! sandbox provider (which sends nothing).

use std::sync::Arc;

use deino_integrations::{
    CredentialVault, FakeBrowser, FakeEgress, FakeOAuthApps, FakeSleeper, IntegrationPorts,
    IntegrationService, RegistryOptions, SqliteConnectionRepo, SqliteDeliveryRepo, registry,
};
use deino_net::FakeNetClient;
use deino_secrets::InMemoryCredentialStore;
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-11T09:00:00Z";
const TOKEN: &str = "sandbox-s3cret-token";

/// An app with the `integrations` plugin over the sandbox provider.
fn integrations_app() -> (App<MockRuntime>, Arc<InMemoryCredentialStore>) {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(test_db());
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let keychain = Arc::new(InMemoryCredentialStore::default());
    let service = IntegrationService::new(IntegrationPorts {
        registry: registry::integrations(RegistryOptions { sandbox: true }),
        connections: Arc::new(SqliteConnectionRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database)),
        vault: CredentialVault::new(keychain.clone()),
        apps: Arc::new(FakeOAuthApps::default()),
        egress: Arc::new(FakeEgress::new(Arc::new(FakeNetClient::default()))),
        browser: Arc::new(FakeBrowser::default()),
        clock: clock.clone(),
        sleeper: Arc::new(FakeSleeper::new(clock)),
    });
    app.manage(Arc::new(service));
    (app, keychain)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:integrations|{command}"), args)
}

/// Connects the sandbox and returns the connection's JSON.
fn connect(window: &WebviewWindow<MockRuntime>) -> Value {
    let input = json!({ "token": TOKEN, "username": null });
    call(
        window,
        "connect_token",
        json!({ "provider": "sandbox", "input": input }),
    )
    .unwrap()
}

#[test]
fn providers_list_shipped_ones_first_then_the_planned_ones() {
    let (app, _) = integrations_app();

    let providers = call(&main_window(&app), "providers", Value::Null).unwrap();

    let providers = providers.as_array().unwrap();
    assert_eq!(
        providers[0],
        json!({
            "id": "sandbox",
            "name": "Sandbox",
            "available": true,
            "capabilities": ["post_message", "create_page", "create_task", "log_activity", "read_context"],
            "methods": [{
                "method": "token",
                "label": "Sandbox token",
                "usernameLabel": null,
                "helpUrl": "https://example.invalid/sandbox",
            }],
            "app": null,
            "connections": [],
        })
    );
    assert_eq!(
        providers[1],
        json!({
            "id": "slack",
            "name": "Slack",
            "available": false,
            "capabilities": ["post_message"],
            "methods": [],
            "app": null,
            "connections": [],
        })
    );
    assert!(
        providers
            .iter()
            .skip(1)
            .all(|provider| provider["available"] == false)
    );
}

#[test]
fn a_pasted_token_connects_and_never_comes_back() {
    let (app, keychain) = integrations_app();
    let window = main_window(&app);

    let connection = connect(&window);
    let providers = call(&window, "providers", Value::Null).unwrap();

    assert_eq!(connection["provider"], "sandbox");
    assert_eq!(connection["accountLabel"], "Sandbox workspace");
    assert_eq!(connection["authMethod"], "token");
    assert_eq!(connection["status"], "connected");
    assert_eq!(connection["lastError"], Value::Null);
    assert!(connection.get("apiBaseUrl").is_none());
    assert!(!providers.to_string().contains(TOKEN));
    assert!(
        keychain
            .stored(&keychain.keys()[0])
            .unwrap()
            .contains(TOKEN)
    );
}

#[test]
fn a_delivery_is_logged_once_per_key() {
    let (app, _) = integrations_app();
    let window = main_window(&app);
    let connection = connect(&window);
    let request = json!({
        "connection": connection["id"],
        "action": { "capability": "post_message", "draft": { "target": "general", "text": "Notes", "replies": [] } },
        "idempotencyKey": "share:m1:sandbox:general",
        "meeting": null,
    });

    let first = call(&window, "deliver", json!({ "request": request })).unwrap();
    let again = call(&window, "deliver", json!({ "request": request })).unwrap();
    let log = call(&window, "deliveries", json!({ "limit": 20 })).unwrap();

    assert_eq!(first, again);
    assert_eq!(first["status"], "succeeded");
    assert_eq!(first["capability"], "post_message");
    assert_eq!(first["attempts"], 1);
    assert_eq!(
        first["external"],
        json!({ "id": "sandbox-message-1", "url": null })
    );
    assert_eq!(log, json!([first]));
}

#[test]
fn targets_disconnect_and_errors_have_their_codes() {
    let (app, _) = integrations_app();
    let window = main_window(&app);
    let connection = connect(&window);
    let id = connection["id"].clone();

    let targets = call(
        &window,
        "targets",
        json!({ "connection": id, "capability": "create_task" }),
    );
    let oauth = call(&window, "connect", json!({ "provider": "sandbox" }));
    let unknown = call(&window, "connect", json!({ "provider": "slack" }));
    call(&window, "disconnect", json!({ "connection": id })).unwrap();
    let gone = call(&window, "check", json!({ "connection": id }));

    assert_eq!(
        targets.unwrap(),
        json!([{ "id": "inbox", "name": "Inbox" }, { "id": "product", "name": "Product" }])
    );
    assert_eq!(oauth.unwrap_err()["code"], "invalid");
    assert_eq!(unknown.unwrap_err()["code"], "not_found");
    assert_eq!(gone.unwrap_err()["code"], "not_found");
    assert_eq!(
        call(&window, "cancel_connect", json!({ "provider": "sandbox" })),
        Ok(json!(false))
    );
}
