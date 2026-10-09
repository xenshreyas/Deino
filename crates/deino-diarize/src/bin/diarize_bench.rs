//! The CAP-03 benchmark: diarization error rate (DER) and real-time factor
//! (RTF) with the production parameters (`policy`, `track::post_process`).
//!
//! ```bash
//! cargo run -p deino-diarize --release --features bench --bin diarize-bench -- \
//!     --models <dir> fixtures                      # FND-04 fixtures with audio
//! cargo run -p deino-diarize --release --features bench --bin diarize-bench -- \
//!     --models <dir> file <audio.wav|.m4a> <reference.rttm> [--speakers N]
//! ```
//!
//! Flags: `--cpu` (no Core ML), `--threads N`. `<dir>` must hold the pinned
//! models (verified by SHA-256). Each run prints one JSON line: DER with a
//! 0.25 s collar and without, its parts, speaker counts, timings, and the
//! machine's load average (timings are only comparable at low load).
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_methods,
    reason = "a command-line benchmark prints its results and measures wall time"
)]

#[path = "diarize_bench/fixtures.rs"]
mod bench_fixtures;
#[path = "diarize_bench/input.rs"]
mod bench_input;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use deino_diarize::metrics::{ReferenceTurn, diarization_error_rate};
use deino_diarize::models::ModelStore;
use deino_diarize::policy::{self, ExpectedSpeakers};
use deino_diarize::track::post_process;
use deino_diarize::units::samples_to_duration;
use deino_diarize::{DiarizeRequest, Diarizer, ModelPaths, RuntimeConfig, SAMPLE_RATE};
use serde_json::json;

/// The collar the spike and anarlog's eval report DER at.
const COLLAR: Duration = Duration::from_millis(250);

/// One clip to score.
pub(crate) struct Case {
    /// A name for the report.
    pub(crate) name: String,
    /// 16 kHz mono samples.
    pub(crate) samples: Vec<f32>,
    /// The ground truth.
    pub(crate) reference: Vec<ReferenceTurn>,
    /// What the calendar would say.
    pub(crate) expected: ExpectedSpeakers,
}

struct Options {
    models: PathBuf,
    runtime: RuntimeConfig,
    command: Vec<String>,
    speakers: Option<usize>,
}

fn main() {
    let result = parse(std::env::args().skip(1).collect()).and_then(|options| run(&options));
    if let Err(message) = result {
        eprintln!("diarize-bench: {message}");
        std::process::exit(1);
    }
}

fn run(options: &Options) -> Result<(), String> {
    let models = ModelStore::new(&options.models)
        .verified_paths()
        .map_err(|e| e.to_string())?;
    let cases = match options
        .command
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["fixtures"] => bench_fixtures::cases()?,
        ["file", audio, rttm] => vec![bench_input::case(audio, rttm, options.speakers)?],
        _ => {
            return Err(
                "usage: diarize-bench --models <dir> (fixtures | file <audio> <rttm>)".into(),
            );
        }
    };
    for case in cases {
        println!("{}", score(&case, &models, &options.runtime)?);
    }
    Ok(())
}

/// Diarizes one case and reports it as JSON.
fn score(
    case: &Case,
    models: &ModelPaths,
    runtime: &RuntimeConfig,
) -> Result<serde_json::Value, String> {
    let duration = samples_to_duration(case.samples.len());
    let bounds = policy::bounds_for(duration, case.expected);
    let config = policy::config_for(duration, runtime.clone());
    let loading = Instant::now();
    let mut diarizer = Diarizer::new(config, models).map_err(|e| e.to_string())?;
    let load = loading.elapsed();
    let processing = Instant::now();
    let request = DiarizeRequest {
        bounds,
        ..DiarizeRequest::default()
    };
    let raw = diarizer
        .diarize(case.samples.as_slice(), &request)
        .map_err(|e| e.to_string())?;
    let result = post_process(raw, bounds);
    let process = processing.elapsed();
    let der = diarization_error_rate(&case.reference, &result.turns, COLLAR);
    let strict = diarization_error_rate(&case.reference, &result.turns, Duration::ZERO);
    let percent = |seconds: f64| 100.0 * seconds / der.total.max(f64::EPSILON);
    Ok(json!({
        "case": case.name,
        "audio_s": duration.as_secs_f64(),
        "der": 100.0 * der.der(), "der_no_collar": 100.0 * strict.der(),
        "missed": percent(der.missed), "false_alarm": percent(der.false_alarm), "confusion": percent(der.confusion),
        "reference_speakers": der.reference_speakers, "hypothesis_speakers": result.speakers.len(),
        "speech_s": result.speakers.iter().map(|s| f64::from(s.speech.get()) / 1_000.0).collect::<Vec<_>>(),
        "turns": result.turns.iter().map(|t| (t.speaker, t.start.get(), t.end.get())).collect::<Vec<_>>(),
        "bounds": format!("{bounds:?}"),
        "load_s": load.as_secs_f64(), "process_s": process.as_secs_f64(),
        "rtf": process.as_secs_f64() / duration.as_secs_f64(),
        "threads": runtime.intra_threads.get(), "core_ml": runtime.accelerate_embedding,
        "load_average": load_average(),
    }))
}

/// The 1-minute load average, from `sysctl` (`None` if unavailable).
fn load_average() -> Option<f64> {
    let output = std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    text.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace())
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn parse(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options {
        models: std::env::var_os("DEINO_DIARIZE_MODELS")
            .map(PathBuf::from)
            .unwrap_or_default(),
        runtime: RuntimeConfig::default(),
        command: Vec::new(),
        speakers: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--models" => options.models = PathBuf::from(value()?),
            "--cpu" => options.runtime.accelerate_embedding = false,
            "--threads" => options.runtime.intra_threads = number(&value()?)?,
            "--speakers" => options.speakers = Some(number(&value()?)?.get()),
            _ => options.command.push(arg),
        }
    }
    Ok(options)
}

fn number(text: &str) -> Result<std::num::NonZeroUsize, String> {
    text.parse()
        .map_err(|_| format!("{text} is not a positive number"))
}

/// The sample rate every input is decoded to.
pub(crate) const RATE: u32 = SAMPLE_RATE;
