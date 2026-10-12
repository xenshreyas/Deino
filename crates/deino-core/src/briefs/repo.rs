//! [`BriefRepo`]: briefs in the `briefs` table, and which meeting notes an
//! event already has.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{CalendarEventId, MeetingId};
use sqlx::types::Json;

use super::{BriefBullet, BriefFeedback, BriefStatus, StoredBrief};

/// Reads and writes briefs.
#[async_trait]
pub trait BriefRepo: Send + Sync {
    /// The brief of `event`, ready or not.
    async fn find(&self, event: &CalendarEventId) -> Result<Option<StoredBrief>, DbError>;

    /// Stores a new generation of a brief, replacing the last one; its read
    /// time and feedback start over.
    async fn save(&self, brief: &StoredBrief) -> Result<(), DbError>;

    /// Records that the user saw the brief, unless they already had.
    async fn mark_read(&self, event: &CalendarEventId, at: DateTime<Utc>) -> Result<(), DbError>;

    /// Records the user's verdict.
    async fn set_feedback(
        &self,
        event: &CalendarEventId,
        feedback: BriefFeedback,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The meetings (outside the trash) linked to `event`: its own note,
    /// which is never a source of its brief.
    async fn notes_of(&self, event: &CalendarEventId) -> Result<Vec<MeetingId>, DbError>;
}

/// [`BriefRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteBriefRepo {
    database: Database,
}

impl SqliteBriefRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `briefs` row.
struct Row {
    event_id: CalendarEventId,
    status: BriefStatus,
    inputs_hash: String,
    bullets: Json<Vec<BriefBullet>>,
    generated_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
    feedback: Option<BriefFeedback>,
}

impl From<Row> for StoredBrief {
    fn from(row: Row) -> Self {
        Self {
            event_id: row.event_id,
            status: row.status,
            inputs_hash: row.inputs_hash,
            bullets: row.bullets.0,
            generated_at: row.generated_at,
            read_at: row.read_at,
            feedback: row.feedback,
        }
    }
}

#[async_trait]
impl BriefRepo for SqliteBriefRepo {
    async fn find(&self, event: &CalendarEventId) -> Result<Option<StoredBrief>, DbError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT
                event_id AS "event_id!: CalendarEventId",
                status AS "status!: BriefStatus",
                inputs_hash AS "inputs_hash!",
                bullets_json AS "bullets!: Json<Vec<BriefBullet>>",
                generated_at AS "generated_at!: DateTime<Utc>",
                read_at AS "read_at: DateTime<Utc>",
                feedback AS "feedback: BriefFeedback"
            FROM briefs WHERE event_id = ?1"#,
            event,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.map(StoredBrief::from))
    }

    async fn save(&self, brief: &StoredBrief) -> Result<(), DbError> {
        let bullets = Json(&brief.bullets);
        sqlx::query!(
            "INSERT INTO briefs (event_id, status, inputs_hash, bullets_json, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (event_id) DO UPDATE SET
                 status = excluded.status, inputs_hash = excluded.inputs_hash,
                 bullets_json = excluded.bullets_json, generated_at = excluded.generated_at,
                 read_at = NULL, feedback = NULL, feedback_at = NULL",
            brief.event_id,
            brief.status,
            brief.inputs_hash,
            bullets,
            brief.generated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn mark_read(&self, event: &CalendarEventId, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE briefs SET read_at = ?2 WHERE event_id = ?1 AND read_at IS NULL",
            event,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn set_feedback(
        &self,
        event: &CalendarEventId,
        feedback: BriefFeedback,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE briefs SET feedback = ?2, feedback_at = ?3 WHERE event_id = ?1",
            event,
            feedback,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn notes_of(&self, event: &CalendarEventId) -> Result<Vec<MeetingId>, DbError> {
        let ids = sqlx::query_scalar!(
            r#"SELECT id AS "id!: MeetingId" FROM meetings
               WHERE calendar_event_id = ?1 AND deleted_at IS NULL"#,
            event,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(ids)
    }
}
