//! Contract tests: every `transcript` command's JSON in and out through the
//! real plugin and ACL, over the fake segment repository.

use std::sync::Arc;

use deino_core::segments::{FakeSegmentRepo, NewSegment, SegmentWriter};
use deino_core::transcript::{
    TranscriptEditDeps, TranscriptEditService, TranscriptIndexer, TranscriptService,
};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, SegmentId, SpeakerId};
use deino_testkit::at;
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-1";
const SEGMENT: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";
const SPEAKER: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6f";

/// Re-indexes nothing: the contract is the JSON.
struct NoReindex;

impl TranscriptIndexer for NoReindex {
    fn changed(&self, _meetings: &[MeetingId]) {}
}

/// A meeting with one Me line.
fn repo() -> Arc<FakeSegmentRepo> {
    let repo = Arc::new(FakeSegmentRepo::default());
    let segment = NewSegment {
        id: SEGMENT.parse::<SegmentId>().unwrap(),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::Mic,
        speaker_id: Some(SPEAKER.parse::<SpeakerId>().unwrap()),
        start: Millis(1_200),
        end: Millis(2_400),
        text: "Ship it on Monday.".into(),
        words: Vec::new(),
    };
    tauri::async_runtime::block_on(repo.append(&[segment])).unwrap();
    repo.title_meeting(&MeetingId::new(MEETING), "Standup");
    repo
}

/// Runs `plugin:transcript|<command>` with each call's arguments in turn,
/// on one app, and returns the last answer.
fn calls(steps: &[(&str, Value)]) -> Result<Value, Value> {
    let app = mock_app(mock_builder().plugin(super::init()));
    let repo = repo();
    app.manage(Arc::new(TranscriptService::new(repo.clone())));
    app.manage(Arc::new(TranscriptEditService::new(TranscriptEditDeps {
        editor: repo.clone(),
        writer: repo,
        indexer: Arc::new(NoReindex),
        clock: Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z"))),
    })));
    let window = main_window(&app);
    let mut last = Ok(Value::Null);
    for (command, args) in steps {
        last = invoke(&window, &format!("plugin:transcript|{command}"), args.clone());
    }
    last
}

fn get(meeting: &str) -> Result<Value, Value> {
    calls(&[("get", json!({ "meetingId": meeting }))])
}

fn scope() -> Value {
    json!({ "kind": "meeting", "meetingId": MEETING })
}

#[test]
fn get_takes_a_meeting_id_and_returns_its_segments_with_camel_case_fields() {
    let transcript = get(MEETING).unwrap();

    let segment = &transcript["segments"][0];
    assert_eq!(segment["id"], SEGMENT);
    assert_eq!(segment["meetingId"], MEETING);
    assert_eq!(segment["speakerId"], SPEAKER);
    assert_eq!(segment["channel"], "mic");
    assert_eq!(segment["start"], 1_200);
    assert_eq!(segment["end"], 2_400);
    assert_eq!(segment["text"], "Ship it on Monday.");
    assert_eq!(segment["isDeleted"], false);
    assert_eq!(segment["originalText"], Value::Null);
}

#[test]
fn get_of_an_unknown_meeting_is_an_empty_transcript() {
    assert_eq!(get("meeting-none"), Ok(json!({ "segments": [] })));
}

#[test]
fn edit_takes_a_segment_id_and_text_and_returns_the_line() {
    let edited = calls(&[(
        "edit",
        json!({ "segmentId": SEGMENT, "text": "Ship it on Friday." }),
    )])
    .unwrap();

    assert_eq!(edited["text"], "Ship it on Friday.");
    assert_eq!(edited["originalText"], "Ship it on Monday.");
    assert_eq!(edited["editedAt"], "2026-10-08T10:00:00Z");
}

#[test]
fn a_blank_edit_is_invalid() {
    let error = calls(&[("edit", json!({ "segmentId": SEGMENT, "text": " " }))]).unwrap_err();

    assert_eq!(error["code"], "invalid");
}

#[test]
fn delete_and_restore_take_a_segment_id_and_return_nothing() {
    let deleted = json!({ "segmentId": SEGMENT });
    let after_delete = calls(&[
        ("delete", deleted.clone()),
        ("get", json!({ "meetingId": MEETING })),
    ]);
    let restored = calls(&[("delete", deleted.clone()), ("restore", deleted)]);

    assert_eq!(after_delete, Ok(json!({ "segments": [] })));
    assert_eq!(restored, Ok(Value::Null));
}

#[test]
fn preview_replace_takes_a_tagged_scope_and_counts_per_meeting() {
    let request = json!({ "find": "monday", "scope": scope(), "matchCase": false });

    let preview = calls(&[("preview_replace", json!({ "request": request }))]).unwrap();

    assert_eq!(
        preview,
        json!({
            "matches": 1,
            "segments": 1,
            "meetings": [{ "meetingId": MEETING, "title": "Standup", "matches": 1 }],
        })
    );
}

#[test]
fn replace_takes_the_request_and_replacement_and_answers_counts() {
    let request = json!({ "find": "Monday", "scope": { "kind": "all" }, "matchCase": true });

    let outcome = calls(&[(
        "replace",
        json!({ "request": request, "replacement": "Friday" }),
    )]);

    assert_eq!(
        outcome,
        Ok(json!({ "matches": 1, "segments": 1, "meetings": 1 }))
    );
}
