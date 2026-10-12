//! Privacy wiring (PRV-02): the app lock and the library status.
//!
//! - [`attach_privacy`] runs right after the settings, before the database
//!   opens: it manages the [`LibraryStatusCell`] the startup screen reads,
//!   and the [`LockService`] over the system clock, the device-owner
//!   authenticator and the current `privacy.appLock` settings, plus the
//!   wake and screen-lock observers. The settings events keep the service
//!   in step with later changes (`features/settings/events.rs`).
//! - [`library_ready`] runs once the database is open.

use std::sync::Arc;

use deino_core::lock::LockService;
use deino_core::settings::{AppLockSettings, SettingsService};
use deino_domain::SystemClock;
use tauri::{AppHandle, Manager};

use crate::app_lock::{DeviceOwnerAuthenticator, observe_wake, supported};
use crate::database::manager::EncryptionReport;
use crate::features::privacy::library::{LibraryStatus, LibraryStatusCell};

/// Manages the library status, the lock service and its wake observers.
/// Runs after [`super::attach_settings`].
pub(crate) async fn attach_privacy(app: &AppHandle) {
    app.manage(Arc::new(LibraryStatusCell::default()));
    let policy = match app.try_state::<Arc<SettingsService>>() {
        Some(settings) => supported(settings.get().await.privacy.app_lock),
        None => AppLockSettings::default(),
    };
    let lock = Arc::new(LockService::new(
        Arc::new(SystemClock),
        Arc::new(DeviceOwnerAuthenticator),
        policy,
    ));
    app.manage(observe_wake(Arc::clone(&lock)));
    app.manage(lock);
}

/// Reports the open database to the startup screen.
pub(crate) fn library_ready(app: &AppHandle, report: EncryptionReport) {
    let status = LibraryStatus::Ready {
        encrypted: report.encrypted,
        outcome: report.outcome,
    };
    // `attach_privacy` manages the cell before any database opens.
    if let Some(cell) = app.try_state::<Arc<LibraryStatusCell>>() {
        cell.set(status);
    }
}
