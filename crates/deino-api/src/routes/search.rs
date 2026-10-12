//! `GET /v1/search`: the app's search (keywords, and meaning when the
//! embedding model answers), with snippets redacted when the user asked
//! Deino to hide sensitive information (NOT-09).

use axum::extract::State;
use axum::http::Uri;
use deino_core::query::{MAX_HITS, SearchHit, SearchQuery};
use deino_domain::MeetingId;
use serde::Deserialize;

use crate::error::{ApiResult, app};
use crate::extract::Params;
use crate::page::{self, Paged};
use crate::{ApiError, ApiState};

/// `GET /v1/search` parameters.
#[derive(Debug, Default, Deserialize, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchParams {
    /// Words, `"phrases"`, `@mentions` and domains, as in the app.
    pub q: String,
    /// Only this meeting.
    pub meeting_id: Option<String>,
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

/// Searches the library; hits page by position (100 at most in all).
pub async fn search(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<SearchParams>,
) -> ApiResult<Paged<SearchHit>> {
    if params.q.trim().is_empty() {
        return Err(ApiError::invalid("q must not be blank"));
    }
    let size = page::page_size(params.limit);
    let filters = page::fingerprint(&(&params.q, &params.meeting_id, size));
    let query = SearchQuery {
        text: params.q,
        meeting: params.meeting_id.map(MeetingId::new),
        limit: MAX_HITS,
    };
    let outcome = state
        .services
        .library
        .search
        .search(&query)
        .await
        .map_err(app)?;
    let mut page = page::by_position(outcome.hits, params.cursor.as_deref(), (size, filters))?;
    redact(&state, &mut page.data).await;
    Ok(Paged { page, uri })
}

/// Hides secrets in the snippets, when the setting is on.
async fn redact(state: &ApiState, hits: &mut [SearchHit]) {
    let snippets = hits.iter().map(|hit| hit.snippet.clone()).collect();
    let redacted = state.services.redactor.redact(snippets).await;
    for (hit, snippet) in hits.iter_mut().zip(redacted) {
        hit.snippet = snippet;
    }
}
