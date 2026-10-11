//! [`SearchService`]: what the palette, the results page and chat (AI-02)
//! call.
//!
//! A search parses the text ([`query`](super::query)), turns `@mentions`
//! and domains into filters when they name something (and back into words
//! when they do not), narrows to the filtered meetings, runs the keyword
//! index and, once the embedding model is ready, fuses in semantic hits by
//! reciprocal rank ([`hybrid`](super::hybrid)).

use std::sync::Arc;

use super::index::{FilterDirectory, KeywordQuery, ReindexReport, ScopedMeeting, SearchIndex};
use super::query::{self, Mention, Term};
use super::semantic::SemanticSearch;
use super::{
    FilterChip, FilterKind, Hit, HitKind, MAX_HITS_PER_KIND, SearchError, SearchFilters,
    SearchRequest, SearchResults, Snippet, fts, hybrid, sections,
};
use deino_domain::MeetingId;

/// The most `@` suggestions offered at once.
pub const MAX_SUGGESTIONS: u32 = 8;

/// A request's terms and filters once its text is understood.
#[derive(Debug, Default)]
struct Plan {
    terms: Vec<Term>,
    open_ended: bool,
    /// Match any term (a question) instead of every term (a search box).
    any: bool,
    filters: SearchFilters,
}

impl Plan {
    /// The FTS5 expression: every term for a search box, any for a question.
    fn expression(&self) -> Option<String> {
        if self.any {
            fts::any_expression(&self.terms)
        } else {
            fts::match_expression(&self.terms, self.open_ended)
        }
    }

    /// The words and phrases as plain text, for the embedding model.
    fn semantic_text(&self) -> String {
        let parts: Vec<&str> = self
            .terms
            .iter()
            .map(|term| match term {
                Term::Word(text) | Term::Phrase(text) => text.as_str(),
            })
            .collect();
        parts.join(" ")
    }
}

/// Ranked hits and whether semantic hits were fused in.
struct Ranked {
    hits: Vec<Hit>,
    semantic: bool,
}

impl Ranked {
    /// Keyword hits alone.
    fn keyword(hits: Vec<Hit>) -> Self {
        Self {
            hits,
            semantic: false,
        }
    }
}

/// Global search: keyword (ORG-08) and semantic (AI-01).
pub struct SearchService {
    index: Arc<dyn SearchIndex>,
    directory: Arc<dyn FilterDirectory>,
    semantic: Arc<dyn SemanticSearch>,
}

impl SearchService {
    /// A service over the keyword index, the filter names and semantic search.
    pub fn new(
        index: Arc<dyn SearchIndex>,
        directory: Arc<dyn FilterDirectory>,
        semantic: Arc<dyn SemanticSearch>,
    ) -> Self {
        Self {
            index,
            directory,
            semantic,
        }
    }

    /// Hits for the palette and the results page, grouped by section.
    pub async fn search(&self, request: &SearchRequest) -> Result<SearchResults, SearchError> {
        let limit = request.limit.clamp(1, MAX_HITS_PER_KIND);
        let plan = self.plan(&request.text, request.filters.clone()).await?;
        let ranked = self.ranked(&plan, limit).await?;
        let mut results = sections::group(ranked.hits, limit as usize, plan.filters);
        results.semantic = ranked.semantic;
        Ok(results)
    }

    /// The `k` best hits for `query` within `filters`, keyword and semantic
    /// fused, best first: the retrieval API chat (AI-02) grounds answers on.
    /// A question's words are matched with OR, not AND.
    /// Moments carry their time; semantic hits carry their transcript
    /// window (`end_ms`).
    pub async fn search_hybrid(
        &self,
        query: &str,
        filters: &SearchFilters,
        k: usize,
    ) -> Result<Vec<Hit>, SearchError> {
        let limit = u32::try_from(k)
            .unwrap_or(MAX_HITS_PER_KIND)
            .clamp(1, MAX_HITS_PER_KIND);
        let plan = Plan {
            any: true,
            ..self.plan(query, filters.clone()).await?
        };
        let mut hits = self.ranked(&plan, limit).await?.hits;
        hits.truncate(k);
        Ok(hits)
    }

