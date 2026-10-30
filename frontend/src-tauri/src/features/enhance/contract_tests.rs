//! Contract tests: the JSON the UI sends to and receives from each `enhance`
//! command and the `NotesRun` event, through the real plugin, ACL and error
//! mapping, over the seeded fixtures and a scripted `notes.enhance`.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use async_trait::async_trait;
use deino_core::enhance::{
    EnhancePorts, EnhanceService, LateProgress, MaterialSources, NotesDraft, NotesPhase,
    NotesProgress, NotesRunEnd, SqliteEnhancedNotesRepo, TemplateSource,
};
use deino_core::templates::{
    BuiltinSync, CATALOG, SqliteTemplateRepo, Template, TemplateError, TemplateRepo,
};
use deino_doc::Doc;
use deino_domain::{MeetingId, TemplateId};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};
use deino_testkit::{FakeClock, at, test_db};
use serde_json::{Value, json};
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};
use tauri::{Listener, Manager, WebviewWindow};

use super::TauriNotesProgress;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

/// How long a test waits for an event that should already have been emitted.
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

/// The built-in 1:1 for every meeting.
struct OneOnOne(Arc<SqliteTemplateRepo>);

#[async_trait]
impl TemplateSource for OneOnOne {
    async fn template_for(&self, _meeting: &MeetingId) -> Result<Template, TemplateError> {
        let id = TemplateId::new("one_on_one");
        self.0.get(&id).await?.ok_or(TemplateError::NotFound(id))
    }
}

const ANSWER: &str = r#"{"sections":[{"heading":"Topics","blocks":[
    {"kind":"bullet","text":"Maya is coming off two weeks of incident work","origin":"ai","evidence":["s2"]}
]}]}"#;

fn window() -> (WebviewWindow<MockRuntime>, MeetingId) {
    let database = block_on(test_db());
    block_on(seed_fixtures(&database)).unwrap();
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    block_on(BuiltinSync::new(templates.clone(), clock.clone()).run(&CATALOG)).unwrap();
    let mock = MockProvider::new()
        .reply(Route::NotesEnhance, ANSWER)
        .reply(Route::NotesEnhance, ANSWER);
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let ports = EnhancePorts {
        sources: MaterialSources::sqlite(&database),
        reader: repo.clone(),
        writer: repo,
        templates: Arc::new(OneOnOne(templates)),
        gateway: Arc::new(mock_gateway(Arc::new(mock)).gateway),
    };
    let progress = Arc::new(LateProgress::default());
    let service = Arc::new(EnhanceService::new(ports, clock).with_progress(progress.clone()));
    let builder = mock_builder().plugin(super::init()).manage(service);
    let meeting = Fixtures::load()
        .unwrap()
        .meeting(MeetingKind::OneOnOne)
        .unwrap()
        .id
        .clone();
    let app = mock_app(builder);
    progress.bind(Arc::new(TauriNotesProgress::new(app.handle().clone())));
    (main_window(&app), meeting)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:enhance|{command}"), args)
}

#[test]
fn a_meeting_has_no_notes_until_they_are_written() {
    let (window, meeting) = window();

    let notes = call(
        &window,
        "get",
        json!({ "meetingId": meeting, "version": null }),
    );

    assert_eq!(notes, Ok(Value::Null));
}

#[test]
fn enhance_returns_the_newest_version_with_its_document_and_history_entry() {
    let (window, meeting) = window();

    let view = call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap();

    assert_eq!(view["meetingId"], json!(meeting));
    assert_eq!(view["isLatest"], json!(true));
    assert_eq!(view["reasons"], json!([]));
    assert_eq!(view["doc"]["type"], json!("doc"));
    let version = &view["version"];
    assert_eq!(version["version"], json!(1));
    assert_eq!(version["templateId"], json!("one_on_one"));
    assert_eq!(version["templateName"], json!("1:1"));
    assert_eq!(version["model"], json!("builtin:mock-local"));
    assert_eq!(version["isUserEdited"], json!(false));
    let versions = call(&window, "versions", json!({ "meetingId": meeting })).unwrap();
    assert_eq!(versions.as_array().unwrap().len(), 1);
}

#[test]
fn an_edited_version_is_only_replaced_with_consent() {
    let (window, meeting) = window();
    call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap();
    let doc = json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "mine", "marks": [] }] }] });

    let saved = call(
        &window,
        "save",
        json!({ "meetingId": meeting, "version": 1, "doc": doc }),
    )
    .unwrap();
    let refused = call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap_err();
    let replaced = call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": true }),
    )
    .unwrap();

    assert_eq!(saved["version"], json!(1));
    assert!(saved["updatedAt"].is_string());
    assert_eq!(refused["code"], json!("conflict"));
    assert_eq!(
        refused["message"],
        json!("Regenerating will replace the notes you edited.")
    );
    assert_eq!(replaced["version"]["version"], json!(2));
}

