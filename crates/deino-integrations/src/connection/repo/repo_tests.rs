use chrono::{TimeDelta, TimeZone};
use deino_db::test_db;
use deino_domain::{AppError, FakeClock};

use super::*;

fn clock() -> FakeClock {
    FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap())
}

fn connection(clock: &FakeClock, provider: IntegrationProvider, account: &str) -> Connection {
    let now = deino_domain::Clock::now(clock);
    Connection {
        id: ConnectionId::generate(clock),
        provider,
        account_id: account.into(),
        account_label: format!("{account} label"),
        auth_method: AuthMethod::OAuth,
        status: ConnectionStatus::Connected,
        scopes: vec!["chat:write".into(), "users:read".into()],
        api_base_url: None,
        token_expires_at: Some(now + TimeDelta::hours(1)),
        connected_at: now,
        updated_at: now,
        last_used_at: None,
        last_error: None,
    }
}

#[tokio::test]
async fn a_connection_round_trips_with_its_scopes() {
    let repo = SqliteConnectionRepo::new(test_db().await);
    let clock = clock();
    let slack = connection(&clock, IntegrationProvider::Slack, "T1");

    let id = repo.upsert(&slack).await.unwrap();

    assert_eq!(id, slack.id);
    assert_eq!(repo.connection(id).await.unwrap(), Some(slack.clone()));
    assert_eq!(
        repo.connections().await.unwrap(),
        std::slice::from_ref(&slack)
    );
    assert_eq!(
        repo.find(IntegrationProvider::Slack, "T1").await.unwrap(),
        Some(id)
    );
    assert_eq!(
        repo.find(IntegrationProvider::Notion, "T1").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn reconnecting_the_same_account_keeps_its_id_and_replaces_its_scopes() {
    let repo = SqliteConnectionRepo::new(test_db().await);
    let clock = clock();
    let first = connection(&clock, IntegrationProvider::Linear, "org-1");
    repo.upsert(&first).await.unwrap();
    clock.advance(TimeDelta::days(1));
    let again = Connection {
        account_label: "Renamed".into(),
        scopes: vec!["read".into()],
        connected_at: deino_domain::Clock::now(&clock),
        ..connection(&clock, IntegrationProvider::Linear, "org-1")
    };

    let id = repo.upsert(&again).await.unwrap();

    let stored = repo.connection(id).await.unwrap().unwrap();
    assert_eq!(id, first.id);
    assert_eq!(stored.account_label, "Renamed");
    assert_eq!(stored.scopes, ["read"]);
    assert_eq!(stored.connected_at, first.connected_at);
}

#[tokio::test]
async fn failures_are_recorded_until_a_request_works_again() {
    let repo = SqliteConnectionRepo::new(test_db().await);
    let clock = clock();
    let slack = connection(&clock, IntegrationProvider::Slack, "T1");
    repo.upsert(&slack).await.unwrap();
    let now = deino_domain::Clock::now(&clock);
    let error = ConnectionError::new(
        &AppError::new(ErrorCode::PermissionDenied, "sign in again"),
        now,
    );

    repo.record_failure(slack.id, ConnectionStatus::NeedsReconnect, &error)
        .await
        .unwrap();
    let failed = repo.connection(slack.id).await.unwrap().unwrap();
    repo.record_success(slack.id, now).await.unwrap();
    let healthy = repo.connection(slack.id).await.unwrap().unwrap();

    assert_eq!(failed.status, ConnectionStatus::NeedsReconnect);
    assert_eq!(failed.last_error, Some(error));
    assert_eq!(healthy.status, ConnectionStatus::Connected);
    assert_eq!(healthy.last_error, None);
    assert_eq!(healthy.last_used_at, Some(now));
}

#[tokio::test]
async fn deleting_a_connection_removes_its_scopes() {
    let database = test_db().await;
    let repo = SqliteConnectionRepo::new(database.clone());
    let clock = clock();
    let slack = connection(&clock, IntegrationProvider::Slack, "T1");
    repo.upsert(&slack).await.unwrap();

    assert!(repo.delete(slack.id).await.unwrap());
    assert!(!repo.delete(slack.id).await.unwrap());

    let scopes: i64 = sqlx::query_scalar("SELECT count(*) FROM integration_connection_scopes")
        .fetch_one(database.reader())
        .await
        .unwrap();
    assert_eq!(scopes, 0);
}

#[tokio::test]
async fn the_schema_refuses_an_unknown_provider_and_a_half_recorded_error() {
    let database = test_db().await;
    let insert = |provider: &str, code: Option<&str>| {
        format!(
            "INSERT INTO integration_connections (id, provider, account_id, account_label, auth_method,
             status, connected_at, updated_at, last_error_code)
             VALUES ('00000000-0000-7000-8000-000000000000', '{provider}', 'a', 'a', 'oauth',
             'connected', 'x', 'x', {})",
            code.map_or("NULL".into(), |code| format!("'{code}'"))
        )
    };

    let unknown = sqlx::query(&insert("myspace", None))
        .execute(database.writer())
        .await;
    let half = sqlx::query(&insert("slack", Some("network")))
        .execute(database.writer())
        .await;

    assert!(unknown.is_err());
    assert!(half.is_err());
}

#[tokio::test]
async fn listing_connections_reads_the_list_index() {
    let database = test_db().await;
    let plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(
        "EXPLAIN QUERY PLAN SELECT id FROM integration_connections ORDER BY provider, connected_at, id",
    )
    .fetch_all(database.reader())
    .await
    .unwrap();

    let details: Vec<&str> = plan.iter().map(|(.., detail)| detail.as_str()).collect();
    assert!(
        details
            .iter()
            .any(|step| step.contains("idx_integration_connections_list")),
        "{details:?}"
    );
}
