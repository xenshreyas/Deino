//! Deino's Tauri application crate (`deino_lib`).
//!
//! This file only assembles the app: module declarations, Tauri plugins,
//! managed state, the setup hook, window/run-loop handlers and command
//! registration. Startup steps live in [`setup`], event handlers in
//! [`lifecycle`] and legacy commands in [`legacy`] (PARITY_PLAN §4.0).

// Declared first so `perf_debug!`/`perf_trace!` are in scope in every module.
#[macro_use]
mod perf;

pub mod anthropic;
pub mod api;
pub mod audio;
pub mod config;
pub mod console_utils;
pub mod database;
pub mod groq;
mod legacy;
mod lifecycle;
pub mod notifications;
pub mod ollama;
pub mod onboarding;
mod onnx_runtime;
pub mod openai;
pub mod openrouter;
pub mod parakeet_engine;
mod setup;
pub mod state;
pub mod summary;
pub mod tray;
pub mod updater;
pub mod utils;
pub mod whisper_engine;

use tauri::{Builder, Wry};

/// User-visible product name, used for notification titles and the tray tooltip.
pub const APP_NAME: &str = "Deino";

/// Builds and runs the Tauri application. Blocks until the app exits.
pub fn run() {
    log::set_max_level(log::LevelFilter::Info);

    let builder = register_platform_plugins(Builder::default());
    state::manage_initial_state(builder)
        .setup(setup::run)
        .on_window_event(lifecycle::on_window_event)
        .invoke_handler(legacy::invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(lifecycle::on_run_event);
}

/// Registers the third-party Tauri plugins the app depends on.
fn register_platform_plugins(builder: Builder<Wry>) -> Builder<Wry> {
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
        log::info!("Second app instance requested with args: {:?}, cwd: {:?}", args, cwd);
        tray::focus_main_window(app);
    }));
    updater::register(builder)
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
}
