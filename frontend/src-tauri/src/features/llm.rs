//! `llm` feature: Settings → AI over the LLM gateway (`deino_llm`), exposed
//! as the inline plugin `plugin:llm|<command>`.
//!
//! Providers and their masked keys, each provider's models (the legacy
//! model-list commands, ported), the per-route model table, a streamed test
//! per route, and Ollama model downloads. Commands take `Arc<LlmAdmin>` from
//! state (wired in `composition.rs` through `crate::llm`). Route choices are
//! settings (`ai.routes`), so their changes reach the UI as `settings-changed`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{LlmStreamEvent, OllamaPull, OllamaPullStatus, PullEvents, TauriPullEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "llm";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![OllamaPull])
        .typ::<LlmStreamEvent>()
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `llm` plugin.
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
