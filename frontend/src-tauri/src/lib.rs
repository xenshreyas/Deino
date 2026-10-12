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

mod app_lock;
#[cfg(test)]
mod bindings;
mod composition;
pub mod config;
mod error;
pub mod features;
mod lifecycle;
mod llm;
mod onnx_runtime;
mod paths;
mod permissions;
mod setup;
pub mod stt;
mod system_open;
pub mod tray;
pub mod updater;

legacy_modules! {
    pub mod api;
    pub mod audio;
    pub mod console_utils;
    pub mod database;
    mod legacy;
    pub mod notifications;
    pub mod parakeet_engine;
    pub mod state;
    pub mod summary;
    pub mod utils;
    pub mod whisper_engine;
}

use features::privacy::guard::{Guarded, guard_legacy};
use tauri::plugin::TauriPlugin;
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
        .invoke_handler(guard_legacy(legacy::invoke_handler()))
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(lifecycle::on_run_event);
}

/// Registers every feature plugin behind the app lock's IPC guard
/// (`features/privacy/guard.rs`).
fn register_feature_plugins(builder: Builder<Wry>) -> Builder<Wry> {
    feature_plugins()
        .into_iter()
        .fold(builder, |builder, plugin| {
            builder.plugin(Guarded::new(plugin))
        })
}

/// Every feature plugin: one line per entry in `features/manifest.rs`
/// and `features/organize_manifest.rs`.
fn feature_plugins() -> Vec<TauriPlugin<Wry>> {
    #[allow(unused_mut, reason = "release builds add no debug-only plugins")]
    let mut plugins = vec![
        features::db::init(),
        features::detection::init(),
        features::developer::init(),
        features::onboarding::init(),
        features::export::init(),
        features::action_items::init(),
        features::calendar::init(),
        features::enhance::init(),
        features::companies::init(),
        features::meetings::init(),
        features::network::init(),
        features::pipeline::init(),
        features::profile::init(),
        features::search::init(),
        features::notes::init(),
        features::people::init(),
        features::recording::init(),
        features::secrets::init(),
        features::llm::init(),
        features::settings::init(),
        features::transcription::init(),
        features::system::init(),
        features::templates::init(),
        features::folders::init(),
        features::integrations::init(),
        features::insights::init(),
        features::tags::init(),
        features::speakers::init(),
        features::transcript::init(),
        features::vocabulary::init(),
        features::speaker_tags::init(),
        features::player::init(),
        features::stats::init(),
        features::privacy::init(),
    ];
    #[cfg(debug_assertions)]
    plugins.extend([features::_example::init(), features::dev::init()]);
    plugins
}

/// Registers the third-party Tauri plugins the app depends on.
fn register_platform_plugins(builder: Builder<Wry>) -> Builder<Wry> {
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
        log::info!("Second app instance requested with args: {args:?}, cwd: {cwd:?}");
        tray::focus_main_window(app);
    }));
    // The meeting prompt is a non-activating panel (features/detection/prompt.rs).
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    updater::register(builder)
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
}
