//! [`SttRuntime`]: the app's speech-to-text state.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::thread;
use std::time::Duration;

use deino_core::models::{ModelError, ModelFileId, ModelService};
use deino_recording::engine::{EngineError, LaneRequest, StreamingLane, StreamingParts};
use deino_stt::clock::MonotonicClock;
use deino_stt::{BackendChoice, EnginePool, EngineSlot, LoadSpec, SttError};

/// Engines unused this long are unloaded (Handy's default), which returns
/// their worker's memory. A recording pins its engines, so they never unload.
const IDLE_UNLOAD: Duration = Duration::from_mins(5);

/// How often idle engines are looked for.
const IDLE_CHECK: Duration = Duration::from_secs(10);

/// Speech-to-text setup failures.
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
    /// A model file to use instead of the downloaded one (debug builds).
    pub model_override: Option<PathBuf>,
}

/// The app's speech-to-text state, managed as `Arc<SttRuntime>`.
pub struct SttRuntime {
    pool: Arc<EnginePool>,
    models: Arc<ModelService>,
    monotonic: Arc<dyn MonotonicClock>,
    model_override: Option<PathBuf>,
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
            pool: parts.pool,
            models: parts.models,
            monotonic: parts.monotonic,
            model_override: parts.model_override,
            idle_stop: Mutex::new(None),
        }
    }

    /// The engine pool.
    pub fn pool(&self) -> Arc<EnginePool> {
        Arc::clone(&self.pool)
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

    /// Loads `lane`'s engine (its own worker; blocking, about a second warm)
    /// and opens a stream delivering into `sink`. The lane pins its engine
    /// for the recording.
    pub fn open_lane(&self, request: LaneRequest) -> Result<StreamingLane, EngineError> {
        let lane = request.lane;
        let slot = EngineSlot::Live(lane.engine());
        let (engine, info) = self.pool.engine(slot).map_err(|source| EngineError {
            lane,
            source: Box::new(source),
        })?;
        if !info.on_gpu {
            log::warn!(target: "stt", "the {} lane runs on the CPU ({}); it may fall behind", lane.as_str(), info.backend);
        }
        let pin: Box<dyn Send> = Box::new(self.pool.pin(slot));
        StreamingLane::start(StreamingParts {
            lane,
            transcriber: engine,
            sink: request.sink,
            start: request.start,
            clock: Arc::clone(&self.monotonic),
            keep_alive: vec![pin],
        })
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
