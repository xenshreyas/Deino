//! Cursor pagination, as Circleback's API does it (C §12.1): a list
//! answers `{"data": [...], "nextCursor": "..."}` plus an RFC 8288
//! `Link: <...&cursor=...>; rel="next"` header, and `nextCursor` is `null`
//! on the last page.
//!
//! Cursors are opaque (base64url JSON) and tied to the filters they came
//! from: a cursor sent with other filters is refused rather than silently
//! paging a different list. Meetings page by key (the last meeting's place
//! in the list), so new or trashed meetings never repeat or skip one; the
//! smaller lists page by position.

use std::hash::{DefaultHasher, Hash, Hasher};

use axum::Json;
use axum::http::header::LINK;
use axum::http::{HeaderValue, Uri};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::ApiError;

/// The most items a page holds when the caller does not say.
pub const DEFAULT_PAGE: u32 = 20;
/// The most items one page holds.
pub const MAX_PAGE: u32 = 100;

/// The page size a caller asked for, defaulted and clamped.
pub fn page_size(limit: Option<u32>) -> u32 {
    limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE)
}

/// What a cursor carries: where to continue, and which filters it is for.
#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    /// A fingerprint of the filters.
    f: u64,
    /// Where to continue.
    at: T,
}

/// A fingerprint of `filters`, to tie a cursor to them.
pub fn fingerprint(filters: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    filters.hash(&mut hasher);
    hasher.finish()
}

/// The cursor for position `at` of the list `filters` select.
pub fn encode<T: Serialize>(at: &T, filters: u64) -> String {
    let envelope = Envelope { f: filters, at };
    let json = serde_json::to_vec(&envelope).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(json)
}

/// The position `cursor` names, if it came from a list with `filters`.
pub fn decode<T: DeserializeOwned>(cursor: &str, filters: u64) -> Result<T, ApiError> {
    let invalid = || ApiError::invalid("the cursor is not valid for this list and its filters");
    let json = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
    let envelope: Envelope<T> = serde_json::from_slice(&json).map_err(|_| invalid())?;
    if envelope.f == filters {
        Ok(envelope.at)
    } else {
        Err(invalid())
    }
}

/// One page of a list.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    /// The items.
    pub data: Vec<T>,
    /// The cursor of the next page; `null` on the last one.
    pub next_cursor: Option<String>,
}

/// A page answered with its `Link` header, for the request at `uri`.
pub struct Paged<T> {
    /// The page.
    pub page: Page<T>,
    /// The request's URI, for the next page's link.
    pub uri: Uri,
}

impl<T: Serialize> IntoResponse for Paged<T> {
    fn into_response(self) -> Response {
        let link = self.page.next_cursor.as_deref().and_then(|cursor| {
            HeaderValue::from_str(&format!("<{}>; rel=\"next\"", next_link(&self.uri, cursor))).ok()
        });
        let mut response = Json(self.page).into_response();
        if let Some(link) = link {
            response.headers_mut().insert(LINK, link);
        }
        response
    }
}

/// The request's path and query with `cursor` in place of its own.
fn next_link(uri: &Uri, cursor: &str) -> String {
    let mut pairs: Vec<&str> = uri
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty() && !pair.starts_with("cursor="))
        .collect();
    let cursor = format!("cursor={cursor}");
    pairs.push(&cursor);
    format!("{}?{}", uri.path(), pairs.join("&"))
}

/// Slices `items` (the whole list, in order) at the position `cursor` names.
pub fn by_position<T>(
    items: Vec<T>,
    cursor: Option<&str>,
    (limit, filters): (u32, u64),
) -> Result<Page<T>, ApiError> {
    let start: usize = cursor.map_or(Ok(0), |cursor| decode(cursor, filters))?;
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let end = start.saturating_add(limit);
    let next_cursor = (items.len() > end).then(|| encode(&end, filters));
    Ok(Page {
        data: items.into_iter().skip(start).take(limit).collect(),
        next_cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_round_trips_only_with_its_own_filters() {
        let filters = fingerprint(&("tag", "Customer"));
        let cursor = encode(&40_usize, filters);

        assert_eq!(decode::<usize>(&cursor, filters), Ok(40));
        assert!(decode::<usize>(&cursor, fingerprint(&("tag", "Hiring"))).is_err());
        assert!(decode::<usize>("not a cursor!", filters).is_err());
    }

    #[test]
    fn positions_page_through_the_whole_list() {
        let items: Vec<u32> = (0..5).collect();
        let first = by_position(items.clone(), None, (2, 7)).unwrap();
        assert_eq!(first.data, vec![0, 1]);
        let next = first.next_cursor.unwrap();
        let second = by_position(items.clone(), Some(&next), (2, 7)).unwrap();
        assert_eq!(second.data, vec![2, 3]);
        let last = by_position(items, second.next_cursor.as_deref(), (2, 7)).unwrap();
        assert_eq!((last.data, last.next_cursor), (vec![4], None));
    }

    #[test]
    fn the_next_link_keeps_the_filters_and_replaces_the_cursor() {
        let uri: Uri = "/v1/meetings?tag=Customer&cursor=old&limit=5"
            .parse()
            .unwrap();
        assert_eq!(
            next_link(&uri, "new"),
            "/v1/meetings?tag=Customer&limit=5&cursor=new"
        );
    }
}
