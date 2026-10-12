//! Replays recorded API answers from `fixtures/`: a [`FixtureClient`] maps
//! each request (method, path and query, or a GraphQL `skip`) to a file, and
//! records what was asked, so tests never reach a real API.

#![allow(dead_code, reason = "each test binary uses part of the support module")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use bytes::Bytes;
use deino_core::importers::{ImportProgress, ImportWorld, Importer, NoImportEvents};
use deino_net::{Headers, NetClient, NetError, Request, Response, Status};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

/// A recorded answer.
#[derive(Clone)]
pub struct Recorded {
    status: u16,
    file: Option<&'static str>,
    headers: Vec<(&'static str, String)>,
}

impl Recorded {
    /// `200` with the body of `fixtures/<file>`.
    pub fn file(file: &'static str) -> Self {
        Self {
            status: 200,
            file: Some(file),
            headers: Vec::new(),
        }
    }

    /// `status` with the body of `fixtures/<file>`.
    pub fn status(status: u16, file: &'static str) -> Self {
        Self {
            status,
            ..Self::file(file)
        }
    }

    /// The same answer with a header.
    pub fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }
}

/// Answers from recorded files; 404 for anything not recorded.
#[derive(Default)]
pub struct FixtureClient {
    routes: Mutex<HashMap<String, Recorded>>,
    requests: Mutex<Vec<Request>>,
}

impl FixtureClient {
    /// A client answering `route` (`"GET /api/meetings?cursor=x"`, or
    /// `"POST /graphql skip=0"`) with `answer`.
    pub fn route(self, route: &str, answer: Recorded) -> Self {
        lock(&self.routes).insert(route.to_owned(), answer);
        self
    }

    /// Every request received, in order.
    pub fn requests(&self) -> Vec<Request> {
        lock(&self.requests).clone()
    }

    /// The route a request matches.
    fn key(request: &Request) -> String {
        let method = request.method.as_str();
        let path = request.url.path();
        if let Some(body) = &request.body {
            let json: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
            let skip = json
                .get("variables")
                .and_then(|variables| variables.get("skip"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            return format!("{method} {path} skip={skip}");
        }
        match request.url.query() {
            Some(query) => format!("{method} {path}?{query}"),
            None => format!("{method} {path}"),
        }
    }
}

#[async_trait]
impl NetClient for FixtureClient {
    async fn send(&self, request: Request) -> Result<Response, NetError> {
        let key = Self::key(&request);
        lock(&self.requests).push(request);
        let Some(answer) = lock(&self.routes).get(&key).cloned() else {
            return Ok(Response::empty(Status::NOT_FOUND));
        };
        let body = answer.file.map_or_else(Bytes::new, fixture);
        let mut headers = Headers::default();
        for (name, value) in answer.headers {
            headers.insert(name, value);
        }
        Ok(Response {
            status: Status::new(answer.status),
            headers,
            body: futures_util::stream::iter([Ok(body)]).boxed(),
        })
    }
}

/// The bytes of `fixtures/<file>`; empty (and so a decoding failure the
/// test reports) when it is missing.
pub fn fixture(file: &str) -> Bytes {
    Bytes::from(std::fs::read(fixtures_dir().join(file)).unwrap_or_default())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The fixtures folder.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Runs `importer` to the end in `world`.
pub async fn run(world: &ImportWorld, importer: &dyn Importer) -> ImportProgress {
    world
        .runner
        .run(importer, &CancellationToken::new(), &NoImportEvents)
        .await
}

/// The client as the importers take it.
pub fn shared(client: &Arc<FixtureClient>) -> Arc<dyn NetClient> {
    client.clone()
}

/// The speakers of the meeting titled `title`.
pub async fn speakers(world: &ImportWorld, title: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT s.label FROM speakers s JOIN meetings m ON m.id = s.meeting_id \
         WHERE m.title = ?1 ORDER BY s.label",
    )
    .bind(title)
    .fetch_all(world.database.reader())
    .await
    .unwrap_or_default()
}

/// The number of `table` rows for the meeting titled `title`.
pub async fn count_for(world: &ImportWorld, table: &str, title: &str) -> i64 {
    let sql = format!(
        "SELECT count(*) FROM {table} t JOIN meetings m ON m.id = t.meeting_id WHERE m.title = ?1"
    );
    sqlx::query_scalar(&sql)
        .bind(title)
        .fetch_one(world.database.reader())
        .await
        .unwrap_or(-1)
}
