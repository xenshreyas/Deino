//! [`WebhookRepo`]: the endpoints, and [`SqliteWebhookRepo`] over the
//! `webhooks` table (SQL only).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, WebhookId};
use sqlx::types::Json;

use super::{DisabledReason, Webhook, WebhookEvent};
use crate::automations::PayloadFields;

/// A failed attempt's effect on its endpoint's streak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Streak {
    /// Failed attempts in a row, this one included.
    pub failures: u32,
    /// When the streak began.
    pub since: DateTime<Utc>,
}

/// Stores the endpoints.
#[async_trait]
pub trait WebhookRepo: Send + Sync {
    /// Every endpoint, oldest first.
    async fn list(&self) -> Result<Vec<Webhook>, DbError>;

    /// One endpoint.
    async fn get(&self, id: &WebhookId) -> Result<Option<Webhook>, DbError>;

    /// Adds `webhook`.
    async fn create(&self, webhook: &Webhook) -> Result<(), DbError>;

    /// Replaces `webhook`'s fields; `false` when it is gone.
    async fn update(&self, webhook: &Webhook) -> Result<bool, DbError>;

    /// Removes an endpoint and its deliveries; `false` when it was gone.
    async fn delete(&self, id: &WebhookId) -> Result<bool, DbError>;

    /// A delivered attempt: the streak ends.
    async fn record_success(&self, id: &WebhookId, at: DateTime<Utc>) -> Result<(), DbError>;

    /// A failed attempt: the streak grows (or starts at `at`).
    async fn record_failure(&self, id: &WebhookId, at: DateTime<Utc>) -> Result<Streak, DbError>;

    /// Turns the endpoint off for `reason`.
    async fn disable(
        &self,
        id: &WebhookId,
        reason: DisabledReason,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The newest finished meeting outside the trash ("Send test").
    async fn latest_meeting(&self) -> Result<Option<MeetingId>, DbError>;
}

/// [`WebhookRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteWebhookRepo {
    database: Database,
}

impl SqliteWebhookRepo {
    /// The repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `webhooks` row.
struct WebhookRow {
    id: WebhookId,
    url: String,
    label: String,
    events: Json<Vec<WebhookEvent>>,
    fields: Json<PayloadFields>,
    enabled: bool,
    disabled_reason: Option<String>,
    failure_streak: i64,
    failing_since: Option<DateTime<Utc>>,
    last_success_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WebhookRow> for Webhook {
    fn from(row: WebhookRow) -> Self {
        Self {
            id: row.id,
            url: row.url,
            label: row.label,
            events: row.events.0,
            fields: row.fields.0,
            enabled: row.enabled,
            disabled_reason: row
                .disabled_reason
                .as_deref()
                .and_then(DisabledReason::parse),
            failure_streak: u32::try_from(row.failure_streak).unwrap_or(u32::MAX),
            failing_since: row.failing_since,
            last_success_at: row.last_success_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl WebhookRepo for SqliteWebhookRepo {
    async fn list(&self) -> Result<Vec<Webhook>, DbError> {
        let rows = sqlx::query_as!(
            WebhookRow,
            r#"SELECT id AS "id: WebhookId", url, label,
                   events_json AS "events: Json<Vec<WebhookEvent>>",
                   fields_json AS "fields: Json<PayloadFields>",
                   enabled AS "enabled: bool", disabled_reason, failure_streak,
                   failing_since AS "failing_since: DateTime<Utc>",
                   last_success_at AS "last_success_at: DateTime<Utc>",
                   created_at AS "created_at: DateTime<Utc>",
                   updated_at AS "updated_at: DateTime<Utc>"
               FROM webhooks ORDER BY created_at, id"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(Webhook::from).collect())
    }

    async fn get(&self, id: &WebhookId) -> Result<Option<Webhook>, DbError> {
        let row = sqlx::query_as!(
            WebhookRow,
            r#"SELECT id AS "id: WebhookId", url, label,
                   events_json AS "events: Json<Vec<WebhookEvent>>",
                   fields_json AS "fields: Json<PayloadFields>",
                   enabled AS "enabled: bool", disabled_reason, failure_streak,
                   failing_since AS "failing_since: DateTime<Utc>",
                   last_success_at AS "last_success_at: DateTime<Utc>",
                   created_at AS "created_at: DateTime<Utc>",
                   updated_at AS "updated_at: DateTime<Utc>"
               FROM webhooks WHERE id = ?1"#,
            id
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(row.map(Webhook::from))
    }

    async fn create(&self, webhook: &Webhook) -> Result<(), DbError> {
        let events = Json(&webhook.events);
        let fields = Json(&webhook.fields);
        sqlx::query!(
            "INSERT INTO webhooks (id, url, label, events_json, fields_json, enabled,
                 disabled_reason, failure_streak, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, 0, ?7, ?8)",
            webhook.id,
            webhook.url,
            webhook.label,
            events,
            fields,
            webhook.enabled,
            webhook.created_at,
            webhook.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn update(&self, webhook: &Webhook) -> Result<bool, DbError> {
        let events = Json(&webhook.events);
        let fields = Json(&webhook.fields);
        let reason = webhook.disabled_reason.map(DisabledReason::as_str);
        let streak = i64::from(webhook.failure_streak);
        let result = sqlx::query!(
            "UPDATE webhooks SET url = ?1, label = ?2, events_json = ?3, fields_json = ?4,
                 enabled = ?5, disabled_reason = ?6, failure_streak = ?7,
                 failing_since = ?8, updated_at = ?9
             WHERE id = ?10",
            webhook.url,
            webhook.label,
            events,
            fields,
            webhook.enabled,
            reason,
            streak,
            webhook.failing_since,
            webhook.updated_at,
            webhook.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &WebhookId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM webhooks WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn record_success(&self, id: &WebhookId, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE webhooks SET failure_streak = 0, failing_since = NULL,
                 last_success_at = ?1, updated_at = ?1
             WHERE id = ?2",
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn record_failure(&self, id: &WebhookId, at: DateTime<Utc>) -> Result<Streak, DbError> {
        let row = sqlx::query!(
            r#"UPDATE webhooks SET failure_streak = failure_streak + 1,
                   failing_since = coalesce(failing_since, ?1), updated_at = ?1
               WHERE id = ?2
               RETURNING failure_streak, failing_since AS "since!: DateTime<Utc>""#,
            at,
            id,
        )
        .fetch_optional(self.database.writer())
        .await?;
        Ok(row.map_or(
            Streak {
                failures: 1,
                since: at,
            },
            |row| Streak {
                failures: u32::try_from(row.failure_streak).unwrap_or(u32::MAX),
                since: row.since,
            },
        ))
    }

    async fn disable(
        &self,
        id: &WebhookId,
        reason: DisabledReason,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let reason = reason.as_str();
        sqlx::query!(
            "UPDATE webhooks SET enabled = 0, disabled_reason = ?1, updated_at = ?2 WHERE id = ?3",
            reason,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn latest_meeting(&self) -> Result<Option<MeetingId>, DbError> {
        let id = sqlx::query_scalar!(
            r#"SELECT id AS "id!: MeetingId" FROM meetings
               WHERE status = 'ready' AND deleted_at IS NULL
               ORDER BY started_at DESC, id DESC LIMIT 1"#
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(id)
    }
}
