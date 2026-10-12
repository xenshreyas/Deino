//! `tags` feature: the IPC surface of `deino_core::tags` (ORG-06), exposed
//! as the inline plugin `plugin:tags|<command>`.
//!
//! - `commands.rs`: tag CRUD and applying tags by hand, thin over
//!   `TagService` (wired in `composition/organize.rs`);
//! - `events.rs`: the typed `MeetingTagged` event, emitted for every tag
//!   newly applied by the user, the auto-tagger or an automation, so
//!   automations (AUT-01) can trigger on it.
//!
//! The `auto_tag` pipeline stage applies tags through the same service, so
//! its tags announce themselves the same way.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{MeetingTagged, TauriTagEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "tags";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![MeetingTagged])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `tags` plugin.
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
