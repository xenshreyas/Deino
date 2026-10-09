//! The owner thread: runs commands one at a time against the worker slot.
//!
//! The model lifecycle is here; cancellable work (one-shot runs, GPU
//! pre-warm) is in `owner/work.rs` and stream commands in `owner/stream.rs`.

mod stream;
mod work;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use super::config::{Deadlines, SupervisorConfig};
use super::failure::Failure;
use super::launcher::WorkerLauncher;
use super::process::WorkerProcess;
use super::{Command, LoadSpec, Shared, lock};
use crate::error::SttError;
use crate::pcm::Millis;
use crate::protocol::{BackendChoice, LoadRequest, LoadedInfo, Request, Response};

/// One cancellable call: what to send and how long it may take.
#[derive(Clone, Copy)]
struct Work<'a> {
    request: &'a Request,
    pcm: &'a [f32],
    deadline: std::time::Duration,
    /// Cancel epoch the work belongs to.
    epoch: u64,
}

/// The open stream, as the owner tracks it.
struct ActiveStream {
    id: u64,
    /// Cancel epoch when the stream started; a later cancel stops it.
    epoch: u64,
    /// Audio received but not yet decoded, which sizes the next deadline.
    pending: Millis,
}

/// The single owner of a supervisor's worker slot.
pub(super) struct Owner {
    shared: Arc<Shared>,
    launcher: WorkerLauncher,
    config: SupervisorConfig,
    worker: Option<WorkerProcess>,
    /// The model the app asked for. Outlives its worker: after a crash, hang
    /// or cancel the next use starts a fresh worker for it.
    spec: Option<LoadSpec>,
    stream: Option<ActiveStream>,
}

/// Sends a reply. The caller may have given up waiting; that is not an error.
fn answer<T>(reply: &Sender<T>, value: T) {
    if reply.send(value).is_err() {
        log::debug!("a transcription caller stopped waiting for its answer");
    }
}

impl Owner {
    pub(super) fn new(
        shared: Arc<Shared>,
        launcher: WorkerLauncher,
        config: SupervisorConfig,
    ) -> Self {
        Self {
            shared,
            launcher,
            config,
            worker: None,
            spec: None,
            stream: None,
        }
    }

