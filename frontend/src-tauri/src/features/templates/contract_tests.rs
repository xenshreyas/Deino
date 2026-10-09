//! Contract tests: the JSON the UI sends to and receives from each templates
//! command, through the real plugin, ACL and error mapping.

use std::sync::Arc;

use chrono::DateTime;
use deino_core::meetings::{FakeMeetingRepo, Meeting};
use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use deino_core::templates::{
    BuiltinSync, CATALOG, FakeMeetingFacts, FakeTemplateRepo, MeetingFacts, MeetingTemplatePorts,
    MeetingTemplateService, NoClassifier, TemplateService,
};
use deino_domain::MeetingId;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::WebviewWindow;
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};

use crate::features::ipc_tests::{invoke, main_window, mock_app};

fn meeting(title: &str) -> MeetingFacts {
    MeetingFacts {
        id: MeetingId::new("meeting-1"),
        title: title.into(),
        template_id: None,
        started_at: None,
        description: String::new(),
        attendee_emails: Vec::new(),
        attendee_count: 0,
    }
}

fn window() -> WebviewWindow<MockRuntime> {
    let repo = Arc::new(FakeTemplateRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    block_on(BuiltinSync::new(repo.clone(), clock.clone()).run(&CATALOG)).unwrap();
    let facts = Arc::new(FakeMeetingFacts::default());
    facts.insert(meeting("Retro: sprint 42"));
    let saved = Meeting {
        id: MeetingId::new("meeting-1"),
        title: "Retro: sprint 42".into(),
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    };
    let settings = block_on(SettingsService::load(
        Arc::new(FakeSettingsStore::with(None)),
        Arc::new(RecordedSettingsEvents::default()),
    ))
    .unwrap();
    let meetings = MeetingTemplateService::new(MeetingTemplatePorts {
        templates: repo.clone(),
        facts,
        meetings: Arc::new(FakeMeetingRepo::with([saved])),
        settings: Arc::new(settings),
        classifier: Arc::new(NoClassifier),
        clock: clock.clone(),
    });
    let library = Arc::new(TemplateService::new(repo, clock));
    let builder = mock_builder()
        .plugin(super::init())
        .manage(library)
        .manage(Arc::new(meetings));
    main_window(&mock_app(builder))
}

fn draft() -> Value {
    json!({
        "name": "Board prep",
        "description": "",
        "meetingContext": "",
        "sections": [{ "title": "Decisions", "instructions": "", "format": "bullets" }],
        "autoRules": [{ "label": "board", "titlePattern": "board", "attendeeDomains": [], "audience": null, "attendeeCount": null }]
    })
}

#[test]
fn list_returns_built_ins_with_camel_case_content() {
    let templates = invoke(&window(), "plugin:templates|list", Value::Null).unwrap();

    let one_on_one = templates
        .as_array()
        .unwrap()
        .iter()
        .find(|template| template["id"] == "one_on_one")
        .unwrap();
    assert_eq!(templates.as_array().unwrap().len(), 30);
    assert_eq!(one_on_one["isBuiltin"], true);
    assert_eq!(one_on_one["content"]["name"], "1:1");
    assert_eq!(one_on_one["content"]["sections"][0]["format"], "paragraph");
    assert_eq!(
        one_on_one["content"]["autoRules"][1]["attendeeCount"],
        json!({ "min": 2, "max": 2 })
    );
    assert_eq!(
        one_on_one["content"]["autoRules"][1]["audience"],
        "internal"
    );
}

#[test]
fn create_takes_content_and_update_refuses_built_ins() {
    let window = window();

    let created = invoke(
        &window,
        "plugin:templates|create",
        json!({ "content": draft() }),
    )
    .unwrap();
    let edit_builtin = invoke(
        &window,
        "plugin:templates|update",
        json!({ "id": "general", "content": draft() }),
    );

    assert_eq!(created["isBuiltin"], false);
    assert_eq!(created["content"]["autoRules"][0]["titlePattern"], "board");
    assert_eq!(edit_builtin.unwrap_err()["code"], "conflict");
}

#[test]
fn invalid_content_is_rejected_with_the_rule() {
    let mut nameless = draft();
    nameless["name"] = json!(" ");

    let error = invoke(
        &window(),
        "plugin:templates|create",
        json!({ "content": nameless }),
    );

    assert_eq!(
        error,
        Err(json!({ "code": "invalid", "message": "name cannot be empty" }))
    );
}

#[test]
fn export_and_import_round_trip_as_a_string() {
    let window = window();

    let exported = invoke(
        &window,
        "plugin:templates|export_json",
        json!({ "id": "standup" }),
    )
    .unwrap();
    let imported = invoke(
        &window,
        "plugin:templates|import_json",
        json!({ "json": exported }),
    )
    .unwrap();

    assert!(exported.as_str().unwrap().contains("deino.template"));
    assert_eq!(imported["content"]["name"], "Stand-up");
}

#[test]
fn a_meetings_template_comes_with_a_tagged_reason() {
    let window = window();

    let choice = invoke(
        &window,
        "plugin:templates|meeting_template",
        json!({ "meeting": "meeting-1" }),
    )
    .unwrap();
    let chosen = invoke(
        &window,
        "plugin:templates|set_meeting_template",
        json!({ "meeting": "meeting-1", "template": "podcast" }),
    );
    let missing = invoke(
        &window,
        "plugin:templates|meeting_template",
        json!({ "meeting": "nope" }),
    );

    assert_eq!(choice["template"]["id"], "retrospective");
    assert_eq!(
        choice["reason"],
        json!({ "kind": "rule", "rule": "retrospective" })
    );
    assert!(chosen.is_ok(), "{chosen:?}");
    assert_eq!(missing.unwrap_err()["code"], "not_found");
}

#[test]
fn preview_returns_labelled_blocks() {
    let preview = invoke(
        &window(),
        "plugin:templates|preview",
        json!({ "content": draft(), "meeting": "meeting-1" }),
    )
    .unwrap();

    assert_eq!(preview["blocks"][0]["heading"], "This meeting");
    assert!(
        preview["blocks"][0]["body"]
            .as_str()
            .unwrap()
            .starts_with("Title: Retro")
    );
}
