//! The in-memory egress log and its hand-off to the database.

use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use deino_db::test_db;
use deino_domain::ServiceId;

use super::*;

/// Generous: the persister only waits on an in-memory database.
const WAIT: Duration = Duration::from_secs(5);

fn entry(bytes_down: u64) -> EgressEntry {
    EgressEntry {
        at: Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap(),
        service: ServiceId::HfModels,
        host: "huggingface.co".to_owned(),
        method: Method::Get,
        status: Some(200),
        outcome: EgressOutcome::Sent,
        bytes_up: 0,
        bytes_down,
    }
}

#[tokio::test]
async fn the_ring_keeps_the_newest_entries_newest_first() {
    let log = EgressLog::default();
    for bytes in 0..(RING_CAPACITY as u64 + 2) {
        log.record(entry(bytes));
    }

    let recent = log.recent(3).await.unwrap();

    let sizes: Vec<u64> = recent.iter().map(|entry| entry.bytes_down).collect();
    let newest = RING_CAPACITY as u64 + 1;
    assert_eq!(sizes, [newest, newest - 1, newest - 2]);
    assert_eq!(log.recent(u32::MAX).await.unwrap().len(), RING_CAPACITY);
}

#[tokio::test]
async fn the_persister_writes_what_came_before_and_after_it_started() {
    let log = Arc::new(EgressLog::default());
    log.record(entry(1));
    let store = Arc::new(SqliteEgressLog::new(test_db().await));
    let persister = log.start_persisting();
    log.record(entry(2));
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(persister.run(store.clone()));

    let saved = tokio::time::timeout(WAIT, async {
        loop {
            let saved = store.recent(10).await.unwrap();
            if saved.len() == 2 {
                return saved;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    let sizes: Vec<u64> = saved.iter().map(|entry| entry.bytes_down).collect();
    assert_eq!(sizes, [2, 1]);
}
