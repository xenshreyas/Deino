//! Benchmarks a real model through real worker processes, without the app.
//!
//! ```sh
//! cargo run -p deino-stt --release --example stt_bench -- \
//!     <model.gguf> <audio-16k-mono.wav> [--repeat N] [--realtime] [--cpu] [--text]
//! ```
//!
//! The example is its own worker: it re-launches itself with
//! `--transcribe-worker`, exactly as the app does. Prints a JSON report
//! (numbers only). The app's equivalent is `deino --transcribe-file`.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use deino_stt::bench::{self, BenchOptions, BenchTarget};
use deino_stt::clock::{MonotonicClock, SystemMonotonic};
use deino_stt::engine::TranscribeCppEngine;
use deino_stt::worker::{self, CPU_ONLY_FLAG};
use deino_stt::{BackendChoice, LoadSpec, StreamWindow, WorkerLauncher};

fn main() {
    if worker::is_worker_invocation(std::env::args_os()) {
        let cpu_only = std::env::args().any(|arg| arg == CPU_ONLY_FLAG);
        std::process::exit(worker::run(TranscribeCppEngine::default(), cpu_only));
    }
    let code = match run() {
        Ok(()) => 0,
        Err(message) => {
            let _ignored = writeln!(std::io::stderr(), "stt_bench: {message}");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, wav, flags @ ..] = args.as_slice() else {
        return Err(
            "usage: stt_bench <model.gguf> <audio.wav> [--repeat N] [--realtime] [--cpu]".into(),
        );
    };
    let repeat = flags
        .iter()
        .position(|flag| flag == "--repeat")
        .and_then(|index| flags.get(index + 1)?.parse().ok())
        .unwrap_or(1);
    let has = |flag: &str| flags.iter().any(|arg| arg == flag);
    let pcm = bench::read_wav(&PathBuf::from(wav)).map_err(|error| error.to_string())?;
    let target = BenchTarget {
        launcher: WorkerLauncher::current_exe().map_err(|error| error.to_string())?,
        spec: LoadSpec {
            path: PathBuf::from(model),
            backend: if has("--cpu") {
                BackendChoice::Cpu
            } else {
                BackendChoice::Auto
            },
        },
    };
    let options = BenchOptions {
        repeat,
        realtime: has("--realtime"),
        window: Some(StreamWindow::LIVE),
        include_text: has("--text"),
    };
    let clock: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonic::new());
    let report = bench::run(target, &pcm, options, &clock).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    writeln!(std::io::stdout(), "{json}").map_err(|error| error.to_string())
}
