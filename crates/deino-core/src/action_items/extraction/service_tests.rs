//! Extraction end to end over `MockProvider` and the real repositories:
//! the prompt's content, resolution, the idempotent regeneration and the
//! skip and failure paths.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{
    ActionItemPriority, ActionItemStatus, AttendeeRole, Channel, Clock, MeetingId, Millis,
    PaletteColor, SegmentId, SpeakerId,
};
use deino_llm::{LlmError, MockProvider, ProviderId, Route, mock_gateway};

use crate::action_items::extraction::*;
use crate::action_items::test_world::{STANDUP, World};
use crate::action_items::{
    ActionItemEdit, ActionItemFilter, ActionItemRepo, RecordedActionItemEvents,
    SqliteActionItemRepo,
};
use crate::meetings::SqliteMeetingRepo;
use crate::notes::{FakeNotesRepo, NotesRepo, StoredNotes};
use crate::people::{Attendee, AttendeeRepo, SqlitePeopleRepo};
use crate::segments::{NewSegment, SegmentWriter, Speaker, SpeakerRepo, SqliteSegmentRepo};

/// Maya's promise, a floated idea and the user's own promise.
const LINES: [(&str, &str); 3] = [
    (
        "maya",
        "I can have it on the endpoint by end of day tomorrow.",
    ),
    ("guest", "Maybe we could try Zapier at some point?"),
    ("me", "I'll send the postmortem to Owen on Monday."),
];

const ANSWER: &str = r#"{"items":[
  {"title":"Add a last-modified field to the jobs endpoint","description":"For Diego's offline cache.",
   "assignee":"Maya","due_phrase":"by end of day tomorrow","due":"2026-10-09","priority":"high",
   "evidence_quote":"I can have it on the endpoint by end of day tomorrow","evidence_segment_id":"s0"},
  {"title":"Send the postmortem to Owen.","description":"","assignee":"me","due_phrase":"on Monday",
   "due":"2026-10-11","evidence_quote":"I'll send the postmortem to Owen on Monday","evidence_segment_id":"s9"}
]}"#;

struct Harness {
    world: World,
    service: ExtractionService,
    mock: Arc<MockProvider>,
    events: Arc<RecordedActionItemEvents>,
    segments: Vec<SegmentId>,
}

/// The standup with three speakers, two attendees and the user's notes.
async fn harness(mock: MockProvider) -> Harness {
    let world = World::new().await;
    let segments = transcript(&world).await;
    let notes = attend_and_note(&world).await;
    let mock = Arc::new(mock);
    let events = Arc::new(RecordedActionItemEvents::default());
    let service = extraction(&world, &mock, notes, events.clone());
    Harness {
        world,
        service,
        mock,
        events,
        segments,
    }
}

/// The user and Maya attend; the user noted something about Maya's item.
async fn attend_and_note(world: &World) -> Arc<FakeNotesRepo> {
    let people = SqlitePeopleRepo::new(world.database.clone());
    for person_id in [world.me, world.maya] {
        let attendee = Attendee {
            meeting_id: standup(),
            person_id,
            role: AttendeeRole::Invitee,
            rsvp: None,
        };
        people.upsert_attendee(&attendee).await.unwrap();
    }
    let notes = Arc::new(FakeNotesRepo::default());
    let stored = StoredNotes {
        meeting_id: standup(),
        doc: deino_doc::Doc::empty(),
        markdown: "- maya: last modified at!!".into(),
        created_at: world.clock.now(),
        updated_at: world.clock.now(),
    };
    notes.save(&stored).await.unwrap();
    notes
}

