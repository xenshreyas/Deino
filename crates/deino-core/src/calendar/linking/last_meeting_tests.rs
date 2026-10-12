//! "Review last meeting": the series or same title and people first, then
//! the latest earlier meeting with exactly the same people.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{AttendeeRole, MeetingId};
use deino_testkit::{at, seed};

use super::LastMeetingService;
use crate::calendar::linking::test_people::{add_person, person_id as person};
use crate::calendar::linking::{RelatedBy, RelatedService, SqliteRelatedRepo};
use crate::calendar::test_events::{ME, stored};
use crate::calendar::{CalendarEvent, EventAttendee};
use crate::people::{Attendee, AttendeeRepo, MeRepo, SqlitePeopleRepo};

/// Seeds meeting `id` titled `title` on Oct `day`, attended by `people`.
async fn meeting(database: &Database, (id, title): (&str, &str), day: u32, people: &[u128]) {
    seed::meeting(
        database,
        id,
        title,
        at(&format!("2026-10-{day:02}T10:00:00Z")),
    )
    .await;
    let repo = SqlitePeopleRepo::new(database.clone());
    for n in people {
        let row = Attendee {
            meeting_id: MeetingId::new(id),
            person_id: person(*n),
            role: AttendeeRole::Invitee,
            rsvp: None,
        };
        repo.upsert_attendee(&row).await.unwrap();
    }
}

/// The user (1), Ada (2) and Bo (3); a weekly sync with Ada on Oct 2, a
/// pricing review with her on Oct 5, a call with Ada and Bo on Oct 7 and a
/// review after the event (Oct 12).
async fn service() -> LastMeetingService {
    let database = test_db().await;
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    add_person(&people, 1, "Maya", Some(ME)).await;
    add_person(&people, 2, "Ada", Some("ada@acme.com")).await;
    add_person(&people, 3, "Bo", Some("bo@acme.com")).await;
    people
        .make_me(&person(1), at("2026-10-01T09:00:00Z"))
        .await
        .unwrap();
    meeting(&database, ("meeting-sync", "Weekly sync"), 2, &[1, 2]).await;
    meeting(&database, ("meeting-pricing", "Pricing review"), 5, &[2]).await;
    meeting(&database, ("meeting-trio", "Ada and Bo"), 7, &[1, 2, 3]).await;
    meeting(&database, ("meeting-later", "Pricing review"), 12, &[2]).await;
    let related = RelatedService::new(Arc::new(SqliteRelatedRepo::new(database)), people.clone());
    LastMeetingService::new(Arc::new(related), people.clone(), people.clone(), people)
}

/// An event `title` (Oct 9) with the user and `emails`.
fn event(title: &str, emails: &[&str]) -> CalendarEvent {
    let mut event = stored(title, 0);
    event
        .attendees
        .extend(emails.iter().map(|email| EventAttendee {
            email: Some((*email).into()),
            name: None,
            rsvp: None,
            is_organizer: false,
        }));
    event
}

async fn last(service: &LastMeetingService, event: &CalendarEvent) -> Option<(String, RelatedBy)> {
    let found = service.last_meeting(event).await.unwrap()?;
    Some((found.id.as_str().to_owned(), found.by))
}

fn hit(id: &str, by: RelatedBy) -> (String, RelatedBy) {
    (id.to_owned(), by)
}

#[tokio::test]
async fn the_same_title_and_people_come_first() {
    let service = service().await;
    let sync = event("Weekly sync", &["ada@acme.com"]);
    assert_eq!(
        last(&service, &sync).await,
        Some(hit("meeting-sync", RelatedBy::TitleAndPeople))
    );
}

#[tokio::test]
async fn else_the_latest_earlier_meeting_with_the_same_people() {
    let service = service().await;
    let design = event("Design review", &["ada@acme.com"]);
    assert_eq!(
        last(&service, &design).await,
        Some(hit("meeting-pricing", RelatedBy::People))
    );
    let trio = event("Planning", &["bo@acme.com", "ada@acme.com"]);
    assert_eq!(
        last(&service, &trio).await,
        Some(hit("meeting-trio", RelatedBy::People))
    );
}

#[tokio::test]
async fn strangers_or_only_the_user_have_no_last_meeting() {
    let service = service().await;
    assert_eq!(
        last(&service, &event("Intro", &["new@else.com"])).await,
        None
    );
    assert_eq!(last(&service, &event("Focus", &[])).await, None);
}
