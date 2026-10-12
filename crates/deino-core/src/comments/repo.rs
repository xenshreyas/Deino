//! [`CommentRepo`]: the `comment_threads` and `comments` tables. Reads never
//! return a trashed meeting's threads.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{CommentId, CommentThreadId, MeetingId, SegmentId};

use super::{Comment, CommentAnchor, CommentTarget, CommentThread, ThreadRow};

/// Where comments are stored.
#[async_trait]
pub trait CommentRepo: Send + Sync {
    /// A meeting's threads, oldest first, each with its comments.
    async fn list(&self, meeting: &MeetingId) -> Result<Vec<CommentThread>, DbError>;

    /// Saves a thread and its first comment; `false` (nothing saved) when a
    /// transcript thread's line is not a line of the meeting.
    async fn insert_thread(&self, thread: &ThreadRow) -> Result<bool, DbError>;

    /// Adds a comment to a thread of a meeting not in the trash; `false` if
    /// there is no such thread.
    async fn insert_comment(&self, comment: &Comment) -> Result<bool, DbError>;

    /// Replaces a comment's body at `at`; `false` if it is gone.
    async fn edit(&self, id: &CommentId, body: &str, at: DateTime<Utc>) -> Result<bool, DbError>;

    /// Resolves (`Some(at)`) or reopens (`None`) a thread; `false` if it is gone.
    async fn set_resolved(
        &self,
        thread: &CommentThreadId,
        at: Option<DateTime<Utc>>,
    ) -> Result<bool, DbError>;

    /// Deletes a comment, and its thread when it was the last; `false` if it
    /// was already gone.
    async fn delete(&self, comment: &CommentId) -> Result<bool, DbError>;
}

/// [`CommentRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteCommentRepo {
    database: Database,
}

impl SqliteCommentRepo {
    /// The repository over the app's database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// A `comment_threads` row.
struct ThreadRecord {
    id: CommentThreadId,
    meeting_id: MeetingId,
    target: CommentTarget,
    segment_id: Option<SegmentId>,
    start: u32,
    end: u32,
    quote: String,
    resolved_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl ThreadRecord {
    /// The thread, with its comments.
    fn into_thread(self, comments: Vec<Comment>) -> CommentThread {
        CommentThread {
            id: self.id,
            meeting_id: self.meeting_id,
            anchor: CommentAnchor {
                target: self.target,
                segment_id: self.segment_id,
                start: self.start,
                end: self.end,
                quote: self.quote,
            },
            resolved_at: self.resolved_at,
            created_at: self.created_at,
            comments,
        }
    }
}

impl SqliteCommentRepo {
    async fn threads(&self, meeting: &MeetingId) -> Result<Vec<ThreadRecord>, DbError> {
        let threads = sqlx::query_as!(
            ThreadRecord,
            r#"SELECT t.id AS "id: CommentThreadId", t.meeting_id AS "meeting_id: MeetingId",
                t.target AS "target: CommentTarget", t.segment_id AS "segment_id: SegmentId",
                t.start_offset AS "start: u32", t.end_offset AS "end: u32", t.quote,
                t.resolved_at AS "resolved_at: DateTime<Utc>",
                t.created_at AS "created_at: DateTime<Utc>"
            FROM comment_threads t JOIN meetings m ON m.id = t.meeting_id
            WHERE t.meeting_id = ?1 AND m.deleted_at IS NULL
            ORDER BY t.created_at, t.rowid"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(threads)
    }

    async fn comments(&self, meeting: &MeetingId) -> Result<Vec<Comment>, DbError> {
        let comments = sqlx::query_as!(
            Comment,
            r#"SELECT c.id AS "id: CommentId", c.thread_id AS "thread_id: CommentThreadId", c.body,
                c.created_at AS "created_at: DateTime<Utc>", c.updated_at AS "updated_at: DateTime<Utc>"
            FROM comments c JOIN comment_threads t ON t.id = c.thread_id
            WHERE t.meeting_id = ?1
            ORDER BY c.created_at, c.rowid"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(comments)
    }
}

#[async_trait]
impl CommentRepo for SqliteCommentRepo {
    async fn list(&self, meeting: &MeetingId) -> Result<Vec<CommentThread>, DbError> {
        let mut by_thread: HashMap<CommentThreadId, Vec<Comment>> = HashMap::new();
        for comment in self.comments(meeting).await? {
            by_thread
                .entry(comment.thread_id)
                .or_default()
                .push(comment);
        }
        let threads = self.threads(meeting).await?;
        Ok(threads
            .into_iter()
            .map(|thread| {
                let comments = by_thread.remove(&thread.id).unwrap_or_default();
                thread.into_thread(comments)
            })
            .collect())
    }

    async fn insert_thread(&self, thread: &ThreadRow) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let anchor = &thread.anchor;
        let inserted = sqlx::query!(
            "INSERT INTO comment_threads (id, meeting_id, target, segment_id, start_offset, end_offset, quote, created_at)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
             WHERE ?4 IS NULL
                OR EXISTS (SELECT 1 FROM transcript_segments WHERE id = ?4 AND meeting_id = ?2)",
            thread.id,
            thread.meeting_id,
            anchor.target,
            anchor.segment_id,
            anchor.start,
            anchor.end,
            anchor.quote,
            thread.first.created_at,
        )
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            return Ok(false);
        }
        insert_comment(&mut transaction, &thread.first).await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn insert_comment(&self, comment: &Comment) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let live = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count: i64" FROM comment_threads t JOIN meetings m ON m.id = t.meeting_id
            WHERE t.id = ?1 AND m.deleted_at IS NULL"#,
            comment.thread_id,
        )
        .fetch_one(&mut *transaction)
        .await?;
        if live == 0 {
            return Ok(false);
        }
        insert_comment(&mut transaction, comment).await?;
        transaction.commit().await?;
        Ok(true)
    }

    async fn edit(&self, id: &CommentId, body: &str, at: DateTime<Utc>) -> Result<bool, DbError> {
        let done = sqlx::query!(
            "UPDATE comments SET body = ?2, updated_at = ?3 WHERE id = ?1",
            id,
            body,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn set_resolved(
        &self,
        thread: &CommentThreadId,
        at: Option<DateTime<Utc>>,
    ) -> Result<bool, DbError> {
        let done = sqlx::query!(
            "UPDATE comment_threads SET resolved_at = ?2 WHERE id = ?1",
            thread,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn delete(&self, comment: &CommentId) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let Some(thread) = sqlx::query_scalar!(
            r#"DELETE FROM comments WHERE id = ?1 RETURNING thread_id AS "thread_id: CommentThreadId""#,
            comment,
        )
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Ok(false);
        };
        sqlx::query!(
            "DELETE FROM comment_threads WHERE id = ?1 AND NOT EXISTS (SELECT 1 FROM comments WHERE thread_id = ?1)",
            thread,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }
}

/// Inserts one comment inside `transaction`.
async fn insert_comment(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    comment: &Comment,
) -> Result<(), DbError> {
    sqlx::query!(
        "INSERT INTO comments (id, thread_id, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        comment.id,
        comment.thread_id,
        comment.body,
        comment.created_at,
        comment.updated_at,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
