//! `search` feature: the IPC surface of `deino_core::search` (ORG-08, AI-01),
//! exposed as the inline plugin `plugin:search|<command>`.
//!
//! - `commands.rs`: the palette's and the results page's `search`, the `@`
//!   filter suggestions, a full keyword `reindex_all`, and Settings → Search
//!   index (the embedding model's download, the backfill, coverage);
//! - `events.rs`: `IndexProgress` (the backfill) and the embedding model's
//!   download events, so the settings page follows along.
//!
//! The services are wired in `composition/search.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{
    EmbeddingModelChanged, EmbeddingModelProgress, IndexProgress, TauriEmbeddingModelEvents,
    TauriIndexEvents,
};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "search";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![
            IndexProgress,
            EmbeddingModelProgress,
            EmbeddingModelChanged
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `search` plugin.
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
