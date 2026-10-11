//! Contract tests: the JSON the UI sends to and receives from each
//! `insights` command, through the real plugin, ACL and error mapping.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::export::{FakeVault, ScriptedDialog};
use deino_core::insights::{
    FieldEntry, FieldValue, FixedDomains, InsightAnswer, InsightFiles, InsightPorts,
    InsightResultRepo, InsightService, InsightValue, RecordedScheduler, RunAnswers,
    SqliteInsightRepo,
};
use deino_domain::{FixedZone, InsightId, MeetingId, Millis};
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const SALES: &str = "meeting-fixture-sales-discovery";

struct Contract {
    window: WebviewWindow<MockRuntime>,
    repo: Arc<SqliteInsightRepo>,
    scheduler: Arc<RecordedScheduler>,
}

/// The fixture meetings, and the plugin over a real repository.
fn contract() -> Contract {
    let database = block_on(test_db());
    block_on(seed_fixtures(&database)).unwrap();
    let repo = Arc::new(SqliteInsightRepo::new(database));
    let scheduler = Arc::new(RecordedScheduler::default());
    let service = InsightService::new(InsightPorts {
        defs: repo.clone(),
        results: repo.clone(),
        records: repo.clone(),
        domains: Arc::new(FixedDomains::default()),
        names: repo.clone(),
        scheduler: scheduler.clone(),
        files: InsightFiles {
            save_dialog: Arc::new(ScriptedDialog::answering(Some(PathBuf::from("/tmp/x.csv")))),
            vault: Arc::new(FakeVault::default()),
            zone: Arc::new(FixedZone::hours(-7).unwrap()),
        },
        clock: Arc::new(FakeClock::new(at("2026-10-09T16:00:00Z"))),
    });
    let builder = mock_builder()
        .plugin(super::init())
        .manage(Arc::new(service));
    Contract {
        window: main_window(&mock_app(builder)),
        repo,
        scheduler,
    }
}

fn call(contract: &Contract, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &contract.window,
        &format!("plugin:insights|{command}"),
        args,
    )
}

/// A typed insight, as the editor sends it.
fn budget_input() -> Value {
    json!({
        "name": "Budget",
        "description": "What budget did they mention?",
        "fields": [
            { "name": "Budget", "kind": { "type": "number" } },
            { "name": "Health", "kind": { "type": "enum", "options": ["Red", "Green"] } }
        ],
        "cardinality": { "kind": "fixed", "count": 1 },
        "applyWhen": { "mode": "automatic", "conditions": [[{ "kind": "external", "external": true }]] }
    })
}

fn create(contract: &Contract) -> Value {
    call(contract, "create", json!({ "input": budget_input() })).unwrap()
}

#[test]
fn templates_are_twelve_ready_inputs() {
    let contract = contract();

    let templates = call(&contract, "templates", Value::Null).unwrap();

    let templates = templates.as_array().unwrap();
    assert_eq!(templates.len(), 12);
    assert_eq!(templates[0]["key"], "discovery_scorecard");
    assert_eq!(
        templates[0]["input"]["cardinality"],
        json!({ "kind": "fixed", "count": 1 })
    );
    assert_eq!(
        templates[0]["input"]["fields"][2],
        json!({ "name": "Budget", "kind": { "type": "number" } })
    );
}

#[test]
fn create_returns_the_definition_and_a_taken_name_is_a_conflict() {
    let contract = contract();

    let def = create(&contract);
    let again = call(&contract, "create", json!({ "input": budget_input() }));

    assert_eq!(def["name"], "Budget");
    assert_eq!(def["revision"], 1);
    assert_eq!(def["applyWhen"]["mode"], "automatic");
    assert_eq!(def["fields"][1]["kind"]["options"], json!(["Red", "Green"]));
    assert_eq!(again.unwrap_err()["code"], "conflict");
    let blank = json!({ "input": { "name": " ", "description": "x", "fields": [],
        "cardinality": { "kind": "auto" }, "applyWhen": { "mode": "manual" } } });
    assert_eq!(
        call(&contract, "create", blank).unwrap_err()["code"],
        "invalid"
    );
}

#[test]
fn run_takes_meeting_ids_and_returns_progress() {
    let contract = contract();
    let def = create(&contract);

    let progress = call(
        &contract,
        "run",
        json!({ "id": def["id"], "meetingIds": [SALES] }),
    )
    .unwrap();
    let unknown = call(
        &contract,
        "run",
        json!({ "id": def["id"], "meetingIds": ["nope"] }),
    );

    assert_eq!(progress, json!({ "pending": 1, "done": 0, "failed": 0 }));
    assert_eq!(contract.scheduler.scheduled(), [MeetingId::new(SALES)]);
    assert_eq!(unknown.unwrap_err()["code"], "not_found");
}

fn answer(contract: &Contract, def: &Value) {
    let id: InsightId = serde_json::from_value(def["id"].clone()).unwrap();
    let answer = InsightAnswer {
        value: InsightValue::Fields {
            fields: vec![FieldEntry {
                name: "Budget".into(),
                value: Some(FieldValue::Number(40_000.0)),
            }],
        },
        speaker: Some("Marcus Webb".into()),
        timestamp_ms: Some(Millis(271_000)),
        segment_id: None,
    };
    block_on(contract.repo.save_answers(&RunAnswers {
        insight_id: id,
        meeting_id: MeetingId::new(SALES),
        revision: 1,
        answers: vec![answer],
        at: at("2026-10-09T16:00:00Z"),
    }))
    .unwrap();
}

#[test]
fn a_meetings_tab_and_an_insights_table_carry_typed_attributed_answers() {
    let contract = contract();
    let def = create(&contract);
    answer(&contract, &def);

    let tab = call(&contract, "for_meeting", json!({ "meetingId": SALES })).unwrap();
    let table = call(&contract, "table", json!({ "id": def["id"] })).unwrap();

    let expected = json!({
        "value": { "kind": "fields", "fields": [
            { "name": "Budget", "value": { "type": "number", "value": 40000.0 } }
        ] },
        "speaker": "Marcus Webb",
        "timestampMs": 271000,
        "segmentId": null
    });
    assert_eq!(tab["entries"][0]["run"]["answers"][0], expected);
    assert_eq!(tab["entries"][0]["stale"], false);
    assert_eq!(tab["others"], json!([]));
    assert_eq!(
        table["rows"][0]["meetingTitle"],
        "Deino × Brightkite Logistics: discovery"
    );
    assert_eq!(table["rows"][0]["run"]["status"], "done");
    assert_eq!(table["progress"]["done"], 1);
}

#[test]
fn candidates_and_the_csv_export_answer_in_camel_case() {
    let contract = contract();
    let def = create(&contract);
    answer(&contract, &def);

    let candidates = call(&contract, "candidates", json!({ "id": def["id"] })).unwrap();
    let saved = call(&contract, "export_csv", json!({ "id": def["id"] })).unwrap();

    let sales = candidates
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["meeting"]["id"] == SALES)
        .unwrap();
    assert_eq!(sales["status"], "done");
    assert!(sales["meeting"]["startedAt"].is_string());
    assert_eq!(saved, json!({ "fileName": "x.csv" }));
    assert_eq!(
        call(&contract, "delete", json!({ "id": def["id"] })).unwrap(),
        Value::Null
    );
}
