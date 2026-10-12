//! Times a watched-folder import end to end with the real engine, without
//! the app (CAP-11's two-hour acceptance run).
//!
//! ```sh
//! cargo run -p deino-recording --release --features deino-stt/metal \
//!     --example import_bench -- <model.gguf> <recording> <scratch dir>
//! ```
//!
//! Watches `<scratch dir>/inbox` with the real `WatchScheduler` (2 s scans,
//! 10 s stability) on an in-memory database, drops `<recording>` in, and
//! waits until its meeting is `ready`: decoded and converted by
//! `NativeIngest`, then transcribed by the `final_transcription` rules over
//! real worker processes (this example re-launches itself as the worker,
//! as the app does). Prints the timings; numbers only, no transcript text.
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_methods,
    reason = "a benchmark: it measures wall time, reports to stdout and fails by panicking"
)]

#[path = "../tests/support/import_rig.rs"]
mod import_rig;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use deino_core::final_transcription::TrackError;
use deino_core::meetings::{Meeting, MeetingLifecycleRepo};
use deino_core::segments::{Page, SegmentReader};
use deino_core::watch::WatchScheduler;
use deino_domain::{MeetingStatus, SystemClock};
use deino_recording::BatchEngines;
use deino_stt::engine::TranscribeCppEngine;
use deino_stt::worker;
use deino_stt::{
    BackendChoice, BatchTranscriber, EngineSupervisor, LoadSpec, SupervisorConfig, WorkerLauncher,
};
use import_rig::{ImportRig, Timings, import_rig};
use tokio_util::sync::CancellationToken;

/// The loaded engine, shared by every utterance.
struct Loaded(Arc<EngineSupervisor>);

impl BatchEngines for Loaded {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        Ok(self.0.clone())
    }
}

fn main() {
    if worker::is_worker_invocation(std::env::args_os()) {
        std::process::exit(worker::run(TranscribeCppEngine::default(), false));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, recording, scratch] = args.as_slice() else {
        eprintln!("usage: import_bench <model.gguf> <recording> <scratch dir>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(run(model.into(), recording.into(), scratch.into()));
}

/// The engine with `model` loaded, through worker processes.
fn engine(model: PathBuf) -> EngineSupervisor {
    let launcher = WorkerLauncher::current_exe().unwrap();
    let engine = EngineSupervisor::new(launcher, SupervisorConfig::default()).unwrap();
    let spec = LoadSpec {
        path: model,
        backend: BackendChoice::Auto,
    };
    engine.load(spec).unwrap();
    engine
}

/// Polls until a meeting is ready, then stops the scheduler.
async fn first_ready(rig: &ImportRig, cancel: &CancellationToken) -> Meeting {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut ready = rig
            .meetings
            .list_by_status(MeetingStatus::Ready)
            .await
            .unwrap();
        if let Some(meeting) = ready.pop() {
            cancel.cancel();
            return meeting;
        }
    }
}

async fn run(model: PathBuf, recording: PathBuf, scratch: PathBuf) {
    let engine = engine(model);
    let rig = import_rig(
        deino_db::test_db().await,
        &scratch,
        Arc::new(Loaded(Arc::new(engine))),
        Arc::new(SystemClock),
    )
    .await;
    rig.watch.add_folder(&rig.inbox).await.unwrap();
    let (scheduler, _nudge, cancel) = WatchScheduler::new(rig.watch.clone());
    let dropped_at = Instant::now();
    std::fs::copy(&recording, rig.inbox.join(recording.file_name().unwrap())).unwrap();
    let copied = dropped_at.elapsed();
    let ((), meeting) = tokio::join!(scheduler.run(), first_ready(&rig, &cancel));
    let total = dropped_at.elapsed();
    let state = rig.meetings.state(&meeting.id).await.unwrap().unwrap();
    let segments = rig.segments.list(&meeting.id, Page::ALL).await.unwrap();
    report(
        &rig.timings,
        dropped_at,
        (copied, total),
        (state.duration.map(|d| d.0), segments.len()),
    );
}

fn report(
    timings: &Timings,
    dropped_at: Instant,
    (copied, total): (Duration, Duration),
    (length, segments): (Option<u32>, usize),
) {
    let (imported, transcribed) = timings.spans();
    let secs = |duration: Duration| duration.as_secs_f64();
    println!("recording length: {length:?} s");
    println!("copy into the watched folder: {:.2} s", secs(copied));
    println!(
        "drop → import starts (stability wait): {:.1} s",
        secs(imported.0 - dropped_at)
    );
    println!(
        "import (decode + AAC + keep video): {:.1} s",
        secs(imported.1 - imported.0)
    );
    println!(
        "final_transcription: {:.1} s",
        secs(transcribed.1 - transcribed.0)
    );
    println!("drop → ready: {:.1} s", secs(total));
    println!("segments: {segments}");
}
