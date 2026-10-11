//! Global search (ORG-08) and the semantic index (AI-01), PARITY_PLAN §4.4,
//! ADR 0028.
//!
//! - **Keyword:** `search_documents` holds one row per searchable thing
//!   (meeting titles, notes, summaries, transcript segments, action items,
//!   people, companies, folders), derived from its source by triggers in the
//!   same transaction as every write; `search_fts` is its FTS5 index (porter
//!   stemming, prefix indexes, BM25). [`SqliteSearchIndex`] queries it.
//! - **The query language** ([`query`]): words, `"phrases"`, `@mentions`
//!   resolved through [`FilterDirectory`], and bare domains (`acme.com`).
//! - **Semantic** ([`semantic`]): chunks embedded on the Mac, searched by an
//!   in-memory cosine index.
//! - **Fusion** ([`hybrid`]): reciprocal rank fusion of both lists.
//! - [`SearchService`]: the palette and results page ([`SearchService::search`])
//!   and chat's retrieval API ([`SearchService::search_hybrid`]).
//! - [`MeetingIndexer`]: the `index` pipeline stage's work for one meeting.
//!
//! Nothing here logs queries, snippets or indexed text.

mod directory;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod fts;
pub mod hybrid;
mod index;
mod meeting_indexer;
mod model;
pub mod query;
pub mod sections;
pub mod semantic;
mod service;
pub mod snippet;
mod sqlite;

pub use error::SearchError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeEmbedder, FakeSemanticSearch};
pub use index::{FilterDirectory, KeywordQuery, ReindexReport, ScopedMeeting, SearchIndex};
pub use meeting_indexer::{MeetingIndexOutcome, MeetingIndexer, SearchIndexing};
pub use model::{
    FilterChip, FilterKind, Highlight, Hit, HitKind, MAX_HITS_PER_KIND, SearchFilters,
    SearchRequest, SearchResults, Snippet,
};
pub use service::{MAX_SUGGESTIONS, SearchService};
pub use sqlite::SqliteSearchIndex;

#[cfg(test)]
mod bench_tests;
