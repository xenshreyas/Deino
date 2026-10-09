//! Commit-time change signals from SQLite hooks.
//!
//! Every connection the notifier installs itself on records which tables a
//! transaction touches (`update_hook`), publishes them once when it commits
//! (`commit_hook`) and forgets them when it rolls back (`rollback_hook`). The
//! app forwards each [`TableChanges`] to the UI as a `DbChanged` event.
//!
//! Adapted from anarlog's `crates/db-change` (MIT, see `THIRD_PARTY_NOTICES.md`),
//! simplified to one signal per transaction and a typed [`Table`] set.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use sqlx::sqlite::{LockedSqliteHandle, SqlitePoolOptions};
use tokio::sync::broadcast;

use crate::Table;

/// Signals buffered for slow subscribers before they observe `Lagged`.
const CHANNEL_CAPACITY: usize = 256;

/// The tables one committed transaction changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableChanges {
    /// Every tracked table the transaction inserted into, updated or deleted from.
    pub tables: Vec<Table>,
    /// Increases by one per committed transaction that changed a tracked table.
    pub seq: u64,
}

/// Broadcasts a [`TableChanges`] for every committed write transaction.
///
/// Each [`Database`](crate::Database) owns one; clones share its channel and
/// its sequence.
///
/// Limits inherited from SQLite's update hook: writes from other processes,
/// `DELETE` statements without a `WHERE` clause (the truncate optimisation)
/// and rows replaced by `ON CONFLICT REPLACE` are not reported.
#[derive(Clone, Debug)]
pub struct ChangeNotifier {
    sender: broadcast::Sender<TableChanges>,
    seq: Arc<AtomicU64>,
}

impl ChangeNotifier {
    /// Creates a notifier with no subscribers.
    pub(crate) fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Receives every [`TableChanges`] published after this call.
    ///
    /// A receiver that falls more than `CHANNEL_CAPACITY` signals behind gets
    /// `RecvError::Lagged` and should assume every table changed.
    pub fn subscribe(&self) -> broadcast::Receiver<TableChanges> {
        self.sender.subscribe()
    }

    /// Installs the hooks on every connection `options` opens.
    ///
    /// Hooks must be in place before the first connection exists; a
    /// connection opened without them is invisible to subscribers.
    pub(crate) fn install(&self, options: SqlitePoolOptions) -> SqlitePoolOptions {
        let notifier = self.clone();
        options.after_connect(move |connection, _| {
            let notifier = notifier.clone();
            Box::pin(async move {
                notifier.hook(&mut connection.lock_handle().await?);
                Ok(())
            })
        })
    }

    /// Registers the update, commit and rollback hooks on one connection.
    fn hook(&self, handle: &mut LockedSqliteHandle<'_>) {
        let pending = Arc::new(PendingTables::default());
        let on_update = Arc::clone(&pending);
        handle.set_update_hook(move |update| on_update.record(update.table));
        let on_commit = Arc::clone(&pending);
        let notifier = self.clone();
        handle.set_commit_hook(move || {
            notifier.publish(on_commit.take());
            // Never veto: returning false would turn the commit into a rollback.
            true
        });
        handle.set_rollback_hook(move || pending.clear());
    }

    /// Sends one signal for a committed transaction's tables, if any.
    fn publish(&self, tables: BTreeSet<Table>) {
        if tables.is_empty() {
            return;
        }
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let changes = TableChanges {
            tables: tables.into_iter().collect(),
            seq,
        };
        // An error only means nobody is subscribed yet, which is normal at startup.
        if self.sender.send(changes).is_err() {
            log::trace!(target: "db", "no change subscribers for seq {seq}");
        }
    }
}

/// The tracked tables written by a connection's open transaction.
///
/// The hooks run on SQLite's thread for every row, so they stay cheap: no
/// allocation for untracked tables, no I/O and no async work.
#[derive(Debug, Default)]
struct PendingTables(Mutex<BTreeSet<Table>>);

impl PendingTables {
    fn record(&self, name: &str) {
        if let Some(table) = Table::from_name(name) {
            self.lock().insert(table);
        }
    }

    fn take(&self) -> BTreeSet<Table> {
        std::mem::take(&mut *self.lock())
    }

    fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeSet<Table>> {
        // The set holds no invariant a panicking writer could break.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
