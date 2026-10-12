//! [`FollowUpRepo`]: follow-ups in the `follow_up_drafts` table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::MeetingId;
use sqlx::types::Json;

use super::{FollowUpStatus, FollowUpSummary, QuietReason, Recipients, StoredFollowUp};

/// Reads and writes follow-ups.
#[async_trait]
pub trait FollowUpRepo: Send + Sync {
    /// The follow-up of `meeting`, whatever its status.
    async fn find(&self, meeting: &MeetingId) -> Result<Option<StoredFollowUp>, DbError>;

    /// Stores what the stage decided, replacing an earlier row unless the
    /// user dismissed or edited it. Returns false when the row was kept.
    async fn save(&self, follow_up: &StoredFollowUp) -> Result<bool, DbError>;

    /// Replaces a ready draft's text and recipients with the user's; false
    /// when the meeting has no ready draft.
    async fn edit(&self, edited: &StoredFollowUp) -> Result<bool, DbError>;

    /// Dismisses the follow-up for good and clears its text; false when
    /// there was nothing to dismiss.
    async fn dismiss(&self, meeting: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError>;

    /// Ready drafts written at or after `since`, newest first, of meetings
    /// outside the trash.
    async fn ready_since(&self, since: DateTime<Utc>) -> Result<Vec<FollowUpSummary>, DbError>;
}

/// [`FollowUpRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteFollowUpRepo {
    pub(super) database: Database,
}

impl SqliteFollowUpRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `follow_up_drafts` row.
struct Row {
    meeting_id: MeetingId,
    status: FollowUpStatus,
    quiet_reason: Option<QuietReason>,
    subject: String,
    body: String,
    recipients: Json<Recipients>,
    generated_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    dismissed_at: Option<DateTime<Utc>>,
}

impl From<Row> for StoredFollowUp {
    fn from(row: Row) -> Self {
        Self {
            meeting_id: row.meeting_id,
            status: row.status,
            quiet_reason: row.quiet_reason,
            subject: row.subject,
            body: row.body,
            recipients: row.recipients.0,
            generated_at: row.generated_at,
            edited_at: row.edited_at,
            dismissed_at: row.dismissed_at,
        }
    }
}

#[async_trait]
impl FollowUpRepo for SqliteFollowUpRepo {
    async fn find(&self, meeting: &MeetingId) -> Result<Option<StoredFollowUp>, DbError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT
                meeting_id AS "meeting_id!: MeetingId",
                status AS "status!: FollowUpStatus",
                quiet_reason AS "quiet_reason: QuietReason",
                subject AS "subject!",
                body AS "body!",
                recipients_json AS "recipients!: Json<Recipients>",
                generated_at AS "generated_at!: DateTime<Utc>",
                edited_at AS "edited_at: DateTime<Utc>",
                dismissed_at AS "dismissed_at: DateTime<Utc>"
            FROM follow_up_drafts WHERE meeting_id = ?1"#,
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.map(StoredFollowUp::from))
    }

    async fn save(&self, follow_up: &StoredFollowUp) -> Result<bool, DbError> {
        let recipients = Json(&follow_up.recipients);
        let done = sqlx::query!(
            "INSERT INTO follow_up_drafts
                 (meeting_id, status, quiet_reason, subject, body, recipients_json, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (meeting_id) DO UPDATE SET
                 status = excluded.status, quiet_reason = excluded.quiet_reason,
                 subject = excluded.subject, body = excluded.body,
                 recipients_json = excluded.recipients_json,
                 generated_at = excluded.generated_at
             WHERE follow_up_drafts.status <> 'dismissed'
                 AND follow_up_drafts.edited_at IS NULL",
            follow_up.meeting_id,
            follow_up.status,
            follow_up.quiet_reason,
            follow_up.subject,
            follow_up.body,
            recipients,
            follow_up.generated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn edit(&self, edited: &StoredFollowUp) -> Result<bool, DbError> {
        let recipients = Json(&edited.recipients);
        let done = sqlx::query!(
            "UPDATE follow_up_drafts
             SET subject = ?2, body = ?3, recipients_json = ?4, edited_at = ?5
             WHERE meeting_id = ?1 AND status = 'ready'",
            edited.meeting_id,
            edited.subject,
            edited.body,
            recipients,
            edited.edited_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn dismiss(&self, meeting: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let done = sqlx::query!(
            "UPDATE follow_up_drafts
             SET status = 'dismissed', quiet_reason = NULL, subject = '', body = '',
                 dismissed_at = ?2
             WHERE meeting_id = ?1 AND status <> 'dismissed'",
            meeting,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn ready_since(&self, since: DateTime<Utc>) -> Result<Vec<FollowUpSummary>, DbError> {
        let summaries = sqlx::query_as!(
            FollowUpSummary,
            r#"SELECT
                f.meeting_id AS "meeting_id!: MeetingId",
                m.title AS "meeting_title!",
                f.subject AS "subject!",
                f.generated_at AS "generated_at!: DateTime<Utc>"
            FROM follow_up_drafts AS f
            JOIN meetings AS m ON m.id = f.meeting_id
            WHERE f.status = 'ready' AND f.generated_at >= ?1 AND m.deleted_at IS NULL
            ORDER BY f.generated_at DESC"#,
            since,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(summaries)
    }
}
