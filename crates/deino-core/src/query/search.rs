//! The [`Search`] port: text search across meetings, notes, AI notes,
//! transcript moments, action items, people, companies and folders.
//!
//! It is deliberately narrow, so the library does not depend on how search
//! works; [`HybridSearch`](super::HybridSearch) answers it with the app's own
//! search (ORG-08/AI-01's `Retriever`).

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use deino_domain::MeetingId;
use schemars::JsonSchema;
use serde::Serialize;

use super::QueryError;

/// How many hits a search returns when the caller does not say.
pub const DEFAULT_HITS: u32 = 20;
/// The most hits one search returns.
pub const MAX_HITS: u32 = 100;

/// What a hit points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HitKind {
    /// A meeting's title.
    Meeting,
    /// The user's own notes of a meeting.
    Notes,
    /// A meeting's AI notes.
    AiNotes,
    /// A moment in a transcript.
    Transcript,
    /// An action item.
    ActionItem,
    /// A person.
    Person,
    /// A company.
    Company,
    /// A folder.
    Folder,
}

/// How a search was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Keywords only (FTS5): no embedding model, or it did not answer in time.
    Keyword,
    /// Keywords and meaning, fused by rank (the local embedding model answered).
    Hybrid,
}

/// One search result.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// What matched.
    pub kind: HitKind,
    /// Its name: the meeting's title for meeting content, else its own.
    pub label: String,
    /// The meeting it belongs to, if any.
    #[schemars(with = "Option<String>")]
    pub meeting_id: Option<MeetingId>,
    /// When that meeting happened, in the user's zone.
    pub held_at: Option<DateTime<FixedOffset>>,
    /// For a transcript moment: milliseconds from the start of the recording.
    pub at_ms: Option<u32>,
    /// The matched text with some context; `…` marks a cut.
    pub snippet: String,
    /// `deino://meeting/<id>`, with `?t=<ms>` for a moment.
    pub link: Option<String>,
    /// Higher is better (rank fusion, BM25 or cosine).
    pub score: f64,
}

/// A search's hits and how it ran.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchOutcome {
    /// Keyword-only or hybrid.
    pub mode: SearchMode,
    /// The best hits first.
    pub hits: Vec<SearchHit>,
}

/// What to search for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery {
    /// Words, `"phrases"`, `@mentions` and domains, as in the app's search.
    pub text: String,
    /// Only this meeting.
    pub meeting: Option<MeetingId>,
    /// The most hits; 0 means [`DEFAULT_HITS`].
    pub limit: u32,
}

impl SearchQuery {
    /// The limit with its default applied and clamped.
    pub fn limit(&self) -> u32 {
        match self.limit {
            0 => DEFAULT_HITS,
            limit => limit.min(MAX_HITS),
        }
    }
}

/// Finds text in the library.
#[async_trait]
pub trait Search: Send + Sync {
    /// The best hits for `query` and whether meaning was searched too. A
    /// blank query finds nothing.
    async fn search(&self, query: &SearchQuery) -> Result<SearchOutcome, QueryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_defaults_and_clamps() {
        assert_eq!(SearchQuery::default().limit(), DEFAULT_HITS);
        let many = SearchQuery {
            limit: 10_000,
            ..SearchQuery::default()
        };
        assert_eq!(many.limit(), MAX_HITS);
    }

    #[test]
    fn modes_serialize_in_snake_case() {
        assert_eq!(
            serde_json::to_value(SearchMode::Hybrid).unwrap(),
            serde_json::json!("hybrid")
        );
    }
}