/// The service over the world's repositories and the mocked gateway.
fn extraction(
    world: &World,
    mock: &Arc<MockProvider>,
    notes: Arc<FakeNotesRepo>,
    events: Arc<RecordedActionItemEvents>,
) -> ExtractionService {
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    let people = Arc::new(SqlitePeopleRepo::new(world.database.clone()));
    let meetings = Arc::new(SqliteMeetingRepo::new(world.database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(world.database.clone()));
    let repo = Arc::new(SqliteActionItemRepo::new(world.database.clone()));
    ExtractionService::new(ExtractionDeps {
        sources: ContextSources {
            meetings: meetings.clone(),
            lifecycle: meetings,
            segments: segments.clone(),
            speakers: segments,
            attendees: people.clone(),
            people: people.clone(),
            me: people,
            notes,
        },
        model: Arc::new(GatewayActionItemModel::new(gateway)),
        items: repo.clone(),
        batch: repo,
        events,
        clock: world.clock.clone(),
        zone: world.zone.clone(),
    })
}

fn standup() -> MeetingId {
    MeetingId::new(STANDUP)
}

/// Speakers "Me", Maya (identified) and an unidentified guest; one line each.
async fn transcript(world: &World) -> Vec<SegmentId> {
    let repo = SqliteSegmentRepo::new(world.database.clone());
    let speaker = |label: &str, person, is_me| Speaker {
        id: SpeakerId::generate(world.clock.as_ref()),
        meeting_id: standup(),
        label: label.into(),
        person_id: person,
        is_me,
        color: PaletteColor::new(1).unwrap(),
    };
    let speakers = [
        ("maya", speaker("Speaker 2", Some(world.maya), false)),
        ("guest", speaker("Speaker 3", None, false)),
        ("me", speaker("Me", Some(world.me), true)),
    ];
    for (_, speaker) in &speakers {
        repo.add_speaker(speaker).await.unwrap();
    }
    let segments: Vec<NewSegment> = (0_u32..)
        .zip(LINES)
        .map(|(index, (who, text))| {
            let (_, speaker) = speakers.iter().find(|(name, _)| *name == who).unwrap();
            let start = index * 4_000 + 3_000;
            NewSegment {
                id: SegmentId::generate(world.clock.as_ref()),
                meeting_id: standup(),
                channel: Channel::System,
                speaker_id: Some(speaker.id),
                start: Millis(start),
                end: Millis(start + 3_500),
                text: text.into(),
                words: Vec::new(),
            }
        })
        .collect();
    repo.append(&segments).await.unwrap();
    segments.into_iter().map(|segment| segment.id).collect()
}

/// The item titled `title`.
fn titled<'a>(
    items: &'a [crate::action_items::ActionItem],
    title: &str,
) -> &'a crate::action_items::ActionItem {
    items.iter().find(|item| item.title == title).unwrap()
}

async fn items(harness: &Harness) -> Vec<crate::action_items::ActionItem> {
    let repo = SqliteActionItemRepo::new(harness.world.database.clone());
    repo.list_for_meeting(&standup()).await.unwrap()
}

