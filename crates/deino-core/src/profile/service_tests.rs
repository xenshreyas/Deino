//! `ProfileService` over fakes: the saved profile, the "Me" person and the
//! system context.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{Email, FakeClock, PersonId};
use deino_testkit::at;

use super::*;
use crate::people::{FakePeopleRepo, MeRepo, Person, PersonRepo};
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, SettingsService};

struct Harness {
    service: ProfileService,
    people: Arc<FakePeopleRepo>,
    clock: Arc<FakeClock>,
}

async fn harness() -> Harness {
    let settings = SettingsService::load(
        Arc::new(FakeSettingsStore::with(None)),
        Arc::new(RecordedSettingsEvents::default()),
    )
    .await
    .unwrap();
    let people = Arc::new(FakePeopleRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    let service = ProfileService::new(
        Arc::new(settings),
        people.clone(),
        people.clone(),
        clock.clone(),
    );
    Harness {
        service,
        people,
        clock,
    }
}

fn input(name: &str, emails: &[&str]) -> ProfileInput {
    ProfileInput {
        name: name.into(),
        emails: emails.iter().map(|email| (*email).to_owned()).collect(),
        role: "CTO".into(),
        ..ProfileInput::default()
    }
}

fn attendee(name: &str, email: &str) -> Person {
    let created_at = at("2026-10-01T09:00:00Z");
    Person {
        id: PersonId::generate(&FakeClock::new(created_at)),
        name: name.into(),
        email: Email::parse(email).ok(),
        company_id: None,
        title: Some("CTO".into()),
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at,
        updated_at: created_at,
    }
}

#[tokio::test]
async fn saving_the_profile_creates_me_with_its_name_and_first_email() {
    let Harness {
        service, people, ..
    } = harness().await;

    let saved = service
        .update(input(" Shreyas ", &["Shreyas@Deino.dev", "s@gmail.com"]))
        .await
        .unwrap();

    let me = people.me().await.unwrap().unwrap();
    assert_eq!(saved, service.get().await);
    assert_eq!(me.name, "Shreyas");
    assert_eq!(me.email, Email::parse("shreyas@deino.dev").ok());
}

#[tokio::test]
async fn an_attendee_with_my_address_becomes_me_instead_of_a_duplicate() {
    let Harness {
        service, people, ..
    } = harness().await;
    let seeded = attendee("Shreyas R.", "shreyas@deino.dev");
    people.create(&seeded).await.unwrap();

    service.sync_me().await.unwrap();
    service
        .update(input("", &["shreyas@deino.dev"]))
        .await
        .unwrap();

    let everyone = people.list().await.unwrap();
    let me = people.me().await.unwrap().unwrap();
    assert_eq!(me.id, seeded.id);
    assert_eq!(
        me.name, "Shreyas R.",
        "a blank profile name keeps the person's name"
    );
    assert_eq!(
        everyone.len(),
        2,
        "the placeholder Me stays a plain person for ORG-04 to merge"
    );
}

#[tokio::test]
async fn a_fresh_install_gets_a_me_person_and_resyncing_writes_nothing() {
    let Harness {
        service,
        people,
        clock,
    } = harness().await;

    let first = service.sync_me().await.unwrap();
    clock.advance(TimeDelta::hours(1));
    let second = service.sync_me().await.unwrap();

    let me = people.me().await.unwrap().unwrap();
    assert_eq!(first, second);
    assert_eq!(me.name, DEFAULT_ME_NAME);
    assert_eq!(me.updated_at, at("2026-10-09T09:00:00Z"));
}

#[tokio::test]
async fn a_rejected_profile_changes_neither_settings_nor_people() {
    let Harness {
        service, people, ..
    } = harness().await;

    let error = service
        .update(input("Shreyas", &["nope"]))
        .await
        .unwrap_err();

    assert!(matches!(error, ProfileError::InvalidEmail(1)));
    assert_eq!(
        service.get().await,
        crate::settings::ProfileSettings::default()
    );
    assert_eq!(people.me().await.unwrap(), None);
}

#[tokio::test]
async fn the_system_context_follows_the_saved_role() {
    let Harness { service, .. } = harness().await;
    assert_eq!(service.context().await, None);

    service.update(input("Shreyas", &[])).await.unwrap();

    let context = service.context().await.unwrap();
    assert!(context.contains("Write for their role (CTO)"));
}
