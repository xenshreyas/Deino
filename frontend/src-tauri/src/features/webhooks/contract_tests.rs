//! Contract tests: the JSON Settings → Developer → Webhooks sends and
//! receives, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use deino_core::action_items::SqliteActionItemRepo;
use deino_core::automations::AssembledMeetings;
use deino_core::export::{ExportAssembler, ExportPorts, SystemZone};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::redaction::NoRedaction;
use deino_core::webhooks::{
    RepoSubjectItems, SqliteDeliveryRepo, SqliteWebhookRepo, WebhookPorts, WebhookSecrets,
    WebhookService,
};
use deino_net::{FakeNetClient, FakeServicePolicy};
use deino_secrets::InMemoryCredentialStore;
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

struct Contract {
    window: WebviewWindow<MockRuntime>,
    client: Arc<FakeNetClient>,
}

/// The fixture meetings, and the plugin over real repositories, a scripted
/// HTTP client and an in-memory Keychain.
fn contract() -> Contract {
    let database = block_on(test_db());
    block_on(seed_fixtures(&database)).unwrap();
    let client = Arc::new(FakeNetClient::default());
    let assembler = ExportAssembler::new(ExportPorts::sqlite(&database, Arc::new(SystemZone)));
    let service = WebhookService::new(WebhookPorts {
        webhooks: Arc::new(SqliteWebhookRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
        secrets: WebhookSecrets::new(Arc::new(InMemoryCredentialStore::default())),
        meetings: Arc::new(AssembledMeetings::new(
            Arc::new(assembler),
            Arc::new(SqliteMeetingRepo::new(database.clone())),
        )),
        items: Arc::new(RepoSubjectItems::new(
            Arc::new(SqliteActionItemRepo::new(database.clone())),
            Arc::new(NoRedaction),
        )),
        client: client.clone(),
        switch: Arc::new(FakeServicePolicy::default()),
        clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
    });
    let window = main_window(&mock_app(
        mock_builder()
            .plugin(super::init())
            .manage(Arc::new(service)),
    ));
    Contract { window, client }
}

fn endpoint() -> Value {
    json!({
        "url": "https://hooks.zapier.com/hooks/catch/1/abc/",
        "label": "Zapier",
        "events": ["meeting.ready", "action_item.created"],
        "fields": { "notes": true, "actionItems": true, "transcript": false, "insights": true,
                    "attendees": true, "link": true, "audioPath": false },
        "enabled": true,
    })
}

#[test]
fn create_shows_the_secret_once_and_list_never_does() {
    let contract = contract();

    let created = invoke(
        &contract.window,
        "plugin:webhooks|create",
        json!({ "input": endpoint() }),
    )
    .unwrap();
    let listed = invoke(&contract.window, "plugin:webhooks|list", json!({})).unwrap();

    let secret = created["secret"].as_str().unwrap();
    assert!(secret.starts_with("whsec_"));
    assert_eq!(
        listed[0]["events"],
        json!(["meeting.ready", "action_item.created"])
    );
    assert_eq!(listed[0]["fields"]["actionItems"], true);
    assert_eq!(listed[0]["disabledReason"], Value::Null);
    assert!(!listed.to_string().contains(secret));
}

#[test]
fn send_test_posts_the_latest_meeting_and_logs_it() {
    let contract = contract();
    let created = invoke(
        &contract.window,
        "plugin:webhooks|create",
        json!({ "input": endpoint() }),
    )
    .unwrap();
    let id = created["webhook"]["id"].clone();

    let delivery = invoke(
        &contract.window,
        "plugin:webhooks|send_test",
        json!({ "id": id }),
    )
    .unwrap();
    let log = invoke(
        &contract.window,
        "plugin:webhooks|deliveries",
        json!({ "id": id }),
    )
    .unwrap();

    assert_eq!(delivery["kind"], "test");
    assert_eq!(delivery["status"], "succeeded");
    assert_eq!(log[0]["id"], delivery["id"]);
    assert_eq!(contract.client.requests().len(), 1);
}

#[test]
fn an_http_url_is_invalid_and_an_unknown_endpoint_is_not_found() {
    let contract = contract();
    let mut plain = endpoint();
    plain["url"] = json!("http://example.com/hook");

    let invalid = invoke(
        &contract.window,
        "plugin:webhooks|create",
        json!({ "input": plain }),
    )
    .unwrap_err();
    let missing = invoke(
        &contract.window,
        "plugin:webhooks|delete",
        json!({ "id": "0192f0c0-0000-7000-8000-000000000000" }),
    )
    .unwrap_err();

    assert_eq!(invalid["code"], "invalid");
    assert_eq!(missing["code"], "not_found");
}
