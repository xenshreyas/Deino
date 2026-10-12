//! Wiring for settings and Keychain secrets (FND-06), which need no database
//! and which everything else reads, so they are attached first.

use std::sync::Arc;

use deino_core::settings::{
    JsonFileSettingsStore, LegacyFileImport, LegacySqliteImport, SettingsError, SettingsService,
};
use deino_secrets::{KeychainSecretStore, SecretStore};
use tauri::{AppHandle, Manager, Runtime};

use crate::features::settings::TauriSettingsEvents;
use crate::paths::AppPaths;

/// Loads the settings, moves the legacy JSON stores into them, opens the
/// Keychain, and manages `Arc<SettingsService>` and `Arc<dyn SecretStore>`.
///
/// Runs first at startup. Keychain items carry the bundle identifier as
/// their service (`com.xenshreyas.deino`), so a build with another
/// identifier never sees this app's keys.
pub(crate) async fn attach_settings(
    app: &AppHandle,
    paths: &AppPaths,
) -> Result<(), SettingsError> {
    let store = Arc::new(JsonFileSettingsStore::new(paths.settings_file()));
    let events = Arc::new(TauriSettingsEvents::new(app.clone()));
    let settings = Arc::new(SettingsService::load(store, events).await?);
    // A store that cannot be imported keeps its defaults; it never blocks startup.
    if let Err(error) = LegacyFileImport::new(paths.data()).run(&settings).await {
        log::warn!(target: "settings", "could not import the legacy settings files: {error}");
    }
    let secrets: Arc<dyn SecretStore> =
        Arc::new(KeychainSecretStore::new(app.config().identifier.clone()));
    app.manage(settings);
    app.manage(secrets);
    Ok(())
}

/// The data migration that moves the legacy SQLite settings and keys into
/// the settings file and the Keychain before their tables are dropped.
/// `None` until [`attach_settings`] has run.
pub(crate) fn legacy_settings_import<R: Runtime>(app: &AppHandle<R>) -> Option<LegacySqliteImport> {
    let settings = app.try_state::<Arc<SettingsService>>()?;
    let secrets = app.try_state::<Arc<dyn SecretStore>>()?;
    Some(LegacySqliteImport::new(
        Arc::clone(&settings),
        Arc::clone(&secrets),
    ))
}
