//! The company port's contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{CompanyId, FakeClock};
use deino_testkit::at;

use super::*;
use crate::testing::query_plan;

fn company(name: &str, domain: Option<&str>) -> Company {
    let created_at = at("2026-10-08T09:00:00Z");
    Company {
        id: CompanyId::generate(&FakeClock::new(created_at)),
        name: name.into(),
        domain: domain.and_then(CompanyDomain::parse),
        logo_path: None,
        notes_md: String::new(),
        summary_md: None,
        created_at,
        updated_at: created_at,
    }
}

async fn sqlite_repo() -> SqliteCompanyRepo {
    SqliteCompanyRepo::new(test_db().await)
}

fn names(companies: &[Company]) -> Vec<&str> {
    companies
        .iter()
        .map(|company| company.name.as_str())
        .collect()
}

/// Companies list by name ignoring case and are found by domain.
async fn companies_list_by_name_and_are_found_by_domain(repo: &impl CompanyRepo) {
    let acme = company("acme", Some("acme.com"));
    repo.create(&company("Globex", None)).await.unwrap();
    repo.create(&acme).await.unwrap();

    let domain = CompanyDomain::parse("acme.com").unwrap();

    assert_eq!(names(&repo.list().await.unwrap()), ["acme", "Globex"]);
    assert_eq!(repo.find_by_domain(&domain).await.unwrap(), Some(acme));
}

/// Updates save every editable field; deletes report what they removed.
async fn companies_update_and_delete(repo: &impl CompanyRepo) {
    let mut acme = company("Acme", Some("acme.com"));
    repo.create(&acme).await.unwrap();

    acme.notes_md = "Our biggest customer".into();
    acme.updated_at = at("2026-10-09T09:00:00Z");
    assert!(repo.update(&acme).await.unwrap());
    assert_eq!(repo.find(&acme.id).await.unwrap(), Some(acme.clone()));

    assert!(repo.delete(&acme.id).await.unwrap());
    assert!(!repo.delete(&acme.id).await.unwrap());
    assert_eq!(repo.find(&acme.id).await.unwrap(), None);
}

#[tokio::test]
async fn sqlite_companies_list_by_name_and_are_found_by_domain() {
    companies_list_by_name_and_are_found_by_domain(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_companies_list_by_name_and_are_found_by_domain() {
    companies_list_by_name_and_are_found_by_domain(&FakeCompanyRepo::default()).await;
}

#[tokio::test]
async fn sqlite_companies_update_and_delete() {
    companies_update_and_delete(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_companies_update_and_delete() {
    companies_update_and_delete(&FakeCompanyRepo::default()).await;
}

#[tokio::test]
async fn domains_are_unique() {
    let repo = sqlite_repo().await;
    repo.create(&company("Acme", Some("acme.com")))
        .await
        .unwrap();

    assert!(
        repo.create(&company("Acme Inc", Some("ACME.com")))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn the_schema_rejects_a_blank_name() {
    let repo = sqlite_repo().await;

    assert!(repo.create(&company("  ", None)).await.is_err());
}

#[tokio::test]
async fn the_list_is_read_in_index_order() {
    let repo = sqlite_repo().await;

    let plan = query_plan(&repo.database, include_str!("queries/list.sql")).await;

    assert_eq!(plan, ["SCAN companies USING INDEX idx_companies_name"]);
}
