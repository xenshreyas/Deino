//! `automations` feature (AUT-01): the IPC surface of
//! `deino_core::automations`, exposed as the inline plugin
//! `plugin:automations|<command>`.
//!
//! - `commands.rs`: the step picker's catalogue, definitions (list, get,
//!   create, update, delete, duplicate), a meeting's ⋯ → Automations menu,
//!   an automation's matching meetings, running it on picked meetings and
//!   on the latest one; thin over `AutomationService`;
//! - `scheduler.rs`: [`PipelineScheduler`], which runs a meeting's
//!   `automations` stage again through the post-meeting pipeline;
//! - `tag_trigger.rs`: [`AutomationTagEvents`], the tags' event port that
//!   also hands each newly applied tag to the automations.
//!
//! The service is built in `composition/automations.rs`. There are no
//! events: the UI refreshes through `DbChanged` (`automations`,
//! `automation_runs`, `automation_run_steps`).

mod commands;
#[cfg(test)]
mod contract_tests;
mod scheduler;
mod tag_trigger;

pub(crate) use scheduler::PipelineScheduler;
pub(crate) use tag_trigger::AutomationTagEvents;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "automations";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `automations` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
