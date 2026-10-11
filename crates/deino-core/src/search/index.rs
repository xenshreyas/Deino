//! The search ports: the keyword index and the `@` filter directory.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

use super::{FilterChip, FilterKind, Hit, SearchError, SearchFilters};

/// A keyword search to run.
#[derive(Clone, Copy, Debug)]
pub struct KeywordQuery<'a> {
    /// The FTS5 `MATCH` expression (see `fts::match_expression`).
    pub expression: &'a str,
    /// Only these meetings' content, or everything outside the trash.
    pub scope: Option<&'a [MeetingId]>,
    /// The most hits of each kind.
    pub limit: u32,
}

/// A meeting that passed a search's filters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedMeeting {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it happened (started, else saved).
    pub date: DateTime<Utc>,
}

/// What a full reindex wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReindexReport {
    /// Documents in the index afterwards.
    pub documents: u32,
}

/// The keyword index (`search_documents` and its FTS5 table `search_fts`).
#[async_trait]
pub trait SearchIndex: Send + Sync {
    /// Hits for `query`, best first (BM25), at most `query.limit` of each
    /// kind. Trashed meetings' content never matches.
    async fn keyword(&self, query: KeywordQuery<'_>) -> Result<Vec<Hit>, SearchError>;

    /// Every meeting outside the trash that passes `filters`, newest first.
    async fn scope(&self, filters: &SearchFilters) -> Result<Vec<ScopedMeeting>, SearchError>;

    /// Derives the meeting's documents again from their sources. The
    /// triggers keep them in step already; this repairs anything else.
    async fn refresh_meeting(&self, meeting: &MeetingId) -> Result<(), SearchError>;

    /// Derives every document again and rebuilds the full-text index.
    async fn reindex_all(&self) -> Result<ReindexReport, SearchError>;
}

/// Names the `@` filters can refer to.
#[async_trait]
pub trait FilterDirectory: Send + Sync {
    /// Filters whose name, or a later word of it, starts with `prefix`
    /// (ignoring ASCII case): people first, then companies, tags, folders
    /// and meetings, by name; at most `limit`, only of `kind` if given.
    async fn suggest(
        &self,
        prefix: &str,
        kind: Option<FilterKind>,
        limit: u32,
    ) -> Result<Vec<FilterChip>, SearchError>;

    /// The filter named exactly `name` (ignoring ASCII case), by the same
    /// preference order, only of `kind` if given.
    async fn resolve(
        &self,
        kind: Option<FilterKind>,
        name: &str,
    ) -> Result<Option<FilterChip>, SearchError>;

    /// Whether any attendee's email or company uses `domain`.
    async fn knows_domain(&self, domain: &str) -> Result<bool, SearchError>;
}
