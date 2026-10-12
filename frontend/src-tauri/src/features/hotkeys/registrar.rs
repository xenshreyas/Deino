//! The system's hotkeys through `tauri-plugin-global-shortcut`: the
//! `HotkeyRegistrar` port, and the handler that runs a pressed hotkey's
//! action whatever app is in front.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_core::hotkeys::{Accelerator, HotkeyAction, HotkeyRegistrar, RegistrarRefused};
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// Each action's registered shortcut.
#[derive(Debug, Default)]
pub(crate) struct ShortcutTable(Mutex<BTreeMap<HotkeyAction, Shortcut>>);

impl ShortcutTable {
    /// The action `shortcut` runs, if it is one of Deino's.
    pub(crate) fn action(&self, shortcut: &Shortcut) -> Option<HotkeyAction> {
        self.entries()
            .iter()
            .find(|(_, registered)| registered.id() == shortcut.id())
            .map(|(action, _)| *action)
    }

    fn entries(&self) -> MutexGuard<'_, BTreeMap<HotkeyAction, Shortcut>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Registers hotkeys with the global-shortcut plugin.
pub(crate) struct GlobalShortcutRegistrar<R: Runtime> {
    app: AppHandle<R>,
    table: Arc<ShortcutTable>,
}

impl<R: Runtime> GlobalShortcutRegistrar<R> {
    /// A registrar that records its shortcuts in `table`.
    pub(crate) fn new(app: AppHandle<R>, table: Arc<ShortcutTable>) -> Self {
        Self { app, table }
    }
}

impl<R: Runtime> HotkeyRegistrar for GlobalShortcutRegistrar<R> {
    fn register(
        &self,
        action: HotkeyAction,
        accelerator: &Accelerator,
    ) -> Result<(), RegistrarRefused> {
        let shortcut: Shortcut = accelerator.to_string().parse().map_err(|error| {
            log::warn!(target: "hotkeys", "the plugin cannot read {accelerator}: {error}");
            RegistrarRefused
        })?;
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| {
                log::info!(target: "hotkeys", "{accelerator} refused for {action:?}: {error}");
                RegistrarRefused
            })?;
        self.table.entries().insert(action, shortcut);
        Ok(())
    }

    fn unregister(&self, action: HotkeyAction) {
        let Some(shortcut) = self.table.entries().remove(&action) else {
            return;
        };
        if let Err(error) = self.app.global_shortcut().unregister(shortcut) {
            log::warn!(target: "hotkeys", "could not release the {action:?} hotkey: {error}");
        }
    }
}

/// The global-shortcut plugin's handler: on a press of one of Deino's
/// hotkeys, runs its action's intent through the link service.
pub(crate) fn on_shortcut<R: Runtime>(
    table: Arc<ShortcutTable>,
) -> impl Fn(&AppHandle<R>, &Shortcut, ShortcutEvent) + Send + Sync + 'static {
    move |app, shortcut, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        if let Some(action) = table.action(shortcut) {
            log::info!(target: "hotkeys", "{action:?} hotkey pressed");
            crate::features::links::adapters::perform(app, action.intent());
        }
    }
}
