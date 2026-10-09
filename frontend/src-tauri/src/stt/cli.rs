//! The headless transcription benchmark:
//!
//! ```sh
//! deino --transcribe-file meeting.wav --model parakeet-unified-en-0.6b-Q8_0.gguf \
//!       --repeat 3 --realtime --json [--text]
//! ```
//!
//! Loads the model in a real worker process (this executable), runs batch
//! and live passes (`deino_stt::bench`) and prints numbers only: real-time
//! factor, first-text and token latency, backlog. `--realtime` paces the live
//! pass like a microphone, which the latency numbers need. The model path
//! falls back to `DEINO_PARAKEET_UNIFIED_MODEL`.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use deino_stt::bench::{self, BenchOptions, BenchReport, BenchTarget};
use deino_stt::clock::{MonotonicClock, SystemMonotonic};
use deino_stt::{BackendChoice, LoadSpec, StreamWindow, WorkerLauncher};

/// The flag that selects the benchmark.
const FLAG: &str = "--transcribe-file";

/// Where the benchmarked model runs.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Device {
    /// The best available device (Metal on Apple Silicon).
    Gpu,
    /// The CPU only.
    Cpu,
}

/// Benchmark arguments.
#[derive(Debug, Parser)]
#[command(
    name = "deino",
    about = "Benchmark local transcription on a 16 kHz mono WAV file"
)]
struct Args {
    /// The audio to transcribe (16 kHz mono WAV).
    #[arg(long = "transcribe-file")]
    file: PathBuf,
    /// The model file (defaults to `DEINO_PARAKEET_UNIFIED_MODEL`).
    #[arg(long, env = super::MODEL_OVERRIDE_ENV)]
    model: PathBuf,
    /// How many times to run each pass.
    #[arg(long, default_value_t = 1)]
    repeat: usize,
    /// Pace the live pass at real time.
    #[arg(long)]
    realtime: bool,
    /// Where the model runs.
    #[arg(long, value_enum, default_value_t = Device::Gpu)]
    device: Device,
    /// Print the full report as JSON.
    #[arg(long)]
    json: bool,
    /// Also print the live transcript (it is never logged).
    #[arg(long)]
    text: bool,
}

/// Runs the benchmark when the arguments ask for it; returns the exit code.
pub fn run_if_requested(args: &[OsString]) -> Option<i32> {
    if !args.iter().any(|arg| arg == FLAG) {
        return None;
    }
    let code = match Args::try_parse_from(args) {
        Ok(args) => report(&args),
        Err(error) => {
            let _ignored = error.print();
            2
        }
    };
    Some(code)
}

fn report(args: &Args) -> i32 {
    // `RUST_LOG=deino_stt::perf=info` shows the live pass's `StreamPerf` lines.
    let _ignored = env_logger::try_init();
    match measure(args) {
        Ok(report) => print(args, &report),
        Err(message) => {
            let _ignored = writeln!(std::io::stderr(), "transcribe-file: {message}");
            1
        }
    }
}

fn measure(args: &Args) -> Result<BenchReport, String> {
    let pcm = bench::read_wav(&args.file).map_err(|error| error.to_string())?;
    let target = BenchTarget {
        launcher: WorkerLauncher::current_exe().map_err(|error| error.to_string())?,
        spec: LoadSpec {
            path: args.model.clone(),
            backend: match args.device {
                Device::Gpu => BackendChoice::Auto,
                Device::Cpu => BackendChoice::Cpu,
            },
        },
    };
    let options = BenchOptions {
        repeat: args.repeat.max(1),
        realtime: args.realtime,
        window: Some(StreamWindow::LIVE),
        include_text: args.text,
    };
    let clock: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonic::new());
    bench::run(target, &pcm, options, &clock).map_err(|error| error.to_string())
}

fn print(args: &Args, report: &BenchReport) -> i32 {
    let text = if args.json {
        serde_json::to_string_pretty(report).unwrap_or_default()
    } else {
        summary(report)
    };
    match writeln!(std::io::stdout(), "{text}") {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// One line per pass.
fn summary(report: &BenchReport) -> String {
    let mut lines = vec![format!(
        "audio {:.1} s, load {} ms on {}",
        report.audio_s, report.load_ms, report.backend
    )];
    for run in &report.batch {
        lines.push(format!("batch: RTF {:.4}, {} words", run.rtf, run.words));
    }
    for run in &report.live {
        let perf = &run.perf;
        lines.push(format!(
            "live: RTF {:.3}, first text p50/p95 {:?}/{:?} ms, token p50/p95 {:?}/{:?} ms, {} words",
            perf.rtf, perf.first_text_p50_ms, perf.first_text_p95_ms, perf.token_p50_ms, perf.token_p95_ms, run.words
        ));
        for line in run.transcript.iter().flatten() {
            lines.push(format!(
                "  [{:7.2}-{:7.2}] {}",
                line.start_s, line.end_s, line.text
            ));
        }
    }
    lines.join("\n")
}
