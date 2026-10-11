//! The seeded fixtures, read back through every repository.
//!
//! `deino-testkit` writes the fixtures with raw SQL (it cannot depend on this
//! crate), so these tests are what keep the two in step: every JSON payload,
//! enum and link the seeder writes must decode through the real ports.

use chrono::TimeDelta;
use deino_db::{Database, test_db};
use deino_domain::{Email, MeetingId, MeetingStatus};
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};

use crate::calendar::{CalendarEventRepo, CalendarRepo, SqliteCalendarRepo};
use crate::meetings::{MeetingLifecycleRepo, MeetingRepo, MeetingTrashRepo, SqliteMeetingRepo};
use crate::notes::{NotesRepo, SqliteNotesRepo};
use crate::people::{AttendeeRepo, PersonRepo, SqlitePeopleRepo};
use crate::segments::{Page, SegmentReader, SpeakerRepo, SqliteSegmentRepo};

async fn seeded() -> (Database, Fixtures) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    (database, Fixtures::load().unwrap())
}

#[tokio::test]
async fn seeded_transcripts_read_back_with_words_and_speakers() {
    let (database, fixtures) = seeded().await;
    let fixture = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let repo = SqliteSegmentRepo::new(database);

    let segments = repo.list(&fixture.id, Page::ALL).await.unwrap();
    let speakers = repo.speakers(&fixture.id).await.unwrap();

    assert_eq!(segments.len(), fixture.segments.len());
    let first = &fixture.segments[0];
    assert_eq!(
        (segments[0].start, segments[0].channel),
        (first.start_ms, first.channel)
    );
    assert_eq!(segments[0].words.len(), first.words.len());
    assert_eq!(speakers.len(), fixture.speakers.len());
    let me: Vec<&str> = speakers
        .iter()
        .filter(|s| s.is_me)
        .map(|s| s.label.as_str())
        .collect();
    let user = fixture.speakers.iter().find(|speaker| {
        speaker.attendee.as_ref().map(Email::as_str) == Some(deino_testkit::fixture::USER_EMAIL)
    });
    assert_eq!(me, [user.unwrap().label.as_str()]);
    assert!(segments.iter().all(|segment| segment.speaker_id.is_some()));
}

#[tokio::test]
async fn seeded_meetings_are_ready_and_linked_to_their_events() {
    let (database, fixtures) = seeded().await;
    let fixture = fixtures.meeting(MeetingKind::Standup).unwrap();
    let meetings = SqliteMeetingRepo::new(database.clone());
    let calendar = SqliteCalendarRepo::new(database);

    let state = meetings.state(&fixture.id).await.unwrap().unwrap();
    let event_id = state.calendar_event_id.unwrap();
    let event = calendar.event(&event_id).await.unwrap().unwrap();

    assert_eq!(state.status, MeetingStatus::Ready);
    assert_eq!(state.started_at, Some(fixture.started_at));
    assert_eq!(event.provider_event_id, fixture.calendar_event.id);
    assert_eq!(event.attendees.len(), fixture.attendees.len());
    assert_eq!(state.series_id, event.series_id);
    let calendars = calendar.calendars().await.unwrap();
    assert_eq!(event.calendar_id, Some(calendars[0].id));
    assert_eq!(calendars[0].title, "Work");
    let listed = meetings.list().await.unwrap();
    assert_eq!(listed.len(), fixtures.meetings().len());
}

#[tokio::test]
async fn seeded_people_attend_their_meetings() {
    let (database, fixtures) = seeded().await;
    let fixture = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let people = SqlitePeopleRepo::new(database);
    let user = Email::parse(deino_testkit::fixture::USER_EMAIL).unwrap();

    let me = people.find_by_email(&user).await.unwrap().unwrap();
    let attended = people.meetings_of(&me.id).await.unwrap();
    let attendees = people.attendees(&fixture.id).await.unwrap();

    assert_eq!(
        attended.len(),
        fixtures.meetings().len(),
        "the user attends every fixture"
    );
    assert_eq!(attendees.len(), fixture.attendees.len());
    assert!(!me.is_me, "fixtures never claim the real user's person");
    let newest_first = attended
        .windows(2)
        .all(|pair| pair[0].created_at >= pair[1].created_at);
    assert!(newest_first);
}

#[tokio::test]
async fn seeded_user_notes_open_as_documents() {
    let (database, fixtures) = seeded().await;
    let repo = SqliteNotesRepo::new(database);
    let one_on_one = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let standup = fixtures.meeting(MeetingKind::Standup).unwrap();

    let notes = repo.get(&one_on_one.id).await.unwrap().unwrap();

    deino_doc::validate(&notes.doc).unwrap();
    assert!(notes.markdown.starts_with("## maya 1:1 10/7\n"));
    assert_eq!(notes.updated_at, one_on_one.ended_at());
    assert_eq!(standup.user_notes, None);
    assert_eq!(repo.get(&standup.id).await.unwrap(), None);
}

#[tokio::test]
async fn reseeding_restores_a_trashed_fixture() {
    let (database, _) = seeded().await;
    let meetings = SqliteMeetingRepo::new(database.clone());
    let id = MeetingId::new("meeting-fixture-standup");
    let later = chrono::DateTime::UNIX_EPOCH + TimeDelta::days(20_000);
    meetings.trash(&id, later).await.unwrap();

    seed_fixtures(&database).await.unwrap();

    let state = meetings.state(&id).await.unwrap().unwrap();
    assert_eq!(state.deleted_at, None);
}
