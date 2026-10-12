//! `GET /v1/people`, `/v1/companies`, `/v1/folders`, `/v1/tags` and
//! `/v1/calendar/events`: the CLI's directory reads, a page at a time.

use axum::extract::State;
use axum::http::Uri;
use chrono::NaiveDate;
use deino_core::query::{
    CompanyView, EventView, EventWindow, FolderView, MAX_PEOPLE, PersonView, TagView,
};
use serde::Deserialize;

use crate::error::{ApiResult, app};
use crate::extract::Params;
use crate::page::{self, Paged};
use crate::ApiState;

/// Paging alone.
#[derive(Debug, Default, Deserialize, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageParams {
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

/// One page of `items`, which `filters` (and the page size) select.
fn paged<T>(
    items: Vec<T>,
    uri: Uri,
    params: &PageParams,
    filters: &impl std::hash::Hash,
) -> ApiResult<Paged<T>> {
    let size = page::page_size(params.limit);
    let fingerprint = page::fingerprint(&(filters, size));
    let page = page::by_position(items, params.cursor.as_deref(), (size, fingerprint))?;
    Ok(Paged { page, uri })
}

/// `GET /v1/people` parameters.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeopleParams {
    /// Only people whose name or email contains this.
    pub q: Option<String>,
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

/// People, by name.
pub async fn people(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<PeopleParams>,
) -> ApiResult<Paged<PersonView>> {
    let directory = &state.services.library.directory;
    let people = directory
        .people(params.q.as_deref(), MAX_PEOPLE)
        .await
        .map_err(app)?;
    let paging = PageParams {
        limit: params.limit,
        cursor: params.cursor,
    };
    paged(people, uri, &paging, &("people", &params.q))
}

/// Companies, by name.
pub async fn companies(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<PageParams>,
) -> ApiResult<Paged<CompanyView>> {
    let companies = state
        .services
        .library
        .directory
        .companies()
        .await
        .map_err(app)?;
    paged(companies, uri, &params, &"companies")
}

/// Folders, as a flat list with paths.
pub async fn folders(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<PageParams>,
) -> ApiResult<Paged<FolderView>> {
    let folders = state
        .services
        .library
        .directory
        .folders()
        .await
        .map_err(app)?;
    paged(folders, uri, &params, &"folders")
}

/// Tags, by name, with how many meetings carry each.
pub async fn tags(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<PageParams>,
) -> ApiResult<Paged<TagView>> {
    let tags = state
        .services
        .library
        .directory
        .tags()
        .await
        .map_err(app)?;
    paged(tags, uri, &params, &"tags")
}

/// `GET /v1/calendar/events` parameters.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventParams {
    /// The first local day (default a week ago).
    pub from: Option<NaiveDate>,
    /// The last local day (default a week ahead).
    pub to: Option<NaiveDate>,
    /// Page size, 1 to 100 (default 20).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous page.
    pub cursor: Option<String>,
}

/// Calendar events starting in the window, by start time.
pub async fn events(
    State(state): State<ApiState>,
    uri: Uri,
    Params(params): Params<EventParams>,
) -> ApiResult<Paged<EventView>> {
    let window = EventWindow {
        from: params.from,
        to: params.to,
    };
    let events = state
        .services
        .library
        .directory
        .events(window)
        .await
        .map_err(app)?;
    let paging = PageParams {
        limit: params.limit,
        cursor: params.cursor,
    };
    paged(events, uri, &paging, &("events", params.from, params.to))
}
