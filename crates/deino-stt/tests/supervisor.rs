//! The supervisor against real worker processes (the fake worker binary,
//! which decodes with the scripted engine) with injected faults: crashes,
//! hangs, GPU→CPU fallback, cancel-by-kill and process cleanup.
//!
//! Every test sets its fault on its own launcher, so tests run in parallel.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers fail loudly, like the tests they serve"
)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use deino_stt::clock::{MonotonicClock, SystemMonotonic};
use deino_stt::engine::scripted::FaultPlan;
use deino_stt::protocol::{RemoteErrorKind, StreamParams};
use deino_stt::supervisor::Deadlines;
use deino_stt::words::text_of;
use deino_stt::{
    BackendChoice, EngineSupervisor, LoadSpec, Millis, SttError, SupervisorConfig, WorkerFault,
    WorkerLauncher,
};
use tempfile::TempDir;

/// Short decoding deadlines, so a hang is caught in two seconds.
fn config() -> SupervisorConfig {
    let quick = Deadlines {
        call_floor: Duration::from_secs(2),
        run_floor: Duration::from_secs(2),
        audio_factor: 10,
    };
    SupervisorConfig {
        gpu: quick,
        cpu: quick,
        ..SupervisorConfig::default()
    }
}

/// A scratch folder holding a stand-in model file (and fault markers).
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("model.gguf"), b"scripted").unwrap();
        std::fs::write(dir.path().join("whisper.batch"), b"scripted").unwrap();
        Self { dir }
    }

    fn spec(&self, file: &str) -> LoadSpec {
        LoadSpec {
            path: self.dir.path().join(file),
            backend: BackendChoice::Auto,
        }
    }

    fn marker(&self) -> PathBuf {
        self.dir.path().join("fault-fired")
    }

    /// A supervisor whose workers inject `fault` (once, when `once`).
    fn engine(&self, fault: Option<&str>, gpu_only: bool, once: bool) -> EngineSupervisor {
        let mut launcher = WorkerLauncher::new(env!("CARGO_BIN_EXE_deino-stt-fake-worker"));
        if let Some(fault) = fault {
            launcher = launcher.env(FaultPlan::ENV, fault);
        }
        if gpu_only {
            launcher = launcher.env(FaultPlan::GPU_ONLY_ENV, "1");
        }
        if once {
            launcher = launcher.env(FaultPlan::ONCE_ENV, self.marker());
        }
        EngineSupervisor::new(launcher, config()).unwrap()
    }
}

fn loud(ms: u64) -> Vec<f32> {
    vec![0.5; Millis(ms).to_samples()]
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

/// Waits (without sleeping) until `condition` holds, failing after 10 s.
fn eventually(what: &str, condition: impl Fn() -> bool) {
    let clock = SystemMonotonic::new();
    while !condition() {
        assert!(
            clock.now() < Duration::from_secs(10),
            "{what} never happened"
        );
        std::thread::yield_now();
    }
}

fn exists(path: &Path) -> bool {
    path.exists()
}

#[test]
fn a_loaded_model_transcribes_in_its_worker() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);

    let info = engine.load(fixture.spec("model.gguf")).unwrap();
    let transcript = engine.transcribe(loud(800)).unwrap();

    assert!(info.on_gpu);
    assert_eq!(transcript.text, "w0 w1");
    assert!(engine.worker_pid().is_some_and(alive));
}

#[test]
fn a_stream_answers_feeds_in_order_and_finalizes() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);
    engine.load(fixture.spec("model.gguf")).unwrap();

    let stream = engine.start_stream(StreamParams::default()).unwrap();
    let first = stream.feed(loud(400)).unwrap();
    let second = stream.feed(loud(400)).unwrap();
    let last = stream.finalize().unwrap().unwrap();

    assert!(first.tokens.is_empty());
    assert_eq!(text_of(&second.tokens), "w0");
    assert_eq!(text_of(&last.tokens), "w1");
}

#[test]
fn a_crash_mid_stream_loses_the_stream_and_the_next_run_recovers_on_the_cpu() {
    let fixture = Fixture::new();
    let engine = fixture.engine(Some("crash@feed#2"), false, true);
    engine.load(fixture.spec("model.gguf")).unwrap();
    let crashed = engine.worker_pid().unwrap();

    let stream = engine.start_stream(StreamParams::default()).unwrap();
    stream.feed(loud(400)).unwrap();
    let lost = stream.feed(loud(400));

    assert!(matches!(lost, Err(SttError::StreamLost)), "{lost:?}");
    assert_eq!(
        stream.finalize().unwrap(),
        None,
        "finalize says: fall back to batch"
    );
    assert!(!alive(crashed));
    assert!(engine.gpu_unavailable(), "the crash happened on the GPU");
    assert_eq!(engine.transcribe(loud(400)).unwrap().text, "w0");
    assert_eq!(engine.loaded().unwrap().backend, "CPU");
}

