//! Contract tests: `get`'s JSON in and out through the real plugin and ACL.

use std::sync::Arc;

use deino_core::people::FakePeopleRepo;
use deino_core::segments::{FakeSegmentRepo, NewSegment, SegmentWriter, Speaker, SpeakerRepo};
use deino_core::transcript::{TranscriptPorts, TranscriptService};
use deino_domain::{Channel, MeetingId, Millis, PaletteColor, SegmentId, SpeakerId};
use serde_json::{Value, json};
use tauri::Manager;
use tauri::test::mock_builder;

use crate::features::ipc_tests::{invoke, main_window, mock_app};

const MEETING: &str = "meeting-1";
const SEGMENT: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";
const SPEAKER: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6f";

/// A transcript service over a meeting with one Me line.
fn service() -> TranscriptService {
    let repo = Arc::new(FakeSegmentRepo::default());
    let speaker = Speaker {
        id: SPEAKER.parse::<SpeakerId>().unwrap(),
        meeting_id: MeetingId::new(MEETING),
        label: "Me".into(),
        person_id: None,
        is_me: true,
        color: PaletteColor::FIRST,
    };
    let segment = NewSegment {
        id: SEGMENT.parse::<SegmentId>().unwrap(),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::Mic,
        speaker_id: Some(speaker.id),
        start: Millis(1_200),
        end: Millis(2_400),
        text: "Ship it on Monday.".into(),
        words: Vec::new(),
    };
    tauri::async_runtime::block_on(async {
        repo.add_speaker(&speaker).await.unwrap();
        repo.append(&[segment]).await.unwrap();
    });
    TranscriptService::new(TranscriptPorts {
        segments: repo.clone(),
        speakers: repo,
        people: Arc::new(FakePeopleRepo::default()),
    })
}

/// `plugin:transcript|get` for `meeting`.
fn get(meeting: &str) -> Result<Value, Value> {
    let app = mock_app(mock_builder().plugin(super::init()));
    app.manage(Arc::new(service()));
    invoke(
        &main_window(&app),
        "plugin:transcript|get",
        json!({ "meetingId": meeting }),
    )
}

#[test]
fn get_takes_a_meeting_id_and_returns_segments_and_named_speakers() {
    let transcript = get(MEETING).unwrap();

    assert_eq!(
        transcript["speakers"],
        json!([{ "id": SPEAKER, "name": "Me", "personId": null, "isMe": true, "color": 1 }])
    );
    let segment = &transcript["segments"][0];
    assert_eq!(segment["id"], SEGMENT);
    assert_eq!(segment["speakerId"], SPEAKER);
    assert_eq!(segment["channel"], "mic");
    assert_eq!(segment["start"], 1_200);
    assert_eq!(segment["end"], 2_400);
    assert_eq!(segment["text"], "Ship it on Monday.");
}

#[test]
fn get_of_an_unknown_meeting_is_an_empty_transcript() {
    assert_eq!(
        get("meeting-none"),
        Ok(json!({ "segments": [], "speakers": [] }))
    );
}