    /// `@` filter suggestions for what follows the `@`.
    pub async fn suggest_filters(
        &self,
        prefix: &str,
        kind: Option<FilterKind>,
    ) -> Result<Vec<FilterChip>, SearchError> {
        self.directory.suggest(prefix, kind, MAX_SUGGESTIONS).await
    }

    /// Rebuilds the keyword index from scratch.
    pub async fn reindex_all(&self) -> Result<ReindexReport, SearchError> {
        self.index.reindex_all().await
    }

    /// Parses `text` and resolves its mentions and domains into `filters`.
    async fn plan(&self, text: &str, mut filters: SearchFilters) -> Result<Plan, SearchError> {
        let parsed = query::parse(text);
        let mut terms = parsed.terms;
        for mention in parsed.mentions {
            match self.resolve(&mention).await? {
                Some(chip) => filters.add(chip),
                None => terms.push(Term::Word(mention.name)),
            }
        }
        for domain in parsed.domains {
            if self.directory.knows_domain(&domain).await? {
                filters.add_domain(domain);
            } else {
                terms.push(Term::Word(domain));
            }
        }
        Ok(Plan {
            terms,
            open_ended: parsed.open_ended,
            any: false,
            filters,
        })
    }

    /// The filter a mention names: the one with exactly that name, else the
    /// only one whose name (or a later word of it) starts with it, so
    /// `@maya` finds Maya Chen while `@ma` among two Mayas finds no one.
    async fn resolve(&self, mention: &Mention) -> Result<Option<FilterChip>, SearchError> {
        if let Some(chip) = self.directory.resolve(mention.kind, &mention.name).await? {
            return Ok(Some(chip));
        }
        let mut candidates = self
            .directory
            .suggest(&mention.name, mention.kind, 2)
            .await?;
        Ok(match candidates.len() {
            1 => candidates.pop(),
            _ => None,
        })
    }

    /// Keyword hits (or the filtered meetings when there is no text),
    /// fused with semantic hits when those are available.
    async fn ranked(&self, plan: &Plan, limit: u32) -> Result<Ranked, SearchError> {
        let scope = if plan.filters.is_empty() {
            None
        } else {
            Some(self.index.scope(&plan.filters).await?)
        };
        let ids: Option<Vec<MeetingId>> = scope
            .as_ref()
            .map(|meetings| meetings.iter().map(|meeting| meeting.id.clone()).collect());
        let Some(expression) = plan.expression() else {
            let hits = scope.map_or_else(Vec::new, |meetings| browse(meetings, limit));
            return Ok(Ranked::keyword(hits));
        };
        let query = KeywordQuery {
            expression: &expression,
            scope: ids.as_deref(),
            limit,
        };
        let keyword = self.index.keyword(query).await?;
        let k = usize::try_from(limit)
            .unwrap_or(usize::MAX)
            .saturating_mul(2);
        let semantic = self
            .semantic
            .search(&plan.semantic_text(), ids.as_deref(), k)
            .await?;
        Ok(match semantic {
            Some(semantic) => Ranked {
                hits: hybrid::fuse(keyword, semantic),
                semantic: true,
            },
            None => Ranked::keyword(keyword),
        })
    }
}

/// The filtered meetings as meeting hits, newest first.
fn browse(meetings: Vec<ScopedMeeting>, limit: u32) -> Vec<Hit> {
    meetings
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .map(|meeting| Hit {
            kind: HitKind::Meeting,
            ref_id: meeting.id.as_str().to_owned(),
            meeting_id: Some(meeting.id),
            ts_ms: None,
            end_ms: None,
            label: Snippet::plain(meeting.title),
            snippet: Snippet::default(),
            date: Some(meeting.date),
            score: 0.0,
        })
        .collect()
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
