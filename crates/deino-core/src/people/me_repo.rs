//! The "Me" port: which person is the user (NOT-05).
//!
//! At most one person has `is_me` (a unique partial index). The profile keeps
//! that person's name and email in step with Settings → Profile, so the user
//! is recognised among attendees and speakers.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CompanyId, Email, PersonId};

#[cfg(any(test, feature = "test-support"))]
use super::FakePeopleRepo;

use super::{Person, SqlitePeopleRepo};

/// Finds and moves the user's own person.
#[async_trait]
pub trait MeRepo: Send + Sync {
    /// The person marked as the user, if any.
    async fn me(&self) -> Result<Option<Person>, DbError>;

    /// Marks `id` as the user and unmarks whoever was before, in one
    /// transaction. Returns `false`, changing nothing, when no person has `id`.
    async fn make_me(&self, id: &PersonId, at: DateTime<Utc>) -> Result<bool, DbError>;
}

#[async_trait]
impl MeRepo for SqlitePeopleRepo {
    async fn me(&self) -> Result<Option<Person>, DbError> {
        let person = sqlx::query_file_as!(Person, "src/people/queries/me.sql")
            .fetch_optional(self.database.reader())
            .await?;
        Ok(person)
    }

    async fn make_me(&self, id: &PersonId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "UPDATE people SET is_me = 0, updated_at = ?2 WHERE is_me = 1 AND id <> ?1",
            id,
            at,
        )
        .execute(&mut *transaction)
        .await?;
        let marked = sqlx::query!(
            "UPDATE people SET is_me = 1, updated_at = CASE is_me WHEN 1 THEN updated_at ELSE ?2 END
             WHERE id = ?1",
            id,
            at,
        )
        .execute(&mut *transaction)
        .await?;
        if marked.rows_affected() == 0 {
            // Dropping the transaction rolls the unmarking back.
            return Ok(false);
        }
        transaction.commit().await?;
        Ok(true)
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl MeRepo for FakePeopleRepo {
    async fn me(&self) -> Result<Option<Person>, DbError> {
        Ok(self.people().into_iter().find(|person| person.is_me))
    }

    async fn make_me(&self, id: &PersonId, at: DateTime<Utc>) -> Result<bool, DbError> {
        Ok(self.update_people(|people| {
            if !people.iter().any(|person| person.id == *id) {
                return false;
            }
            for person in people.iter_mut() {
                let is_me = person.id == *id;
                if person.is_me != is_me {
                    person.is_me = is_me;
                    person.updated_at = at;
                }
            }
            true
        }))
    }
}
