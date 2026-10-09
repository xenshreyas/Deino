//! `system` feature: facts about the running app, exposed as the inline
//! plugin `plugin:system|<command>`.
//!
//! This is the smallest possible feature plugin and the worked example of the
//! registration pattern (ADR 0002). It has no service layer because it only
//! reads Tauri's package info.

mod commands;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "system";

/// Builds the `system` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(tauri::generate_handler![commands::app_version])
        .build()
}
