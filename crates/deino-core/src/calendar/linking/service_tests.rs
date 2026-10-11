//! [`EventLinkService`] over the real schema: linking pulls in the title,
//! series and attendees, re-linking replaces them and makes the notes offer
//! to regenerate, unlinking undoes it, and recording starts auto-link.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_db::{Database, test_db};
use deino_domain::{
    AttendeeRole, CalendarAccountId, CalendarEventId, CalendarProvider, Email, FakeClock,
    MeetingId, Rsvp, SeriesId,
};
use deino_testkit::{at, seed};

use super::test_people::{add_person, person_id};
use super::*;
use crate::calendar::{
    AttendeeDirectory, CalendarAccount, CalendarAccountRepo, CalendarError, CalendarEvent,
    CalendarEventRepo, EventAttendee, SqliteCalendarRepo, TimeRange,
};
use crate::companies::SqliteCompanyRepo;
use crate::enhance::{EnhanceBasis, MaterialSources, RegenerateReason};
use crate::meetings::{MeetingLifecycleRepo, MeetingRepo, SqliteMeetingRepo};
use crate::people::{Attendee, AttendeeRepo, MeRepo, PersonRepo, SqlitePeopleRepo};
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};
use crate::testing::lock;

const MEETING: &str = "meeting-1";
const ME: &str = "maya@deino.dev";

fn ten(minutes: i64) -> DateTime<Utc> {
    at("2026-10-09T10:00:00Z") + TimeDelta::minutes(minutes)
}

fn meeting() -> MeetingId {
    MeetingId::new(MEETING)
}

/// The calendar as the agenda shows it: every event overlapping a range.
#[derive(Default)]
struct FixedAgenda(Mutex<Vec<CalendarEvent>>);

#[async_trait]
impl EventAgenda for FixedAgenda {
    async fn events(&self, range: TimeRange) -> Result<Vec<CalendarEvent>, CalendarError> {
        let events = lock(&self.0).clone();
        Ok(events
            .into_iter()
            .filter(|event| event.start_at < range.end && event.end_at > range.start)
            .collect())
    }
}

struct Harness {
    database: Database,
    service: EventLinkService,
    agenda: Arc<FixedAgenda>,
    calendar: SqliteCalendarRepo,
    people: SqlitePeopleRepo,
    account: CalendarAccountId,
}

async fn harness() -> Harness {
    let database = test_db().await;
    let clock = Arc::new(FakeClock::new(ten(2)));
    let people = SqlitePeopleRepo::new(database.clone());
    add_person(&people, 1, "Maya", Some(ME)).await;
    people.make_me(&person_id(1), ten(0)).await.unwrap();
    let calendar = SqliteCalendarRepo::new(database.clone());
    let account = CalendarAccount {
        id: CalendarAccountId::generate(clock.as_ref()),
        provider: CalendarProvider::Apple,
        name: "On My Mac".into(),
        created_at: ten(0),
    };
    calendar.add_account(&account).await.unwrap();
    seed::meeting(&database, MEETING, "New meeting", ten(2)).await;
    let agenda = Arc::new(FixedAgenda::default());
    let service = EventLinkService::new(deps(&database, &agenda, clock).await);
    Harness {
        database,
        service,
        agenda,
        calendar,
        people,
        account: account.id,
    }
}

async fn deps(
    database: &Database,
    agenda: &Arc<FixedAgenda>,
    clock: Arc<FakeClock>,
) -> EventLinkDeps {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::default()),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    let directory = AttendeeDirectory::new(
        people.clone(),
        Arc::new(SqliteCompanyRepo::new(database.clone())),
        clock.clone(),
    );
    EventLinkDeps {
        meetings: meetings.clone(),
        lifecycle: meetings,
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        agenda: agenda.clone(),
        attendance: EventAttendance::new(directory, people.clone(), people, Arc::new(settings)),
        clock,
    }
}

fn invitee(email: &str, rsvp: Rsvp, is_organizer: bool) -> EventAttendee {
    EventAttendee {
        email: Some(email.into()),
        name: None,
        rsvp: Some(rsvp),
        is_organizer,
    }
}

impl Harness {
    /// Stores an event `title` from `start` (minutes after 10:00), 30 minutes long.
    async fn event(&self, title: &str, start: i64, attendees: Vec<EventAttendee>) -> CalendarEvent {
        let clock = FakeClock::new(ten(0));
        let event = CalendarEvent {
            id: CalendarEventId::generate(&clock),
            account_id: self.account,
            calendar_id: None,
            provider_event_id: title.into(),
            ical_uid: None,
            series_id: Some(SeriesId::new(format!("{title} series"))),
            title: title.into(),
            description: format!("About {title}"),
            start_at: ten(start),
            end_at: ten(start + 30),
            is_all_day: false,
            conference_url: None,
            platform: None,
            organizer_email: None,
            attendees,
            my_rsvp: Some(Rsvp::Accepted),
            is_external: true,
            record_override: None,
            created_at: ten(0),
            updated_at: ten(0),
        };
        self.calendar.upsert_event(&event).await.unwrap();
        lock(&self.agenda.0).push(event.clone());
        event
    }

