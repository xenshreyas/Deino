//! The `/v1` routes and the router that serves them.
//!
//! | Route | Methods |
//! |---|---|
//! | `/v1/auth/test` | `GET`: which key is calling |
//! | `/v1/meetings` | `GET` (filters, cursor), `POST` (import a transcript) |
//! | `/v1/meetings/{id}` | `GET` (`include=transcript`), `PUT`, `DELETE` (to the trash) |
//! | `/v1/meetings/{id}/transcript` | `GET` (cursor) |
//! | `/v1/search` | `GET` |
//! | `/v1/action-items`, `/v1/action-items/{id}` | `GET`, `POST`; `PUT`, `DELETE` |
//! | `/v1/tags`, `/v1/tags/{id}` | `GET`, `POST`; `PUT`, `DELETE` |
//! | `/v1/people`, `/v1/companies`, `/v1/folders`, `/v1/calendar/events` | `GET` |

mod action_items;
mod directory;
mod meeting_writes;
mod meetings;
mod search;
mod tags;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, put};
use axum::{Extension, Json, Router, middleware};
use deino_core::api_keys::{ApiCaller, ApiKeyScope};
use deino_domain::ApiKeyId;
use serde::Serialize;

use crate::guard::guard;
use crate::{ApiError, ApiState};

/// The largest request body: a long meeting's transcript JSON.
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

/// Every route, behind the guard (host, key, scope, lock).
pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/v1/auth/test", get(auth_test))
        .route(
            "/v1/meetings",
            get(meetings::list).post(meeting_writes::create),
        )
        .route(
            "/v1/meetings/{id}",
            get(meetings::read)
                .put(meeting_writes::update)
                .delete(meeting_writes::delete),
        )
        .route("/v1/meetings/{id}/transcript", get(meetings::transcript))
        .route("/v1/search", get(search::search))
        .route(
            "/v1/action-items",
            get(action_items::list).post(action_items::create),
        )
        .route(
            "/v1/action-items/{id}",
            put(action_items::update).delete(action_items::delete),
        )
        .route("/v1/tags", get(directory::tags).post(tags::create))
        .route("/v1/tags/{id}", put(tags::update).delete(tags::delete))
        .route("/v1/people", get(directory::people))
        .route("/v1/companies", get(directory::companies))
        .route("/v1/folders", get(directory::folders))
        .route("/v1/calendar/events", get(directory::events))
        .route_layer(middleware::from_fn_with_state(state.clone(), guard))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .fallback(|| async { ApiError::not_found("no such route; every route starts with /v1") })
        .with_state(state)
}

/// Who is calling.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Whoami {
    /// The key's ID.
    pub key_id: ApiKeyId,
    /// `read` or `read_write`.
    pub scope: ApiKeyScope,
}

/// `GET /v1/auth/test`: checks a key, as Circleback's `/oauth/test` does.
async fn auth_test(Extension(caller): Extension<ApiCaller>) -> Json<Whoami> {
    Json(Whoami {
        key_id: caller.key_id,
        scope: caller.scope,
    })
}
