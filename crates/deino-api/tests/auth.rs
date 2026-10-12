//! The guard: keys, scopes, the app lock and the `Host` check.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

mod support;

use std::sync::atomic::Ordering;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use deino_core::api_keys::ApiKeyScope;
use serde_json::json;
use support::Harness;
use tower::ServiceExt;

#[tokio::test]
async fn a_missing_or_malformed_key_is_401_with_the_error_envelope() {
    let api = Harness::new().await;

    let missing = api.call((Method::GET, "/v1/meetings"), None, None).await;
    let garbage = api.get("/v1/meetings", "deino_not_a_key").await;

    for reply in [missing, garbage] {
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.body["error"]["code"], "unauthorized");
        assert!(reply.body["error"]["message"].is_string());
    }
}

#[tokio::test]
async fn a_revoked_key_is_401() {
    let api = Harness::new().await;
    let token = api.token(ApiKeyScope::ReadWrite).await;
    assert_eq!(api.get("/v1/meetings", &token).await.status, StatusCode::OK);
    let key = api.services.keys.list().await.unwrap()[0].id;

    api.services.keys.revoke(&key).await.unwrap();

    let reply = api.get("/v1/meetings", &token).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_read_only_key_reads_but_every_write_is_403() {
    let api = Harness::new().await;
    let token = api.token(ApiKeyScope::Read).await;
    assert_eq!(api.get("/v1/tags", &token).await.status, StatusCode::OK);

    let writes = [
        (Method::POST, "/v1/tags", Some(json!({ "name": "New" }))),
        (
            Method::PUT,
            "/v1/meetings/meeting-fixture-standup",
            Some(json!({ "title": "x" })),
        ),
        (Method::DELETE, "/v1/meetings/meeting-fixture-standup", None),
        (
            Method::POST,
            "/v1/action-items",
            Some(json!({ "title": "x" })),
        ),
    ];
    for (method, uri, body) in writes {
        let reply = api.call((method.clone(), uri), Some(&token), body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert_eq!(reply.body["error"]["code"], "read_only_key");
    }
    let still_there = api
        .get("/v1/meetings/meeting-fixture-standup", &token)
        .await;
    assert_eq!(still_there.status, StatusCode::OK);
}

#[tokio::test]
async fn nothing_is_served_while_deino_is_locked() {
    let api = Harness::new().await;
    let token = api.writer().await;
    api.lock.0.store(true, Ordering::SeqCst);

    for uri in ["/v1/meetings", "/v1/search?q=pricing", "/v1/people"] {
        let reply = api.get(uri, &token).await;
        assert_eq!(reply.status, StatusCode::LOCKED, "{uri}");
        assert_eq!(reply.body["error"]["code"], "locked");
    }
    api.lock.0.store(false, Ordering::SeqCst);
    assert_eq!(api.get("/v1/meetings", &token).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_foreign_host_header_is_refused_before_the_key_is_checked() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let request = Request::builder()
        .uri("/v1/meetings")
        .header(header::HOST, "attacker.example")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = api.app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn auth_test_names_the_key_and_unknown_routes_are_404() {
    let api = Harness::new().await;
    let token = api.token(ApiKeyScope::Read).await;

    let whoami = api.get("/v1/auth/test", &token).await;
    let unknown = api.get("/v2/meetings", &token).await;

    assert_eq!(whoami.status, StatusCode::OK);
    assert_eq!(whoami.body["scope"], "read");
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body["error"]["code"], "not_found");
}
