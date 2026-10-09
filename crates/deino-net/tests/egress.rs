//! End to end over a real socket: a scoped client talks to a local Ollama
//! stand-in through the real HTTP client, and the request lands in the
//! `egress_log` table with its host and sizes and nothing else.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/server.rs"]
mod server;

use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use deino_db::test_db;
use deino_domain::{FakeClock, ServiceId};
use deino_net::{
    EgressLog, EgressOutcome, EgressReader, FakeServicePolicy, Method, Net, NetClient, NetError,
    NetParts, Request, ReqwestNetClient, SqliteEgressLog,
};
use server::{Reply, Server};

/// Generous: the persister only waits on an in-memory database.
const WAIT: Duration = Duration::from_secs(5);

fn net(policy: Arc<FakeServicePolicy>, egress: Arc<EgressLog>) -> Net {
    let clock = FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
    Net::new(NetParts {
        client: Arc::new(ReqwestNetClient::new().unwrap()),
        policy,
        egress,
        clock: Arc::new(clock),
    })
}

async fn saved(store: &SqliteEgressLog, count: usize) -> Vec<deino_net::EgressEntry> {
    tokio::time::timeout(WAIT, async {
        loop {
            let rows = store.recent(10).await.unwrap();
            if rows.len() >= count {
                return rows;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn a_request_reaches_the_server_and_its_row_lands_in_the_egress_log() {
    let server = Server::start(vec![Reply::new(200, br#"{"models":[]}"#)]).await;
    let egress = Arc::new(EgressLog::default());
    let store = Arc::new(SqliteEgressLog::new(test_db().await));
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(egress.start_persisting().run(store.clone()));
    let ollama = net(Arc::new(FakeServicePolicy::default()), egress).scoped(ServiceId::LlmOllama);

    let url = format!("{}/api/tags?secret=sk-123", server.base);
    let body = ollama.fetch(Request::get(&url).unwrap()).await.unwrap();

    assert_eq!(&body[..], br#"{"models":[]}"#);
    let rows = saved(&store, 1).await;
    let host = server.base.trim_start_matches("http://");
    assert_eq!(rows[0].host, host);
    assert_eq!(rows[0].service, ServiceId::LlmOllama);
    assert_eq!(rows[0].method, Method::Get);
    assert_eq!(
        (rows[0].status, rows[0].outcome),
        (Some(200), EgressOutcome::Sent)
    );
    assert_eq!(rows[0].bytes_down, body.len() as u64);
    assert!(server.requests()[0].starts_with("get /api/tags?secret=sk-123 "));
}

#[tokio::test]
async fn offline_mode_sends_nothing_and_logs_the_block() {
    let server = Server::start(Vec::new()).await;
    let policy = Arc::new(FakeServicePolicy::default());
    policy.set_offline(true);
    let egress = Arc::new(EgressLog::default());
    let ollama = net(policy, egress.clone()).scoped(ServiceId::LlmOllama);

    let url = format!("{}/api/tags", server.base);
    let result = ollama.send(Request::get(&url).unwrap()).await;

    assert!(matches!(
        result,
        Err(NetError::Offline(ServiceId::LlmOllama))
    ));
    assert!(server.requests().is_empty());
    let logged = egress.recent(10).await.unwrap();
    assert_eq!(logged[0].outcome, EgressOutcome::Blocked);
}
