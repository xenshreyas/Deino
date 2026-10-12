//! Contract tests: the JSON of the `vocabulary` commands through the real
//! plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::segments::{NewSegment, SegmentWriter, SqliteSegmentRepo, WordTiming};
use deino_core::vocabulary::FixupService;
use deino_db::Database;
use deino_domain::{Channel, Clock, MeetingId, Millis, SegmentId};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager, WebviewWindow};

use crate::composition::Vocabulary;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-11T10:00:00Z";
const MEETING: &str = "meeting-1";

/// An app with the `vocabulary` plugin over a meeting where "Sreyas" was said.
fn vocabulary_app() -> App<MockRuntime> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let database = tauri::async_runtime::block_on(misheard_meeting());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    Vocabulary::build(&database, &clock).manage(&app);
    app
}

async fn misheard_meeting() -> Database {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(NOW)).await;
    let said = NewSegment {
        id: SegmentId::generate(&FakeClock::new(at(NOW))),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(0),
        end: Millis(900),
        text: "Thanks Sreyas".into(),
        words: ["Thanks", "Sreyas"]
            .iter()
            .zip([0, 400])
            .map(|(text, start)| WordTiming {
                text: (*text).into(),
                start: Millis(start),
                end: Millis(start + 400),
            })
            .collect(),
    };
    SqliteSegmentRepo::new(database.clone())
        .append(&[said])
        .await
        .unwrap();
    database
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:vocabulary|{command}"), args)
}

fn term(text: &str, sounds_like: &[&str]) -> Value {
    json!({ "input": { "kind": "term", "text": text, "soundsLike": sounds_like } })
}

#[test]
fn entries_round_trip_with_camel_case_fields() {
    let app = vocabulary_app();
    let window = main_window(&app);

    let created = call(&window, "create", term(" Shreyas ", &["Sreyas"])).unwrap();
    let listed = call(&window, "list", json!({})).unwrap();

    assert_eq!(created["kind"], "term");
    assert_eq!(created["text"], "Shreyas");
    assert_eq!(created["soundsLike"], json!(["Sreyas"]));
    assert!(created["createdAt"].is_string());
    assert_eq!(listed, json!([created]));

    let id = created["id"].clone();
    let updated = call(
        &window,
        "update",
        json!({ "id": id, "input": { "kind": "term", "text": "Shreyas", "soundsLike": [] } }),
    )
    .unwrap();
    assert_eq!(updated["soundsLike"], json!([]));
    assert_eq!(
        call(&window, "delete", json!({ "id": id })),
        Ok(Value::Null)
    );
}

#[test]
fn bad_and_repeated_entries_and_missing_ones_map_to_error_codes() {
    let app = vocabulary_app();
    let window = main_window(&app);
    call(&window, "create", term("Deino", &[])).unwrap();

    let blank = call(&window, "create", term("  ", &[])).unwrap_err();
    let repeated = call(&window, "create", term("deino", &[])).unwrap_err();
    let missing = call(
        &window,
        "delete",
        json!({ "id": "0192a5f0-0000-7000-8000-000000000000" }),
    )
    .unwrap_err();

    assert_eq!(blank["code"], "invalid");
    assert_eq!(repeated["code"], "conflict");
    assert_eq!(missing["code"], "not_found");
}

#[test]
fn a_meetings_corrections_are_listed_and_undone() {
    let app = vocabulary_app();
    let window = main_window(&app);
    call(&window, "create", term("Shreyas", &["Sreyas"])).unwrap();
    let fixups = Arc::clone(&*app.state::<Arc<FixupService>>());
    tauri::async_runtime::block_on(fixups.run(&MeetingId::new(MEETING))).unwrap();

    let listed = call(&window, "corrections", json!({ "meetingId": MEETING })).unwrap();
    let undone = call(
        &window,
        "undo",
        json!({ "meetingId": MEETING, "segmentId": null }),
    )
    .unwrap();

    assert_eq!(listed[0]["originalText"], "Thanks Sreyas");
    assert_eq!(listed[0]["correctedText"], "Thanks Shreyas");
    assert_eq!(listed[0]["undoable"], true);
    assert_eq!(undone, json!({ "restored": 1, "kept": 0 }));
    assert_eq!(
        call(&window, "corrections", json!({ "meetingId": MEETING })).unwrap(),
        json!([])
    );
}
