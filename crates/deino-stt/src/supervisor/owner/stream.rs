//! Stream commands on the owner thread.
//!
//! A feed never respawns the worker: the decoder state would be lost, so a
//! lost stream reports [`SttError::StreamLost`] and finalizes to `None`; the
//! caller re-transcribes the audio it retained.

use super::super::failure::Failure;
use super::{ActiveStream, Owner, Work};
use crate::error::SttError;
use crate::pcm::Millis;
use crate::protocol::{Request, Response, StreamDelta, StreamParams};

impl Owner {
    pub(super) fn stream_begin(
        &mut self,
        id: u64,
        epoch: u64,
        params: StreamParams,
    ) -> Result<(), SttError> {
        self.end_stream();
        if self.shared.cancelled_since(epoch) {
            return Err(SttError::Cancelled);
        }
        self.ensure_worker()?;
        let worker = self.worker.as_mut().ok_or(SttError::NotLoaded)?;
        if !worker.info().is_some_and(|info| info.supports_streaming) {
            return Err(SttError::StreamingUnsupported);
        }
        match worker.call(&Request::StreamBegin(params), &[], self.config.control) {
            Ok(Response::Ok) => {
                self.stream = Some(ActiveStream {
                    id,
                    epoch,
                    pending: Millis::ZERO,
                });
                Ok(())
            }
            Ok(_) => Err(Failure::unexpected("stream begin").into()),
            Err(failure) => {
                if failure.worker_lost() {
                    self.worker_lost("stream begin", &failure);
                }
                Err(failure.into())
            }
        }
    }

    pub(super) fn feed(&mut self, id: u64, pcm: &[f32]) -> Result<StreamDelta, SttError> {
        let Some(stream) = self.stream.as_ref().filter(|stream| stream.id == id) else {
            return Err(SttError::StreamLost);
        };
        let epoch = stream.epoch;
        let deadline = self
            .deadlines()
            .stream(Millis::from_samples(pcm.len()) + stream.pending);
        let work = Work {
            request: &Request::Feed,
            pcm,
            deadline,
            epoch,
        };
        match self.call_work(&work) {
            Ok(Response::Fed(delta)) => {
                if let Some(stream) = self.stream.as_mut() {
                    stream.pending = Millis::from_engine(delta.progress.pending_ms());
                }
                Ok(delta)
            }
            Ok(_) => {
                self.end_stream();
                Err(Failure::unexpected("stream feed").into())
            }
            Err(Failure::Cancelled) => {
                self.stream = None;
                Err(SttError::Cancelled)
            }
            Err(Failure::Remote(error)) => {
                log::warn!("stream feed failed ({error}); closing the stream");
                self.end_stream();
                Err(SttError::StreamLost)
            }
            Err(failure) => {
                self.stream = None;
                self.worker_lost("stream feed", &failure);
                Err(SttError::StreamLost)
            }
        }
    }

    pub(super) fn finalize(
        &mut self,
        id: u64,
        epoch: u64,
    ) -> Result<Option<StreamDelta>, SttError> {
        // Checked first: a stream lost to a cancel reports the cancel, not
        // "no result", which would start a batch run.
        if self.shared.cancelled_since(epoch) {
            self.reset(id);
            return Err(SttError::Cancelled);
        }
        let Some(stream) = self.stream.take_if(|stream| stream.id == id) else {
            return Ok(None);
        };
        let deadline = self.deadlines().stream(stream.pending);
        let work = Work {
            request: &Request::Finalize,
            pcm: &[],
            deadline,
            epoch,
        };
        match self.call_work(&work) {
            Ok(Response::Finalized(delta)) => Ok(Some(delta)),
            Ok(_) => Ok(None),
            Err(Failure::Cancelled) => {
                self.reset_worker_stream();
                Err(SttError::Cancelled)
            }
            Err(Failure::Remote(error)) => {
                log::warn!("stream finalize failed: {error}");
                Ok(None)
            }
            Err(failure) => {
                self.worker_lost("stream finalize", &failure);
                Ok(None)
            }
        }
    }

    /// Abandons stream `id` if it is the open one.
    pub(super) fn reset(&mut self, id: u64) {
        if self.stream.as_ref().is_some_and(|stream| stream.id == id) {
            self.end_stream();
        }
    }

    /// Closes the open stream, if any, in the worker too.
    pub(super) fn end_stream(&mut self) {
        if self.stream.take().is_some() {
            self.reset_worker_stream();
        }
    }

    fn reset_worker_stream(&mut self) {
        let Some(worker) = self.worker.as_mut() else {
            return;
        };
        match worker.call(&Request::StreamReset, &[], self.config.control) {
            Ok(_) => {}
            // Expected when the stream already ended in the worker.
            Err(Failure::Remote(error)) => log::debug!("stream reset: {error}"),
            Err(failure) => self.worker_lost("stream reset", &failure),
        }
    }
}