    /// Design review at 10:00: the user, Ada organising, Bob, and Carol who declined.
    async fn design_review(&self) -> CalendarEvent {
        let attendees = vec![
            invitee(ME, Rsvp::Accepted, false),
            invitee("ada@acme.com", Rsvp::Accepted, true),
            invitee("bob@acme.com", Rsvp::Tentative, false),
            invitee("carol@acme.com", Rsvp::Declined, false),
        ];
        self.event("Design review", 0, attendees).await
    }

    /// Pricing review at 10:30: the user, Bob and Dan.
    async fn pricing_review(&self) -> CalendarEvent {
        let attendees = vec![
            invitee(ME, Rsvp::Accepted, true),
            invitee("bob@acme.com", Rsvp::Accepted, false),
            invitee("dan@brightkite.com", Rsvp::Accepted, false),
        ];
        self.event("Pricing review", 30, attendees).await
    }

    /// The meeting's attendees as (email, role).
    async fn attendees(&self) -> Vec<(String, AttendeeRole)> {
        let mut rows = Vec::new();
        for row in self.people.attendees(&meeting()).await.unwrap() {
            let person = self.people.find(&row.person_id).await.unwrap().unwrap();
            let email = person
                .email
                .map(|email| email.as_str().to_owned())
                .unwrap_or_default();
            rows.push((email, row.role));
        }
        rows.sort();
        rows
    }

    async fn title(&self) -> String {
        let meetings = SqliteMeetingRepo::new(self.database.clone());
        meetings.find(&meeting()).await.unwrap().unwrap().title
    }

    async fn basis(&self) -> (EnhanceBasis, crate::enhance::Material) {
        let material = MaterialSources::sqlite(&self.database)
            .gather(&meeting())
            .await
            .unwrap()
            .unwrap();
        (EnhanceBasis::of(&material), material)
    }
}

fn row(email: &str, role: AttendeeRole) -> (String, AttendeeRole) {
    (email.to_owned(), role)
}

#[tokio::test]
async fn linking_pulls_in_the_title_series_and_attendees() {
    let harness = harness().await;
    let design = harness.design_review().await;

    let change = harness.service.link(&meeting(), &design.id).await.unwrap();

    assert_eq!(change.title.as_deref(), Some("Design review"));
    assert_eq!((change.attendees_added, change.attendees_removed), (3, 0));
    assert_eq!(change.event.map(|event| event.id), Some(design.id));
    assert_eq!(harness.title().await, "Design review");
    let state = SqliteMeetingRepo::new(harness.database.clone())
        .state(&meeting())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.calendar_event_id, Some(design.id));
    assert_eq!(state.series_id, design.series_id);
    assert_eq!(
        harness.attendees().await,
        [
            row("ada@acme.com", AttendeeRole::Organizer),
            row("bob@acme.com", AttendeeRole::Invitee),
            row(ME, AttendeeRole::Invitee),
        ],
        "the user is the Me person; Carol declined"
    );
    let carol = Email::parse("carol@acme.com").unwrap();
    assert!(
        harness
            .people
            .find_by_email(&carol)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        harness.service.linked(&meeting()).await.unwrap(),
        Some(design)
    );
}

#[tokio::test]
async fn relinking_replaces_the_attendees_and_offers_to_regenerate_the_notes() {
    let harness = harness().await;
    let design = harness.design_review().await;
    let pricing = harness.pricing_review().await;
    harness.service.link(&meeting(), &design.id).await.unwrap();
    let (written, _) = harness.basis().await;

    let change = harness.service.link(&meeting(), &pricing.id).await.unwrap();

    assert_eq!((change.attendees_added, change.attendees_removed), (1, 1));
    assert_eq!(
        harness.attendees().await,
        [
            row("bob@acme.com", AttendeeRole::Invitee),
            row("dan@brightkite.com", AttendeeRole::Invitee),
            row(ME, AttendeeRole::Organizer),
        ]
    );
    assert_eq!(harness.title().await, "Pricing review");
    let (_, now) = harness.basis().await;
    assert_eq!(
        written.reasons(&now),
        [
            RegenerateReason::EventChanged,
            RegenerateReason::AttendeesChanged
        ]
    );
}

