//! Deino's Tauri application crate (`deino_lib`).
//!
//! This file only assembles the app: module declarations, Tauri plugins,
//! managed state, the setup hook, window/run-loop handlers and command
//! registration. Startup steps live in [`setup`], event handlers in
//! [`lifecycle`], the dependency wiring in [`composition`], legacy commands in
//! [`legacy`], and new IPC in one inline plugin per feature under [`features`]
//! (PARITY_PLAN §4.0, ADR 0002, ADR 0007).

// Declared first so `perf_debug!`/`perf_trace!` are in scope in every module.
#[macro_use]
mod perf;
// Declared second: `legacy_modules!` exempts inherited modules from the strict lints.
#[macro_use]
mod legacy_lints;

#[cfg(test)]
mod bindings;
mod composition;
pub mod config;
mod error;
pub mod features;
mod lifecycle;
pub mod onboarding;
mod onnx_runtime;
mod paths;
mod setup;
pub mod updater;

legacy_modules! {
    pub mod anthropic;
    pub mod api;
    pub mod audio;
    pub mod console_utils;
    pub mod database;
    pub mod groq;
    mod legacy;
    pub mod notifications;
    pub mod ollama;
    pub mod openai;
    pub mod openrouter;
    pub mod parakeet_engine;
    pub mod state;
    pub mod summary;
    pub mod tray;
    pub mod utils;
    pub mod whisper_engine;
}

use tauri::{Builder, Wry};

/// User-visible product name, used for notification titles and the tray tooltip.
pub const APP_NAME: &str = "Deino";

/// Builds and runs the Tauri application. Blocks until the app exits.
///
/// # Panics
///
/// When Tauri cannot build the app (an invalid config or context).
#[allow(
    clippy::expect_used,
    reason = "the app cannot run at all if Tauri fails to build"
)]
pub fn run() {
    log::set_max_level(log::LevelFilter::Info);

    let builder = register_platform_plugins(Builder::default());
    let builder = register_feature_plugins(state::manage_initial_state(builder));
    builder
        .setup(setup::run)
        .on_window_event(lifecycle::on_window_event)
        .invoke_handler(legacy::invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(lifecycle::on_run_event);
}

/// Registers every feature plugin: one line per entry in `features/manifest.rs`.
fn register_feature_plugins(builder: Builder<Wry>) -> Builder<Wry> {
    let builder = builder
        .plugin(features::db::init())
        .plugin(features::meetings::init())
        .plugin(features::secrets::init())
        .plugin(features::settings::init())
        .plugin(features::system::init());
    #[cfg(debug_assertions)]
    let builder = builder
        .plugin(features::_example::init())
        .plugin(features::dev::init());
    builder
}

/// Registers the third-party Tauri plugins the app depends on.
fn register_platform_plugins(builder: Builder<Wry>) -> Builder<Wry> {
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
        log::info!("Second app instance requested with args: {args:?}, cwd: {cwd:?}");
        tray::focus_main_window(app);
    }));
    updater::register(builder)
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
}
