//! Cancellable work on the worker: one-shot transcription and GPU pre-warm.

use std::sync::Arc;

use super::super::failure::Failure;
use super::super::lock;
use super::super::process::WorkerProcess;
use super::{Owner, Work};
use crate::error::SttError;
use crate::pcm::Millis;
use crate::protocol::{BatchParams, BatchTranscript, Request, Response};

impl Owner {
    /// A call that `cancel` can stop. `Cancelled` if a cancel came after
    /// `work.epoch`, whatever the worker answered.
    pub(super) fn call_work(&mut self, work: &Work<'_>) -> Result<Response, Failure> {
        let Work {
            request,
            pcm,
            deadline,
            epoch,
        } = *work;
        let worker = self
            .worker
            .as_mut()
            .ok_or_else(|| Failure::Crashed("no worker is running".into()))?;
        let sent = {
            // Check, send and publish under the lock `cancel` takes after
            // bumping the epoch: either this sees the cancel and sends
            // nothing, or the cancel sees the call and kills its worker.
            let mut in_flight = lock(&self.shared.in_flight);
            if self.shared.cancelled_since(epoch) {
                return Err(Failure::Cancelled);
            }
            *in_flight = Some(Arc::clone(worker.control()));
            worker.send(request, pcm)?
        };
        let result = if sent {
            worker.wait(request, deadline)
        } else {
            Err(worker.died())
        };
        // Gone if `cancel` took it, which means it killed the worker.
        let killed = lock(&self.shared.in_flight).take().is_none();
        if killed || self.shared.cancelled_since(epoch) {
            if killed || result.as_ref().is_err_and(Failure::worker_lost) {
                self.set_worker(None);
            }
            return Err(Failure::Cancelled);
        }
        result
    }

    /// One-shot transcription; a lost worker is retried once in a fresh one
    /// (on the CPU if it was on the GPU), but not after a hang on the CPU.
    pub(super) fn transcribe(
        &mut self,
        pcm: &[f32],
        epoch: u64,
    ) -> Result<BatchTranscript, SttError> {
        // An open stream would make the worker refuse the run.
        self.end_stream();
        let request = Request::Run(BatchParams::default());
        let mut retried = false;
        loop {
            if self.shared.cancelled_since(epoch) {
                return Err(SttError::Cancelled);
            }
            self.ensure_worker()?;
            let deadline = self.deadlines().run(Millis::from_samples(pcm.len()));
            let work = Work {
                request: &request,
                pcm,
                deadline,
                epoch,
            };
            match self.call_work(&work) {
                Ok(Response::Transcript(transcript)) => return Ok(transcript),
                Ok(_) => return Err(Failure::unexpected("transcription").into()),
                Err(failure) if failure.worker_lost() && !retried => {
                    let on_gpu = self.worker.as_ref().is_some_and(WorkerProcess::on_gpu);
                    let hung_on_cpu = matches!(failure, Failure::Hung(_)) && !on_gpu;
                    self.worker_lost("transcription", &failure);
                    if hung_on_cpu {
                        return Err(failure.into());
                    }
                    log::warn!("retrying the transcription in a fresh worker");
                    retried = true;
                }
                Err(failure) => return Err(failure.into()),
            }
        }
    }

    /// Opens and closes a model-less worker that opens the GPU.
    pub(super) fn prewarm(&self) -> Result<(), SttError> {
        if !self.config.gpu_allowed || self.gpu_unavailable() {
            return Ok(());
        }
        match self.hello(false, true) {
            Ok(_worker) => Ok(()),
            Err(failure) => {
                if failure.worker_lost() {
                    self.shared
                        .mark_gpu_unavailable(&format!("{failure:?} while opening the GPU"));
                }
                Err(failure.into())
            }
        }
    }
}
