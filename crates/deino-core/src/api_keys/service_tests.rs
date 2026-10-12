//! [`ApiKeyService`] over the real schema, argon2 and the OS random source.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{AppError, ErrorCode, FakeClock};
use deino_testkit::{at, test_db};

use super::*;
use crate::api_keys::{ApiKeyScope, Argon2Hasher, OsTokens, SqliteApiKeyRepo};

struct World {
    service: ApiKeyService,
    clock: Arc<FakeClock>,
}

async fn world() -> World {
    let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let service = ApiKeyService::new(ApiKeyDeps {
        repo: Arc::new(SqliteApiKeyRepo::new(test_db().await)),
        hasher: Arc::new(Argon2Hasher),
        tokens: Arc::new(OsTokens),
        clock: clock.clone(),
    });
    World { service, clock }
}

fn new_key(name: &str, scope: ApiKeyScope) -> NewApiKey {
    NewApiKey {
        name: name.into(),
        scope,
    }
}

#[tokio::test]
async fn a_new_key_is_listed_without_its_secret_and_its_token_authenticates() {
    let World { service, .. } = world().await;

    let created = service
        .create(&new_key("  Raycast  ", ApiKeyScope::Read))
        .await
        .unwrap();

    assert_eq!(created.key.name, "Raycast");
    let listed = service.list().await.unwrap();
    assert_eq!(listed, vec![created.key.clone()]);
    let caller = service.authenticate(&created.token).await.unwrap();
    assert_eq!(caller.key_id, created.key.id);
    assert_eq!(caller.scope, ApiKeyScope::Read);
}

#[tokio::test]
async fn a_wrong_unknown_or_malformed_token_is_unauthorized() {
    let World { service, .. } = world().await;
    let created = service
        .create(&new_key("Script", ApiKeyScope::ReadWrite))
        .await
        .unwrap();
    // Change the last character to one it is not, so the secret really differs.
    let (head, last) = created.token.split_at(created.token.len() - 1);
    let wrong_secret = format!("{head}{}", if last == "0" { "1" } else { "0" });
    let other = service
        .create(&new_key("Other", ApiKeyScope::Read))
        .await
        .unwrap();
    let swapped = created.token.replace(
        &created.key.id.as_uuid().simple().to_string(),
        &other.key.id.as_uuid().simple().to_string(),
    );

    for token in [wrong_secret.as_str(), swapped.as_str(), "", "deino_x_y"] {
        let error = service.authenticate(token).await.unwrap_err();
        assert!(matches!(error, ApiKeyError::Unauthorized), "{token}");
    }
}

#[tokio::test]
async fn a_revoked_key_is_refused_but_still_listed() {
    let World { service, clock } = world().await;
    let created = service
        .create(&new_key("Zapier", ApiKeyScope::Read))
        .await
        .unwrap();
    clock.advance(TimeDelta::minutes(5));

    let revoked = service.revoke(&created.key.id).await.unwrap();

    assert_eq!(revoked.revoked_at, Some(at("2026-10-12T09:05:00Z")));
    let error = service.authenticate(&created.token).await.unwrap_err();
    assert!(matches!(error, ApiKeyError::Unauthorized));
    assert_eq!(service.list().await.unwrap(), vec![revoked]);
}

#[tokio::test]
async fn use_is_recorded_at_most_once_a_minute() {
    let World { service, clock } = world().await;
    let created = service
        .create(&new_key("Cron", ApiKeyScope::Read))
        .await
        .unwrap();

    service.authenticate(&created.token).await.unwrap();
    clock.advance(TimeDelta::seconds(30));
    service.authenticate(&created.token).await.unwrap();
    let first = service.list().await.unwrap()[0].last_used_at;
    clock.advance(TimeDelta::seconds(31));
    service.authenticate(&created.token).await.unwrap();
    let second = service.list().await.unwrap()[0].last_used_at;

    assert_eq!(first, Some(at("2026-10-12T09:00:00Z")));
    assert_eq!(second, Some(at("2026-10-12T09:01:01Z")));
}

#[tokio::test]
async fn names_are_checked_and_missing_keys_are_not_found() {
    let World { service, .. } = world().await;
    let long = "x".repeat(61);

    for name in ["", "   ", long.as_str()] {
        let error = service
            .create(&new_key(name, ApiKeyScope::Read))
            .await
            .unwrap_err();
        assert_eq!(AppError::from(error).code, ErrorCode::Invalid);
    }
    let created = service
        .create(&new_key("Gone", ApiKeyScope::Read))
        .await
        .unwrap();
    service.delete(&created.key.id).await.unwrap();
    let error = service.revoke(&created.key.id).await.unwrap_err();
    assert_eq!(AppError::from(error).code, ErrorCode::NotFound);
    assert!(service.delete(&created.key.id).await.is_err());
    assert!(service.authenticate(&created.token).await.is_err());
}
