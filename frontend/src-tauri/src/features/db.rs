//! `db` feature: tells the UI which tables changed, so cached queries refresh.
//!
//! Every commit on a database's writer produces a `TableChanges` signal
//! (`deino_db::ChangeNotifier`). [`forward_changes`] turns each one into the
//! typed event [`DbChanged`] (`plugin:db:db-changed`), and the UI invalidates
//! every TanStack query tagged with one of its tables (ADR 0008). The plugin
//! has events only, so it has no manifest entry and no capability.

use deino_db::{ChangeNotifier, Table, TableChanges};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;
use tokio::sync::broadcast::{Receiver, error::RecvError};

/// Plugin name, the prefix of its event names.
pub const NAME: &str = "db";

/// The tables a committed transaction changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type, Event)]
pub struct DbChanged {
    /// Every table whose cached queries are now stale.
    pub tables: Vec<Table>,
    /// The database's commit sequence number, increasing by one per commit.
    pub seq: u64,
}

/// The plugin's events, for mounting and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .events(tauri_specta::collect_events![DbChanged])
}

/// Builds the `db` plugin, which registers [`DbChanged`] with the app.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .setup(move |app, _| {
            builder.mount_events(app);
            Ok(())
        })
        .build()
}

/// Emits a [`DbChanged`] for every commit `changes` reports, for as long as
/// the app runs.
pub fn forward_changes<R: Runtime>(app: &AppHandle<R>, changes: &ChangeNotifier) {
    tauri::async_runtime::spawn(forward(app.clone(), changes.subscribe()));
}

async fn forward<R: Runtime>(app: AppHandle<R>, mut receiver: Receiver<TableChanges>) {
    let mut seq = 0;
    loop {
        let event = match receiver.recv().await {
            Ok(changes) => DbChanged {
                tables: changes.tables,
                seq: changes.seq,
            },
            // Signals were dropped: refresh everything rather than show stale data.
            Err(RecvError::Lagged(skipped)) => DbChanged {
                tables: Table::ALL.to_vec(),
                seq: seq + skipped,
            },
            Err(RecvError::Closed) => return,
        };
        seq = event.seq;
        if let Err(error) = event.emit(&app) {
            log::warn!(target: "db", "could not emit db-changed {seq}: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use deino_testkit::{at, seed, test_db};
    use serde_json::{Value, json};
    use tauri::Listener;
    use tauri::test::mock_builder;

    use crate::features::ipc_tests::mock_app;

    /// How long a test waits for an event that should already have been emitted.
    const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

    #[test]
    fn a_committed_write_reaches_the_ui_as_db_changed() {
        let app = mock_app(mock_builder().plugin(super::init()));
        let database = tauri::async_runtime::block_on(test_db());
        let (sender, received) = mpsc::channel();
        app.listen_any("plugin:db:db-changed", move |event| {
            sender
                .send(event.payload().to_owned())
                .expect("the test is still listening");
        });

        super::forward_changes(app.handle(), database.changes());
        let write = seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        );
        tauri::async_runtime::block_on(write);

        let payload: Value =
            serde_json::from_str(&received.recv_timeout(EVENT_TIMEOUT).unwrap()).unwrap();
        assert_eq!(payload, json!({ "tables": ["meetings"], "seq": 1 }));
    }
}
