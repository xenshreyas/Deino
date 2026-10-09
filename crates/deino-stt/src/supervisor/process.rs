//! One worker process, owned by a supervisor's thread.
//!
//! Strict request/response. Three helper threads keep the pipes moving: one
//! writes frames to stdin (so a cancel never interleaves with a large frame),
//! one reads responses from stdout, and one drains stderr (a full pipe would
//! block the worker), re-logging worker log lines and keeping the last native
//! lines for crash reports. Dropping a [`WorkerProcess`] closes its stdin,
//! gives it a grace period to exit, kills it otherwise and reaps it. Ported
//! from Handy's `engine_supervisor/supervisor.rs` (MIT).

mod pipes;

use std::collections::VecDeque;
use std::process::Child;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::failure::Failure;
use super::launcher::WorkerLauncher;
use super::lock;
use crate::protocol::{LoadedInfo, Request, Response, frame, request_name};

/// How long to wait for a dead worker's last stderr lines and exit status.
const DEATH_REPORT_WAIT: Duration = Duration::from_secs(1);

/// The parts of a worker other threads touch: its stdin queue and the
/// process itself (to kill it on cancel).
pub(super) struct Control {
    pid: u32,
    child: Mutex<Child>,
    stdin: Mutex<Option<Sender<Vec<u8>>>>,
}

impl Control {
    /// The worker's process id.
    pub(super) fn pid(&self) -> u32 {
        self.pid
    }

    /// Queues a frame for the worker's stdin; false once stdin is closed.
    fn send(&self, frame: Vec<u8>) -> bool {
        lock(&self.stdin)
            .as_ref()
            .is_some_and(|stdin| stdin.send(frame).is_ok())
    }

    /// Closing stdin tells the worker to exit.
    fn close_stdin(&self) {
        lock(&self.stdin).take();
    }

    /// Kills the worker; harmless when it already exited.
    pub(super) fn kill(&self) {
        if let Err(error) = lock(&self.child).kill() {
            log::debug!("killing transcription worker {}: {error}", self.pid);
        }
    }
}

/// A running worker process.
pub(super) struct WorkerProcess {
    control: Arc<Control>,
    responses: Receiver<Response>,
    stderr_done: Receiver<()>,
    native_tail: Arc<Mutex<VecDeque<String>>>,
    shutdown_grace: Duration,
    /// Set once a model is loaded.
    info: Option<LoadedInfo>,
}

impl WorkerProcess {
    /// Starts a worker; a `cpu_only` one never registers a GPU backend.
    pub(super) fn spawn(
        launcher: &WorkerLauncher,
        cpu_only: bool,
        shutdown_grace: Duration,
    ) -> Result<Self, Failure> {
        let mut child = launcher.command(cpu_only).spawn().map_err(Failure::Spawn)?;
        let pipes = pipes::take(&mut child).map_err(Failure::Spawn)?;
        let control = Arc::new(Control {
            pid: child.id(),
            child: Mutex::new(child),
            stdin: Mutex::new(Some(pipes.stdin)),
        });
        log::debug!(
            "started {}transcription worker {}",
            if cpu_only { "CPU-only " } else { "" },
            control.pid
        );
        Ok(Self {
            control,
            responses: pipes.responses,
            stderr_done: pipes.stderr_done,
            native_tail: pipes.native_tail,
            shutdown_grace,
            info: None,
        })
    }

    /// The shared control block, for cancel.
    pub(super) fn control(&self) -> &Arc<Control> {
        &self.control
    }

    /// The loaded model, once a load succeeded.
    pub(super) fn info(&self) -> Option<&LoadedInfo> {
        self.info.as_ref()
    }

    /// Records the loaded model.
    pub(super) fn set_info(&mut self, info: LoadedInfo) {
        self.info = Some(info);
    }

    /// Whether the loaded model runs on a GPU.
    pub(super) fn on_gpu(&self) -> bool {
        self.info.as_ref().is_some_and(|info| info.on_gpu)
    }

