use chrono::{DateTime, Utc};
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, Channel, FakeClock, MeetingId, Millis,
    SegmentId,
};
use deino_testkit::at;

use super::*;
use crate::action_items::{ActionItem, SqliteActionItemRepo};
use crate::export::testing::{STANDUP, seeded_assembler, sqlite_ports};
use crate::people::{PersonRepo, SqlitePeopleRepo};
use crate::segments::{NewSegment, SegmentWriter, SqliteSegmentRepo};

#[tokio::test]
async fn the_standup_reads_back_with_names_times_and_attendees() {
    let (_, assembler) = seeded_assembler().await;

    let export = assembler.assemble(&MeetingId::new(STANDUP)).await.unwrap();

    assert_eq!(export.title, "Eng standup");
    assert_eq!(export.started_at.to_rfc3339(), "2026-10-08T08:31:04-07:00");
    let names: Vec<&str> = export
        .attendees
        .iter()
        .map(|person| person.name.as_str())
        .collect();
    // Noor Haddad declined, so she is not an attendee.
    assert_eq!(
        names,
        [
            "Shreyas",
            "Maya Chen",
            "Diego Alvarez",
            "Priya Nair",
            "Tom Becker"
        ]
    );
    assert!(export.attendees[0].is_me);
    let first = &export.transcript[0];
    assert_eq!(
        (first.speaker.as_str(), first.is_me, first.start),
        ("Shreyas", true, Millis(696))
    );
    assert_eq!(export.transcript[2].speaker, "Maya Chen");
    assert_eq!(export.notes, None);
}

#[tokio::test]
async fn an_unknown_meeting_is_not_found() {
    let (_, assembler) = seeded_assembler().await;

    let error = assembler
        .assemble(&MeetingId::new("meeting-x"))
        .await
        .unwrap_err();

    assert!(matches!(error, ExportError::NotFound(id) if id.as_str() == "meeting-x"));
}

#[tokio::test]
async fn action_items_carry_their_assignee_due_date_and_state() {
    let (database, assembler) = seeded_assembler().await;
    let maya = SqlitePeopleRepo::new(database.clone())
        .find_by_email(&deino_domain::Email::parse("maya@deino.dev").unwrap())
        .await
        .unwrap()
        .unwrap();
    let item = ActionItem {
        id: ActionItemId::generate(&FakeClock::new(at("2026-10-08T16:00:00Z"))),
        meeting_id: Some(MeetingId::new(STANDUP)),
        title: "Add the field".into(),
        description: "For the offline cache".into(),
        assignee_person_id: Some(maya.id),
        // 02:00 UTC on the 10th is still the 9th at UTC−07:00.
        due_at: Some(at("2026-10-10T02:00:00Z")),
        status: ActionItemStatus::Done,
        priority: None,
        source_segment_id: None,
        completed_at: Some(at("2026-10-09T10:00:00Z")),
        created_by: ActionItemOrigin::Ai,
        created_at: at("2026-10-08T16:00:00Z"),
        updated_at: at("2026-10-08T16:00:00Z"),
    };
    SqliteActionItemRepo::new(database)
        .create(&item)
        .await
        .unwrap();

    let export = assembler.assemble(&MeetingId::new(STANDUP)).await.unwrap();

    let exported = &export.action_items[0];
    assert_eq!(exported.assignee.as_deref(), Some("Maya Chen"));
    assert_eq!(
        exported.due.map(|date| date.to_string()).as_deref(),
        Some("2026-10-09")
    );
    assert!(exported.done);
}

#[tokio::test]
async fn unattributed_speech_is_named_by_its_channel() {
    let database = deino_db::test_db().await;
    deino_testkit::seed::meeting(&database, "meeting-1", "Call", at("2026-10-08T09:00:00Z")).await;
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    let segment = |channel: Channel, start: u32| NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new("meeting-1"),
        channel,
        speaker_id: None,
        start: Millis(start),
        end: Millis(start + 500),
        text: "hello".into(),
        words: Vec::new(),
    };
    let segments = [
        segment(Channel::Mic, 0),
        segment(Channel::System, 1_000),
        segment(Channel::Mixed, 2_000),
    ];
    SqliteSegmentRepo::new(database.clone())
        .append(&segments)
        .await
        .unwrap();

    let export = ExportAssembler::new(sqlite_ports(&database))
        .assemble(&MeetingId::new("meeting-1"))
        .await
        .unwrap();

    let speakers: Vec<(&str, bool)> = export
        .transcript
        .iter()
        .map(|line| (line.speaker.as_str(), line.is_me))
        .collect();
    assert_eq!(
        speakers,
        [("Me", true), ("Them", false), ("Speaker", false)]
    );
    let created: DateTime<Utc> = at("2026-10-08T09:00:00Z");
    assert_eq!(export.started_at.to_utc(), created);
}
