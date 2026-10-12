//! The event body: field selection, the envelope, and NOT-09 redaction
//! through the export assembler and the action item subject.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{ActionItemId, ActionItemOrigin, ActionItemStatus, MeetingId};
use deino_testkit::fixture::seed_fixtures;

use super::payload::{PayloadRequest, build_event_payload, meeting_data};
use super::testing::{FixedMeeting, MEETING, start};
use super::{RepoSubjectItems, SubjectItems};
use crate::action_items::{ActionItem, ActionItemRepo, SqliteActionItemRepo};
use crate::automations::{AssembledMeetings, MeetingSource, PayloadFields};
use crate::export::{ExportAssembler, ExportPorts, FixedZone};
use crate::meetings::SqliteMeetingRepo;
use crate::redaction::{REDACTED, TextRedactor};

const NONE: PayloadFields = PayloadFields {
    notes: false,
    action_items: false,
    transcript: false,
    insights: false,
    attendees: false,
    link: false,
    audio_path: false,
};

async fn body(fields: PayloadFields) -> serde_json::Value {
    let meeting = MeetingId::new(MEETING);
    let request = PayloadRequest {
        kind: "notes.generated",
        meeting: &meeting,
        fields,
        at: start(),
    };
    let payload = build_event_payload(&FixedMeeting, request, None)
        .await
        .unwrap();
    serde_json::to_value(payload).unwrap()
}

#[tokio::test]
async fn with_no_fields_only_the_envelope_and_the_meeting_header_are_sent() {
    let json = body(NONE).await;

    assert_eq!(json["type"], "notes.generated");
    assert_eq!(json["timestamp"], "2026-10-12T09:00:00Z");
    let data = json["data"].as_object().unwrap();
    let mut keys: Vec<&str> = data.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["createdAt", "duration", "folders", "id", "name", "tags"]
    );
    assert_eq!(data["name"], "Acme sync");
    assert_eq!(data["duration"], 1_800);
}

#[tokio::test]
async fn each_chosen_field_adds_exactly_its_part_in_circleback_shape() {
    let all = PayloadFields {
        notes: true,
        action_items: true,
        transcript: true,
        insights: true,
        attendees: true,
        link: true,
        audio_path: true,
    };
    let json = body(all).await;
    let data = &json["data"];

    assert_eq!(data["notes"], "## Summary\n\nAcme renews.");
    assert_eq!(data["actionItems"][0]["title"], "Send the quote");
    assert_eq!(data["transcript"][0]["text"], "We renew.");
    assert_eq!(data["attendees"][0]["email"], "maya@acme.test");
    assert_eq!(data["insights"], serde_json::json!([]));
    assert_eq!(data["url"], "deino://meeting/meeting-acme");
    assert_eq!(data["audioPath"], "/recordings/meeting-acme/audio.m4a");

    let only_transcript = body(PayloadFields {
        transcript: true,
        ..NONE
    })
    .await;
    assert!(only_transcript["data"]["transcript"].is_array());
    assert!(only_transcript["data"].get("notes").is_none());
    assert!(only_transcript["data"].get("actionItems").is_none());
}

/// Redacts a fixture word, standing in for a card number or an API key.
struct HideWord(&'static str);

#[async_trait]
impl TextRedactor for HideWord {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .map(|text| text.replace(self.0, REDACTED))
            .collect()
    }
}

/// An action item on `meeting` whose title holds the "secret".
fn secret_item(meeting: &MeetingId) -> ActionItem {
    let at = start();
    ActionItem {
        id: ActionItemId::generate(&deino_domain::FakeClock::new(at)),
        meeting_id: Some(meeting.clone()),
        title: "Call Fernhollow about the invoice".into(),
        description: String::new(),
        assignee_person_id: None,
        due_at: None,
        status: ActionItemStatus::Open,
        priority: None,
        source_segment_id: None,
        completed_at: None,
        created_by: ActionItemOrigin::User,
        created_at: at,
        updated_at: at,
    }
}

#[tokio::test]
async fn with_redaction_on_the_payload_hides_secrets_in_notes_transcript_and_items() {
    let database = deino_db::test_db().await;
    seed_fixtures(&database).await.unwrap();
    let redactor: Arc<dyn TextRedactor> = Arc::new(HideWord("Fernhollow"));
    let zone = FixedZone(chrono::FixedOffset::west_opt(7 * 3_600).unwrap());
    let ports = ExportPorts::sqlite(&database, Arc::new(zone)).with_redactor(redactor.clone());
    let source = AssembledMeetings::new(
        Arc::new(ExportAssembler::new(ports)),
        Arc::new(SqliteMeetingRepo::new(database.clone())),
    );
    let meeting = MeetingId::new("meeting-fixture-standup");
    let all = PayloadFields {
        transcript: true,
        ..PayloadFields::default()
    };

    let repo = Arc::new(SqliteActionItemRepo::new(database.clone()));
    let item = secret_item(&meeting);
    repo.create(&item).await.unwrap();

    let export = source.export(&meeting).await.unwrap();
    let text = serde_json::to_string(&meeting_data(export, all)).unwrap();

    assert!(!text.contains("Fernhollow"), "a secret left the Mac");
    assert!(text.contains(REDACTED));
    let items = RepoSubjectItems::new(repo, redactor);
    let subject = items.subject(&item.id).await.unwrap().unwrap();
    assert_eq!(subject.title, format!("Call {REDACTED} about the invoice"));
    let gone = ActionItemId::generate(&deino_domain::FakeClock::new(start()));
    let unknown = items.subject(&gone).await.unwrap();
    assert!(unknown.is_none());
}
