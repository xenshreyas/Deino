//! The parent side: [`EngineSupervisor`], the single owner of one worker.
//!
//! One owner thread works through a command queue and holds the only worker
//! slot, the loaded model's spec and any open stream, so "at most one worker
//! per supervisor" holds by construction. Callers block on their command's
//! reply; waiting in the queue behind a model load is never a timeout. Only
//! [`EngineSupervisor::cancel`] bypasses the queue: it kills the worker doing
//! the work, and the next use starts a fresh one.
//!
//! Faults are absorbed here, so callers never see processes:
//!
//! - a worker that crashes or hangs on the GPU marks the GPU unavailable and
//!   the work is retried once in a CPU-only worker that never loads a GPU
//!   driver (GPU→CPU fallback);
//! - deadlines scale with the audio ([`Deadlines`]); a missed one kills the
//!   worker;
//! - a lost stream is never respawned (its decoder state is gone): its next
//!   feed reports [`SttError::StreamLost`] and the caller transcribes the
//!   audio it retained in one batch run instead.
//!
//! Ported from Handy's `engine_supervisor/supervisor.rs` (MIT), minus device
//! selection, which Deino does not offer.

mod config;
mod failure;
mod launcher;
mod owner;
mod process;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

pub use config::{Deadlines, SupervisorConfig};
pub use launcher::WorkerLauncher;
use process::Control;

use crate::error::SttError;
use crate::protocol::{BackendChoice, BatchTranscript, LoadedInfo, StreamDelta, StreamParams};

/// What to load. Kept by the supervisor, so a dead worker can be replaced
/// transparently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadSpec {
    /// The model file.
    pub path: PathBuf,
    /// Where to run it.
    pub backend: BackendChoice,
}

type Reply<T> = Sender<T>;

/// The owner thread's work queue.
enum Command {
    Load(LoadSpec, Reply<Result<LoadedInfo, SttError>>),
    Unload(Reply<()>),
    Transcribe {
        pcm: Vec<f32>,
        epoch: u64,
        reply: Reply<Result<BatchTranscript, SttError>>,
    },
    StreamBegin {
        id: u64,
        epoch: u64,
        params: StreamParams,
        reply: Reply<Result<(), SttError>>,
    },
    Feed {
        id: u64,
        pcm: Vec<f32>,
        reply: Reply<Result<StreamDelta, SttError>>,
    },
    Finalize {
        id: u64,
        epoch: u64,
        reply: Reply<Result<Option<StreamDelta>, SttError>>,
    },
    StreamReset {
        id: u64,
    },
    Prewarm(Reply<Result<(), SttError>>),
}

/// State readable without going through the queue.
#[derive(Default)]
struct Shared {
    /// A worker with GPU backends crashed or hung: later workers are CPU-only.
    gpu_unavailable: AtomicBool,
    /// The loaded model (it stays loaded while its worker is replaced).
    loaded: Mutex<Option<LoadedInfo>>,
    /// Bumped by every cancel; work queued before the bump is cancelled.
    cancel_epoch: AtomicU64,
    /// The worker the owner is waiting on, which a cancel kills.
    in_flight: Mutex<Option<Arc<Control>>>,
    /// The current worker's process id.
    worker_pid: Mutex<Option<u32>>,
    next_stream: AtomicU64,
}

impl Shared {
    fn epoch(&self) -> u64 {
        self.cancel_epoch.load(Ordering::SeqCst)
    }

    fn cancelled_since(&self, epoch: u64) -> bool {
        self.epoch() != epoch
    }

    fn mark_gpu_unavailable(&self, reason: &str) {
        if !self.gpu_unavailable.swap(true, Ordering::AcqRel) {
            log::warn!("transcription GPU marked unavailable: {reason}");
        }
    }
}

/// Recovers a lock whose holder panicked: the guarded values stay valid.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Owns one transcription worker process. Cheap to share behind an `Arc`.
pub struct EngineSupervisor {
    commands: Sender<Command>,
    shared: Arc<Shared>,
}

impl EngineSupervisor {
    /// Starts the owner thread. No worker runs until something needs one.
    pub fn new(launcher: WorkerLauncher, config: SupervisorConfig) -> Result<Self, SttError> {
        let (commands, queue) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        let owner = owner::Owner::new(Arc::clone(&shared), launcher, config);
        thread::Builder::new()
            .name("stt-supervisor".into())
            .spawn(move || owner.run(&queue))
            .map_err(SttError::Spawn)?;
        Ok(Self { commands, shared })
    }

