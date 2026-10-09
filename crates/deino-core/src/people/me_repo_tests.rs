//! The "Me" port's contract, run against the SQLite repository and the fake.

use deino_db::test_db;
use deino_domain::{Email, FakeClock, PersonId};
use deino_testkit::at;

use super::*;
use crate::testing::query_plan;

const CREATED_AT: &str = "2026-10-08T09:00:00Z";
const MOVED_AT: &str = "2026-10-09T09:00:00Z";

fn person(name: &str, email: &str) -> Person {
    let created_at = at(CREATED_AT);
    Person {
        id: PersonId::generate(&FakeClock::new(created_at)),
        name: name.into(),
        email: Email::parse(email).ok(),
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

async fn sqlite_repo() -> SqlitePeopleRepo {
    SqlitePeopleRepo::new(test_db().await)
}

/// Marking a person moves the mark: exactly one person is the user after.
async fn making_someone_me_moves_the_mark<R: PersonRepo + MeRepo>(repo: &R) {
    let ada = person("Ada", "ada@example.com");
    let grace = person("Grace", "grace@example.com");
    repo.create(&ada).await.unwrap();
    repo.create(&grace).await.unwrap();
    assert_eq!(repo.me().await.unwrap(), None);

    assert!(repo.make_me(&ada.id, at(MOVED_AT)).await.unwrap());
    assert!(repo.make_me(&grace.id, at(MOVED_AT)).await.unwrap());

    let me = repo.me().await.unwrap().unwrap();
    assert_eq!(me.id, grace.id);
    assert_eq!(me.updated_at, at(MOVED_AT));
    assert!(!repo.find(&ada.id).await.unwrap().unwrap().is_me);
}

/// An unknown ID changes nothing, so the user is never left without a mark.
async fn an_unknown_person_cannot_become_me<R: PersonRepo + MeRepo>(repo: &R) {
    let ada = person("Ada", "ada@example.com");
    repo.create(&ada).await.unwrap();
    repo.make_me(&ada.id, at(MOVED_AT)).await.unwrap();
    let stranger = PersonId::generate(&FakeClock::new(at(MOVED_AT)));

    assert!(!repo.make_me(&stranger, at(MOVED_AT)).await.unwrap());

    assert_eq!(repo.me().await.unwrap().map(|me| me.id), Some(ada.id));
}

#[tokio::test]
async fn sqlite_making_someone_me_moves_the_mark() {
    making_someone_me_moves_the_mark(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_making_someone_me_moves_the_mark() {
    making_someone_me_moves_the_mark(&FakePeopleRepo::default()).await;
}

#[tokio::test]
async fn sqlite_an_unknown_person_cannot_become_me() {
    an_unknown_person_cannot_become_me(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_an_unknown_person_cannot_become_me() {
    an_unknown_person_cannot_become_me(&FakePeopleRepo::default()).await;
}

#[tokio::test]
async fn me_is_found_through_its_unique_index() {
    let repo = sqlite_repo().await;

    let plan = query_plan(&repo.database, include_str!("queries/me.sql")).await;

    assert_eq!(
        plan,
        ["SEARCH people USING INDEX idx_people_is_me (is_me=?)"]
    );
}
