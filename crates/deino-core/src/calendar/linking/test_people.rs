//! People rows for the linking tests.

use deino_domain::{Email, PersonId};
use deino_testkit::at;

use crate::people::{Person, PersonRepo, SqlitePeopleRepo};

/// The person with ID `n`.
pub(super) fn person_id(n: u128) -> PersonId {
    uuid::Uuid::from_u128(n).to_string().parse().unwrap()
}

/// Stores a person `n` named `name`, with `email` if any.
pub(super) async fn add_person(repo: &SqlitePeopleRepo, n: u128, name: &str, email: Option<&str>) {
    let created = at("2026-10-01T09:00:00Z");
    let person = Person {
        id: person_id(n),
        name: name.into(),
        email: email.map(|email| Email::parse(email).unwrap()),
        company_id: None,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at: created,
        updated_at: created,
    };
    repo.create(&person).await.unwrap();
}
