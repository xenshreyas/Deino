//! `dev` feature (debug builds only): developer tools over IPC, exposed as
//! the inline plugin `plugin:dev|<command>`.
//!
//! Today it has one command, `seed_fixtures`, which writes the repository's
//! fixture meetings (`fixtures/meetings/*.json`) into the open database
//! through `deino_testkit::FixtureSeeder`, so a developer can click through
//! the real app with realistic data (Settings → Developer). The seeder writes
//! on the app's writer connection, so the UI refreshes through `DbChanged`.
//!
//! The module exists only when debug assertions are on (the `cfg` below,
//! which `build.rs` also reads), so release builds have no such plugin, no
//! such permission and no capability granting it.

#![cfg(debug_assertions)]

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "dev";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `dev` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
