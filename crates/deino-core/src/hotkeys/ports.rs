//! [`HotkeyRegistrar`]: the system's global hotkeys, as the app wires them
//! (`tauri-plugin-global-shortcut`).

use super::accelerator::Accelerator;
use super::model::HotkeyAction;

/// The system refused a hotkey, most likely because another app holds it.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the system refused the hotkey")]
pub struct RegistrarRefused;

/// Registers system-wide hotkeys. Pressing a registered one runs the
/// action's intent (the app's adapter does that).
pub trait HotkeyRegistrar: Send + Sync {
    /// Makes `accelerator` run `action` in every app. Replaces the action's
    /// current hotkey only when the caller unregistered it first.
    fn register(
        &self,
        action: HotkeyAction,
        accelerator: &Accelerator,
    ) -> Result<(), RegistrarRefused>;

    /// Releases `action`'s hotkey; nothing when it has none.
    fn unregister(&self, action: HotkeyAction);
}
