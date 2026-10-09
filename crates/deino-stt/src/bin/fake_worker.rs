//! A worker process around the scripted engine, for supervisor tests.
//!
//! Behaves exactly like the app's `--transcribe-worker` mode, but decodes
//! with `ScriptedEngine` (no model, no native library) and crashes or hangs
//! where `DEINO_STT_FAULT` says, so tests exercise real processes, pipes,
//! deadlines and kills.

use deino_stt::engine::scripted::{FaultPlan, ScriptedEngine};
use deino_stt::worker::{self, CPU_ONLY_FLAG};

fn main() {
    let fault = FaultPlan::from_env(|key| std::env::var(key).ok());
    let cpu_only = std::env::args().any(|arg| arg == CPU_ONLY_FLAG);
    std::process::exit(worker::run(ScriptedEngine::new(fault), cpu_only));
}