#[tokio::test]
async fn the_prompt_carries_the_local_date_people_notes_and_numbered_lines() {
    let harness =
        harness(MockProvider::new().reply(Route::ExtractActionItems, r#"{"items":[]}"#)).await;

    harness.service.extract(&standup()).await.unwrap();

    let request = &harness.mock.requests()[0];
    let user = request.messages[0].text();
    assert_eq!(request.route, Route::ExtractActionItems);
    assert!(
        request
            .system
            .as_deref()
            .unwrap_or_default()
            .contains("clear commitments")
    );
    for expected in [
        "Title: Eng standup\nDate: Thursday 8 October 2026, 08:30 (UTC-07:00)",
        "- Shreyas Rao (me)\n- Maya Chen",
        "<user_notes>\n- maya: last modified at!!\n</user_notes>",
        "[s0 00:03] Maya Chen: I can have it on the endpoint by end of day tomorrow.",
        "[s1 00:07] Speaker 3: Maybe we could try Zapier",
        "[s2 00:11] Me: I'll send the postmortem",
    ] {
        assert!(user.contains(expected), "{expected:?} not in\n{user}");
    }
}

#[tokio::test]
async fn answers_are_resolved_to_people_local_due_days_and_checked_evidence() {
    let harness = harness(MockProvider::new().reply(Route::ExtractActionItems, ANSWER)).await;

    let outcome = harness.service.extract(&standup()).await.unwrap();

    assert_eq!(
        outcome,
        ExtractionOutcome::Extracted {
            saved: 2,
            replaced: 0
        }
    );
    let stored = items(&harness).await;
    let maya = titled(&stored, "Add a last-modified field to the jobs endpoint");
    assert_eq!(maya.assignee_person_id, Some(harness.world.maya));
    assert_eq!(maya.due_at, Some(harness.world.due("2026-10-09")));
    assert_eq!(maya.priority, Some(ActionItemPriority::High));
    assert_eq!(maya.source_segment_id, Some(harness.segments[0]));
    let mine = titled(&stored, "Send the postmortem to Owen");
    assert_eq!(mine.assignee_person_id, Some(harness.world.me));
    // "on Monday" from Thursday is the 12th, whatever the model computed.
    assert_eq!(mine.due_at, Some(harness.world.due("2026-10-12")));
    // The cited "s9" does not exist; the quote is on line 2.
    assert_eq!(mine.source_segment_id, Some(harness.segments[2]));
}

#[tokio::test]
async fn regenerating_replaces_only_untouched_ai_items() {
    let answers = MockProvider::new()
        .reply(Route::ExtractActionItems, ANSWER)
        .reply(Route::ExtractActionItems, ANSWER);
    let harness = harness(answers).await;
    harness.service.extract(&standup()).await.unwrap();
    let first = items(&harness).await;
    let ticked = titled(&first, "Send the postmortem to Owen").id;
    let replaced = titled(&first, "Add a last-modified field to the jobs endpoint").id;
    harness.world.clock.advance(TimeDelta::minutes(1));
    let done = ActionItemEdit::Status {
        status: ActionItemStatus::Done,
    };
    harness
        .world
        .service()
        .0
        .update(ticked, &done)
        .await
        .unwrap();

    let outcome = harness.service.extract(&standup()).await.unwrap();

    // Maya's untouched item is replaced; the done one is kept, not repeated.
    assert_eq!(
        outcome,
        ExtractionOutcome::Extracted {
            saved: 1,
            replaced: 1
        }
    );
    let second = items(&harness).await;
    assert_eq!(second.len(), 2);
    let kept = second.iter().find(|item| item.id == ticked).unwrap();
    assert_eq!(kept.status, ActionItemStatus::Done);
    assert!(second.iter().all(|item| item.id != replaced));
    // Created twice, and the replaced item announced as deleted.
    assert_eq!(harness.events.changes().len(), 3);
}

#[tokio::test]
async fn items_written_by_the_user_survive_any_regeneration() {
    let harness =
        harness(MockProvider::new().reply(Route::ExtractActionItems, r#"{"items":[]}"#)).await;
    let mine = harness
        .world
        .service()
        .0
        .create(crate::action_items::NewActionItem {
            title: "Order pizza".into(),
            meeting_id: Some(standup()),
            ..crate::action_items::NewActionItem::default()
        })
        .await
        .unwrap();

    harness.service.extract(&standup()).await.unwrap();

    let kept: Vec<_> = items(&harness)
        .await
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(kept, [mine.id]);
    let listed = harness
        .world
        .service()
        .0
        .list(&ActionItemFilter::meeting(standup()))
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn a_meeting_without_transcript_is_skipped_without_asking_the_model() {
    let harness = harness(MockProvider::new()).await;
    let empty = MeetingId::new(crate::action_items::test_world::ACME_CALL);

    let outcome = harness.service.extract(&empty).await.unwrap();

    assert_eq!(outcome, ExtractionOutcome::NoTranscript);
    assert!(harness.mock.requests().is_empty());
}

#[tokio::test]
async fn model_failures_keep_existing_items_and_say_whether_to_retry() {
    let limited = LlmError::RateLimited {
        provider: ProviderId::Anthropic,
        retry_after: None,
    };
    let harness = harness(MockProvider::new().fail(Route::ExtractActionItems, limited)).await;

    let error = harness.service.extract(&standup()).await.unwrap_err();
    let missing = harness
        .service
        .extract(&MeetingId::new("meeting-nope"))
        .await
        .unwrap_err();

    assert!(error.is_retryable());
    assert!(!missing.is_retryable());
    assert!(items(&harness).await.is_empty());
}
