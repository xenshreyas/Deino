//! `calendar` feature: the IPC surface of `deino_core::calendar` (ORG-01),
//! exposed as the inline plugin `plugin:calendar|<command>`.
//!
//! - `commands.rs`: sync status and switch (`status`, `enable`, `disable`,
//!   `refresh`, `open_access_settings`), Settings → Calendars
//!   (`list_calendars`, `set_calendar_visible`) and what the UI lists
//!   (`upcoming`, `event`), each thin over `CalendarService`;
//! - `events.rs`: the typed `CalendarChanged` event, emitted by
//!   [`TauriCalendarEvents`] whenever the status or a calendar's visibility
//!   changes. Synced rows reach the UI through `DbChanged`.
//!
//! The service, the `EventKit` source and the sync scheduler are wired in
//! `composition/calendar.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{CalendarChanged, TauriCalendarEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "calendar";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![CalendarChanged])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `calendar` plugin.
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