    fn ask<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> Result<T, SttError> {
        let (reply, answer) = mpsc::channel();
        self.commands
            .send(command(reply))
            .map_err(|_closed| SttError::Stopped)?;
        answer.recv().map_err(|_closed| SttError::Stopped)
    }

    /// Loads a model in a fresh worker, replacing the current one. The old
    /// worker has exited before the new one starts, so two models are never
    /// held at once.
    pub fn load(&self, spec: LoadSpec) -> Result<LoadedInfo, SttError> {
        self.ask(|reply| Command::Load(spec, reply))?
    }

    /// Unloads the model and waits until its worker has exited, which
    /// returns all of its CPU and GPU memory.
    pub fn unload(&self) {
        if self.ask(Command::Unload).is_err() {
            log::debug!("unload after the supervisor stopped");
        }
    }

    /// The loaded model. Never waits on the queue.
    pub fn loaded(&self) -> Option<LoadedInfo> {
        lock(&self.shared.loaded).clone()
    }

    /// The current worker's process id, if one is running.
    pub fn worker_pid(&self) -> Option<u32> {
        *lock(&self.shared.worker_pid)
    }

    /// Whether a GPU fault moved this supervisor to the CPU.
    pub fn gpu_unavailable(&self) -> bool {
        self.shared.gpu_unavailable.load(Ordering::Acquire)
    }

    /// Transcribes 16 kHz mono PCM in one run. A worker crash or hang is
    /// retried once in a fresh worker (on the CPU if it was on the GPU).
    pub fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let epoch = self.shared.epoch();
        self.ask(|reply| Command::Transcribe { pcm, epoch, reply })?
    }

    /// Opens a stream on the loaded model, closing any open one.
    pub fn start_stream(&self, params: StreamParams) -> Result<StreamHandle, SttError> {
        let id = self.shared.next_stream.fetch_add(1, Ordering::Relaxed) + 1;
        let epoch = self.shared.epoch();
        self.ask(|reply| Command::StreamBegin {
            id,
            epoch,
            params,
            reply,
        })??;
        Ok(StreamHandle {
            id,
            epoch,
            commands: self.commands.clone(),
            finished: false,
        })
    }

    /// Starts a model-less worker that opens the GPU, then stops it. The
    /// first GPU open after an install or update compiles ggml's Metal
    /// library (about 15 s); doing it at startup keeps that off the first
    /// recording. A no-op on CPU-only hosts.
    pub fn prewarm(&self) -> Result<(), SttError> {
        self.ask(Command::Prewarm)?
    }

    /// Stops the work in progress and any queued behind it, by killing the
    /// worker doing it. Model loads are never cancelled.
    pub fn cancel(&self) {
        self.shared.cancel_epoch.fetch_add(1, Ordering::SeqCst);
        if let Some(control) = lock(&self.shared.in_flight).take() {
            log::info!(
                "cancelling transcription by stopping worker {}",
                control.pid()
            );
            control.kill();
        }
    }
}

/// An open stream in the worker. Feeds and finalize run in order. Dropping
/// the handle without finalizing abandons the stream.
pub struct StreamHandle {
    id: u64,
    epoch: u64,
    commands: Sender<Command>,
    finished: bool,
}

impl StreamHandle {
    /// Feeds 16 kHz mono audio and returns what it committed.
    /// [`SttError::StreamLost`] means the stream is gone: transcribe the
    /// retained audio in one run and open a new stream.
    pub fn feed(&self, pcm: Vec<f32>) -> Result<StreamDelta, SttError> {
        let (reply, answer) = mpsc::channel();
        let id = self.id;
        self.commands
            .send(Command::Feed { id, pcm, reply })
            .map_err(|_closed| SttError::Stopped)?;
        answer.recv().map_err(|_closed| SttError::Stopped)?
    }

    /// Flushes the stream. `Ok(None)` when it was lost: fall back to a batch
    /// run over the retained audio.
    pub fn finalize(mut self) -> Result<Option<StreamDelta>, SttError> {
        self.finished = true;
        let (reply, answer) = mpsc::channel();
        let (id, epoch) = (self.id, self.epoch);
        if self
            .commands
            .send(Command::Finalize { id, epoch, reply })
            .is_err()
        {
            return Ok(None);
        }
        answer.recv().unwrap_or(Ok(None))
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        if !self.finished
            && self
                .commands
                .send(Command::StreamReset { id: self.id })
                .is_err()
        {
            log::debug!("stream {} dropped after the supervisor stopped", self.id);
        }
    }
}