#[tokio::test]
async fn relinking_keeps_people_who_spoke() {
    let harness = harness().await;
    let design = harness.design_review().await;
    let pricing = harness.pricing_review().await;
    harness.service.link(&meeting(), &design.id).await.unwrap();
    let ada = harness
        .people
        .find_by_email(&Email::parse("ada@acme.com").unwrap())
        .await
        .unwrap()
        .unwrap();
    let spoke = Attendee {
        meeting_id: meeting(),
        person_id: ada.id,
        role: AttendeeRole::SpeakerOnly,
        rsvp: None,
    };
    harness.people.upsert_attendee(&spoke).await.unwrap();

    let change = harness.service.link(&meeting(), &pricing.id).await.unwrap();

    assert_eq!(change.attendees_removed, 0);
    assert!(
        harness
            .attendees()
            .await
            .contains(&row("ada@acme.com", AttendeeRole::SpeakerOnly))
    );
}

#[tokio::test]
async fn unlinking_clears_the_link_and_its_invitees_but_keeps_the_title() {
    let harness = harness().await;
    let design = harness.design_review().await;
    harness.service.link(&meeting(), &design.id).await.unwrap();

    let change = harness.service.unlink(&meeting()).await.unwrap();

    assert_eq!(change.event, None);
    assert_eq!((change.title, change.attendees_removed), (None, 3));
    assert!(harness.attendees().await.is_empty());
    assert_eq!(harness.title().await, "Design review");
    let state = SqliteMeetingRepo::new(harness.database.clone())
        .state(&meeting())
        .await
        .unwrap()
        .unwrap();
    assert_eq!((state.calendar_event_id, state.series_id), (None, None));
}

#[tokio::test]
async fn a_recording_start_links_the_nearest_event_once() {
    let harness = harness().await;
    let design = harness.design_review().await;
    harness.pricing_review().await;

    let linked = harness
        .service
        .auto_link(&meeting(), TitleRule::Keep)
        .await
        .unwrap();
    let again = harness
        .service
        .auto_link(&meeting(), TitleRule::FromEvent)
        .await
        .unwrap();

    assert_eq!(
        linked.and_then(|change| change.event).map(|event| event.id),
        Some(design.id)
    );
    assert_eq!(again, None, "an already linked meeting is left alone");
    assert_eq!(
        harness.title().await,
        "New meeting",
        "the title it was started with stays"
    );
}

#[tokio::test]
async fn a_recording_start_without_a_near_event_links_nothing() {
    let harness = harness().await;
    harness.event("Lunch", 120, Vec::new()).await;

    let linked = harness
        .service
        .auto_link(&meeting(), TitleRule::FromEvent)
        .await
        .unwrap();

    assert_eq!(linked, None);
    assert_eq!(harness.service.linked(&meeting()).await.unwrap(), None);
}

#[tokio::test]
async fn suggestions_are_the_events_around_the_meeting_closest_first() {
    let harness = harness().await;
    harness.event("Yesterday", -20 * 60, Vec::new()).await;
    harness.design_review().await;
    harness.pricing_review().await;
    harness.event("Tomorrow", 24 * 60, Vec::new()).await;

    let suggested = harness.service.suggestions(&meeting()).await.unwrap();

    let titles: Vec<_> = suggested.iter().map(|event| event.title.as_str()).collect();
    assert_eq!(titles, ["Design review", "Pricing review", "Yesterday"]);
}

#[tokio::test]
async fn a_broadcast_invitation_adds_no_new_people() {
    let harness = harness().await;
    let mut attendees: Vec<_> = (0..30)
        .map(|n| invitee(&format!("guest{n}@acme.com"), Rsvp::Accepted, false))
        .collect();
    attendees.push(invitee(ME, Rsvp::Accepted, false));
    let all_hands = harness.event("All hands", 0, attendees).await;

    let change = harness
        .service
        .link(&meeting(), &all_hands.id)
        .await
        .unwrap();

    assert_eq!(change.attendees_added, 1, "only the user");
    assert_eq!(harness.people.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn unknown_meetings_and_events_are_not_found() {
    let harness = harness().await;
    let design = harness.design_review().await;
    let missing_event = CalendarEventId::generate(&FakeClock::new(ten(0)));

    let no_event = harness.service.link(&meeting(), &missing_event).await;
    let no_meeting = harness
        .service
        .link(&MeetingId::new("meeting-missing"), &design.id)
        .await;

    assert!(matches!(no_event, Err(LinkError::EventNotFound(id)) if id == missing_event));
    assert!(matches!(no_meeting, Err(LinkError::MeetingNotFound(_))));
    assert!(matches!(
        harness
            .service
            .suggestions(&MeetingId::new("meeting-missing"))
            .await,
        Err(LinkError::MeetingNotFound(_))
    ));
}
