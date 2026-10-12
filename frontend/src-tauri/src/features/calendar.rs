//! `calendar` feature: the IPC surface of `deino_core::calendar` (ORG-01),
//! exposed as the inline plugin `plugin:calendar|<command>`.
//!
//! - `commands.rs`: sync status and switch (`status`, `enable`, `disable`,
//!   `refresh`, `open_access_settings`), Settings → Calendars
//!   (`list_calendars`, `set_calendar_visible`) and what the UI lists
//!   (`upcoming`, `event`), each thin over `CalendarService`, plus event
//!   linking over `EventLinkService` and `RelatedService`;
//! - `events.rs`: the typed `CalendarChanged` event, emitted by
//!   [`TauriCalendarEvents`] whenever the status or a calendar's visibility
//!   changes. Synced rows reach the UI through `DbChanged`;
//! - `lookup.rs`: [`SyncedCalendar`], meeting detection's (CAP-06)
//!   `CalendarLookup` over the synced events;
//! - `start_link.rs`: [`LinkOnStart`], the recorder's start observer that
//!   links a new recording to the nearest event (ORG-03). The picker's
//!   commands (`meeting_event`, `event_suggestions`, `link_event`,
//!   `unlink_event`) and `related_meetings` are in `commands.rs`;
//! - `rules_commands.rs` (ORG-02): `record_plan` (Coming up with each
//!   event's auto-record decision), `choose_record` (the record toggle,
//!   this event or all occurrences) and `join` (Join & record, Join, Record
//!   only) over `AutoRecordService` and `JoinService`;
//! - `join_adapters.rs`: [`SystemCallOpener`] and [`LiveRecorder`], Join's
//!   ports.
//!
//! The service, the `EventKit` source, the Google and Microsoft connectors
//! and the sync scheduler are wired in `composition/calendar.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;
mod join_adapters;
#[cfg(test)]
mod linking_contract_tests;
mod lookup;
mod rules_commands;
mod start_link;

pub use events::{CalendarChanged, TauriCalendarEvents};
pub use join_adapters::{LiveRecorder, SystemCallOpener};
pub use lookup::SyncedCalendar;
pub use start_link::LinkOnStart;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "calendar";

super::registry::register_plugin!(Guarded);

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
