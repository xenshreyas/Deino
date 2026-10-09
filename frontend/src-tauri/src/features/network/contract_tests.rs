//! Contract tests: the JSON the Network page relies on, through the real
//! plugin and ACL.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use deino_domain::ServiceId;
use deino_net::{
    EgressEntry, EgressLog, EgressOutcome, FakeServicePolicy, Method, NetworkControls,
};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn window() -> WebviewWindow<MockRuntime> {
    let policy = Arc::new(FakeServicePolicy::default());
    let egress = Arc::new(EgressLog::default());
    egress.record(EgressEntry {
        at: Utc.with_ymd_and_hms(2026, 10, 9, 9, 30, 0).unwrap(),
        service: ServiceId::LlmAnthropic,
        host: "api.anthropic.com".into(),
        method: Method::Post,
        status: Some(200),
        outcome: EgressOutcome::Sent,
        bytes_up: 2_048,
        bytes_down: 512,
    });
    let controls = Arc::new(NetworkControls::new(policy.clone(), policy, egress));
    let app = mock_app(mock_builder().plugin(super::init()).manage(controls));
    main_window(&app)
}

#[test]
fn services_list_every_service_with_its_hosts_and_state() {
    let services = invoke(&window(), "plugin:network|services", Value::Null).unwrap();

    let anthropic = services
        .as_array()
        .unwrap()
        .iter()
        .find(|service| service["id"] == "llm_anthropic")
        .unwrap();
    assert_eq!(
        anthropic,
        &json!({
            "id": "llm_anthropic",
            "kind": "llm",
            "hosts": ["api.anthropic.com"],
            "enabled": true,
        })
    );
    assert_eq!(services.as_array().unwrap().len(), ServiceId::ALL.len());
}

#[test]
fn switching_a_service_returns_its_new_status() {
    let window = window();

    let slack = invoke(
        &window,
        "plugin:network|set_service_enabled",
        json!({ "service": "slack", "enabled": true }),
    );
    let unknown = invoke(
        &window,
        "plugin:network|set_service_enabled",
        json!({ "service": "myspace", "enabled": true }),
    );

    assert_eq!(slack.unwrap()["enabled"], json!(true));
    assert!(unknown.is_err());
}

#[test]
fn the_egress_log_carries_sizes_and_hosts_but_no_url() {
    let log = invoke(
        &window(),
        "plugin:network|egress_log",
        json!({ "limit": 50 }),
    );

    assert_eq!(
        log,
        Ok(json!([{
            "at": "2026-10-09T09:30:00Z",
            "service": "llm_anthropic",
            "host": "api.anthropic.com",
            "method": "POST",
            "status": 200,
            "outcome": "sent",
            "bytesUp": 2048,
            "bytesDown": 512,
        }]))
    );
}
