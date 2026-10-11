//! The meetings persistence port and its SQLite implementation.
//!
//! Each port method is one unit of work: the SQLite implementation runs
//! multi-statement writes in a single transaction on the writer connection,
//! and reads on the read pool. Queries are compile-time checked against the
//! migrated schema (`sqlx::query*!` with the committed `.sqlx/` data).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::MeetingId;

use super::{Meeting, MeetingTitle};

/// Where meetings are stored.
#[async_trait]
pub trait MeetingRepo: Send + Sync {
    /// Every meeting outside the trash, newest first.
    async fn list(&self) -> Result<Vec<Meeting>, DbError>;

    /// The meeting with `id`, if it exists.
    async fn find(&self, id: &MeetingId) -> Result<Option<Meeting>, DbError>;

    /// Sets the meeting's title and stamps `updated_at` with `at`.
    ///
    /// Returns the renamed meeting, or `None` when no meeting has `id`.
    async fn rename(
        &self,
        id: &MeetingId,
        title: &MeetingTitle,
        at: DateTime<Utc>,
    ) -> Result<Option<Meeting>, DbError>;
}

/// [`MeetingRepo`], [`MeetingLifecycleRepo`](super::MeetingLifecycleRepo) and
/// [`MeetingTrashRepo`](super::MeetingTrashRepo) over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteMeetingRepo {
    pub(super) database: Database,
}

impl SqliteMeetingRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl MeetingRepo for SqliteMeetingRepo {
    async fn list(&self) -> Result<Vec<Meeting>, DbError> {
        // A query file, so the EXPLAIN QUERY PLAN test checks this exact text.
        let meetings = sqlx::query_file_as!(Meeting, "src/meetings/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn find(&self, id: &MeetingId) -> Result<Option<Meeting>, DbError> {
        let meeting = sqlx::query_as!(
            Meeting,
            r#"SELECT
                id AS "id!: MeetingId",
                title,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM meetings
            WHERE id = ?1"#,
            id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(meeting)
    }

    async fn rename(
        &self,
        id: &MeetingId,
        title: &MeetingTitle,
        at: DateTime<Utc>,
    ) -> Result<Option<Meeting>, DbError> {
        let title = title.as_str();
        let renamed = sqlx::query_as!(
            Meeting,
            r#"UPDATE meetings SET title = ?1, updated_at = ?2 WHERE id = ?3
            RETURNING
                id AS "id!: MeetingId",
                title,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>""#,
            title,
            at,
            id,
        )
        .fetch_optional(self.database.writer())
        .await?;
        Ok(renamed)
    }
}

#[cfg(test)]
mod tests {
    use deino_db::test_db;
    use deino_testkit::{at, seed};

    use super::*;
    use crate::testing::query_plan;

    async fn repo_with(meetings: &[(&str, &str, &str)]) -> SqliteMeetingRepo {
        let database = test_db().await;
        for (id, title, created_at) in meetings {
            seed::meeting(&database, id, title, at(created_at)).await;
        }
        SqliteMeetingRepo::new(database)
    }

    fn ids(meetings: &[Meeting]) -> Vec<&str> {
        meetings.iter().map(|meeting| meeting.id.as_str()).collect()
    }

    #[tokio::test]
    async fn list_returns_newest_first_with_ties_broken_by_id() {
        let repo = repo_with(&[
            ("meeting-a", "Old", "2026-10-01T09:00:00Z"),
            ("meeting-b", "Tie", "2026-10-08T09:00:00Z"),
            ("meeting-c", "Tie", "2026-10-08T09:00:00Z"),
        ])
        .await;

        let meetings = repo.list().await.unwrap();

        assert_eq!(ids(&meetings), ["meeting-c", "meeting-b", "meeting-a"]);
    }

    #[tokio::test]
    async fn list_is_an_index_scan_without_a_sort() {
        let repo = repo_with(&[]).await;

        let plan = query_plan(&repo.database, include_str!("queries/list.sql")).await;

        assert_eq!(
            plan,
            ["SCAN meetings USING INDEX idx_meetings_live_created_at_id"]
        );
    }

    #[tokio::test]
    async fn list_hides_meetings_in_the_trash() {
        let repo = repo_with(&[
            ("meeting-a", "Kept", "2026-10-01T09:00:00Z"),
            ("meeting-b", "Trashed", "2026-10-02T09:00:00Z"),
        ])
        .await;
        sqlx::query(
            "UPDATE meetings SET deleted_at = '2026-10-03T09:00:00Z' WHERE id = 'meeting-b'",
        )
        .execute(repo.database.writer())
        .await
        .unwrap();

        let meetings = repo.list().await.unwrap();

        assert_eq!(ids(&meetings), ["meeting-a"]);
    }

    #[tokio::test]
    async fn find_returns_the_meeting_or_none() {
        let repo = repo_with(&[("meeting-a", "Standup", "2026-10-08T09:00:00Z")]).await;

        let found = repo
            .find(&MeetingId::new("meeting-a"))
            .await
            .unwrap()
            .unwrap();
        let missing = repo.find(&MeetingId::new("meeting-z")).await.unwrap();

        assert_eq!(found.title, "Standup");
        assert_eq!(found.created_at, at("2026-10-08T09:00:00Z"));
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn rename_updates_the_title_and_timestamp() {
        let repo = repo_with(&[("meeting-a", "Standup", "2026-10-08T09:00:00Z")]).await;
        let (id, title) = (
            MeetingId::new("meeting-a"),
            MeetingTitle::parse("Planning").unwrap(),
        );

        let renamed = repo
            .rename(&id, &title, at("2026-10-09T10:00:00Z"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(renamed.title, "Planning");
        assert_eq!(renamed.updated_at, at("2026-10-09T10:00:00Z"));
        assert_eq!(renamed.created_at, at("2026-10-08T09:00:00Z"));
    }

    #[tokio::test]
    async fn rename_of_a_missing_meeting_changes_nothing() {
        let repo = repo_with(&[]).await;
        let (id, title) = (
            MeetingId::new("meeting-z"),
            MeetingTitle::parse("Planning").unwrap(),
        );

        let renamed = repo
            .rename(&id, &title, at("2026-10-09T10:00:00Z"))
            .await
            .unwrap();

        assert_eq!(renamed, None);
    }
}
