use chrono::{DateTime, TimeDelta, TimeZone};
use deino_db::test_db;
use deino_domain::{AuthMethod, Clock, ConnectionStatus, FakeClock};

use super::*;
use crate::{Connection, ConnectionRepo, SqliteConnectionRepo};

struct Harness {
    repo: SqliteDeliveryRepo,
    connections: SqliteConnectionRepo,
    clock: FakeClock,
    connection: ConnectionId,
}

async fn harness() -> Harness {
    let database = test_db().await;
    let clock = FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap());
    let connections = SqliteConnectionRepo::new(database.clone());
    let now = clock.now();
    let connection = Connection {
        id: ConnectionId::generate(&clock),
        provider: IntegrationProvider::Slack,
        account_id: "T1".into(),
        account_label: "Acme".into(),
        auth_method: AuthMethod::OAuth,
        status: ConnectionStatus::Connected,
        scopes: Vec::new(),
        api_base_url: None,
        token_expires_at: None,
        connected_at: now,
        updated_at: now,
        last_used_at: None,
        last_error: None,
    };
    connections.upsert(&connection).await.unwrap();
    Harness {
        repo: SqliteDeliveryRepo::new(database),
        connections,
        clock,
        connection: connection.id,
    }
}

fn new_delivery(harness: &Harness, key: &str) -> NewDelivery {
    NewDelivery {
        id: DeliveryId::generate(&harness.clock),
        connection_id: harness.connection,
        provider: IntegrationProvider::Slack,
        capability: Capability::PostMessage,
        meeting_id: None,
        idempotency_key: key.into(),
        at: harness.clock.now(),
    }
}

#[tokio::test]
async fn one_row_per_key_however_often_it_begins() {
    let harness = harness().await;
    let first = new_delivery(&harness, "share:1");

    let begun = harness.repo.begin(&first).await.unwrap();
    let again = harness
        .repo
        .begin(&new_delivery(&harness, "share:1"))
        .await
        .unwrap();

    assert_eq!(begun.id, first.id);
    assert_eq!(again.id, first.id);
    assert_eq!(begun.status, DeliveryStatus::Pending);
    assert_eq!(harness.repo.recent(10).await.unwrap().len(), 1);
}

/// Claims `id` at `at`, counting `sending` rows older than ten minutes as stale.
async fn claim(harness: &Harness, id: DeliveryId, at: DateTime<Utc>) -> bool {
    let stale_before = at - TimeDelta::minutes(10);
    harness.repo.claim(id, stale_before, at).await.unwrap()
}

#[tokio::test]
async fn a_claim_is_exclusive_until_the_attempt_fails() {
    let harness = harness().await;
    let delivery = harness
        .repo
        .begin(&new_delivery(&harness, "k"))
        .await
        .unwrap();
    let now = harness.clock.now();
    let failure = DeliveryFailure {
        code: ErrorCode::Network,
        message: "could not reach Slack".into(),
    };

    assert!(claim(&harness, delivery.id, now).await);
    assert!(!claim(&harness, delivery.id, now).await);
    harness.repo.fail(delivery.id, &failure, now).await.unwrap();
    assert!(claim(&harness, delivery.id, now).await);

    let row = harness.repo.by_key("k").await.unwrap().unwrap();
    assert_eq!((row.status, row.attempts), (DeliveryStatus::Sending, 2));
    assert_eq!(row.error, Some(failure));
}

#[tokio::test]
async fn an_attempt_that_died_with_the_app_can_be_claimed_once_stale() {
    let harness = harness().await;
    let delivery = harness
        .repo
        .begin(&new_delivery(&harness, "k"))
        .await
        .unwrap();
    let now = harness.clock.now();
    assert!(claim(&harness, delivery.id, now).await);

    assert!(!claim(&harness, delivery.id, now + TimeDelta::minutes(9)).await);
    assert!(claim(&harness, delivery.id, now + TimeDelta::minutes(11)).await);
}

#[tokio::test]
async fn a_succeeded_delivery_keeps_its_reference_and_cannot_be_claimed() {
    let harness = harness().await;
    let delivery = harness
        .repo
        .begin(&new_delivery(&harness, "k"))
        .await
        .unwrap();
    let now = harness.clock.now();
    harness.repo.claim(delivery.id, now, now).await.unwrap();
    let external = ExternalRef {
        id: "1700000000.0001".into(),
        url: Some("https://acme.slack.com/archives/C1/p1".into()),
    };

    harness
        .repo
        .succeed(delivery.id, &external, now)
        .await
        .unwrap();

    let row = harness.repo.by_key("k").await.unwrap().unwrap();
    assert_eq!(row.status, DeliveryStatus::Succeeded);
    assert_eq!(row.external, Some(external));
    assert!(
        !harness
            .repo
            .claim(delivery.id, now + TimeDelta::days(1), now)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn disconnecting_keeps_the_log_and_newest_come_first() {
    let harness = harness().await;
    let older = harness
        .repo
        .begin(&new_delivery(&harness, "a"))
        .await
        .unwrap();
    harness.clock.advance(TimeDelta::seconds(1));
    let newer = harness
        .repo
        .begin(&new_delivery(&harness, "b"))
        .await
        .unwrap();

    harness
        .connections
        .delete(harness.connection)
        .await
        .unwrap();

    let log = harness.repo.recent(10).await.unwrap();
    let ids: Vec<DeliveryId> = log.iter().map(|delivery| delivery.id).collect();
    assert_eq!(ids, [newer.id, older.id]);
    assert!(log.iter().all(|delivery| delivery.connection_id.is_none()));
    assert_eq!(harness.repo.recent(1).await.unwrap().len(), 1);
}
