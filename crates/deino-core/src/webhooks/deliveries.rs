//! [`DeliveryRepo`]: the delivery queue and log, and [`SqliteDeliveryRepo`]
//! over the `webhook_deliveries` table (SQL only).

mod rows;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, WebhookDeliveryId, WebhookId};

pub use rows::{AttemptRecord, Claimed, NewDelivery};
use rows::{ClaimRow, DeliveryRow};

use super::{Delivery, DeliveryStatus};

/// How many finished deliveries the log keeps per endpoint.
pub const KEEP_FINISHED: i64 = 200;

/// Stores deliveries: the queue the dispatcher works through and the log.
#[async_trait]
pub trait DeliveryRepo: Send + Sync {
    /// Queues `delivery`; returns the delivery that will carry it: the new
    /// one, the one with the same idempotency key, or a coalescing event's
    /// delivery still waiting for its first attempt.
    async fn enqueue(&self, delivery: &NewDelivery) -> Result<WebhookDeliveryId, DbError>;

    /// Moves a pending delivery to `sending`; `None` when it is not pending.
    async fn claim(
        &self,
        id: &WebhookDeliveryId,
        at: DateTime<Utc>,
    ) -> Result<Option<Claimed>, DbError>;

    /// Stores an attempt's outcome.
    async fn record(&self, record: &AttemptRecord) -> Result<(), DbError>;

    /// Pending deliveries due at `now`, earliest first.
    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<WebhookDeliveryId>, DbError>;

    /// When the earliest pending delivery is due.
    async fn next_due(&self) -> Result<Option<DateTime<Utc>>, DbError>;

    /// At launch: attempts cut off by a quit go back to `pending`, due `at`.
    async fn recover(&self, at: DateTime<Utc>) -> Result<u64, DbError>;

    /// Gives up every pending delivery of `webhook` with `error`.
    async fn fail_pending(
        &self,
        webhook: &WebhookId,
        error: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The newest `limit` deliveries of `webhook`.
    async fn list(&self, webhook: &WebhookId, limit: u32) -> Result<Vec<Delivery>, DbError>;

    /// One delivery.
    async fn get(&self, id: &WebhookDeliveryId) -> Result<Option<Delivery>, DbError>;

    /// Drops finished deliveries of `webhook` beyond the newest
    /// [`KEEP_FINISHED`].
    async fn prune(&self, webhook: &WebhookId) -> Result<(), DbError>;
}

/// [`DeliveryRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteDeliveryRepo {
    database: Database,
}

impl SqliteDeliveryRepo {
    /// The repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// A coalescing event's delivery still waiting for its first attempt.
    async fn waiting(&self, delivery: &NewDelivery) -> Result<Option<WebhookDeliveryId>, DbError> {
        let event = delivery.kind.as_str();
        let id = sqlx::query_scalar!(
            r#"SELECT id AS "id: WebhookDeliveryId" FROM webhook_deliveries
               WHERE webhook_id = ?1 AND event = ?2 AND status = 'pending' AND attempts = 0
                 AND meeting_id IS ?3 AND subject_id IS ?4
               LIMIT 1"#,
            delivery.webhook_id,
            event,
            delivery.meeting_id,
            delivery.subject_id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(id)
    }

    /// The delivery already holding `key` for `webhook`.
    async fn keyed(&self, webhook: &WebhookId, key: &str) -> Result<WebhookDeliveryId, DbError> {
        let id = sqlx::query_scalar!(
            r#"SELECT id AS "id: WebhookDeliveryId" FROM webhook_deliveries
               WHERE webhook_id = ?1 AND idempotency_key = ?2"#,
            webhook,
            key,
        )
        .fetch_one(self.database.reader())
        .await?;
        Ok(id)
    }
}

#[async_trait]
impl DeliveryRepo for SqliteDeliveryRepo {
    async fn enqueue(&self, delivery: &NewDelivery) -> Result<WebhookDeliveryId, DbError> {
        if delivery.coalesce
            && let Some(waiting) = self.waiting(delivery).await?
        {
            return Ok(waiting);
        }
        let event = delivery.kind.as_str();
        let inserted = sqlx::query!(
            "INSERT INTO webhook_deliveries (id, webhook_id, event, meeting_id, subject_id,
                 idempotency_key, status, attempts, body, next_attempt_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, ?7, ?8, ?9, ?9)
             ON CONFLICT DO NOTHING",
            delivery.id,
            delivery.webhook_id,
            event,
            delivery.meeting_id,
            delivery.subject_id,
            delivery.idempotency_key,
            delivery.body,
            delivery.due_at,
            delivery.created_at,
        )
        .execute(self.database.writer())
        .await?;
        match (&delivery.idempotency_key, inserted.rows_affected()) {
            (Some(key), 0) => self.keyed(&delivery.webhook_id, key).await,
            _ => Ok(delivery.id),
        }
    }

