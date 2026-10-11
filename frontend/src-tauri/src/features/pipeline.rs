//! `pipeline` feature: the IPC surface of `deino_pipeline`, exposed as the
//! inline plugin `plugin:pipeline|<command>`.
//!
//! - `commands.rs`: `status` and `rerun`, thin over `PipelineService`;
//! - `events.rs`: the typed `PipelineProgress` event, emitted for every
//!   stage status change by [`TauriPipelineProgress`].
//!
//! The service is built and started in `composition.rs`; the meeting page's
//! status strip (`frontend/src/features/pipeline/`) reads `status` and
//! refreshes on `PipelineProgress`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{PipelineProgress, TauriPipelineProgress};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "pipeline";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![PipelineProgress])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `pipeline` plugin.
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
