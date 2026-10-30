//! The headless after-meeting benchmark:
//!
//! ```sh
//! deino --final-bench <copy.sqlite> --meeting <id> \
//!       --models <app data>/models --speakers <resources>/speakers \
//!       [--whisper large-v3-turbo] [--follow <dir>]
//! ```
//!
//! Runs the `final_transcription` and `diarize` stages on a meeting in a
//! copy of the database, wired as the app wires them for the Whisper engine
//! (the shared pass memory, the voice check, the speaker engines and their
//! memories), and prints how long each took. With `--follow`, a pass during
//! the meeting runs first over `<dir>/mic.m4a` and `<dir>/system.m4a` (the
//! tracks as recorded so far, such as copies of the recording's CMAF files
//! cut at a fragment), untimed, as the last pass before the meeting ended
//! would have. The new transcript is in the copy. Never point it at the
//! real database.
#![allow(clippy::disallowed_methods, reason = "a benchmark measures wall time")]

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use deino_core::diarization::{DiarizeDeps, DiarizeService, TrackDiarizer, WordAligner};
use deino_core::final_transcription::{
    FinalPass, FinalTranscriptionDeps, FinalTranscriptionService, SqliteOriginalTranscripts,
    TrackProgress,
};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::voiceprints::SqliteVoiceprintRepo;
use deino_db::{Database, DbLocation, MIGRATOR};
use deino_diarize::align::{LineMemory, OnnxWordAligner};
use deino_diarize::models::{ModelStore, bundled};
use deino_diarize::track::OnnxTrackDiarizer;
use deino_diarize::vbx::Plda;
use deino_diarize::{RuntimeConfig, ScanCache};
use deino_domain::{MeetingId, SystemClock};
use deino_recording::{
    DeepFilterCheck, FollowAlong, FollowDeps, NativeDecoder, OfflineTranscriber, PassMemory,
};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use stand_ins::{Always, NoDownloads, NothingRecording, WhisperEngines};

mod stand_ins;

/// The flag that selects the benchmark.
const FLAG: &str = "--final-bench";

#[derive(Debug, Parser)]
#[command(
    name = "deino",
    about = "Benchmark the after-meeting stages on a copy of the database"
)]
struct Args {
    #[arg(long = "final-bench")]
    db: PathBuf,
    #[arg(long)]
    meeting: String,
    #[arg(long)]
    models: PathBuf,
    #[arg(long)]
    speakers: PathBuf,
    #[arg(long, default_value = "large-v3-turbo")]
    whisper: String,
    #[arg(long)]
    follow: Option<PathBuf>,
}

/// Runs the benchmark when the arguments ask for it; returns the exit code.
pub fn run_if_requested(args: &[OsString]) -> Option<i32> {
    if !args.iter().any(|arg| arg == FLAG) {
        return None;
    }
    let _ignored = env_logger::try_init();
    let outcome = Args::try_parse_from(args)
        .map_err(|error| error.to_string())
        .and_then(|args| {
            let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
            runtime.block_on(run(&args))
        });
    Some(match outcome {
        Ok(line) => i32::from(writeln!(std::io::stdout(), "{line}").is_err()),
        Err(message) => {
            let _ignored = writeln!(std::io::stderr(), "final-bench: {message}");
            1
        }
    })
}

/// The stages' engines over one shared memory, as the app wires them.
struct Wiring {
    transcriber: OfflineTranscriber,
    memory: PassMemory,
    diarizer: Arc<dyn TrackDiarizer>,
    aligner: Arc<dyn WordAligner>,
}

async fn run(args: &Args) -> Result<String, String> {
    let database = Database::open(&DbLocation::File(args.db.clone()), &MIGRATOR)
        .await
        .map_err(|error| error.to_string())?;
    let meeting = MeetingId::new(args.meeting.clone());
    let wiring = wiring(args)?;
    let followed_s = match &args.follow {
        Some(dir) => Some(follow(&wiring, dir).await?),
        None => None,
    };
    let started = Instant::now();
    final_pass(&database, &meeting, &wiring).await?;
    let final_s = started.elapsed().as_secs_f64();
    let diarize_started = Instant::now();
    diarize(&database, &meeting, &wiring).await?;
    Ok(serde_json::json!({
        "followed_s": followed_s,
        "final_transcription_s": final_s,
        "diarize_s": diarize_started.elapsed().as_secs_f64(),
        "total_s": started.elapsed().as_secs_f64(),
    })
    .to_string())
}