    async fn claim(
        &self,
        id: &WebhookDeliveryId,
        at: DateTime<Utc>,
    ) -> Result<Option<Claimed>, DbError> {
        let row = sqlx::query_as!(
            ClaimRow,
            r#"UPDATE webhook_deliveries SET status = 'sending', next_attempt_at = NULL,
                   updated_at = ?2
               WHERE id = ?1 AND status = 'pending'
               RETURNING id AS "id: WebhookDeliveryId", webhook_id AS "webhook_id: WebhookId",
                   event, meeting_id AS "meeting_id: MeetingId", subject_id, body, attempts,
                   created_at AS "created_at: DateTime<Utc>""#,
            id,
            at,
        )
        .fetch_optional(self.database.writer())
        .await?;
        Ok(row.and_then(ClaimRow::into_claimed))
    }

    async fn record(&self, record: &AttemptRecord) -> Result<(), DbError> {
        let status = record.status.as_str();
        let attempts = i64::from(record.attempts);
        let finished = (record.status != DeliveryStatus::Pending).then_some(record.at);
        let body = record.body.as_deref().filter(|_| finished.is_none());
        sqlx::query!(
            "UPDATE webhook_deliveries SET status = ?1, attempts = ?2, next_attempt_at = ?3,
                 last_status = ?4, last_error = ?5, body = ?6, finished_at = ?7, updated_at = ?8
             WHERE id = ?9",
            status,
            attempts,
            record.next_attempt_at,
            record.last_status,
            record.last_error,
            body,
            finished,
            record.at,
            record.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<WebhookDeliveryId>, DbError> {
        let ids = sqlx::query_scalar!(
            r#"SELECT id AS "id: WebhookDeliveryId" FROM webhook_deliveries
               WHERE status = 'pending' AND next_attempt_at <= ?1
               ORDER BY next_attempt_at, id LIMIT ?2"#,
            now,
            limit,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(ids)
    }

    async fn next_due(&self) -> Result<Option<DateTime<Utc>>, DbError> {
        let next = sqlx::query_scalar!(
            r#"SELECT min(next_attempt_at) AS "next: DateTime<Utc>" FROM webhook_deliveries
               WHERE status = 'pending'"#
        )
        .fetch_one(self.database.reader())
        .await?;
        Ok(next)
    }

    async fn recover(&self, at: DateTime<Utc>) -> Result<u64, DbError> {
        let result = sqlx::query!(
            "UPDATE webhook_deliveries SET status = 'pending', next_attempt_at = ?1, updated_at = ?1
             WHERE status = 'sending'",
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected())
    }

    async fn fail_pending(
        &self,
        webhook: &WebhookId,
        error: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE webhook_deliveries SET status = 'failed', next_attempt_at = NULL, body = NULL,
                 last_error = ?1, finished_at = ?2, updated_at = ?2
             WHERE webhook_id = ?3 AND status = 'pending'",
            error,
            at,
            webhook,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn list(&self, webhook: &WebhookId, limit: u32) -> Result<Vec<Delivery>, DbError> {
        let rows = sqlx::query_as!(
            DeliveryRow,
            r#"SELECT id AS "id: WebhookDeliveryId", webhook_id AS "webhook_id: WebhookId",
                   event, meeting_id AS "meeting_id: MeetingId", status, attempts,
                   next_attempt_at AS "next_attempt_at: DateTime<Utc>", last_status, last_error,
                   created_at AS "created_at: DateTime<Utc>",
                   finished_at AS "finished_at: DateTime<Utc>"
               FROM webhook_deliveries WHERE webhook_id = ?1
               ORDER BY created_at DESC, id DESC LIMIT ?2"#,
            webhook,
            limit,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(DeliveryRow::into_delivery)
            .collect())
    }

    async fn get(&self, id: &WebhookDeliveryId) -> Result<Option<Delivery>, DbError> {
        let row = sqlx::query_as!(
            DeliveryRow,
            r#"SELECT id AS "id: WebhookDeliveryId", webhook_id AS "webhook_id: WebhookId",
                   event, meeting_id AS "meeting_id: MeetingId", status, attempts,
                   next_attempt_at AS "next_attempt_at: DateTime<Utc>", last_status, last_error,
                   created_at AS "created_at: DateTime<Utc>",
                   finished_at AS "finished_at: DateTime<Utc>"
               FROM webhook_deliveries WHERE id = ?1"#,
            id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.and_then(DeliveryRow::into_delivery))
    }

    async fn prune(&self, webhook: &WebhookId) -> Result<(), DbError> {
        sqlx::query!(
            "DELETE FROM webhook_deliveries
             WHERE webhook_id = ?1 AND status IN ('succeeded', 'failed') AND id NOT IN (
                 SELECT id FROM webhook_deliveries
                 WHERE webhook_id = ?1 AND status IN ('succeeded', 'failed')
                 ORDER BY created_at DESC, id DESC LIMIT ?2)",
            webhook,
            KEEP_FINISHED,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}
