//! A watched folder end to end over real SQLite and real decoding: files
//! land in a temporary folder, `WatchService` imports them through
//! `ImportService` and `NativeIngest`, and an observer stands in for the
//! pipeline: when a meeting enters `processing` it runs the
//! `final_transcription` rules (CAP-08's `OfflineTranscriber` over the
//! given batch engine, decoding `audio.m4a` with `ExtAudioFile`) and moves
//! the meeting to `ready`, or `failed`.
#![allow(dead_code, reason = "the test and the bench use different parts")]
#![allow(
    clippy::disallowed_methods,
    clippy::panic,
    reason = "the bench reads wall time through `Timings`, and a broken rig fails by panicking"
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use async_trait::async_trait;
use deino_core::final_transcription::{
    FinalOutcome, FinalPass, FinalTranscriptionDeps, FinalTranscriptionService, TrackError,
};
use deino_core::import::{
    FileImporter, ImportDeps, ImportError, ImportRequest, ImportService, ImportedMeeting,
};
use deino_core::meetings::{
    MeetingLifecycleService, MeetingStatusObserver, SqliteMeetingRepo, StatusChange,
};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::watch::{LocalWatchFs, SqliteWatchRepo, WatchDeps, WatchService};
use deino_db::Database;
use deino_domain::{Clock, MeetingEvent, MeetingId, MeetingStatus};
use deino_recording::{BatchEngines, NativeDecoder, NativeIngest, OfflineTranscriber};
use deino_stt::BatchTranscriber;
use deino_stt::transcriber::FakeTranscriber;
use tokio_util::sync::CancellationToken;

/// The scripted batch engine (`deino-stt`'s `FakeTranscriber`).
pub struct ScriptedBatch;

impl BatchEngines for ScriptedBatch {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        Ok(Arc::new(FakeTranscriber::new()))
    }
}

/// When the import and the transcription started and ended.
#[derive(Debug, Default)]
pub struct Timings {
    marks: Mutex<Vec<Instant>>,
}

impl Timings {
    fn mark(&self) {
        self.marks.lock().unwrap().push(Instant::now());
    }

    /// (import start, import end) and (transcription start, end).
    pub fn spans(&self) -> ((Instant, Instant), (Instant, Instant)) {
        let marks = self.marks.lock().unwrap();
        let [import, processing, done] = marks[..] else {
            panic!("expected three marks, got {}", marks.len());
        };
        ((import, processing), (processing, done))
    }
}

/// Marks when each import starts.
struct TimedImporter {
    inner: Arc<ImportService>,
    timings: Arc<Timings>,
}

#[async_trait]
impl FileImporter for TimedImporter {
    async fn import(&self, request: &ImportRequest) -> Result<ImportedMeeting, ImportError> {
        self.timings.mark();
        self.inner.import(request).await
    }
}

/// The rig's parts.
pub struct ImportRig {
    pub database: Database,
    pub watch: Arc<WatchService>,
    pub imports: Arc<ImportService>,
    pub meetings: Arc<SqliteMeetingRepo>,
    pub segments: Arc<SqliteSegmentRepo>,
    /// The watched folder.
    pub inbox: PathBuf,
    /// Where meeting folders go.
    pub recordings: PathBuf,
    /// The import's and the transcription's timing.
    pub timings: Arc<Timings>,
}

/// A rig over `database`, with folders under `root`, transcribing with
/// `engines` and timing with `clock`.
pub async fn import_rig(
    database: Database,
    root: &Path,
    engines: Arc<dyn BatchEngines>,
    clock: Arc<dyn Clock>,
) -> ImportRig {
    let (inbox, recordings) = (root.join("inbox"), root.join("recordings"));
    std::fs::create_dir_all(&inbox).unwrap();
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let timings = Arc::new(Timings::default());
    let lifecycle = lifecycle(&meetings, &segments, (engines, &timings), &clock);
    let imports = Arc::new(ImportService::new(ImportDeps {
        ingest: Arc::new(NativeIngest::new()),
        meetings: meetings.clone(),
        lifecycle,
        recordings: recordings.clone(),
        clock: clock.clone(),
    }));
    let watch = Arc::new(WatchService::new(WatchDeps {
        repo: Arc::new(SqliteWatchRepo::new(database.clone())),
        fs: Arc::new(LocalWatchFs),
        importer: Arc::new(TimedImporter {
            inner: imports.clone(),
            timings: timings.clone(),
        }),
        clock,
    }));
    ImportRig {
        database,
        watch,
        imports,
        meetings,
        segments,
        inbox,
        recordings,
        timings,
    }
}

/// The lifecycle service, with the pipeline's stand-in subscribed.
fn lifecycle(
    meetings: &Arc<SqliteMeetingRepo>,
    segments: &Arc<SqliteSegmentRepo>,
    (engines, timings): (Arc<dyn BatchEngines>, &Arc<Timings>),
    clock: &Arc<dyn Clock>,
) -> Arc<MeetingLifecycleService> {
    let lifecycle = Arc::new(MeetingLifecycleService::new(
        meetings.clone(),
        clock.clone(),
    ));
    lifecycle.subscribe(Arc::new(Transcribe {
        service: final_service(meetings, segments, engines, clock),
        lifecycle: Arc::downgrade(&lifecycle),
        timings: timings.clone(),
    }));
    lifecycle
}

fn final_service(
    meetings: &Arc<SqliteMeetingRepo>,
    segments: &Arc<SqliteSegmentRepo>,
    engines: Arc<dyn BatchEngines>,
    clock: &Arc<dyn Clock>,
) -> FinalTranscriptionService {
    FinalTranscriptionService::new(FinalTranscriptionDeps {
        meetings: meetings.clone(),
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments.clone(),
        transcriber: Arc::new(OfflineTranscriber::new(engines, Arc::new(NativeDecoder))),
        originals: Arc::new(deino_core::final_transcription::FakeOriginalTranscripts::default()),
        clock: clock.clone(),
    })
}

/// The pipeline's stand-in: transcribes, then completes the meeting.
struct Transcribe {
    service: FinalTranscriptionService,
    lifecycle: Weak<MeetingLifecycleService>,
    timings: Arc<Timings>,
}

#[async_trait]
impl MeetingStatusObserver for Transcribe {
    async fn status_changed(&self, id: &MeetingId, change: StatusChange) {
        if change.to != MeetingStatus::Processing {
            return;
        }
        self.timings.mark();
        let pass = FinalPass {
            repass: false,
            cloud: None,
            cancel: CancellationToken::new(),
            progress: deino_core::final_transcription::TrackProgress::default(),
        };
        let outcome = self.service.run(id, &pass).await;
        self.timings.mark();
        let event = match outcome {
            Ok(FinalOutcome::Written { .. }) => MeetingEvent::Complete,
            outcome => {
                eprintln!("final_transcription for {id}: {outcome:?}");
                MeetingEvent::Fail
            }
        };
        if let Some(lifecycle) = self.lifecycle.upgrade() {
            lifecycle.transition(id, event).await.unwrap();
        }
    }
}
