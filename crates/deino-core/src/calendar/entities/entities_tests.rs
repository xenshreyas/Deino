//! People and companies added from invitations.

use std::sync::Arc;

use deino_domain::{Email, Rsvp};

use super::*;
use crate::calendar::SyncWindow;
use crate::calendar::batch::SyncedEvent;
use crate::calendar::mapping::{BatchContext, to_batch};
use crate::calendar::test_events::{ME, NOW, clock, meeting, person, snapshot};
use crate::companies::FakeCompanyRepo;
use crate::people::FakePeopleRepo;

struct Harness {
    directory: AttendeeDirectory,
    people: Arc<FakePeopleRepo>,
    companies: Arc<FakeCompanyRepo>,
}

fn harness() -> Harness {
    let people = Arc::new(FakePeopleRepo::default());
    let companies = Arc::new(FakeCompanyRepo::default());
    let directory = AttendeeDirectory::new(people.clone(), companies.clone(), Arc::new(clock()));
    Harness {
        directory,
        people,
        companies,
    }
}

/// `events` as stored, with the user's own addresses they show.
fn synced(events: Vec<crate::calendar::SourceEvent>) -> (Vec<SyncedEvent>, InternalDomains) {
    let clock = clock();
    let context = BatchContext {
        provider: deino_domain::CalendarProvider::Apple,
        window: SyncWindow::around(deino_testkit::at(NOW)),
        clock: &clock,
        profile_emails: &[],
    };
    let (batch, own) = to_batch(&snapshot(events), &context);
    (batch.accounts[0].calendars[0].events.clone(), own)
}

fn email(address: &str) -> Email {
    Email::parse(address).unwrap()
}

#[tokio::test]
async fn invitees_become_people_at_their_companies_and_the_user_does_not() {
    let Harness {
        directory,
        people,
        companies,
    } = harness();
    let (events, own) = synced(vec![meeting("evt-1", 1)]);

    let added = directory.record(&events, &own).await.unwrap();

    assert_eq!(added, 2);
    let marcus = people
        .find_by_email(&email("marcus@brightkite.com"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(marcus.name, "Marcus");
    let company = companies
        .find(&marcus.company_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (company.name.as_str(), company.domain.unwrap().as_str()),
        ("Brightkite", "brightkite.com")
    );
    assert!(people.find_by_email(&email(ME)).await.unwrap().is_none());
}

#[tokio::test]
async fn personal_mail_providers_add_people_without_a_company() {
    let Harness {
        directory,
        people,
        companies,
    } = harness();
    let mut event = meeting("evt-1", 1);
    event
        .attendees
        .push(person("sam.lee@gmail.com", Some(Rsvp::Accepted)));
    let (events, own) = synced(vec![event]);

    directory.record(&events, &own).await.unwrap();

    let sam = people
        .find_by_email(&email("sam.lee@gmail.com"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!((sam.name.as_str(), sam.company_id), ("Sam Lee", None));
    assert_eq!(
        companies.list().await.unwrap().len(),
        2,
        "deino.dev and brightkite.com only"
    );
}

#[tokio::test]
async fn existing_people_are_left_as_the_user_edited_them() {
    let Harness {
        directory, people, ..
    } = harness();
    let (events, own) = synced(vec![meeting("evt-1", 1)]);
    directory.record(&events, &own).await.unwrap();
    let mut marcus = people
        .find_by_email(&email("marcus@brightkite.com"))
        .await
        .unwrap()
        .unwrap();
    marcus.name = "Marcus Webb".into();
    people.update(&marcus).await.unwrap();
    let fresh = AttendeeDirectory::new(
        people.clone(),
        Arc::new(FakeCompanyRepo::default()),
        Arc::new(clock()),
    );

    let added = fresh.record(&events, &own).await.unwrap();

    assert_eq!(added, 0);
    let stored = people
        .find_by_email(&email("marcus@brightkite.com"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.name, "Marcus Webb");
}

#[tokio::test]
async fn declined_events_declined_invitees_and_broadcasts_add_nobody() {
    let Harness {
        directory, people, ..
    } = harness();
    let mut declined_by_me = meeting("declined", 1);
    declined_by_me.attendees[0].rsvp = Some(Rsvp::Declined);
    let mut declined_by_guest = meeting("guest", 2);
    declined_by_guest.attendees[2].rsvp = Some(Rsvp::Declined);
    declined_by_guest.attendees.remove(1);
    declined_by_guest.organizer = None;
    let mut all_hands = meeting("all-hands", 3);
    all_hands.attendees = (0..=MAX_ATTENDEES_FOR_PEOPLE)
        .map(|n| person(&format!("employee{n}@deino.dev"), Some(Rsvp::Accepted)))
        .collect();
    let (events, own) = synced(vec![declined_by_me, declined_by_guest, all_hands]);

    let added = directory.record(&events, &own).await.unwrap();

    assert_eq!(added, 0);
    assert!(people.list().await.unwrap().is_empty());
}
