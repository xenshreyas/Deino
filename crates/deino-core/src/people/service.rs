//! The people rules the app exposes so far: finding someone to mention.
//!
//! ORG-04 grows this service with upserts from calendars and speakers,
//! merging and summaries.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use super::{Person, PersonRepo};

/// How many people a search returns: a suggestion list, not a directory.
pub const SEARCH_LIMIT: u32 = 8;

/// The longest search text honoured; longer input is cut, not refused.
pub const MAX_SEARCH_CHARS: usize = 100;

/// A people failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PeopleError {
    /// The database failed; the source carries the detail.
    #[error("could not load people")]
    Storage(#[from] DbError),
}

impl ErrorClass for PeopleError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

/// Reads people for the UI.
pub struct PeopleService {
    repo: Arc<dyn PersonRepo>,
}

impl PeopleService {
    /// Creates the service over its port.
    pub fn new(repo: Arc<dyn PersonRepo>) -> Self {
        Self { repo }
    }

    /// Up to [`SEARCH_LIMIT`] people whose name, a word of it, or email
    /// starts with `query` (trimmed, at most [`MAX_SEARCH_CHARS`]); with an
    /// empty query, the first people by name (`@` alone in the editor).
    pub async fn search(&self, query: &str) -> Result<Vec<Person>, PeopleError> {
        let query: String = query.trim().chars().take(MAX_SEARCH_CHARS).collect();
        Ok(self.repo.search(&query, SEARCH_LIMIT).await?)
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, PersonId};
    use deino_testkit::at;

    use super::*;
    use crate::people::FakePeopleRepo;

    fn person(name: &str) -> Person {
        let created_at = at("2026-10-08T09:00:00Z");
        Person {
            id: PersonId::generate(&FakeClock::new(created_at)),
            name: name.into(),
            email: None,
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

    async fn service_with(names: &[&str]) -> PeopleService {
        let repo = FakePeopleRepo::default();
        for name in names {
            repo.create(&person(name)).await.unwrap();
        }
        PeopleService::new(Arc::new(repo))
    }

    #[tokio::test]
    async fn search_trims_the_query() {
        let service = service_with(&["Priya Shah", "Maya"]).await;

        let found = service.search("  pri ").await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Priya Shah");
    }

    #[tokio::test]
    async fn search_returns_at_most_the_limit() {
        let names: Vec<String> = (0..20).map(|n| format!("Person {n:02}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let service = service_with(&refs).await;

        let found = service.search("").await.unwrap();

        assert_eq!(found.len(), SEARCH_LIMIT as usize);
        assert_eq!(found[0].name, "Person 00");
    }
}