    /// Serves commands until every supervisor handle is gone.
    pub(super) fn run(mut self, queue: &Receiver<Command>) {
        while let Ok(command) = queue.recv() {
            self.handle(command);
        }
        self.unload();
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Load(spec, reply) => answer(&reply, self.load(spec)),
            Command::Unload(reply) => {
                self.unload();
                answer(&reply, ());
            }
            Command::Transcribe { pcm, epoch, reply } => {
                answer(&reply, self.transcribe(&pcm, epoch));
            }
            Command::StreamBegin {
                id,
                epoch,
                params,
                reply,
            } => answer(&reply, self.stream_begin(id, epoch, params)),
            Command::Feed { id, pcm, reply } => answer(&reply, self.feed(id, &pcm)),
            Command::Finalize { id, epoch, reply } => answer(&reply, self.finalize(id, epoch)),
            Command::StreamReset { id } => self.reset(id),
            Command::Prewarm(reply) => answer(&reply, self.prewarm()),
        }
    }

    fn load(&mut self, spec: LoadSpec) -> Result<LoadedInfo, SttError> {
        self.unload();
        self.spec = Some(spec);
        let started = self.start();
        if started.is_err() {
            self.spec = None;
        }
        started
    }

    fn unload(&mut self) {
        // The worker's exit ends any stream; no need to reset it first.
        self.stream = None;
        self.set_worker(None);
        self.spec = None;
        *lock(&self.shared.loaded) = None;
    }

    /// Replaces the worker slot; dropping the old worker reaps it.
    fn set_worker(&mut self, worker: Option<WorkerProcess>) {
        *lock(&self.shared.worker_pid) = worker.as_ref().map(|worker| worker.control().pid());
        self.worker = worker;
    }

    /// Whether a worker for `spec` registers only the CPU backends.
    fn cpu_only(&self, spec: &LoadSpec) -> bool {
        spec.backend == BackendChoice::Cpu || !self.config.gpu_allowed
    }

    /// Starts a worker, says hello and loads `spec` in it.
    fn start_worker(&self, spec: &LoadSpec) -> Result<WorkerProcess, Failure> {
        let mut worker = self.hello(self.cpu_only(spec), false)?;
        let load = Request::Load(LoadRequest {
            path: spec.path.clone(),
            backend: spec.backend,
        });
        match worker.call(&load, &[], self.config.load)? {
            Response::Loaded(info) => worker.set_info(info),
            _ => return Err(Failure::unexpected("model load")),
        }
        Ok(worker)
    }

    fn hello(&self, cpu_only: bool, warm_gpu: bool) -> Result<WorkerProcess, Failure> {
        let mut worker =
            WorkerProcess::spawn(&self.launcher, cpu_only, self.config.shutdown_grace)?;
        match worker.call(&Request::Hello { warm_gpu }, &[], self.config.hello)? {
            Response::Hello => Ok(worker),
            _ => Err(Failure::unexpected("hello")),
        }
    }

    /// Starts a worker for the current spec. CPU while the GPU is marked
    /// unavailable; if a GPU-capable worker is lost on the way (backend init
    /// or load), marks the GPU unavailable and retries once on the CPU.
    fn start(&mut self) -> Result<LoadedInfo, SttError> {
        let spec = self.spec.clone().ok_or(SttError::NotLoaded)?;
        let wanted = if self.gpu_unavailable() {
            on_cpu(&spec)
        } else {
            spec
        };
        let worker = match self.start_worker(&wanted) {
            Ok(worker) => worker,
            Err(failure) if failure.worker_lost() && !self.cpu_only(&wanted) => {
                self.shared
                    .mark_gpu_unavailable(&format!("a GPU worker {failure:?} while loading"));
                log::warn!("retrying the model load in a CPU-only worker");
                self.start_worker(&on_cpu(&wanted))?
            }
            Err(failure) => return Err(failure.into()),
        };
        let info = worker.info().cloned().ok_or(SttError::NotLoaded)?;
        *lock(&self.shared.loaded) = Some(info.clone());
        self.set_worker(Some(worker));
        Ok(info)
    }

    fn gpu_unavailable(&self) -> bool {
        self.shared
            .gpu_unavailable
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Makes sure a worker holds the model, starting a fresh one after a
    /// crash or cancel. When the model cannot come back, it reads unloaded.
    fn ensure_worker(&mut self) -> Result<(), SttError> {
        if self.worker.is_some() {
            return Ok(());
        }
        if self.spec.is_none() {
            return Err(SttError::NotLoaded);
        }
        log::info!("starting a fresh transcription worker for the loaded model");
        self.start().map(drop).inspect_err(|_| {
            self.spec = None;
            *lock(&self.shared.loaded) = None;
        })
    }

    /// Drops a worker that crashed or hung. If it ran on a GPU, later workers
    /// use the CPU.
    fn worker_lost(&mut self, during: &str, failure: &Failure) {
        let on_gpu = self.worker.as_ref().is_some_and(WorkerProcess::on_gpu);
        self.set_worker(None);
        if on_gpu {
            self.shared
                .mark_gpu_unavailable(&format!("{failure:?} during {during} on the GPU"));
        }
    }

    /// Decoding deadlines for the device the current model runs on.
    fn deadlines(&self) -> Deadlines {
        let on_gpu = self.worker.as_ref().is_some_and(WorkerProcess::on_gpu);
        self.config.deadlines(on_gpu)
    }
}

/// `spec` on the CPU.
fn on_cpu(spec: &LoadSpec) -> LoadSpec {
    LoadSpec {
        path: spec.path.clone(),
        backend: BackendChoice::Cpu,
    }
}
