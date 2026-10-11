//! Contract tests: the JSON of the `speakers` commands through the real
//! plugin, ACL and error mapping, over SQLite.

use std::sync::Arc;

use deino_core::segments::{NewSegment, NewSpeaker, SegmentWriter, Speaker, SpeakerRepo};
use deino_core::segments::{SpeakerVoice, SqliteSegmentRepo};
use deino_db::Database;
use deino_domain::{Channel, Clock, MeetingId, Millis, PaletteColor, SegmentId, SpeakerId};
use deino_testkit::{FakeClock, at, seed, test_db};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, WebviewWindow};

use crate::composition::Speakers;
use crate::features::ipc_tests::{invoke, main_window, mock_app};

const NOW: &str = "2026-10-11T10:00:00Z";
const MEETING: &str = "meeting-1";

/// An app with the `speakers` plugin over a meeting with one diarized
/// speaker who spoke for 30 s.
fn speakers_app() -> (App<MockRuntime>, SpeakerId) {
    let app = mock_app(mock_builder().plugin(super::init()));
    let (database, speaker) = tauri::async_runtime::block_on(diarized_meeting());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    Speakers::build(&database, &clock).manage(&app);
    (app, speaker)
}

async fn diarized_meeting() -> (Database, SpeakerId) {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(NOW)).await;
    let clock = FakeClock::new(at(NOW));
    let speaker = Speaker {
        id: SpeakerId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        label: "Speaker 1".into(),
        person_id: None,
        is_me: false,
        color: PaletteColor::cycle(1),
    };
    let voice = SpeakerVoice {
        centroid: vec![0.0625; 256],
        embedding_model: "wespeaker@test".into(),
        speech: Millis(30_000),
    };
    let repo = SqliteSegmentRepo::new(database.clone());
    let new = NewSpeaker {
        speaker: speaker.clone(),
        voice: Some(voice),
    };
    repo.replace_speakers(&MeetingId::new(MEETING), &[new])
        .await
        .unwrap();
    let said = NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: Some(speaker.id),
        start: Millis(0),
        end: Millis(30_000),
        text: "hello".into(),
        words: Vec::new(),
    };
    repo.append(&[said]).await.unwrap();
    (database, speaker.id)
}

fn call(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    invoke(window, &format!("plugin:speakers|{command}"), args)
}

#[test]
fn participants_come_back_with_camel_case_fields_and_shares() {
    let (app, speaker) = speakers_app();

    let panel = call(
        &main_window(&app),
        "participants",
        json!({ "meetingId": MEETING }),
    )
    .unwrap();

    assert_eq!(panel[0]["speakerId"], speaker.to_string());
    assert_eq!(panel[0]["label"], "Speaker 1");
    assert_eq!(panel[0]["color"], 2);
    assert_eq!(panel[0]["isMe"], false);
    assert_eq!(panel[0]["person"], json!(null));
    assert_eq!(panel[0]["speech"], 30_000);
    assert_eq!(panel[0]["sharePercent"], 100);
    assert_eq!(panel[0]["suggestion"], json!(null));
}

#[test]
fn renaming_all_segments_takes_tagged_names_and_scopes_and_enrols() {
    let (app, speaker) = speakers_app();
    let window = main_window(&app);
    let rename = json!({
        "meetingId": MEETING,
        "speakerId": speaker.to_string(),
        "name": { "kind": "new", "name": "Alex" },
        "scope": { "kind": "all" },
    });

    let renamed = call(&window, "rename", json!({ "rename": rename })).unwrap();

    assert_eq!(renamed["person"]["name"], "Alex");
    assert_eq!(renamed["speakerId"], speaker.to_string());
    assert_eq!(renamed["enrolled"], true);
    let profiles = call(&window, "voice_profiles", json!({})).unwrap();
    assert_eq!(profiles[0]["name"], "Alex");
    assert_eq!(
        profiles[0]["voiceprints"][0]["sourceMeetingTitle"],
        "Standup"
    );
    let id = profiles[0]["voiceprints"][0]["id"].clone();
    call(&window, "delete_voiceprint", json!({ "id": id })).unwrap();
    assert_eq!(
        call(&window, "voice_profiles", json!({})).unwrap(),
        json!([])
    );
}

#[test]
fn failures_carry_their_codes() {
    let (app, speaker) = speakers_app();
    let window = main_window(&app);
    let merge =
        json!({ "meetingId": MEETING, "from": speaker.to_string(), "into": speaker.to_string() });
    let blank = json!({
        "meetingId": MEETING,
        "speakerId": speaker.to_string(),
        "name": { "kind": "new", "name": "  " },
        "scope": { "kind": "all" },
    });

    let same = call(&window, "merge", merge).unwrap_err();
    let invalid = call(&window, "rename", json!({ "rename": blank })).unwrap_err();
    let none = call(
        &window,
        "dismiss_suggestion",
        json!({ "speakerId": speaker.to_string() }),
    );

    assert_eq!(same["code"], "invalid");
    assert_eq!(invalid["code"], "invalid");
    assert_eq!(none.unwrap_err()["code"], "not_found");
}
