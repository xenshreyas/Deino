//! Deino's executable: sets up logging and runs the app in `deino_lib`.
//!
//! The same executable is also a transcription worker process
//! (`--transcribe-worker`) and a headless transcription benchmark
//! (`--transcribe-file`); both are dispatched before anything else runs.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    if deino_lib::stt::worker_entry::is_requested() {
        std::process::exit(deino_lib::stt::worker_entry::run());
    }
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(code) = deino_lib::stt::cli::run_if_requested(&args) {
        std::process::exit(code);
    }

    deino_lib::logging::init();
    log::info!("Starting application...");
    deino_lib::run();
}
