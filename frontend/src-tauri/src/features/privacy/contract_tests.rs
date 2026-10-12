//! Contract tests: the `privacy` commands' JSON, and the IPC guard every
//! feature plugin is registered behind, through the real plugins and ACL on
//! Tauri's mock runtime. The lock runs on a fake clock and a fake
//! authenticator, so no system prompt is ever shown.

use std::sync::Arc;

use deino_core::lock::{AuthError, FakeAuthenticator, LockService};
use deino_core::settings::AppLockSettings;
use deino_testkit::{FakeClock, at};
use serde_json::{Value, json};
use tauri::test::{MockRuntime, mock_builder};
use tauri::{App, Manager};

use super::guard::Guarded;
use super::library::LibraryStatusCell;
use crate::features::ipc_tests::{invoke, main_window, mock_app};
use crate::features::registry::access;
use crate::features::{meetings, recording, system};

/// An app with the `privacy`, `meetings`, `recording` and `system` plugins,
/// each behind the guard with the access it registered, and an enabled lock.
struct Harness {
    app: App<MockRuntime>,
    lock: Arc<LockService>,
    auth: Arc<FakeAuthenticator>,
}

fn harness() -> Harness {
    let app = mock_app(
        mock_builder()
            .plugin(Guarded::new(super::init(), access(super::NAME)))
            .plugin(Guarded::new(meetings::init(), access(meetings::NAME)))
            .plugin(Guarded::new(recording::init(), access(recording::NAME)))
            .plugin(Guarded::new(system::init(), access(system::NAME))),
    );
    let auth = Arc::new(FakeAuthenticator::default());
    let policy = AppLockSettings {
        enabled: true,
        ..AppLockSettings::default()
    };
    let clock = Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z")));
    let lock = Arc::new(LockService::new(clock, auth.clone(), policy));
    app.manage(Arc::clone(&lock));
    app.manage(Arc::new(LibraryStatusCell::default()));
    Harness { app, lock, auth }
}

impl Harness {
    fn call(&self, command: &str) -> Result<Value, Value> {
        invoke(&main_window(&self.app), command, json!({}))
    }
}

fn locked_error() -> Value {
    json!({ "code": "permission_denied", "message": "Deino is locked" })
}

#[test]
fn library_reads_are_refused_while_locked() {
    let h = harness();
    h.lock.lock_now();

    assert_eq!(h.call("plugin:meetings|list"), Err(locked_error()));
}

#[test]
fn library_reads_pass_the_guard_while_unlocked() {
    let h = harness();

    // No meeting service is managed here, so the command itself fails; what
    // matters is that the guard let it through.
    let answer = h.call("plugin:meetings|list");

    assert_ne!(answer, Err(locked_error()));
}

#[test]
fn recording_and_the_lock_screen_keep_working_while_locked() {
    let h = harness();
    h.lock.lock_now();

    assert_ne!(h.call("plugin:recording|state"), Err(locked_error()));
    assert!(h.call("plugin:system|app_version").is_ok());
    assert_eq!(h.call("plugin:privacy|state"), Ok(json!("locked")));
}

#[test]
fn unlocking_asks_the_authenticator_once() {
    let h = harness();
    h.lock.lock_now();

    assert_eq!(h.call("plugin:privacy|unlock"), Ok(json!("unlocked")));

    assert_eq!(h.auth.prompts(), 1);
    assert!(h.call("plugin:meetings|list") != Err(locked_error()));
}

#[test]
fn a_cancelled_unlock_is_a_typed_error_and_stays_locked() {
    let h = harness();
    h.lock.lock_now();
    h.auth.answer(Some(AuthError::Cancelled));

    let error = h.call("plugin:privacy|unlock").unwrap_err();

    assert_eq!(error["code"], json!("cancelled"));
    assert_eq!(h.call("plugin:privacy|touch"), Ok(json!("locked")));
}

#[test]
fn lock_and_library_have_their_shapes() {
    let h = harness();

    assert_eq!(h.call("plugin:privacy|lock"), Ok(json!("locked")));
    assert_eq!(
        h.call("plugin:privacy|library"),
        Ok(json!({ "kind": "opening" }))
    );
}
