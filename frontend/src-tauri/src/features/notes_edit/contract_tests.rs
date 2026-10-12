//! Contract tests: the JSON of the notes-edit commands, through the real
//! plugin, ACL and error mapping, over the seeded fixture library with one
//! version of enhanced notes.

use std::sync::Arc;

use deino_core::enhance::{
    EnhancedNotesWriter, MaterialSources, NewVersion, SqliteEnhancedNotesRepo,
};
use deino_core::notes::edit::{NoteEditPorts, NoteEditService};
use deino_core::redaction::NoRedaction;
use deino_doc::Flavor;
use deino_domain::MeetingId;
use deino_testkit::{FakeClock, at, seed_fixtures, test_db};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-fixture-sales-discovery";
const NOTES: &str = "### Pricing\n\n- Niall owns pricing\n- Niall sends the deck\n";

/// An app with the plugin over the fixtures, the meeting's notes at version 1.
fn edit_app() -> tauri::App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(async {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        database
    });
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let doc = deino_doc::from_markdown(NOTES).unwrap();
    let first = NewVersion {
        meeting_id: MeetingId::new(MEETING),
        markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
        doc,
        template_id: None,
        model: "builtin:mock-local".into(),
        basis: None,
        is_user_edited: false,
        at: at("2026-10-12T08:00:00Z"),
    };
    tauri::async_runtime::block_on(repo.insert(&first)).unwrap();
    let service = NoteEditService::new(
        NoteEditPorts {
            reader: repo.clone(),
            writer: repo,
            transcript: Arc::new(MaterialSources::sqlite(&database)),
            redactor: Arc::new(NoRedaction),
            gateway: None,
        },
        Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
    );
    app.manage(Arc::new(service));
    app
}

fn call(app: &tauri::App<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(
        &main_window(app),
        &format!("plugin:notes-edit|{command}"),
        args,
    )
}

fn rename_request() -> Value {
    json!({ "request": {
        "meetingId": MEETING,
        "version": 1,
        "instruction": "Niall is spelled Nyal",
        "selection": [],
    }})
}

#[test]
fn propose_returns_a_camel_case_proposal_with_its_diff() {
    let app = edit_app();

    let proposal = call(&app, "propose", rename_request()).unwrap();

    assert_eq!(proposal["meetingId"], MEETING);
    assert_eq!(proposal["baseVersion"], 1);
    assert_eq!(proposal["method"], "rename");
    assert_eq!(proposal["occurrences"], 2);
    assert_eq!(proposal["doc"]["type"], "doc");
    let row = &proposal["changes"][0];
    assert_eq!(
        (row["change"].as_str(), row["kind"].as_str()),
        (Some("changed"), Some("bullet"))
    );
    assert_eq!(row["section"], "Pricing");
    assert_eq!(
        row["segments"][0],
        json!({ "change": "removed", "text": "Niall" })
    );
}

#[test]
fn accept_stores_a_new_version_and_a_stale_one_is_a_conflict() {
    let app = edit_app();
    let proposal = call(&app, "propose", rename_request()).unwrap();
    let accept = json!({ "meetingId": MEETING, "baseVersion": 1, "doc": proposal["doc"] });

    let accepted = call(&app, "accept", accept.clone()).unwrap();
    let stale = call(&app, "accept", accept).unwrap_err();

    assert_eq!(accepted, json!({ "version": 2, "previousVersion": 1 }));
    assert_eq!(stale["code"], "conflict");
}

#[test]
fn an_ai_edit_without_a_model_says_where_to_set_one_up() {
    let app = edit_app();
    let request = json!({ "request": {
        "meetingId": MEETING,
        "version": 1,
        "instruction": "make this more detailed",
    }});

    let error = call(&app, "propose", request).unwrap_err();

    assert!(
        error["message"].as_str().unwrap().contains("Settings → AI"),
        "{error}"
    );
}
