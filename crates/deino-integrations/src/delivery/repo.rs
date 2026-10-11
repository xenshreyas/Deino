//! The delivery log port and its SQLite implementation.
//!
//! Idempotency lives in the database: one row per key (a UNIQUE index), a
//! compare-and-set claim before each attempt, so two callers with the same
//! key never both send, and a succeeded row is returned instead of sending
//! again.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{
    Capability, ConnectionId, DeliveryId, DeliveryStatus, ErrorCode, IntegrationProvider, MeetingId,
};

use super::{Delivery, DeliveryFailure, ExternalRef};

/// A delivery to record before its first attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewDelivery {
    /// Its ID, used if the key is new.
    pub id: DeliveryId,
    /// The account it is sent with.
    pub connection_id: ConnectionId,
    /// The provider.
    pub provider: IntegrationProvider,
    /// What it writes.
    pub capability: Capability,
    /// The meeting it comes from.
    pub meeting_id: Option<MeetingId>,
    /// Its key.
    pub idempotency_key: String,
    /// When it was requested.
    pub at: DateTime<Utc>,
}

/// Where deliveries are recorded.
#[async_trait]
pub trait DeliveryRepo: Send + Sync {
    /// Records `delivery` as pending unless its key exists; returns the row
    /// for the key either way.
    async fn begin(&self, delivery: &NewDelivery) -> Result<Delivery, DbError>;

    /// Claims the delivery for one attempt: `pending` or `failed` rows, and
    /// `sending` rows last touched before `stale_before` (a crash mid-send),
    /// move to `sending` with one more attempt. False when someone else
    /// holds it or it already succeeded.
    async fn claim(
        &self,
        id: DeliveryId,
        stale_before: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Marks the claimed delivery succeeded with what the provider created.
    async fn succeed(
        &self,
        id: DeliveryId,
        external: &ExternalRef,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// Marks the claimed delivery failed.
    async fn fail(
        &self,
        id: DeliveryId,
        failure: &DeliveryFailure,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The delivery with `key`.
    async fn by_key(&self, key: &str) -> Result<Option<Delivery>, DbError>;

    /// The newest `limit` deliveries, newest first.
    async fn recent(&self, limit: u32) -> Result<Vec<Delivery>, DbError>;
}

/// [`DeliveryRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteDeliveryRepo {
    database: Database,
}

impl SqliteDeliveryRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `integration_deliveries` row.
struct DeliveryRow {
    id: DeliveryId,
    connection_id: Option<ConnectionId>,
    provider: IntegrationProvider,
    capability: Capability,
    meeting_id: Option<MeetingId>,
    idempotency_key: String,
    status: DeliveryStatus,
    attempts: u32,
    external_id: Option<String>,
    external_url: Option<String>,
    error_code: Option<ErrorCode>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DeliveryRow> for Delivery {
    fn from(row: DeliveryRow) -> Self {
        let external = row.external_id.map(|id| ExternalRef {
            id,
            url: row.external_url,
        });
        let error = match (row.error_code, row.error_message) {
            (Some(code), Some(message)) => Some(DeliveryFailure { code, message }),
            _ => None,
        };
        Self {
            id: row.id,
            connection_id: row.connection_id,
            provider: row.provider,
            capability: row.capability,
            meeting_id: row.meeting_id,
            idempotency_key: row.idempotency_key,
            status: row.status,
            attempts: row.attempts,
            external,
            error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl DeliveryRepo for SqliteDeliveryRepo {
    async fn begin(&self, delivery: &NewDelivery) -> Result<Delivery, DbError> {
        sqlx::query!(
            "INSERT INTO integration_deliveries (
                 id, connection_id, provider, capability, meeting_id, idempotency_key,
                 status, attempts, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, ?7, ?7)
             ON CONFLICT (idempotency_key) DO NOTHING",
            delivery.id,
            delivery.connection_id,
            delivery.provider,
            delivery.capability,
            delivery.meeting_id,
            delivery.idempotency_key,
            delivery.at,
        )
        .execute(self.database.writer())
        .await?;
        let row = sqlx::query_file_as!(
            DeliveryRow,
            "src/delivery/queries/by_key.sql",
            delivery.idempotency_key
        )
        .fetch_one(self.database.writer())
        .await?;
        Ok(row.into())
    }

    async fn claim(
        &self,
        id: DeliveryId,
        stale_before: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE integration_deliveries
             SET status = 'sending', attempts = attempts + 1, updated_at = ?3
             WHERE id = ?1
               AND (status IN ('pending', 'failed') OR (status = 'sending' AND updated_at < ?2))",
            id,
            stale_before,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    async fn succeed(
        &self,
        id: DeliveryId,
        external: &ExternalRef,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE integration_deliveries
             SET status = 'succeeded', external_id = ?2, external_url = ?3,
                 error_code = NULL, error_message = NULL, updated_at = ?4
             WHERE id = ?1",
            id,
            external.id,
            external.url,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn fail(
        &self,
        id: DeliveryId,
        failure: &DeliveryFailure,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE integration_deliveries
             SET status = 'failed', error_code = ?2, error_message = ?3, updated_at = ?4
             WHERE id = ?1",
            id,
            failure.code,
            failure.message,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn by_key(&self, key: &str) -> Result<Option<Delivery>, DbError> {
        let row = sqlx::query_file_as!(DeliveryRow, "src/delivery/queries/by_key.sql", key)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(row.map(Delivery::from))
    }

    async fn recent(&self, limit: u32) -> Result<Vec<Delivery>, DbError> {
        let rows = sqlx::query_file_as!(DeliveryRow, "src/delivery/queries/recent.sql", limit)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(Delivery::from).collect())
    }
}

#[cfg(test)]
mod repo_tests;