fn wiring(args: &Args) -> Result<Wiring, String> {
    let engines = Arc::new(WhisperEngines {
        models: args.models.clone(),
        model: args.whisper.clone(),
        runtime: Handle::current(),
        engine: std::sync::Mutex::new(None),
    });
    let transcriber = OfflineTranscriber::new(engines, Arc::new(NativeDecoder))
        .with_voice_check(Arc::new(DeepFilterCheck));
    let plda =
        Plda::load(&args.speakers.join(bundled::VBX.file_name)).map_err(|e| e.to_string())?;
    let store = ModelStore::new(&args.speakers);
    let diarizer = OnnxTrackDiarizer::new(store, Arc::new(NoDownloads), RuntimeConfig::default())
        .with_vbx(Arc::new(plda))
        .with_scan_cache(Arc::new(ScanCache::default()));
    let aligner = OnnxWordAligner::new(
        args.speakers.join(bundled::ALIGNER.file_name),
        RuntimeConfig::default(),
    )
    .with_memory(Arc::new(LineMemory::default()));
    Ok(Wiring {
        transcriber,
        memory: PassMemory::new(),
        diarizer: Arc::new(diarizer),
        aligner: Arc::new(aligner),
    })
}

/// The last pass during the meeting, over the tracks in `dir`.
async fn follow(wiring: &Wiring, dir: &std::path::Path) -> Result<f64, String> {
    let follower = FollowAlong::new(FollowDeps {
        recorder: Arc::new(NothingRecording),
        recordings: dir.to_owned(),
        transcriber: wiring
            .transcriber
            .clone()
            .with_memory(wiring.memory.following()),
        memory: wiring.memory.clone(),
        aligner: Arc::clone(&wiring.aligner),
        diarizer: Arc::clone(&wiring.diarizer),
        wanted: Arc::new(Always),
        forget: Arc::new(|| {}),
        clock: Arc::new(SystemClock),
    });
    let started = Instant::now();
    let tracks = (Some(dir.join("mic.m4a")), Some(dir.join("system.m4a")));
    follower.pass_over(tracks, true).await?;
    Ok(started.elapsed().as_secs_f64())
}

async fn final_pass(
    database: &Database,
    meeting: &MeetingId,
    wiring: &Wiring,
) -> Result<(), String> {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let service = FinalTranscriptionService::new(FinalTranscriptionDeps {
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments,
        transcriber: Arc::new(
            wiring
                .transcriber
                .clone()
                .with_memory(wiring.memory.clone()),
        ),
        originals: Arc::new(SqliteOriginalTranscripts::new(database.clone())),
        clock: Arc::new(SystemClock),
    });
    let pass = FinalPass {
        repass: true,
        requested: true,
        cloud: None,
        cancel: CancellationToken::new(),
        progress: TrackProgress::new(|_| {}),
    };
    let outcome = service.run(meeting, &pass).await.map_err(|e| chain(&e))?;
    log::info!(target: "final_bench", "final transcription: {outcome:?}");
    Ok(())
}

async fn diarize(database: &Database, meeting: &MeetingId, wiring: &Wiring) -> Result<(), String> {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let service = DiarizeService::new(DiarizeDeps {
        aligner: Arc::clone(&wiring.aligner),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        attendees: Arc::new(SqlitePeopleRepo::new(database.clone())),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments,
        diarizer: Arc::clone(&wiring.diarizer),
        exemplars: Arc::new(SqliteVoiceprintRepo::new(database.clone())),
        clock: Arc::new(SystemClock),
    });
    let outcome = service.run(meeting).await.map_err(|e| chain(&e))?;
    log::info!(target: "final_bench", "diarize: {outcome:?}");
    Ok(())
}

/// An error with its sources.
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
