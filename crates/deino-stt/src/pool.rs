//! One engine (worker process) per lane, plus one for batch work.
//!
//! A loaded model serves one stream at a time (transcribe-cpp returns `Busy`
//! for a second), so the microphone and system lanes (CAP-02) each get their
//! own worker and model: about 2 GB of unified memory for two `Q8_0` workers,
//! which is why Macs under 32 GB default to `Q5_K_M` (`deino-core::models`).
//! Import and retranscription use a separate batch engine, so they never
//! interrupt a live stream.
//!
//! Engines start lazily and unload after a stretch of idleness, which kills
//! their worker and returns all of its memory. A live session pins its
//! engine, so it is never unloaded mid-recording.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::clock::MonotonicClock;
use crate::error::SttError;
use crate::protocol::LoadedInfo;
use crate::supervisor::{EngineSupervisor, LoadSpec, SupervisorConfig, WorkerLauncher};

/// An audio lane, matching `transcript_segments.channel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lane {
    /// The user's microphone.
    Microphone,
    /// The other side of the call (system audio).
    System,
    /// Both mixed, until CAP-02 splits the live pipeline.
    Mixed,
}

impl Lane {
    /// The lane's name in logs and on segments.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Microphone => "mic",
            Self::System => "system",
            Self::Mixed => "mixed",
        }
    }
}

/// Which engine a caller wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EngineSlot {
    /// The engine streaming a live lane.
    Live(Lane),
    /// The engine for import, retranscription and benchmarks.
    Batch,
}

/// An engine and its bookkeeping.
struct Slot {
    engine: Arc<EngineSupervisor>,
    /// The model the engine was asked to load.
    loaded: Option<LoadSpec>,
    last_used: Duration,
    pins: usize,
}

/// A slot's engine with the model it should and does hold.
struct Claim {
    engine: Arc<EngineSupervisor>,
    wanted: LoadSpec,
    loaded: Option<LoadSpec>,
}

#[derive(Default)]
struct Inner {
    model: Option<LoadSpec>,
    slots: HashMap<EngineSlot, Slot>,
}

/// The app's engines.
pub struct EnginePool {
    launcher: WorkerLauncher,
    config: SupervisorConfig,
    clock: Arc<dyn MonotonicClock>,
    inner: Mutex<Inner>,
}

/// Keeps an engine loaded while held (a live session holds one).
pub struct PinGuard {
    pool: Arc<EnginePool>,
    slot: EngineSlot,
}

impl Drop for PinGuard {
    fn drop(&mut self) {
        let mut inner = self.pool.lock();
        let now = self.pool.clock.now();
        if let Some(slot) = inner.slots.get_mut(&self.slot) {
            slot.pins = slot.pins.saturating_sub(1);
            slot.last_used = now;
        }
    }
}

impl EnginePool {
    /// A pool that starts workers with `launcher`.
    pub fn new(
        launcher: WorkerLauncher,
        config: SupervisorConfig,
        clock: Arc<dyn MonotonicClock>,
    ) -> Self {
        Self {
            launcher,
            config,
            clock,
            inner: Mutex::new(Inner::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the model every engine runs. Engines switch on their next use.
    pub fn set_model(&self, model: Option<LoadSpec>) {
        self.lock().model = model;
    }

    /// The model engines run.
    pub fn model(&self) -> Option<LoadSpec> {
        self.lock().model.clone()
    }

    /// The engine for `slot`, with the current model loaded (loading it on
    /// the caller's thread when needed: about a second, longer on first use).
    pub fn engine(
        &self,
        slot: EngineSlot,
    ) -> Result<(Arc<EngineSupervisor>, LoadedInfo), SttError> {
        let Claim {
            engine,
            wanted,
            loaded,
        } = self.claim(slot)?;
        let info = match engine.loaded() {
            Some(info) if loaded.as_ref() == Some(&wanted) => info,
            _ => {
                let info = engine.load(wanted.clone())?;
                if let Some(slot) = self.lock().slots.get_mut(&slot) {
                    slot.loaded = Some(wanted);
                }
                info
            }
        };
        Ok((engine, info))
    }

    /// Finds or creates the slot's engine and marks it used.
    fn claim(&self, slot: EngineSlot) -> Result<Claim, SttError> {
        let now = self.clock.now();
        let mut inner = self.lock();
        let wanted = inner.model.clone().ok_or(SttError::NotLoaded)?;
        let entry = match inner.slots.entry(slot) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Slot {
                engine: Arc::new(EngineSupervisor::new(self.launcher.clone(), self.config)?),
                loaded: None,
                last_used: now,
                pins: 0,
            }),
        };
        entry.last_used = now;
        Ok(Claim {
            engine: Arc::clone(&entry.engine),
            wanted,
            loaded: entry.loaded.clone(),
        })
    }

    /// Keeps `slot`'s engine loaded until the guard drops.
    pub fn pin(self: &Arc<Self>, slot: EngineSlot) -> PinGuard {
        if let Some(entry) = self.lock().slots.get_mut(&slot) {
            entry.pins += 1;
        }
        PinGuard {
            pool: Arc::clone(self),
            slot,
        }
    }

    /// Unloads engines unused for `idle` and not pinned; returns how many.
    pub fn unload_idle(&self, idle: Duration) -> usize {
        let now = self.clock.now();
        let idle_engines: Vec<Arc<EngineSupervisor>> = {
            let mut inner = self.lock();
            let mut engines = Vec::new();
            for slot in inner.slots.values_mut() {
                let unused = now.saturating_sub(slot.last_used) >= idle;
                if unused && slot.pins == 0 && slot.loaded.take().is_some() {
                    engines.push(Arc::clone(&slot.engine));
                }
            }
            engines
        };
        for engine in &idle_engines {
            engine.unload();
        }
        idle_engines.len()
    }

    /// Unloads every engine (the app is quitting, or the model was deleted).
    pub fn unload_all(&self) {
        let engines: Vec<Arc<EngineSupervisor>> = {
            let mut inner = self.lock();
            inner
                .slots
                .values_mut()
                .map(|slot| {
                    slot.loaded = None;
                    Arc::clone(&slot.engine)
                })
                .collect()
        };
        for engine in engines {
            engine.unload();
        }
    }

    /// Opens the GPU once in a short-lived worker, so its shaders are
    /// compiled before the first recording. Blocks; call it off the UI path.
    pub fn prewarm(&self) -> Result<(), SttError> {
        EngineSupervisor::new(self.launcher.clone(), self.config)?.prewarm()
    }

    /// Process ids of the running workers, for CPU accounting.
    pub fn worker_pids(&self) -> Vec<u32> {
        self.lock()
            .slots
            .values()
            .filter_map(|slot| slot.engine.worker_pid())
            .collect()
    }
}
