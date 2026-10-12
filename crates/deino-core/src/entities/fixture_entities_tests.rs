//! ORG-04's first acceptance check: the eight fixture meetings' invitations,
//! synced into an empty database, produce exactly the people and companies
//! the fixtures describe.
//!
//! The expectation is read from the fixtures themselves: every invitee who
//! accepted at least one invitation, except the user, is a person under the
//! name the calendar gave; the companies are the fixtures' `companies`. Sam
//! Ortiz declined his only invitation and is nobody; Jordan Ellis writes
//! from his own `jordanellis.dev`, which is no company.

use std::collections::BTreeSet;
use std::sync::Arc;

use deino_domain::{Clock, Email, Rsvp};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, USER_EMAIL};

use crate::calendar::{AttendeeDirectory, EventAttendee, SyncedEvent};
use crate::companies::{CompanyRepo, SqliteCompanyRepo};
use crate::entities::testing::World;
use crate::people::{PersonRepo, SqlitePeopleRepo};
use crate::profile::InternalDomains;

/// The fixture's calendar invitation as a sync would store it.
fn invitation(meeting: &FixtureMeeting, clock: &dyn Clock) -> SyncedEvent {
    let attendees: Vec<EventAttendee> = meeting
        .attendees
        .iter()
        .map(|attendee| EventAttendee {
            email: Some(attendee.email.as_str().to_owned()),
            name: Some(attendee.name.clone()),
            rsvp: Some(attendee.rsvp),
            is_organizer: attendee.role == deino_domain::AttendeeRole::Organizer,
        })
        .collect();
    let my_rsvp = meeting
        .attendees
        .iter()
        .find(|attendee| attendee.email.as_str() == USER_EMAIL)
        .map(|attendee| attendee.rsvp);
    SyncedEvent {
        id: deino_domain::CalendarEventId::generate(clock),
        provider_event_id: meeting.calendar_event.id.clone(),
        ical_uid: None,
        series_id: None,
        title: meeting.title.clone(),
        description: String::new(),
        start_at: meeting.started_at,
        end_at: meeting.ended_at(),
        is_all_day: false,
        conference_url: None,
        platform: None,
        organizer_email: None,
        attendees,
        my_rsvp,
        is_external: false,
    }
}

/// Everyone who accepted (or might attend) some invitation, but the user.
fn expected_people(fixtures: &Fixtures) -> BTreeSet<(String, String)> {
    fixtures
        .meetings()
        .iter()
        .flat_map(|meeting| &meeting.attendees)
        .filter(|attendee| attendee.rsvp != Rsvp::Declined && attendee.email.as_str() != USER_EMAIL)
        .map(|attendee| (attendee.email.as_str().to_owned(), attendee.name.clone()))
        .collect()
}

#[tokio::test]
async fn fixture_invitations_produce_the_expected_people_and_companies() {
    let world = World::new().await;
    let fixtures = Fixtures::load().unwrap();
    let people = Arc::new(SqlitePeopleRepo::new(world.database.clone()));
    let companies = Arc::new(SqliteCompanyRepo::new(world.database.clone()));
    let directory = AttendeeDirectory::over(people.clone(), companies.clone(), world.clock.clone());
    let events: Vec<SyncedEvent> = fixtures
        .meetings()
        .iter()
        .map(|meeting| invitation(meeting, world.clock.as_ref()))
        .collect();
    let own = InternalDomains::from_emails(&[Email::parse(USER_EMAIL).unwrap()]);

    directory.record(&events, &own).await.unwrap();

    let stored: BTreeSet<(String, String)> = people
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|person| (person.email.unwrap().as_str().to_owned(), person.name))
        .collect();
    assert_eq!(stored, expected_people(&fixtures));
    assert!(
        !stored
            .iter()
            .any(|(email, _)| email == "sam@quarrystone.vc")
    );
    let domains: BTreeSet<String> = companies
        .list()
        .await
        .unwrap()
        .into_iter()
        .filter_map(|company| company.domain.map(|domain| domain.as_str().to_owned()))
        .collect();
    let expected: BTreeSet<String> = fixtures
        .meetings()
        .iter()
        .flat_map(|meeting| &meeting.companies)
        .map(|company| company.domain.as_str().to_owned())
        .collect();
    assert_eq!(domains, expected);
}
