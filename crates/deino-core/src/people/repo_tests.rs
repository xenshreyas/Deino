//! The person port's contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{CompanyId, FakeClock, PersonId};
use deino_testkit::at;

use super::*;
use crate::companies::{Company, CompanyRepo, SqliteCompanyRepo};
use crate::testing::query_plan;

const CREATED_AT: &str = "2026-10-08T09:00:00Z";

fn person(name: &str, email: Option<&str>) -> Person {
    let created_at = at(CREATED_AT);
    Person {
        id: PersonId::generate(&FakeClock::new(created_at)),
        name: name.into(),
        email: email.and_then(EmailAddress::parse),
        company_id: None,
        title: None,
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

/// A SQLite repository and a company that exists in its database.
async fn sqlite_repo() -> (SqlitePeopleRepo, CompanyId) {
    let database = test_db().await;
    let created_at = at(CREATED_AT);
    let company = Company {
        id: CompanyId::generate(&FakeClock::new(created_at)),
        name: "Acme".into(),
        domain: None,
        logo_path: None,
        notes_md: String::new(),
        summary_md: None,
        created_at,
        updated_at: created_at,
    };
    SqliteCompanyRepo::new(database.clone())
        .create(&company)
        .await
        .unwrap();
    (SqlitePeopleRepo::new(database), company.id)
}

fn names(people: &[Person]) -> Vec<&str> {
    people.iter().map(|person| person.name.as_str()).collect()
}

/// Lists hide merged people and sort by name; lookups still find them.
async fn people_list_by_name_without_merged_ones(repo: &impl PersonRepo, company: CompanyId) {
    let ada = person("ada", Some("Ada@Example.com"));
    let mut grace = person("Grace", None);
    let mut merged = person("Ada L.", None);
    merged.merged_into = Some(ada.id);
    grace.company_id = Some(company);
    for someone in [&grace, &ada, &merged] {
        repo.create(someone).await.unwrap();
    }

    let email = EmailAddress::parse("ada@example.com").unwrap();

    assert_eq!(names(&repo.list().await.unwrap()), ["ada", "Grace"]);
    assert_eq!(
        names(&repo.list_by_company(&company).await.unwrap()),
        ["Grace"]
    );
    assert_eq!(repo.find_by_email(&email).await.unwrap(), Some(ada));
    assert_eq!(repo.find(&merged.id).await.unwrap(), Some(merged));
}

/// Updates save every editable field; deletes report what they removed.
async fn people_update_and_delete(repo: &impl PersonRepo) {
    let mut ada = person("Ada", None);
    repo.create(&ada).await.unwrap();

    ada.title = Some("Analyst".into());
    ada.updated_at = at("2026-10-09T09:00:00Z");
    assert!(repo.update(&ada).await.unwrap());
    assert_eq!(repo.find(&ada.id).await.unwrap(), Some(ada.clone()));

    assert!(repo.delete(&ada.id).await.unwrap());
    assert!(!repo.delete(&ada.id).await.unwrap());
}

#[tokio::test]
async fn sqlite_people_list_by_name_without_merged_ones() {
    let (repo, company) = sqlite_repo().await;
    people_list_by_name_without_merged_ones(&repo, company).await;
}

#[tokio::test]
async fn fake_people_list_by_name_without_merged_ones() {
    let company = CompanyId::generate(&FakeClock::new(at(CREATED_AT)));
    people_list_by_name_without_merged_ones(&FakePeopleRepo::default(), company).await;
}

#[tokio::test]
async fn sqlite_people_update_and_delete() {
    people_update_and_delete(&sqlite_repo().await.0).await;
}

#[tokio::test]
async fn fake_people_update_and_delete() {
    people_update_and_delete(&FakePeopleRepo::default()).await;
}

#[tokio::test]
async fn emails_are_unique_and_only_one_person_is_me() {
    let (repo, _) = sqlite_repo().await;
    let mut me = person("Me", Some("me@example.com"));
    me.is_me = true;
    repo.create(&me).await.unwrap();

    assert!(
        repo.create(&person("Also me", Some("ME@example.com")))
            .await
            .is_err()
    );
    let mut impostor = person("Impostor", None);
    impostor.is_me = true;
    assert!(repo.create(&impostor).await.is_err());
}

#[tokio::test]
async fn deleting_a_company_keeps_its_people() {
    let (repo, company) = sqlite_repo().await;
    let mut grace = person("Grace", None);
    grace.company_id = Some(company);
    repo.create(&grace).await.unwrap();

    SqliteCompanyRepo::new(repo.database.clone())
        .delete(&company)
        .await
        .unwrap();

    let stored = repo.find(&grace.id).await.unwrap().unwrap();
    assert_eq!(stored.company_id, None);
}

#[tokio::test]
async fn a_person_cannot_be_merged_into_themselves() {
    let (repo, _) = sqlite_repo().await;
    let mut ada = person("Ada", None);
    repo.create(&ada).await.unwrap();

    ada.merged_into = Some(ada.id);

    assert!(repo.update(&ada).await.is_err());
}

#[tokio::test]
async fn lists_are_read_through_their_indexes() {
    let (repo, _) = sqlite_repo().await;

    let list = query_plan(&repo.database, include_str!("queries/list.sql")).await;
    let by_company = query_plan(&repo.database, include_str!("queries/list_by_company.sql")).await;

    assert_eq!(list, ["SCAN people USING INDEX idx_people_listed_name"]);
    assert_eq!(
        by_company,
        [
            "SEARCH people USING INDEX idx_people_company_id (company_id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
}
