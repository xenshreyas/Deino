//! Contract tests: the JSON the UI sends to and receives from each
//! `automations` command, through the real plugin, ACL and error mapping.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::automations::{
    ActionKind, AutomationPorts, AutomationService, FactsSource, RecordedAutomationScheduler,
    ScriptedStep, SqliteAutomationRepo, StepRegistry,
};
use deino_core::export::{FolderDialog, ScriptedDialog};
use deino_core::insights::{FixedDomains, SqliteInsightRepo};
use deino_core::meetings::SqliteMeetingRepo;
use deino_domain::MeetingId;
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const SALES: &str = "meeting-fixture-sales-discovery";

struct Contract {
    window: WebviewWindow<MockRuntime>,
    scheduler: Arc<RecordedAutomationScheduler>,
}

/// The fixture meetings, and the plugin over a real repository with a
/// notification step registered.
fn contract() -> Contract {
    let database = block_on(test_db());
    block_on(seed_fixtures(&database)).unwrap();
    let repo = Arc::new(SqliteAutomationRepo::new(database.clone()));
    let scheduler = Arc::new(RecordedAutomationScheduler::default());
    let service = AutomationService::new(AutomationPorts {
        automations: repo.clone(),
        runs: repo,
        facts: FactsSource::new(
            Arc::new(SqliteInsightRepo::new(database.clone())),
            Arc::new(FixedDomains::default()),
        ),
        meetings: Arc::new(SqliteMeetingRepo::new(database)),
        steps: StepRegistry::new().with(Arc::new(ScriptedStep::new(ActionKind::Notify, vec![]))),
        scheduler: scheduler.clone(),
        clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
    });
    let dialog: Arc<dyn FolderDialog> = Arc::new(ScriptedDialog::answering(Some(PathBuf::from(
        "/Users/me/Exports",
    ))));
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(service))
        .manage(dialog);
    Contract {
        window: main_window(&mock_app(builder)),
        scheduler,
    }
}

fn call(contract: &Contract, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &contract.window,
        &format!("plugin:automations|{command}"),
        args,
    )
}

/// An automation as the builder sends it.
fn recap_input() -> Value {
    json!({
        "name": "Recap",
        "enabled": true,
        "autoRun": true,
        "stopOnFailure": false,
        "conditions": [[
            { "kind": "title_contains", "text": "discovery" },
            { "kind": "invitee_count", "comparison": "greater", "count": 1 }
        ]],
        "steps": [
            { "kind": "notify", "message": "Done" },
            { "kind": "run_shell", "command": "cat > payload.json", "fields": {
                "notes": true, "actionItems": true, "transcript": false, "insights": true,
                "attendees": true, "link": true, "audioPath": false
            } }
        ]
    })
}

fn create(contract: &Contract) -> Value {
    call(contract, "create", json!({ "input": recap_input() })).unwrap()
}

#[test]
fn the_catalog_lists_every_step_with_provider_steps_as_wip() {
    let contract = contract();

    let catalog = call(&contract, "catalog", Value::Null).unwrap();

    let catalog = catalog.as_array().unwrap();
    assert_eq!(catalog.len(), 13);
    assert_eq!(catalog[0]["kind"], "generate_insight");
    let notify = catalog
        .iter()
        .find(|entry| entry["kind"] == "notify")
        .unwrap();
    assert_eq!(notify["availability"], "ready");
    assert_eq!(notify["label"], "Notification");
    let slack = catalog
        .iter()
        .find(|entry| entry["kind"] == "slack")
        .unwrap();
    assert_eq!(slack["availability"], "wip");
}

#[test]
fn create_returns_the_automation_and_names_stay_unique() {
    let contract = contract();

    let recap = create(&contract);
    let again = call(&contract, "create", json!({ "input": recap_input() }));
    let copy = call(&contract, "duplicate", json!({ "id": recap["id"] })).unwrap();

    assert_eq!(recap["autoRun"], true);
    assert_eq!(recap["conditions"], recap_input()["conditions"]);
    assert_eq!(recap["steps"][1]["fields"]["audioPath"], false);
    assert_eq!(again.unwrap_err()["code"], "conflict");
    assert_eq!(
        (copy["name"].as_str(), &copy["enabled"]),
        (Some("Recap (copy)"), &json!(false))
    );
    let empty = json!({ "input": { "name": "x", "enabled": true, "autoRun": true,
        "stopOnFailure": false, "conditions": [], "steps": [] } });
    assert_eq!(
        call(&contract, "create", empty).unwrap_err()["code"],
        "invalid"
    );
    let list = call(&contract, "list", Value::Null).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);
}

#[test]
fn run_and_test_latest_queue_the_stage_and_report_unknown_meetings() {
    let contract = contract();
    let recap = create(&contract);

    call(
        &contract,
        "run",
        json!({ "id": recap["id"], "meetingIds": [SALES] }),
    )
    .unwrap();
    let unknown = call(
        &contract,
        "run",
        json!({ "id": recap["id"], "meetingIds": ["nope"] }),
    );
    let latest = call(&contract, "test_latest", json!({ "id": recap["id"] })).unwrap();

    assert_eq!(unknown.unwrap_err()["code"], "not_found");
    assert!(latest.is_string());
    let scheduled = contract.scheduler.scheduled();
    assert_eq!(scheduled[0], MeetingId::new(SALES));
    assert_eq!(scheduled.len(), 2);
}

#[test]
fn the_meeting_menu_and_matching_meetings_carry_runs() {
    let contract = contract();
    let recap = create(&contract);
    call(
        &contract,
        "run",
        json!({ "id": recap["id"], "meetingIds": [SALES] }),
    )
    .unwrap();

    let menu = call(&contract, "for_meeting", json!({ "meetingId": SALES })).unwrap();
    let matching = call(&contract, "matching", json!({ "id": recap["id"] })).unwrap();

    assert_eq!(menu[0]["automation"]["name"], "Recap");
    assert_eq!(menu[0]["matches"], true);
    assert_eq!(menu[0]["run"]["status"], "pending");
    assert_eq!(menu[0]["run"]["trigger"], json!({ "kind": "manual" }));
    assert_eq!(menu[0]["run"]["steps"], json!([]));
    let entry = &matching.as_array().unwrap()[0];
    assert_eq!(entry["meeting"]["id"], SALES);
    assert!(entry["meeting"]["startedAt"].is_string());
    let missing = call(&contract, "for_meeting", json!({ "meetingId": "nope" }));
    assert_eq!(missing.unwrap_err()["code"], "not_found");
}

#[test]
fn the_run_log_carries_the_meeting_title_and_the_run() {
    let contract = contract();
    let recap = create(&contract);
    call(
        &contract,
        "run",
        json!({ "id": recap["id"], "meetingIds": [SALES] }),
    )
    .unwrap();

    let log = call(&contract, "recent_runs", json!({ "id": recap["id"] })).unwrap();

    assert!(log[0]["meetingTitle"].is_string());
    assert_eq!(log[0]["run"]["meetingId"], SALES);
    assert_eq!(log[0]["run"]["status"], "pending");
    let unknown = call(
        &contract,
        "recent_runs",
        json!({ "id": "01920000-0000-7000-8000-000000000999" }),
    );
    assert_eq!(unknown.unwrap_err()["code"], "not_found");
}

#[test]
fn choose_folder_returns_the_folder_the_dialog_picked() {
    let contract = contract();

    let folder = call(&contract, "choose_folder", Value::Null).unwrap();

    assert_eq!(folder, "/Users/me/Exports");
}
