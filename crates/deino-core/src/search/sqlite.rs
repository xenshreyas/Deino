//! [`SearchIndex`] over SQLite: FTS5 queries on `search_fts`, filters as
//! JSON arrays, and the document rules of the ORG-08 migration as query
//! files (`queries/documents/`) for refreshing and rebuilding the index.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::Database;
use deino_domain::{MeetingId, Millis};
use sqlx::SqliteConnection;

use super::index::{KeywordQuery, ReindexReport, ScopedMeeting, SearchIndex};
use super::{FilterKind, Hit, HitKind, SearchError, SearchFilters, snippet};

/// [`SearchIndex`] and [`FilterDirectory`](super::FilterDirectory) over
/// Deino's database.
#[derive(Clone, Debug)]
pub struct SqliteSearchIndex {
    pub(super) database: Database,
}

impl SqliteSearchIndex {
    /// An index over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One row of `keyword.sql`.
struct KeywordRow {
    kind: HitKind,
    ref_id: String,
    meeting_id: Option<MeetingId>,
    ts_ms: Option<Millis>,
    label: String,
    snippet: String,
    date: Option<DateTime<Utc>>,
    score: f64,
}

impl From<KeywordRow> for Hit {
    fn from(row: KeywordRow) -> Self {
        Self {
            kind: row.kind,
            ref_id: row.ref_id,
            meeting_id: row.meeting_id,
            ts_ms: row.ts_ms,
            end_ms: None,
            label: snippet::from_marked(&row.label),
            snippet: snippet::from_marked(&row.snippet),
            date: row.date,
            score: row.score,
        }
    }
}

#[async_trait]
impl SearchIndex for SqliteSearchIndex {
    async fn keyword(&self, query: KeywordQuery<'_>) -> Result<Vec<Hit>, SearchError> {
        let scope = query
            .scope
            .map(|meetings| json_array(meetings.iter().map(MeetingId::as_str)));
        let rows = sqlx::query_file_as!(
            KeywordRow,
            "src/search/queries/keyword.sql",
            query.expression,
            scope,
            query.limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows.into_iter().map(Hit::from).collect())
    }

    async fn scope(&self, filters: &SearchFilters) -> Result<Vec<ScopedMeeting>, SearchError> {
        let ids = |kind| json_array(filters.ids(kind));
        let (people, companies, tags) = (
            ids(FilterKind::Person),
            ids(FilterKind::Company),
            ids(FilterKind::Tag),
        );
        let (folders, meetings) = (ids(FilterKind::Folder), ids(FilterKind::Meeting));
        let domains = json_array(filters.domains.iter().map(String::as_str));
        let rows = sqlx::query_file_as!(
            ScopedMeeting,
            "src/search/queries/scope.sql",
            people,
            companies,
            tags,
            folders,
            meetings,
            domains
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows)
    }

    async fn refresh_meeting(&self, meeting: &MeetingId) -> Result<(), SearchError> {
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "DELETE FROM search_documents WHERE meeting_id = ?1",
            meeting
        )
        .execute(&mut *transaction)
        .await?;
        insert_meeting_documents(&mut transaction, Some(meeting)).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn reindex_all(&self) -> Result<ReindexReport, SearchError> {
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!("DELETE FROM search_documents")
            .execute(&mut *transaction)
            .await?;
        insert_meeting_documents(&mut transaction, None).await?;
        sqlx::query_file!("src/search/queries/documents/others.sql")
            .execute(&mut *transaction)
            .await?;
        rebuild_full_text(&mut transaction).await?;
        let documents =
            sqlx::query_scalar!(r#"SELECT count(*) AS "count!: u32" FROM search_documents"#)
                .fetch_one(&mut *transaction)
                .await?;
        transaction.commit().await?;
        Ok(ReindexReport { documents })
    }
}

/// Writes the documents of `meeting` (or of every meeting) from their
/// sources: title, notes, summary, transcript and action items.
async fn insert_meeting_documents(
    connection: &mut SqliteConnection,
    meeting: Option<&MeetingId>,
) -> Result<(), SearchError> {
    sqlx::query_file!("src/search/queries/documents/meeting.sql", meeting)
        .execute(&mut *connection)
        .await?;
    sqlx::query_file!("src/search/queries/documents/note.sql", meeting)
        .execute(&mut *connection)
        .await?;
    sqlx::query_file!("src/search/queries/documents/summary.sql", meeting)
        .execute(&mut *connection)
        .await?;
    sqlx::query_file!("src/search/queries/documents/segment.sql", meeting)
        .execute(&mut *connection)
        .await?;
    sqlx::query_file!("src/search/queries/documents/action_item.sql", meeting)
        .execute(&mut *connection)
        .await?;
    Ok(())
}

/// Rebuilds `search_fts` from `search_documents`, repairing any drift, and
/// merges its segments for faster queries.
async fn rebuild_full_text(connection: &mut SqliteConnection) -> Result<(), SearchError> {
    sqlx::query!("INSERT INTO search_fts (search_fts) VALUES ('rebuild')")
        .execute(&mut *connection)
        .await?;
    sqlx::query!("INSERT INTO search_fts (search_fts) VALUES ('optimize')")
        .execute(&mut *connection)
        .await?;
    Ok(())
}

/// `values` as a JSON array of strings, for `json_each` in the queries.
pub(super) fn json_array<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    let strings: Vec<&str> = values.into_iter().collect();
    // Serialising strings cannot fail.
    serde_json::to_string(&strings).unwrap_or_else(|_| "[]".to_owned())
}

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
