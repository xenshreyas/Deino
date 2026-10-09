//! [`SttRuntime`]: the app's speech-to-text state.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::thread;
use std::time::Duration;

use deino_core::models::{ModelError, ModelFileId, ModelService};
use deino_domain::Clock;
use deino_stt::clock::MonotonicClock;
use deino_stt::{BackendChoice, EnginePool, EngineSlot, Lane, LoadSpec, SessionSink, SttError};
use tauri::{AppHandle, Runtime};

use super::Status;
use super::live::sink::{SinkParts, TauriLiveSink};
use super::live::{LiveAudioSink, LiveStart, LiveTranscription};
use super::status::TranscriptionActivity;

/// Engines unused this long are unloaded (Handy's default), which returns
/// their worker's memory. A recording pins its engine, so it never unloads.
const IDLE_UNLOAD: Duration = Duration::from_mins(5);

/// How often idle engines are looked for.
const IDLE_CHECK: Duration = Duration::from_secs(10);

/// Speech-to-text setup failures, as legacy callers see them.
#[derive(Debug, thiserror::Error)]
pub enum SttSetupError {
    /// The model is unknown or not downloaded.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// The engine failed to start or load.
    #[error(transparent)]
    Engine(#[from] SttError),
}

/// What the runtime is built from.
pub struct SttParts {
    /// The engines.
    pub pool: Arc<EnginePool>,
    /// The downloadable models.
    pub models: Arc<ModelService>,
    /// Measures latency for `StreamPerf`.
    pub monotonic: Arc<dyn MonotonicClock>,
    /// Stamps wall-clock times on transcript lines.
    pub clock: Arc<dyn Clock>,
    /// A model file to use instead of the downloaded one (debug builds).
    pub model_override: Option<PathBuf>,
}

/// The app's speech-to-text state, managed as `Arc<SttRuntime>`.
pub struct SttRuntime {
    pool: Arc<EnginePool>,
    models: Arc<ModelService>,
    monotonic: Arc<dyn MonotonicClock>,
    clock: Arc<dyn Clock>,
    model_override: Option<PathBuf>,
    activity: Arc<TranscriptionActivity>,
    live: Mutex<Option<Arc<LiveTranscription>>>,
    sequence: Arc<AtomicU64>,
    /// Dropping it stops the idle-unload thread.
    idle_stop: Mutex<Option<Sender<()>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl SttRuntime {
    /// A runtime over its parts. Call [`Self::start_background`] once.
    pub fn new(parts: SttParts) -> Self {
        Self {
            activity: Arc::new(TranscriptionActivity::new(Arc::clone(&parts.monotonic))),
            pool: parts.pool,
            models: parts.models,
            monotonic: parts.monotonic,
            clock: parts.clock,
            model_override: parts.model_override,
            live: Mutex::new(None),
            sequence: Arc::new(AtomicU64::new(0)),
            idle_stop: Mutex::new(None),
        }
    }

    /// The engine pool.
    pub fn pool(&self) -> Arc<EnginePool> {
        Arc::clone(&self.pool)
    }

    /// The shared activity counters.
    pub fn activity(&self) -> &Arc<TranscriptionActivity> {
        &self.activity
    }

    /// Points every engine at `model` (or the recommended file when empty).
    /// Fails when it is not downloaded.
    pub async fn select_model(&self, model: &str) -> Result<ModelFileId, SttSetupError> {
        let id = if model.trim().is_empty() {
            self.models.recommended()?
        } else {
            ModelFileId::new(model.trim())
        };
        let path = match &self.model_override {
            Some(path) => path.clone(),
            None => self.models.ready_path(&id).await?,
        };
        self.pool.set_model(Some(LoadSpec {
            path,
            backend: BackendChoice::Auto,
        }));
        Ok(id)
    }

    /// A sink that emits a lane's text from `app`.
    pub fn live_sink<R: Runtime>(&self, app: AppHandle<R>) -> Box<dyn SessionSink> {
        Box::new(TauriLiveSink::new(SinkParts {
            app,
            lane: Lane::Mixed,
            clock: Arc::clone(&self.clock),
            sequence: Arc::clone(&self.sequence),
            activity: Arc::clone(&self.activity),
        }))
    }

    /// Loads the live engine (blocking: about a second warm) and starts the
    /// lane, finishing any lane left over from an earlier recording.
    pub fn start_live(
        &self,
        sink: Box<dyn SessionSink>,
    ) -> Result<Arc<dyn LiveAudioSink>, SttSetupError> {
        self.finish_live();
        let slot = EngineSlot::Live(Lane::Mixed);
        let (engine, info) = self.pool.engine(slot)?;
        if !info.on_gpu {
            log::warn!(target: "stt", "live transcription runs on the CPU ({}); it may fall behind", info.backend);
        }
        let live = Arc::new(LiveTranscription::start(LiveStart {
            engine,
            sink,
            clock: Arc::clone(&self.monotonic),
            pin: self.pool.pin(slot),
            job: self.activity.begin_job(),
        })?);
        *lock(&self.live) = Some(Arc::clone(&live));
        Ok(live)
    }

    /// Finishes the live lane, if any (blocking until its last segment).
    pub fn finish_live(&self) {
        let Some(live) = lock(&self.live).take() else {
            return;
        };
        if let Some(report) = live.finish() {
            log::info!(
                target: "stt",
                "live lane finished: audio {:.1} s, RTF {:.3}, first text p50 {:?} ms, {} recoveries",
                report.audio_s,
                report.rtf,
                report.first_text_p50_ms,
                report.recoveries
            );
        }
    }

    /// What transcription is doing now; live backlog counts in seconds.
    pub fn status(&self) -> Status {
        let snapshot = self.activity.snapshot();
        let backlog = lock(&self.live).as_ref().map_or(0, |live| live.backlog().0);
        let backlog_seconds = usize::try_from(backlog.div_ceil(1000)).unwrap_or(usize::MAX);
        Status {
            queued: snapshot.queued + backlog_seconds,
            processing: snapshot.processing,
            idle_ms: u64::try_from(snapshot.idle.as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// Starts the idle-unload watcher and pre-warms the GPU shaders.
    pub fn start_background(&self) -> std::io::Result<()> {
        let (stop, stopped) = mpsc::channel();
        *lock(&self.idle_stop) = Some(stop);
        let pool = Arc::downgrade(&self.pool);
        thread::Builder::new()
            .name("stt-idle-unload".into())
            .spawn(move || unload_idle_engines(&pool, &stopped))?;
        let pool = Arc::clone(&self.pool);
        thread::Builder::new()
            .name("stt-prewarm".into())
            .spawn(move || {
                if let Err(error) = pool.prewarm() {
                    log::warn!(target: "stt", "GPU pre-warm failed: {error}");
                }
            })?;
        Ok(())
    }
}

/// Every [`IDLE_CHECK`], unloads engines idle for [`IDLE_UNLOAD`], until the
/// runtime (and with it `stop`'s sender) goes away.
fn unload_idle_engines(pool: &Weak<EnginePool>, stop: &Receiver<()>) {
    while let Err(RecvTimeoutError::Timeout) = stop.recv_timeout(IDLE_CHECK) {
        let Some(pool) = pool.upgrade() else { return };
        let unloaded = pool.unload_idle(IDLE_UNLOAD);
        if unloaded > 0 {
            log::info!(target: "stt", "unloaded {unloaded} idle transcription engines");
        }
    }
}