    /// Sends a request without waiting; false when the worker is gone.
    pub(super) fn send(&self, request: &Request, pcm: &[f32]) -> Result<bool, Failure> {
        let frame = frame::encode(request, pcm).map_err(|error| {
            Failure::Remote(crate::protocol::RemoteError::new(
                crate::protocol::RemoteErrorKind::Inference,
                format!("cannot encode the request: {error}"),
            ))
        })?;
        Ok(self.control.send(frame))
    }

    /// Sends a request and waits up to `deadline` for its answer. After a
    /// `Crashed` or `Hung` failure the process is dead and should be dropped.
    pub(super) fn call(
        &mut self,
        request: &Request,
        pcm: &[f32],
        deadline: Duration,
    ) -> Result<Response, Failure> {
        if !self.send(request, pcm)? {
            return Err(self.died());
        }
        self.wait(request, deadline)
    }

    /// Waits for the answer to `request`, already sent.
    pub(super) fn wait(
        &mut self,
        request: &Request,
        deadline: Duration,
    ) -> Result<Response, Failure> {
        match self.responses.recv_timeout(deadline) {
            Ok(Response::Error(error)) => Err(Failure::Remote(error)),
            Ok(response) => Ok(response),
            Err(RecvTimeoutError::Timeout) => {
                self.control.kill();
                log::error!(
                    "transcription worker {} did not answer {} within {deadline:?}; killed it{}",
                    self.control.pid,
                    request_name(request),
                    self.tail_report()
                );
                Err(Failure::Hung(deadline))
            }
            Err(RecvTimeoutError::Disconnected) => Err(self.died()),
        }
    }

    /// The worker closed its stdout: collect its exit status and last output.
    pub(super) fn died(&mut self) -> Failure {
        // Let the stderr thread catch the final lines (the abort message).
        let _stderr_closed = self.stderr_done.recv_timeout(DEATH_REPORT_WAIT);
        let status = {
            let mut child = lock(&self.control.child);
            if let Ok(Some(status)) = child.try_wait() {
                status.to_string()
            } else {
                let _killed = child.kill();
                child
                    .wait()
                    .map_or_else(|error| error.to_string(), |status| status.to_string())
            }
        };
        log::error!(
            "transcription worker {} died: {status}{}",
            self.control.pid,
            self.tail_report()
        );
        let last = lock(&self.native_tail).back().cloned();
        Failure::Crashed(last.map_or_else(|| status.clone(), |line| format!("{status}: {line}")))
    }

    /// Recent raw native output, for crash and hang logs.
    fn tail_report(&self) -> String {
        let tail = lock(&self.native_tail);
        if tail.is_empty() {
            return String::new();
        }
        let lines: Vec<&str> = tail.iter().map(String::as_str).collect();
        format!(". Native output before it:\n{}", lines.join("\n"))
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        self.control.close_stdin();
        // The response channel disconnects when the worker closes stdout,
        // which it does by exiting.
        let exited = wait_for_disconnect(&self.responses, self.shutdown_grace);
        let mut child = lock(&self.control.child);
        if !exited {
            log::warn!(
                "transcription worker {} did not exit within {:?}; killing it",
                self.control.pid,
                self.shutdown_grace
            );
            let _killed = child.kill();
        }
        if let Err(error) = child.wait() {
            log::warn!(
                "could not reap transcription worker {}: {error}",
                self.control.pid
            );
        }
    }
}

/// Waits up to `grace` for `responses` to disconnect, dropping stray answers.
fn wait_for_disconnect(responses: &Receiver<Response>, grace: Duration) -> bool {
    loop {
        match responses.recv_timeout(grace) {
            Ok(_stray) => {}
            Err(RecvTimeoutError::Disconnected) => return true,
            Err(RecvTimeoutError::Timeout) => return false,
        }
    }
}
