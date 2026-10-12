//! The upsert rules, over the in-memory people and company repositories.

use std::sync::Arc;

use deino_domain::{Email, FakeClock};
use deino_testkit::at;

use super::*;
use crate::companies::FakeCompanyRepo;
use crate::people::FakePeopleRepo;

struct Harness {
    directory: EntityDirectory,
    people: Arc<FakePeopleRepo>,
    companies: Arc<FakeCompanyRepo>,
}

fn harness() -> Harness {
    let people = Arc::new(FakePeopleRepo::default());
    let companies = Arc::new(FakeCompanyRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-11T12:00:00Z")));
    Harness {
        directory: EntityDirectory::new(people.clone(), companies.clone(), clock),
        people,
        companies,
    }
}

fn seed(email: Option<&str>, name: Option<&str>) -> PersonSeed {
    PersonSeed {
        email: email.map(|email| Email::parse(email).unwrap()),
        name: name.map(str::to_owned),
    }
}

#[tokio::test]
async fn a_new_address_adds_the_person_at_the_company_of_its_domain() {
    let Harness {
        directory,
        people,
        companies,
    } = harness();

    let added = directory
        .upsert_person(&seed(Some("hannah.pike@brightkite-logistics.com"), None))
        .await
        .unwrap()
        .unwrap();

    assert!(added.created);
    let hannah = people.find(&added.id).await.unwrap().unwrap();
    assert_eq!(hannah.name, "Hannah Pike");
    let company = companies
        .find(&hannah.company_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(company.name, "Brightkite Logistics");
}

#[tokio::test]
async fn a_known_address_finds_the_person_it_was_merged_into() {
    let Harness {
        directory, people, ..
    } = harness();
    let first = directory
        .upsert_person(&seed(Some("maya@deino.dev"), Some("Maya")))
        .await
        .unwrap()
        .unwrap();
    let survivor = directory
        .upsert_person(&seed(None, Some("Maya Chen")))
        .await
        .unwrap()
        .unwrap();
    let mut merged = people.find(&first.id).await.unwrap().unwrap();
    merged.merged_into = Some(survivor.id);
    people.update(&merged).await.unwrap();

    let again = directory
        .upsert_person(&seed(Some("MAYA@deino.dev"), Some("Someone Else")))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        again,
        Upserted {
            id: survivor.id,
            created: false
        }
    );
}

#[tokio::test]
async fn a_name_without_an_address_reuses_a_namesake() {
    let Harness {
        directory, people, ..
    } = harness();

    let first = directory
        .upsert_person(&seed(None, Some(" Owen Price ")))
        .await
        .unwrap();
    let second = directory
        .upsert_person(&seed(None, Some("owen price")))
        .await
        .unwrap();

    assert_eq!(first.map(|u| u.id), second.map(|u| u.id));
    assert!(first.unwrap().created && !second.unwrap().created);
    assert_eq!(people.list().await.unwrap().len(), 1);
    assert_eq!(
        directory
            .upsert_person(&seed(None, Some("  ")))
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn personal_providers_and_vanity_domains_make_no_company() {
    let Harness {
        directory,
        people,
        companies,
    } = harness();

    for (email, name) in [
        ("sam.lee@gmail.com", "Sam Lee"),
        ("jordan@jordanellis.dev", "Jordan Ellis"),
    ] {
        let added = directory
            .upsert_person(&seed(Some(email), Some(name)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            people.find(&added.id).await.unwrap().unwrap().company_id,
            None,
            "{email}"
        );
    }

    assert!(companies.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_domain_of_a_merged_company_resolves_to_its_survivor() {
    let Harness {
        directory,
        companies,
        ..
    } = harness();
    let absorbed = directory
        .upsert_company(&CompanyDomain::parse("acme.io").unwrap())
        .await
        .unwrap();
    let survivor = directory
        .upsert_company(&CompanyDomain::parse("acme.com").unwrap())
        .await
        .unwrap();
    let mut merged = companies.find(&absorbed).await.unwrap().unwrap();
    merged.merged_into = Some(survivor);
    companies.update(&merged).await.unwrap();

    let person = directory
        .upsert_person(&seed(Some("ana@acme.io"), Some("Ana")))
        .await
        .unwrap()
        .unwrap();

    let stored = directory.people.find(&person.id).await.unwrap().unwrap();
    assert_eq!(stored.company_id, Some(survivor));
    assert_eq!(companies.list().await.unwrap().len(), 1);
}
