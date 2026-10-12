//! The Companies rules over the real schema: forms, pages and merges.

use deino_domain::{AttendeeRole, ErrorClass, ErrorCode};

use super::*;
use crate::companies::CompanyDomain;
use crate::entities::PersonSeed;
use crate::entities::testing::{NOW, World};

fn fields(name: &str, domain: Option<&str>) -> CompanyFields {
    CompanyFields {
        name: name.into(),
        domain: domain.map(str::to_owned),
    }
}

#[tokio::test]
async fn adding_a_company_reads_its_domain_and_refuses_a_taken_one() {
    let world = World::new().await;
    let service = CompanyService::new(world.ports());

    let acme = service
        .create(&fields("Acme", Some("https://www.acme.com/")))
        .await
        .unwrap();
    let clash = service
        .create(&fields("Acme Inc", Some("ACME.com")))
        .await
        .unwrap_err();
    let bad = service
        .create(&fields("Globex", Some("globex")))
        .await
        .unwrap_err();

    assert_eq!(acme.domain, CompanyDomain::parse("acme.com"));
    assert_eq!(clash.code(), ErrorCode::Conflict);
    assert_eq!(bad.code(), ErrorCode::Invalid);
}

#[tokio::test]
async fn a_new_domain_asks_its_site_for_a_logo_again() {
    let world = World::new().await;
    let service = CompanyService::new(world.ports());
    let acme = world.company("Acme", Some("acme.com")).await;
    world
        .run("UPDATE companies SET logo_checked_at = ?1", &[NOW])
        .await;

    let renamed = service
        .update(&acme.id, &fields("Acme", Some("acme.com")))
        .await
        .unwrap();
    let moved = service
        .update(&acme.id, &fields("Acme", Some("acme.io")))
        .await
        .unwrap();

    assert!(renamed.logo_checked_at.is_some());
    assert_eq!(moved.logo_checked_at, None);
    service.set_notes(&acme.id, "Renewal in Q1").await.unwrap();
    assert_eq!(
        world.stored_company(&acme.id).await.notes_md,
        "Renewal in Q1"
    );
}

#[tokio::test]
async fn the_page_lists_people_meetings_items_and_events_by_domain() {
    let world = World::new().await;
    let service = CompanyService::new(world.ports());
    let fernhollow = world.company("Fernhollow", Some("fernhollow.co")).await;
    let mut owen = world.person("Owen Price", Some("owen@fernhollow.co")).await;
    world.employ(&mut owen, &fernhollow).await;
    let call = world
        .meeting("call", "Escalation", "2026-10-05T10:00:00Z")
        .await;
    world.attend(&call, &owen.id, AttendeeRole::Invitee).await;
    world.run(
        "INSERT INTO action_items (id, title, assignee_person_id, status, created_by, created_at, updated_at)
         VALUES ('01929999-0000-7000-8000-0000000000c2', 'Send the fix ETA', ?1, 'open', 'user', ?2, ?2)",
        &[&owen.id.to_string(), NOW],
    ).await;

    let page = service.page(&fernhollow.id).await.unwrap();

    assert_eq!(page.people[0].name, "Owen Price");
    assert_eq!(page.meetings[0].title, "Escalation");
    assert_eq!(page.action_items[0].title, "Send the fix ETA");
    assert!(page.upcoming.is_empty());
}

#[tokio::test]
async fn merging_moves_people_and_keeps_the_old_domain_resolving() {
    let world = World::new().await;
    let ports = world.ports();
    let service = CompanyService::new(ports.clone());
    let kept = world.company("Acme", Some("acme.com")).await;
    let gone = world.company("Acme Europe", Some("acme.eu")).await;
    let mut ana = world.person("Ana", Some("ana@acme.eu")).await;
    world.employ(&mut ana, &gone).await;

    let merged = service.merge(&kept.id, &gone.id).await.unwrap();

    assert_eq!(merged.id, kept.id);
    assert_eq!(world.stored_person(&ana.id).await.company_id, Some(kept.id));
    assert_eq!(
        world.stored_company(&gone.id).await.merged_into,
        Some(kept.id)
    );
    let rows = service.list().await.unwrap();
    assert_eq!(rows.len(), 1);
    let seed = PersonSeed {
        email: Some(deino_domain::Email::parse("ben@acme.eu").unwrap()),
        name: None,
    };
    let ben = ports.directory.upsert_person(&seed).await.unwrap().unwrap();
    assert_eq!(world.stored_person(&ben.id).await.company_id, Some(kept.id));
    let page = service.page(&gone.id).await.unwrap();
    assert_eq!(page.company.id, kept.id);
}
