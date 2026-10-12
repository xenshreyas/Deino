//! The app lock's OS adapters (PRV-02): the `Authenticator` behind
//! `deino_core::lock::LockService`, and the observers that lock the app
//! when the Mac wakes or its screen locks.
//!
//! On macOS both use `LocalAuthentication` and `NSWorkspace` through objc2
//! (`app_lock/macos.rs`); elsewhere unlocking is unavailable and nothing is
//! observed (`app_lock/other.rs`).

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod other;

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::lock::{AuthError, Authenticator, LockService};
use deino_core::settings::AppLockSettings;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(target_os = "macos"))]
use other as platform;

/// Asks the Mac's owner with Touch ID, falling back to the Mac's password
/// (`LAPolicyDeviceOwnerAuthentication`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeviceOwnerAuthenticator;

#[async_trait]
impl Authenticator for DeviceOwnerAuthenticator {
    async fn authenticate(&self, reason: &str) -> Result<(), AuthError> {
        let reason = reason.to_owned();
        // The system prompt blocks its caller until the owner answers.
        tauri::async_runtime::spawn_blocking(move || platform::authenticate(&reason))
            .await
            .unwrap_or(Err(AuthError::Failed))
    }
}

/// `policy` as this platform can honour it: without a way to unlock (off
/// macOS) the lock never engages, so it can never shut the owner out.
pub(crate) fn supported(policy: AppLockSettings) -> AppLockSettings {
    AppLockSettings {
        enabled: policy.enabled && platform::CAN_AUTHENTICATE,
        ..policy
    }
}

/// The registered wake observers; they stay registered while this lives
/// (it is managed in Tauri state for the app's lifetime).
pub(crate) struct WakeObservers {
    _observers: platform::Observers,
}

/// Registers observers that tell `lock` when the Mac wakes or its screen
/// locks.
pub(crate) fn observe_wake(lock: Arc<LockService>) -> WakeObservers {
    WakeObservers {
        _observers: platform::observe_wake(lock),
    }
}
