//! The connection port and its SQLite implementation.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{AuthMethod, ConnectionId, ConnectionStatus, ErrorCode, IntegrationProvider};

use super::{Connection, ConnectionError};

/// Where connections are stored.
#[async_trait]
pub trait ConnectionRepo: Send + Sync {
    /// Every connection, by provider, oldest first, with its scopes.
    async fn connections(&self) -> Result<Vec<Connection>, DbError>;

    /// The connection with `id`.
    async fn connection(&self, id: ConnectionId) -> Result<Option<Connection>, DbError>;

    /// The ID of `provider`'s connection to `account_id`, if there is one.
    async fn find(
        &self,
        provider: IntegrationProvider,
        account_id: &str,
    ) -> Result<Option<ConnectionId>, DbError>;

    /// Adds `connection`, or refreshes the row for the same account (keeping
    /// its ID), and replaces its scopes, in one transaction. Returns the ID.
    async fn upsert(&self, connection: &Connection) -> Result<ConnectionId, DbError>;

    /// Records a request that worked: clears the error, marks it connected.
    async fn record_success(&self, id: ConnectionId, at: DateTime<Utc>) -> Result<(), DbError>;

    /// Records a failure, and the status it leaves the connection in.
    async fn record_failure(
        &self,
        id: ConnectionId,
        status: ConnectionStatus,
        error: &ConnectionError,
    ) -> Result<(), DbError>;

    /// Saves when the refreshed access token expires.
    async fn set_token_expiry(
        &self,
        id: ConnectionId,
        expires_at: Option<DateTime<Utc>>,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// Deletes the connection and its scopes; its deliveries stay in the log.
    async fn delete(&self, id: ConnectionId) -> Result<bool, DbError>;
}

/// [`ConnectionRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteConnectionRepo {
    database: Database,
}

impl SqliteConnectionRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    async fn scopes(&self) -> Result<BTreeMap<ConnectionId, Vec<String>>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT connection_id AS "connection_id: ConnectionId", scope
               FROM integration_connection_scopes ORDER BY connection_id, scope"#
        )
        .fetch_all(self.database.reader())
        .await?;
        let mut scopes: BTreeMap<ConnectionId, Vec<String>> = BTreeMap::new();
        for row in rows {
            scopes.entry(row.connection_id).or_default().push(row.scope);
        }
        Ok(scopes)
    }
}

/// One `integration_connections` row.
struct ConnectionRow {
    id: ConnectionId,
    provider: IntegrationProvider,
    account_id: String,
    account_label: String,
    auth_method: AuthMethod,
    status: ConnectionStatus,
    api_base_url: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
    connected_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    last_error_code: Option<ErrorCode>,
    last_error_message: Option<String>,
    last_error_at: Option<DateTime<Utc>>,
}

impl ConnectionRow {
    fn into_connection(self, scopes: Vec<String>) -> Connection {
        let last_error = match (
            self.last_error_code,
            self.last_error_message,
            self.last_error_at,
        ) {
            (Some(code), Some(message), Some(at)) => Some(ConnectionError { code, message, at }),
            _ => None,
        };
        Connection {
            id: self.id,
            provider: self.provider,
            account_id: self.account_id,
            account_label: self.account_label,
            auth_method: self.auth_method,
            status: self.status,
            scopes,
            api_base_url: self.api_base_url,
            token_expires_at: self.token_expires_at,
            connected_at: self.connected_at,
            updated_at: self.updated_at,
            last_used_at: self.last_used_at,
            last_error,
        }
    }
}

#[async_trait]
impl ConnectionRepo for SqliteConnectionRepo {
    async fn connections(&self) -> Result<Vec<Connection>, DbError> {
        let rows = sqlx::query_file_as!(ConnectionRow, "src/connection/queries/connections.sql")
            .fetch_all(self.database.reader())
            .await?;
        let mut scopes = self.scopes().await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let granted = scopes.remove(&row.id).unwrap_or_default();
                row.into_connection(granted)
            })
            .collect())
    }

    async fn connection(&self, id: ConnectionId) -> Result<Option<Connection>, DbError> {
        let row = sqlx::query_file_as!(ConnectionRow, "src/connection/queries/connection.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        let Some(row) = row else { return Ok(None) };
        let scopes = sqlx::query_scalar!(
            "SELECT scope FROM integration_connection_scopes WHERE connection_id = ?1 ORDER BY scope",
            id
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(Some(row.into_connection(scopes)))
    }

    async fn find(
        &self,
        provider: IntegrationProvider,
        account_id: &str,
    ) -> Result<Option<ConnectionId>, DbError> {
        let id = sqlx::query_scalar!(
            r#"SELECT id AS "id: ConnectionId" FROM integration_connections
               WHERE provider = ?1 AND account_id = ?2"#,
            provider,
            account_id
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(id)
    }

    async fn upsert(&self, connection: &Connection) -> Result<ConnectionId, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let id = sqlx::query_file_scalar!(
            "src/connection/queries/upsert.sql",
            connection.id,
            connection.provider,
            connection.account_id,
            connection.account_label,
            connection.auth_method,
            connection.api_base_url,
            connection.token_expires_at,
            connection.connected_at,
        )
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query!(
            "DELETE FROM integration_connection_scopes WHERE connection_id = ?1",
            id
        )
        .execute(&mut *transaction)
        .await?;
        for scope in &connection.scopes {
            sqlx::query!(
                "INSERT OR IGNORE INTO integration_connection_scopes (connection_id, scope) VALUES (?1, ?2)",
                id,
                scope
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(id)
    }

    async fn record_success(&self, id: ConnectionId, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE integration_connections
             SET status = 'connected', last_used_at = ?2, updated_at = ?2,
                 last_error_code = NULL, last_error_message = NULL, last_error_at = NULL
             WHERE id = ?1",
            id,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn record_failure(
        &self,
        id: ConnectionId,
        status: ConnectionStatus,
        error: &ConnectionError,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE integration_connections
             SET status = ?2, last_error_code = ?3, last_error_message = ?4, last_error_at = ?5,
                 updated_at = ?5
             WHERE id = ?1",
            id,
            status,
            error.code,
            error.message,
            error.at
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn set_token_expiry(
        &self,
        id: ConnectionId,
        expires_at: Option<DateTime<Utc>>,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE integration_connections SET token_expires_at = ?2, updated_at = ?3 WHERE id = ?1",
            id,
            expires_at,
            at
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn delete(&self, id: ConnectionId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM integration_connections WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod repo_tests;
