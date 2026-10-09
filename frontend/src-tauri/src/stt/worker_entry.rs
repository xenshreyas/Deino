//! The executable as a transcription worker process.
//!
//! `deino --transcribe-worker [--cpu-only]` serves the worker protocol on
//! stdin and stdout with the real transcribe-cpp engine. `main` checks for it
//! before anything else (logging, Tauri, single-instance) runs.

use deino_stt::engine::TranscribeCppEngine;
use deino_stt::worker::{self, CPU_ONLY_FLAG};

/// Whether this process was started as a worker.
pub fn is_requested() -> bool {
    worker::is_worker_invocation(std::env::args_os())
}

/// Serves the worker protocol until the app closes stdin; returns the exit code.
pub fn run() -> i32 {
    let cpu_only = std::env::args_os().any(|arg| arg == CPU_ONLY_FLAG);
    worker::run(TranscribeCppEngine::default(), cpu_only)
}
