//! The person port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{CompanyId, Email, PersonId};

use super::Person;

/// Where people are stored.
#[async_trait]
pub trait PersonRepo: Send + Sync {
    /// Adds a person. Fails if another person has their email, or if both
    /// claim to be the user.
    async fn create(&self, person: &Person) -> Result<(), DbError>;

    /// The person with `id`, merged or not.
    async fn find(&self, id: &PersonId) -> Result<Option<Person>, DbError>;

    /// The person with `email`, for upserting calendar attendees.
    async fn find_by_email(&self, email: &Email) -> Result<Option<Person>, DbError>;

    /// The oldest unmerged person named exactly `name` (ignoring ASCII
    /// case), for upserting speakers and mentions that have no address.
    async fn find_by_name(&self, name: &str) -> Result<Option<Person>, DbError>;

    /// Every person not merged into another, by name (case-insensitive).
    async fn list(&self) -> Result<Vec<Person>, DbError>;

    /// The company's people not merged into another, by name.
    async fn list_by_company(&self, company: &CompanyId) -> Result<Vec<Person>, DbError>;

    /// Unmerged people whose name, any word of it, or email starts with
    /// `prefix` (ignoring ASCII case), by name, at most `limit` of them. An
    /// empty prefix matches everyone.
    async fn search(&self, prefix: &str, limit: u32) -> Result<Vec<Person>, DbError>;

    /// Saves every editable field of `person` and its `updated_at`.
    async fn update(&self, person: &Person) -> Result<bool, DbError>;

    /// Deletes the person; their meetings stay, without them as attendee.
    async fn delete(&self, id: &PersonId) -> Result<bool, DbError>;
}

/// [`PersonRepo`] and [`AttendeeRepo`](super::AttendeeRepo) over Deino's
/// SQLite database.
#[derive(Clone, Debug)]
pub struct SqlitePeopleRepo {
    pub(super) database: Database,
}

impl SqlitePeopleRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl PersonRepo for SqlitePeopleRepo {
    async fn create(&self, person: &Person) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO people (id, name, email, company_id, title, avatar_path, notes_md, summary_md,
                summary_updated_at, is_me, merged_into, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            person.id,
            person.name,
            person.email,
            person.company_id,
            person.title,
            person.avatar_path,
            person.notes_md,
            person.summary_md,
            person.summary_updated_at,
            person.is_me,
            person.merged_into,
            person.created_at,
            person.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn find(&self, id: &PersonId) -> Result<Option<Person>, DbError> {
        let person = sqlx::query_file_as!(Person, "src/people/queries/find.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(person)
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Person>, DbError> {
        let person = sqlx::query_file_as!(Person, "src/people/queries/find_by_email.sql", email)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(person)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Person>, DbError> {
        let person = sqlx::query_file_as!(Person, "src/people/queries/find_by_name.sql", name)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(person)
    }

    async fn list(&self) -> Result<Vec<Person>, DbError> {
        let people = sqlx::query_file_as!(Person, "src/people/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(people)
    }

    async fn list_by_company(&self, company: &CompanyId) -> Result<Vec<Person>, DbError> {
        let people =
            sqlx::query_file_as!(Person, "src/people/queries/list_by_company.sql", company)
                .fetch_all(self.database.reader())
                .await?;
        Ok(people)
    }

    async fn search(&self, prefix: &str, limit: u32) -> Result<Vec<Person>, DbError> {
        let pattern = escape_like(prefix);
        let people = sqlx::query_file_as!(Person, "src/people/queries/search.sql", pattern, limit)
            .fetch_all(self.database.reader())
            .await?;
        Ok(people)
    }

    async fn update(&self, person: &Person) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE people
             SET name = ?1, email = ?2, company_id = ?3, title = ?4, avatar_path = ?5, notes_md = ?6,
                 summary_md = ?7, summary_updated_at = ?8, is_me = ?9, merged_into = ?10, updated_at = ?11
             WHERE id = ?12",
            person.name,
            person.email,
            person.company_id,
            person.title,
            person.avatar_path,
            person.notes_md,
            person.summary_md,
            person.summary_updated_at,
            person.is_me,
            person.merged_into,
            person.updated_at,
            person.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &PersonId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM people WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// `text` as a LIKE prefix: `\`, `%` and `_` match themselves (`ESCAPE '\'`).
fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
