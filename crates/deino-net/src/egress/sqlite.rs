//! [`SqliteEgressLog`]: the egress log in the app's database (`egress_log`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::ServiceId;

use super::{EgressEntry, EgressOutcome, EgressReader, EgressWriter};
use crate::Method;

/// Rows kept: older ones are pruned on every append.
pub const MAX_ROWS: i64 = 10_000;

/// [`EgressReader`] and [`EgressWriter`] over the `egress_log` table.
#[derive(Clone, Debug)]
pub struct SqliteEgressLog {
    database: Database,
}

impl SqliteEgressLog {
    /// A log over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// A row as stored; SQLite integers are signed.
struct Row {
    at: DateTime<Utc>,
    service: ServiceId,
    host: String,
    method: Method,
    status: Option<i64>,
    outcome: EgressOutcome,
    bytes_up: i64,
    bytes_down: i64,
}

impl From<Row> for EgressEntry {
    fn from(row: Row) -> Self {
        Self {
            at: row.at,
            service: row.service,
            host: row.host,
            method: row.method,
            status: row.status.and_then(|status| u16::try_from(status).ok()),
            outcome: row.outcome,
            bytes_up: u64::try_from(row.bytes_up).unwrap_or_default(),
            bytes_down: u64::try_from(row.bytes_down).unwrap_or_default(),
        }
    }
}

/// A byte count as SQLite stores it (saturating; no transfer reaches 2^63).
fn signed(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[async_trait]
impl EgressWriter for SqliteEgressLog {
    async fn append(&self, entries: &[EgressEntry]) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        for entry in entries {
            let (bytes_up, bytes_down) = (signed(entry.bytes_up), signed(entry.bytes_down));
            sqlx::query!(
                "INSERT INTO egress_log
                    (at, service, host, method, status, outcome, bytes_up, bytes_down)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                entry.at,
                entry.service,
                entry.host,
                entry.method,
                entry.status,
                entry.outcome,
                bytes_up,
                bytes_down,
            )
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query!(
            "DELETE FROM egress_log WHERE id <= (SELECT MAX(id) FROM egress_log) - ?1",
            MAX_ROWS,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl EgressReader for SqliteEgressLog {
    async fn recent(&self, limit: u32) -> Result<Vec<EgressEntry>, DbError> {
        let rows = sqlx::query_file_as!(Row, "src/egress/queries/recent.sql", limit)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows.into_iter().map(EgressEntry::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use deino_db::test_db;

    use super::*;

    fn entry(host: &str, bytes_down: u64) -> EgressEntry {
        EgressEntry {
            at: Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap(),
            service: ServiceId::LlmAnthropic,
            host: host.to_owned(),
            method: Method::Post,
            status: Some(200),
            outcome: EgressOutcome::Sent,
            bytes_up: 1_200,
            bytes_down,
        }
    }

    #[tokio::test]
    async fn entries_round_trip_newest_first() {
        let log = SqliteEgressLog::new(test_db().await);
        let first = entry("api.anthropic.com", 10);
        let second = EgressEntry {
            status: None,
            outcome: EgressOutcome::Blocked,
            ..entry("localhost:11434", 0)
        };

        log.append(&[first.clone(), second.clone()]).await.unwrap();

        assert_eq!(log.recent(10).await.unwrap(), [second, first.clone()]);
        assert_eq!(log.recent(1).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn every_service_and_method_fits_the_schema() {
        let log = SqliteEgressLog::new(test_db().await);
        let entries: Vec<EgressEntry> = ServiceId::ALL
            .iter()
            .zip(Method::ALL.iter().cycle())
            .map(|(&service, &method)| EgressEntry {
                service,
                method,
                ..entry("example.test", 0)
            })
            .collect();

        log.append(&entries).await.unwrap();

        assert_eq!(log.recent(100).await.unwrap().len(), ServiceId::ALL.len());
    }

    #[tokio::test]
    async fn the_schema_refuses_a_path_or_query_in_the_host() {
        let log = SqliteEgressLog::new(test_db().await);

        let path = log.append(&[entry("api.openai.com/v1/chat", 0)]).await;
        let query = log.append(&[entry("api.openai.com?key=sk", 0)]).await;

        assert!(path.is_err());
        assert!(query.is_err());
    }

    #[tokio::test]
    async fn the_newest_entries_are_listed_through_the_primary_key() {
        let database = test_db().await;
        let sql = include_str!("queries/recent.sql");
        let plan: Vec<(i64, i64, i64, String)> =
            sqlx::query_as(&format!("EXPLAIN QUERY PLAN {sql}"))
                .bind(10)
                .fetch_all(database.reader())
                .await
                .unwrap();

        let steps: Vec<&str> = plan.iter().map(|(.., detail)| detail.as_str()).collect();
        assert_eq!(steps, ["SCAN egress_log"]);
    }
}