#[test]
fn citations_bring_their_excerpt_speaker_and_time() {
    let (window, meeting) = window();
    let view = call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap();
    let citation = find_citation(&view["doc"]).expect("the AI line cites the transcript");

    let cited = call(
        &window,
        "cited",
        json!({ "meetingId": meeting, "segmentIds": [citation["segmentId"]] }),
    )
    .unwrap();

    let excerpt = &cited[0];
    assert_eq!(excerpt["segmentId"], citation["segmentId"]);
    assert_eq!(excerpt["start"], citation["startMs"]);
    assert!(
        excerpt["end"].is_number() && excerpt["speaker"].is_string() && excerpt["text"].is_string()
    );
}

#[test]
fn restore_and_missing_versions() {
    let (window, meeting) = window();
    call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap();

    let restored = call(
        &window,
        "restore",
        json!({ "meetingId": meeting, "version": 1 }),
    )
    .unwrap();
    let missing = call(
        &window,
        "get",
        json!({ "meetingId": meeting, "version": 9 }),
    )
    .unwrap_err();

    assert_eq!(restored["version"]["version"], json!(2));
    assert_eq!(missing["code"], json!("not_found"));
}

/// The first `citation` node's attributes in a TipTap document.
fn find_citation(node: &Value) -> Option<&Value> {
    if node["type"] == json!("citation") {
        return Some(&node["attrs"]);
    }
    node["content"].as_array()?.iter().find_map(find_citation)
}

#[test]
fn draft_is_null_and_stop_is_false_while_nothing_is_written() {
    let (window, meeting) = window();

    let draft = call(&window, "draft", json!({ "meetingId": meeting }));
    let stopped = call(&window, "stop", json!({ "meetingId": meeting }));

    assert_eq!(draft, Ok(Value::Null));
    assert_eq!(stopped, Ok(json!(false)));
}

/// Every `NotesRun` payload the app emits, as JSON.
fn notes_runs(window: &WebviewWindow<MockRuntime>) -> mpsc::Receiver<Value> {
    let (sender, received) = mpsc::channel();
    window
        .app_handle()
        .listen_any("plugin:enhance:notes-run", move |event| {
            let payload = serde_json::from_str(event.payload()).expect("the payload is JSON");
            sender.send(payload).expect("the test is still listening");
        });
    received
}

#[test]
fn a_draft_and_the_run_s_end_arrive_as_notes_run() {
    let (window, meeting) = window();
    let received = notes_runs(&window);
    let progress = TauriNotesProgress::new(window.app_handle().clone());

    progress.draft(&NotesDraft {
        meeting_id: meeting.clone(),
        phase: NotesPhase::Parts { done: 1, total: 3 },
        doc: Doc::empty(),
        lines: 0,
        stale: true,
    });
    progress.ended(
        &meeting,
        NotesRunEnd::Written {
            version: 2,
            kept: 4,
        },
    );

    let draft = received.recv_timeout(EVENT_TIMEOUT).unwrap();
    assert_eq!(draft["meetingId"], json!(meeting));
    assert_eq!(draft["update"]["kind"], json!("draft"));
    let body = &draft["update"]["draft"];
    assert_eq!(
        body["phase"],
        json!({ "kind": "parts", "done": 1, "total": 3 })
    );
    assert_eq!(body["doc"]["type"], json!("doc"));
    assert_eq!(
        (body["lines"].clone(), body["stale"].clone()),
        (json!(0), json!(true))
    );
    let ended = received.recv_timeout(EVENT_TIMEOUT).unwrap();
    assert_eq!(
        ended["update"],
        json!({ "kind": "ended", "end": { "kind": "written", "version": 2, "kept": 4 } })
    );
}

#[test]
fn writing_the_notes_streams_drafts_then_ends_with_the_new_version() {
    let (window, meeting) = window();
    let received = notes_runs(&window);

    call(
        &window,
        "enhance",
        json!({ "meetingId": meeting, "replaceEdits": false }),
    )
    .unwrap();

    let mut runs: Vec<Value> = Vec::new();
    while runs
        .last()
        .is_none_or(|run| run["update"]["kind"] != json!("ended"))
    {
        runs.push(received.recv_timeout(EVENT_TIMEOUT).unwrap());
    }
    let first = &runs[0]["update"];
    assert_eq!(first["draft"]["phase"], json!({ "kind": "reading" }));
    let saving = &runs[runs.len() - 2]["update"]["draft"];
    assert_eq!(saving["phase"], json!({ "kind": "saving" }));
    assert_eq!(saving["lines"], json!(2));
    let end = &runs[runs.len() - 1]["update"]["end"];
    assert_eq!(
        (&end["kind"], &end["version"]),
        (&json!("written"), &json!(1))
    );
    assert!(end["kept"].is_u64());
}
