//! `insights` feature (AI-06): the IPC surface of `deino_core::insights`,
//! exposed as the inline plugin `plugin:insights|<command>`.
//!
//! - `commands.rs`: the template gallery, definitions (list, get, create,
//!   update, delete), a meeting's Insights tab, an insight's table, the
//!   past meetings it could run on, running it on picked meetings, and the
//!   CSV export; thin over `InsightService`;
//! - `scheduler.rs`: [`PipelineScheduler`], which runs a meeting's
//!   `insights` stage again through the post-meeting pipeline.
//!
//! The service is built in `composition/insights.rs`. There are no events:
//! the UI refreshes through `DbChanged` (`insight_defs`, `insight_results`)
//! and the pipeline's `PipelineProgress`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod scheduler;

pub(crate) use scheduler::PipelineScheduler;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "insights";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `insights` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
