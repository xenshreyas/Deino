//! `action-items` feature (AI-05): the IPC surface of
//! `deino_core::action_items`, exposed as the inline plugin
//! `plugin:action-items|<command>`.
//!
//! - `commands.rs`: list, today, create, update, bulk update, delete, "mark
//!   all mine done", the filter options and the saved views, thin over
//!   `ActionItemService` and `SavedViewService`;
//! - `events.rs`: the typed `ActionItemChanged` event (for INT-03 and
//!   AUT-01 listeners), emitted by [`TauriActionItemEvents`];
//! - `reminders.rs`: the morning reminder's notifier and its scheduler.
//!
//! The services are built in `composition/action_items.rs`; the UI refreshes
//! through `DbChanged` (`action_items`, `action_item_views`).

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;
mod reminders;

pub use events::{ActionItemChanged, TauriActionItemEvents};
pub(crate) use reminders::{SystemNotifier, start_reminders};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "action-items";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![ActionItemChanged])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `action-items` plugin.
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
