//! The egress log: one [`EgressEntry`] per outgoing request, so the user can
//! see exactly what left the Mac (Settings → Privacy → Network).
//!
//! An entry records when, which service, which host, the method, the status
//! and how many bytes went each way. It has no field that could hold a path,
//! a query string, a header or a body, so it cannot leak a key or meeting
//! content even by mistake.
//!
//! [`EgressLog`] keeps the newest entries in memory (a ring buffer) and,
//! once [`EgressLog::start_persisting`] is called, hands each one to an
//! [`EgressPersister`] that writes batches through the [`EgressWriter`] port
//! ([`SqliteEgressLog`] in the app). Recording never blocks a request.

mod meter;
mod sqlite;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::ServiceId;
use serde::Serialize;
use tokio::sync::mpsc;

pub(crate) use meter::Exchange;
pub use sqlite::SqliteEgressLog;

use crate::Method;

/// Entries kept in memory.
pub const RING_CAPACITY: usize = 500;

/// Entries waiting for the persister before new ones are dropped from the
/// table (they stay in the ring).
const PERSIST_QUEUE: usize = 1024;

/// Entries written in one transaction.
const PERSIST_BATCH: usize = 64;

/// How a request ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum EgressOutcome {
    /// The request was sent and answered (any status).
    Sent,
    /// The connection failed or broke; some bytes may have left.
    Failed,
    /// Policy stopped it before anything was sent.
    Blocked,
}

/// One outgoing request: where it went and how big it was, never what it carried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EgressEntry {
    /// When the request started.
    pub at: DateTime<Utc>,
    /// The service it was made for.
    pub service: ServiceId,
    /// The host, with the port when it is not the scheme's default.
    pub host: String,
    /// The method.
    pub method: Method,
    /// The response status; `None` when no response arrived.
    pub status: Option<u16>,
    /// How it ended.
    pub outcome: EgressOutcome,
    /// Request body bytes handed to the connection.
    pub bytes_up: u64,
    /// Response body bytes received.
    pub bytes_down: u64,
}

/// Reads recent entries, newest first.
#[async_trait]
pub trait EgressReader: Send + Sync {
    /// At most `limit` entries, newest first.
    async fn recent(&self, limit: u32) -> Result<Vec<EgressEntry>, DbError>;
}

/// Stores entries durably.
#[async_trait]
pub trait EgressWriter: Send + Sync {
    /// Appends `entries`, oldest first, in one unit of work.
    async fn append(&self, entries: &[EgressEntry]) -> Result<(), DbError>;
}

/// The in-memory log, and the hand-off to an optional persister.
#[derive(Debug, Default)]
pub struct EgressLog {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    ring: VecDeque<EgressEntry>,
    persist: Option<mpsc::Sender<EgressEntry>>,
}

impl EgressLog {
    /// Records `entry`: in the ring, and for the persister if one runs.
    pub fn record(&self, entry: EgressEntry) {
        let mut state = self.lock();
        if let Some(sender) = &state.persist
            && sender.try_send(entry.clone()).is_err()
        {
            log::warn!(target: "net", "egress log persister is behind; an entry stays in memory only");
        }
        if state.ring.len() == RING_CAPACITY {
            state.ring.pop_front();
        }
        state.ring.push_back(entry);
    }

    /// Starts handing entries to a persister, seeded with every entry already
    /// in memory so nothing recorded before the database opened is lost.
    pub fn start_persisting(&self) -> EgressPersister {
        let (sender, receiver) = mpsc::channel(PERSIST_QUEUE);
        let mut state = self.lock();
        state.persist = Some(sender);
        EgressPersister {
            backlog: state.ring.iter().cloned().collect(),
            receiver,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl EgressReader for EgressLog {
    async fn recent(&self, limit: u32) -> Result<Vec<EgressEntry>, DbError> {
        let state = self.lock();
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        Ok(state.ring.iter().rev().take(limit).cloned().collect())
    }
}

/// Writes recorded entries to an [`EgressWriter`], in batches, in order.
#[derive(Debug)]
pub struct EgressPersister {
    backlog: Vec<EgressEntry>,
    receiver: mpsc::Receiver<EgressEntry>,
}

impl EgressPersister {
    /// Runs until the [`EgressLog`] is dropped. A failed write is logged and
    /// skipped: those entries remain visible in memory.
    pub async fn run(mut self, writer: Arc<dyn EgressWriter>) {
        let backlog = std::mem::take(&mut self.backlog);
        for batch in backlog.chunks(PERSIST_BATCH) {
            write(writer.as_ref(), batch).await;
        }
        while let Some(first) = self.receiver.recv().await {
            let mut batch = vec![first];
            while batch.len() < PERSIST_BATCH {
                let Ok(next) = self.receiver.try_recv() else {
                    break;
                };
                batch.push(next);
            }
            write(writer.as_ref(), &batch).await;
        }
    }
}

async fn write(writer: &dyn EgressWriter, batch: &[EgressEntry]) {
    if let Err(error) = writer.append(batch).await {
        log::warn!(target: "net", "could not save {} egress log entries: {error}", batch.len());
    }
}

#[cfg(test)]
mod egress_tests;
