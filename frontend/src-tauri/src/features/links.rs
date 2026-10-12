//! `links` feature (DEV-05): `deino://` links, exposed as the inline plugin
//! `plugin:links|<command>`.
//!
//! - `commands.rs`: `take`, the navigations waiting for the window, which
//!   the app lock's guard refuses while Deino is locked (the plugin is
//!   `Guarded`), so a link reveals nothing before the unlock;
//! - [`events`]: `NavigationWaiting` (take now) and `LinkRejected`;
//! - [`adapters`]: `deino_core::links`' ports over the recorder, the lock
//!   and the main window, and [`adapters::open_urls`], which follows the
//!   URLs macOS hands Deino.
//!
//! `composition/links.rs` registers `tauri-plugin-deep-link` and wires the
//! service; the scheme itself is declared in `tauri.conf.json`.

pub(crate) mod adapters;
mod commands;
#[cfg(test)]
mod contract_tests;
pub mod events;

pub use events::{LinkRejected, NavigationWaiting};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "links";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![
            NavigationWaiting,
            LinkRejected
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `links` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app, _| {
            builder.mount_events(app);
            Ok(())
        })
        .build()
}
