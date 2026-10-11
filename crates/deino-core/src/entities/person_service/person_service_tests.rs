//! The People rules over the real schema: forms, pages and merges.

use deino_domain::{AttendeeRole, ErrorClass, ErrorCode, PersonId};

use super::*;
use crate::entities::testing::{NOW, World};

fn fields(name: &str, email: Option<&str>) -> PersonFields {
    PersonFields {
        name: name.into(),
        email: email.map(str::to_owned),
        title: Some("  ".into()),
        company_id: None,
    }
}

#[tokio::test]
async fn adding_a_person_checks_the_form_and_files_them_under_their_domain() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());

    let owen = service
        .create(&fields(" Owen Price ", Some("Owen@Fernhollow.co")))
        .await
        .unwrap();
    let clash = service
        .create(&fields("Owen P", Some("owen@fernhollow.co")))
        .await
        .unwrap_err();
    let blank = service.create(&fields(" ", None)).await.unwrap_err();

    assert_eq!(owen.name, "Owen Price");
    assert_eq!(owen.title, None, "a blank title is none");
    let company = world.stored_company(&owen.company_id.unwrap()).await;
    assert_eq!(company.name, "Fernhollow");
    assert_eq!(clash.code(), ErrorCode::Conflict);
    assert_eq!(blank.code(), ErrorCode::Invalid);
}

#[tokio::test]
async fn editing_saves_the_form_and_notes_are_saved_on_their_own() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());
    let acme = world.company("Acme", Some("acme.com")).await;
    let ana = world.person("Ana", None).await;
    let mut input = fields("Ana Ruiz", Some("ana@acme.com"));
    input.title = Some("CFO".into());
    input.company_id = Some(acme.id);

    let saved = service.update(&ana.id, &input).await.unwrap();
    service.set_notes(&ana.id, "Prefers email.").await.unwrap();

    let stored = world.stored_person(&ana.id).await;
    assert_eq!(
        stored,
        Person {
            notes_md: "Prefers email.".into(),
            ..saved
        }
    );
    let missing = service
        .set_notes(&PersonId::generate(world.clock.as_ref()), "x")
        .await;
    assert_eq!(missing.unwrap_err().code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn a_mention_of_someone_new_adds_them_once() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());

    let first = service.from_mention("Ruth Abernathy").await.unwrap();
    let again = service.from_mention("ruth abernathy").await.unwrap();

    assert_eq!(first.id, again.id);
}

#[tokio::test]
async fn the_page_shows_company_meetings_open_items_and_upcoming_events() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());
    let deino = world.company("Deino", Some("deino.dev")).await;
    let mut maya = world.person("Maya Chen", Some("maya@deino.dev")).await;
    world.employ(&mut maya, &deino).await;
    let one = world
        .meeting("one", "1:1 Maya", "2026-10-07T10:00:00Z")
        .await;
    world.attend(&one, &maya.id, AttendeeRole::Invitee).await;
    world.run(
        "INSERT INTO action_items (id, title, assignee_person_id, status, created_by, created_at, updated_at)
         VALUES ('01929999-0000-7000-8000-000000000001', 'Draft the queue design', ?1, 'open', 'ai', ?2, ?2)",
        &[&maya.id.to_string(), NOW],
    ).await;
    upcoming_event(&world, "maya@deino.dev").await;

    let page = service.page(&maya.id).await.unwrap();

    assert_eq!(
        page.company.map(|company| company.name).as_deref(),
        Some("Deino")
    );
    assert_eq!(page.meetings.len(), 1);
    assert_eq!(page.action_items[0].title, "Draft the queue design");
    assert_eq!(page.upcoming[0].title, "Queue review");
}

/// A synced event tomorrow inviting `email`.
async fn upcoming_event(world: &World, email: &str) {
    world
        .run(
            "INSERT INTO calendar_accounts (id, provider, name, created_at)
         VALUES ('01929999-0000-7000-8000-0000000000a1', 'apple', 'On My Mac', ?1)",
            &[NOW],
        )
        .await;
    let attendees =
        format!(r#"[{{"email":"{email}","name":null,"rsvp":null,"isOrganizer":false}}]"#);
    world.run(
        "INSERT INTO calendar_events (id, account_id, provider_event_id, title, start_at, end_at,
            attendees_json, created_at, updated_at)
         VALUES ('01929999-0000-7000-8000-0000000000e1', '01929999-0000-7000-8000-0000000000a1',
            'evt', 'Queue review', '2026-10-12T15:00:00Z', '2026-10-12T15:30:00Z', ?1, ?2, ?2)",
        &[&attendees, NOW],
    ).await;
}

#[tokio::test]
async fn merging_needs_two_listed_people() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());
    let ana = world.person("Ana", None).await;

    let same = service.merge(&ana.id, &ana.id).await.unwrap_err();
    let missing = service
        .merge(&ana.id, &PersonId::generate(world.clock.as_ref()))
        .await
        .unwrap_err();

    assert!(matches!(same, EntityError::SameEntity));
    assert_eq!(missing.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn the_page_of_a_merged_person_is_their_survivors() {
    let world = World::new().await;
    let service = PersonService::new(world.ports());
    let kept = world.person("Tom Becker", None).await;
    let gone = world.person("Tom", None).await;
    service.merge(&kept.id, &gone.id).await.unwrap();

    let page = service.page(&gone.id).await.unwrap();

    assert_eq!(page.person.id, kept.id);
}
