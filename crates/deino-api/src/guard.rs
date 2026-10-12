//! The one middleware every `/v1` request passes, in order:
//!
//! 1. **Host:** only `127.0.0.1`, `localhost` or `[::1]`, so a web page
//!    that rebinds its own domain to 127.0.0.1 cannot reach the API.
//! 2. **Key:** `Authorization: Bearer deino_…` names a live key (401).
//! 3. **Scope:** a read-only key may only `GET` (403 `read_only_key`).
//! 4. **Lock:** nothing is served while Deino is locked (423).
//!
//! Then it logs the method, the route (never the query string or a body),
//! the status and the key's ID.

use axum::extract::{MatchedPath, Request, State};
use axum::http::header::{AUTHORIZATION, HOST};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use deino_core::api_keys::ApiCaller;

use crate::{ApiError, ApiState};

/// Checks the request and, when it passes, runs the handler with the
/// caller in the request's extensions.
pub async fn guard(State(state): State<ApiState>, mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |path| path.as_str().to_owned());
    let caller = match admit(&state, request.method(), request.headers()).await {
        Ok(caller) => caller,
        Err(error) => {
            log::info!(target: "api", "{method} {route} -> {} ({})", error.status.as_u16(), error.code);
            return error.into_response();
        }
    };
    request.extensions_mut().insert(caller);
    let response = next.run(request).await;
    log::info!(
        target: "api",
        "{method} {route} -> {} (key {})",
        response.status().as_u16(),
        caller.key_id
    );
    response
}

async fn admit(
    state: &ApiState,
    method: &Method,
    headers: &HeaderMap,
) -> Result<ApiCaller, ApiError> {
    if !loopback_host(headers, state.port) {
        return Err(ApiError::forbidden_host());
    }
    let token = bearer(headers).ok_or_else(ApiError::unauthorized)?;
    let caller = state
        .services
        .keys
        .authenticate(token)
        .await
        .map_err(|_| ApiError::unauthorized())?;
    if !reads_only(method) && !caller.scope.can_write() {
        return Err(ApiError::read_only());
    }
    if state.services.lock.is_locked() {
        return Err(ApiError::locked());
    }
    Ok(caller)
}

/// The bearer token of the `Authorization` header.
fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

fn reads_only(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD)
}

/// True when the `Host` header names this Mac's loopback address (and,
/// when the server knows it, its port).
fn loopback_host(headers: &HeaderMap, port: Option<u16>) -> bool {
    let Some(host) = headers.get(HOST).and_then(|host| host.to_str().ok()) else {
        return false;
    };
    let (name, given_port) = match host.rsplit_once(':') {
        Some((name, port)) if !name.ends_with(':') && !port.contains(']') => (name, Some(port)),
        _ => (host, None),
    };
    let named = matches!(name, "127.0.0.1" | "localhost" | "[::1]");
    let ported = match (port, given_port) {
        (Some(port), Some(given)) => given.parse() == Ok(port),
        (Some(_), None) => false,
        (None, _) => true,
    };
    named && ported
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn host(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn only_loopback_hosts_on_the_server_port_pass() {
        for ok in ["127.0.0.1:4000", "localhost:4000", "[::1]:4000"] {
            assert!(loopback_host(&host(ok), Some(4000)), "{ok}");
        }
        for refused in ["evil.example:4000", "127.0.0.1:5000", "127.0.0.1", "10.0.0.2:4000"] {
            assert!(!loopback_host(&host(refused), Some(4000)), "{refused}");
        }
        assert!(!loopback_host(&HeaderMap::new(), Some(4000)));
    }

    #[test]
    fn the_bearer_scheme_is_read_case_insensitively() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("bearer deino_x_y"));
        assert_eq!(bearer(&headers), Some("deino_x_y"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(bearer(&headers), None);
    }
}
