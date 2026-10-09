//! Auto-update gate.
//!
//! Deino ships with automatic updates **disabled** (PARITY_PLAN §8 D6): new
//! builds are installed by hand from CI artifacts. The `tauri-plugin-updater`
//! crate stays compiled in so that turning updates back on is a small change:
//!
//! 1. Generate a minisign key pair with `pnpm tauri signer generate`.
//! 2. Add `plugins.updater.pubkey` and `plugins.updater.endpoints` to
//!    `tauri.conf.json`, and set `bundle.createUpdaterArtifacts` to `true`.
//! 3. Set [`AUTO_UPDATE_ENABLED`] to `true`.
//!
//! While the gate is off the updater plugin is not registered, the tray has no
//! "Check for Updates" item, and the UI learns through [`updates_enabled`] that
//! it must not check. The app therefore never contacts an update server.

use tauri::{Builder, Runtime};

/// Whether this build may check for, download and install updates.
///
/// Off until Deino has its own signing key and release host (§8 D6).
pub const AUTO_UPDATE_ENABLED: bool = false;

/// Registers the updater plugin on `builder` when updates are enabled.
///
/// The plugin refuses to start without a `plugins.updater` section in the
/// Tauri config, so it must stay unregistered while the gate is off.
pub fn register<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    if AUTO_UPDATE_ENABLED {
        builder.plugin(tauri_plugin_updater::Builder::new().build())
    } else {
        builder
    }
}

/// Tells the UI whether update checks are enabled in this build.
#[tauri::command]
pub fn updates_enabled() -> bool {
    AUTO_UPDATE_ENABLED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_are_disabled_by_default() {
        assert!(!updates_enabled());
    }
}
