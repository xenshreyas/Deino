//! System-wide hotkeys (DEV-05): `tauri-plugin-global-shortcut`, whose
//! handler runs a pressed hotkey's action through the link service, and
//! the [`HotkeyService`] that registers the saved hotkeys.
//!
//! Runs after `links` (order 70), whose service the handler uses. A
//! hotkey that cannot be registered is reported in Settings → Shortcuts;
//! it never stops the app from starting.

use std::error::Error;
use std::sync::Arc;

use deino_core::hotkeys::HotkeyService;
use deino_core::settings::SettingsService;
use tauri::async_runtime::block_on;
use tauri::{AppHandle, Manager};

use super::attach::Phase;
use crate::features::hotkeys::registrar::{GlobalShortcutRegistrar, ShortcutTable, on_shortcut};

super::attach::attach!("hotkeys", 71, Phase::Startup(attach_hotkeys));

/// Registers the global-shortcut plugin and the saved hotkeys.
fn attach_hotkeys(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|state| Arc::clone(&state))
    else {
        log::warn!(target: "hotkeys", "no settings service; hotkeys stay off");
        return Ok(());
    };
    let table = Arc::new(ShortcutTable::default());
    let plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(on_shortcut(Arc::clone(&table)))
        .build();
    app.plugin(plugin)?;
    let registrar = Arc::new(GlobalShortcutRegistrar::new(app.clone(), table));
    let service = Arc::new(HotkeyService::new(registrar, settings));
    let hotkeys = block_on(service.start());
    let active = hotkeys
        .iter()
        .filter(|hotkey| hotkey.state == deino_core::hotkeys::HotkeyState::Active)
        .count();
    log::info!(target: "hotkeys", "{active} of {} hotkeys registered", hotkeys.len());
    app.manage(service);
    Ok(())
}