#[test]
fn a_hung_feed_is_killed_at_its_deadline() {
    let fixture = Fixture::new();
    let engine = fixture.engine(Some("hang@feed"), false, false);
    engine.load(fixture.spec("model.gguf")).unwrap();
    let hung = engine.worker_pid().unwrap();
    let stream = engine.start_stream(StreamParams::default()).unwrap();
    let clock = SystemMonotonic::new();

    let lost = stream.feed(loud(80));

    assert!(matches!(lost, Err(SttError::StreamLost)), "{lost:?}");
    assert!(
        clock.now() >= Duration::from_secs(2),
        "waited for the deadline"
    );
    assert!(!alive(hung));
}

#[test]
fn a_gpu_crash_while_loading_falls_back_to_a_cpu_only_worker() {
    let fixture = Fixture::new();
    let engine = fixture.engine(Some("crash@load"), true, false);

    let info = engine.load(fixture.spec("model.gguf")).unwrap();

    assert!(!info.on_gpu);
    assert_eq!(info.backend, "CPU");
    assert!(engine.gpu_unavailable());
}

#[test]
fn a_gpu_crash_during_a_run_is_retried_once_on_the_cpu() {
    let fixture = Fixture::new();
    let engine = fixture.engine(Some("crash@run"), true, false);
    engine.load(fixture.spec("model.gguf")).unwrap();

    let transcript = engine.transcribe(loud(400)).unwrap();

    assert_eq!(transcript.text, "w0");
    assert_eq!(engine.loaded().unwrap().backend, "CPU");
}

#[test]
fn a_run_that_crashes_on_the_cpu_too_is_reported_not_retried_forever() {
    let fixture = Fixture::new();
    let engine = fixture.engine(Some("crash@run"), false, false);
    engine.load(fixture.spec("model.gguf")).unwrap();

    let failed = engine.transcribe(loud(400));

    assert!(
        matches!(
            failed,
            Err(SttError::WorkerLost(WorkerFault::Crashed { .. }))
        ),
        "{failed:?}"
    );
}

#[test]
fn cancel_kills_a_hung_run_long_before_its_deadline() {
    let fixture = Fixture::new();
    let engine = Arc::new(fixture.engine(Some("hang@run"), false, true));
    engine.load(fixture.spec("model.gguf")).unwrap();
    let hung = engine.worker_pid().unwrap();
    let runner = Arc::clone(&engine);
    let clock = SystemMonotonic::new();

    let run = std::thread::spawn(move || runner.transcribe(loud(400)));
    eventually("the run reaching the worker", || exists(&fixture.marker()));
    engine.cancel();

    assert!(matches!(run.join().unwrap(), Err(SttError::Cancelled)));
    assert!(
        clock.now() < Duration::from_secs(2),
        "did not wait for the deadline"
    );
    assert!(!alive(hung));
    assert_eq!(
        engine.transcribe(loud(400)).unwrap().text,
        "w0",
        "a fresh worker serves"
    );
}

#[test]
fn unload_stops_the_worker_and_frees_the_model() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);
    engine.load(fixture.spec("model.gguf")).unwrap();
    let pid = engine.worker_pid().unwrap();

    engine.unload();

    assert!(!alive(pid));
    assert_eq!(engine.loaded(), None);
    assert!(matches!(
        engine.transcribe(loud(400)),
        Err(SttError::NotLoaded)
    ));
}

#[test]
fn dropping_the_supervisor_stops_its_worker() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);
    engine.load(fixture.spec("model.gguf")).unwrap();
    let pid = engine.worker_pid().unwrap();

    drop(engine);

    eventually("the worker exiting", || !alive(pid));
}

#[test]
fn a_model_that_cannot_stream_refuses_a_stream_but_transcribes() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);
    engine.load(fixture.spec("whisper.batch")).unwrap();

    let refused = engine.start_stream(StreamParams::default());

    assert!(matches!(refused, Err(SttError::StreamingUnsupported)));
    assert_eq!(engine.transcribe(loud(400)).unwrap().text, "w0");
}

#[test]
fn a_missing_model_file_is_a_load_error() {
    let fixture = Fixture::new();
    let engine = fixture.engine(None, false, false);

    let failed = engine.load(fixture.spec("absent.gguf"));

    assert!(
        matches!(&failed, Err(SttError::Engine(error)) if error.kind == RemoteErrorKind::Load),
        "{failed:?}"
    );
    assert_eq!(engine.loaded(), None);
}
