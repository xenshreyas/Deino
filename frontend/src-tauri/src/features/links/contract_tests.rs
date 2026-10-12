//! Contract tests: `plugin:links|take`'s JSON, through the real plugin, the
//! app lock's guard and the app-lock adapter, on Tauri's mock runtime.

use std::sync::Arc;

use deino_core::links::{FakeRecorder, FakeShell, LinkService};
use deino_core::lock::{FakeAuthenticator, LockService};
use deino_core::settings::AppLockSettings;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::async_runtime::block_on;
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager};

use super::adapters::AppLock;
use crate::features::ipc_tests::{invoke, main_window, mock_app};
use crate::features::privacy::guard::Guarded;
use crate::features::registry::access;

struct Harness {
    app: App<MockRuntime>,
    lock: Arc<LockService>,
    links: Arc<LinkService>,
}

fn harness() -> Harness {
    let app = mock_app(mock_builder().plugin(Guarded::new(super::init(), access(super::NAME))));
    let policy = AppLockSettings {
        enabled: true,
        ..AppLockSettings::default()
    };
    let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let auth = Arc::new(FakeAuthenticator::default());
    let lock = Arc::new(LockService::new(clock, auth, policy));
    app.manage(Arc::clone(&lock));
    let links = Arc::new(LinkService::new(
        Arc::new(FakeRecorder::default()),
        Arc::new(AppLock(app.handle().clone())),
        Arc::new(FakeShell::default()),
    ));
    app.manage(Arc::clone(&links));
    Harness { app, lock, links }
}

impl Harness {
    fn take(&self) -> Result<Value, Value> {
        invoke(&main_window(&self.app), "plugin:links|take", json!({}))
    }

    fn open(&self, link: &str) {
        block_on(self.links.open(link)).unwrap();
    }
}

#[test]
fn take_returns_each_navigation_as_tagged_json() {
    let h = harness();
    h.open("deino://meeting/meeting-fixture-standup?t=231259");
    h.open("deino://new-note");
    h.open("deino://search?q=release%20cut");
    h.open("deino://recipe?data=abc");

    assert_eq!(
        h.take(),
        Ok(json!([
            { "kind": "meeting", "meetingId": "meeting-fixture-standup", "atMs": 231259 },
            { "kind": "new_note", "eventId": null },
            { "kind": "search", "query": "release cut" },
            { "kind": "recipe" },
        ]))
    );
    assert_eq!(h.take(), Ok(json!([])), "taking empties the queue");
}

#[test]
fn while_locked_the_guard_refuses_take_and_the_link_waits() {
    let h = harness();
    h.lock.lock_now();
    h.open("deino://meeting/meeting-fixture-standup");

    assert_eq!(
        h.take(),
        Err(json!({ "code": "permission_denied", "message": "Deino is locked" }))
    );
    assert!(
        h.links.take().is_empty(),
        "the service hands out nothing either"
    );

    block_on(h.lock.unlock("test")).unwrap();
    assert_eq!(
        h.take(),
        Ok(json!([{ "kind": "meeting", "meetingId": "meeting-fixture-standup", "atMs": null }]))
    );
}
