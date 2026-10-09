//! The engine pool against real worker processes: concurrent lanes, pinning
//! and idle unload.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers fail loudly, like the tests they serve"
)]

use std::sync::Arc;
use std::time::Duration;

use deino_stt::clock::FakeMonotonic;
use deino_stt::protocol::StreamParams;
use deino_stt::words::text_of;
use deino_stt::{
    BackendChoice, EnginePool, EngineSlot, Lane, LoadSpec, Millis, SupervisorConfig, WorkerLauncher,
};
use tempfile::TempDir;

fn pool(dir: &TempDir, clock: Arc<FakeMonotonic>) -> Arc<EnginePool> {
    let model = dir.path().join("model.gguf");
    std::fs::write(&model, b"scripted").unwrap();
    let launcher = WorkerLauncher::new(env!("CARGO_BIN_EXE_deino-stt-fake-worker"));
    let pool = EnginePool::new(launcher, SupervisorConfig::default(), clock);
    pool.set_model(Some(LoadSpec {
        path: model,
        backend: BackendChoice::Auto,
    }));
    Arc::new(pool)
}

fn loud(ms: u64) -> Vec<f32> {
    vec![0.5; Millis(ms).to_samples()]
}

#[test]
fn the_microphone_and_system_lanes_stream_at_once_in_separate_workers() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir, Arc::new(FakeMonotonic::default()));

    let (mic, _) = pool.engine(EngineSlot::Live(Lane::Microphone)).unwrap();
    let (system, _) = pool.engine(EngineSlot::Live(Lane::System)).unwrap();
    let mic_stream = mic.start_stream(StreamParams::default()).unwrap();
    let system_stream = system.start_stream(StreamParams::default()).unwrap();
    let mut heard = Vec::new();
    for _ in 0..3 {
        heard.extend(mic_stream.feed(loud(400)).unwrap().tokens);
        heard.extend(system_stream.feed(loud(400)).unwrap().tokens);
    }

    assert_ne!(mic.worker_pid(), system.worker_pid());
    assert_eq!(text_of(&heard), "w0 w0 w1 w1");
    assert_eq!(pool.worker_pids().len(), 2);
}

#[test]
fn idle_engines_unload_but_pinned_ones_stay_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(FakeMonotonic::default());
    let pool = pool(&dir, clock.clone());
    let (live, _) = pool.engine(EngineSlot::Live(Lane::Mixed)).unwrap();
    let (batch, _) = pool.engine(EngineSlot::Batch).unwrap();
    let pin = pool.pin(EngineSlot::Live(Lane::Mixed));

    clock.advance(Duration::from_mins(10));
    let unloaded = pool.unload_idle(Duration::from_mins(5));

    assert_eq!(unloaded, 1);
    assert!(batch.loaded().is_none());
    assert!(live.loaded().is_some(), "a recording keeps its engine");
    drop(pin);
    clock.advance(Duration::from_mins(10));
    assert_eq!(pool.unload_idle(Duration::from_mins(5)), 1);
    assert!(
        pool.engine(EngineSlot::Batch).unwrap().1.supports_streaming,
        "reloads on use"
    );
}
