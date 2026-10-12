//! The keys' storage port and its SQLite adapter (SQL only).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::ApiKeyId;

use super::{ApiKey, ApiKeyScope, StoredApiKey};

/// Where keys are stored.
#[async_trait]
pub trait ApiKeyRepo: Send + Sync {
    /// Every key, newest first.
    async fn list(&self) -> Result<Vec<ApiKey>, DbError>;
    /// The key with `id` and its hash.
    async fn find(&self, id: &ApiKeyId) -> Result<Option<StoredApiKey>, DbError>;
    /// Saves a new key.
    async fn insert(&self, key: &StoredApiKey) -> Result<(), DbError>;
    /// Marks the key revoked at `at`, unless it already is.
    async fn revoke(&self, id: &ApiKeyId, at: DateTime<Utc>) -> Result<(), DbError>;
    /// Removes the key; returns false when there was none.
    async fn delete(&self, id: &ApiKeyId) -> Result<bool, DbError>;
    /// Records that a request used the key at `at`.
    async fn touch(&self, id: &ApiKeyId, at: DateTime<Utc>) -> Result<(), DbError>;
}

/// [`ApiKeyRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteApiKeyRepo {
    database: Database,
}

impl SqliteApiKeyRepo {
    /// The repository over the app's database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `api_keys` row.
struct Row {
    id: ApiKeyId,
    name: String,
    scope: ApiKeyScope,
    secret_hash: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl From<Row> for StoredApiKey {
    fn from(row: Row) -> Self {
        Self {
            key: ApiKey {
                id: row.id,
                name: row.name,
                scope: row.scope,
                created_at: row.created_at,
                last_used_at: row.last_used_at,
                revoked_at: row.revoked_at,
            },
            secret_hash: row.secret_hash,
        }
    }
}

#[async_trait]
impl ApiKeyRepo for SqliteApiKeyRepo {
    async fn list(&self) -> Result<Vec<ApiKey>, DbError> {
        let rows = sqlx::query_file_as!(Row, "src/api_keys/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(|row| StoredApiKey::from(row).key).collect())
    }

    async fn find(&self, id: &ApiKeyId) -> Result<Option<StoredApiKey>, DbError> {
        let row = sqlx::query_file_as!(Row, "src/api_keys/queries/find.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(row.map(StoredApiKey::from))
    }

    async fn insert(&self, stored: &StoredApiKey) -> Result<(), DbError> {
        let key = &stored.key;
        sqlx::query!(
            "INSERT INTO api_keys (id, name, scope, secret_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            key.id,
            key.name,
            key.scope,
            stored.secret_hash,
            key.created_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn revoke(&self, id: &ApiKeyId, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE api_keys SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
            id,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &ApiKeyId) -> Result<bool, DbError> {
        let done = sqlx::query!("DELETE FROM api_keys WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn touch(&self, id: &ApiKeyId, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!("UPDATE api_keys SET last_used_at = ?2 WHERE id = ?1", id, at)
            .execute(self.database.writer())
            .await?;
        Ok(())
    }
}
