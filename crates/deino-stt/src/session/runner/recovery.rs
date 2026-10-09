//! Lost streams, rotation and the final flush.
//!
//! A lost stream (or a failed finalize) is never fatal: the retained audio is
//! transcribed in one batch run and a new stream opens where the old one
//! stopped. A long stream is finalized and reopened at a silence.

use std::time::Duration;

use super::super::assembler::Assembler;
use super::super::retained::Retained;
use super::{Runner, on_timeline};
use crate::error::SttError;
use crate::pcm::Millis;
use crate::perf::PerfReport;
use crate::session::SegmentOrigin;

impl Runner {
    /// The stream is gone: transcribes the retained audio in one run, then
    /// (when `reopen`) opens a new stream where the old one stopped.
    pub(super) fn recover(&mut self, reopen: bool) {
        self.stream = None;
        self.assembler.discard_open();
        let (start, pcm) = self.retained.take();
        if !pcm.is_empty() {
            self.perf.recovery();
            self.transcribe_retained(start, pcm);
        }
        self.publish_live(Some(""));
        if reopen {
            self.reopen();
        }
    }

    fn transcribe_retained(&mut self, start: Millis, pcm: Vec<f32>) {
        let audio = Millis::from_samples(pcm.len());
        match self.transcriber.transcribe(pcm) {
            Ok(batch) => {
                let mut assembler = Assembler::new(self.config.policy);
                let pushed = assembler.push(on_timeline(&batch.tokens, start));
                for closed in pushed.closed.into_iter().chain(assembler.flush()) {
                    self.emit(&closed, SegmentOrigin::BatchFallback);
                }
            }
            Err(error) => log::error!(
                "lane {}: batch fallback failed ({error}); {} ms of audio has no transcript",
                self.config.label,
                audio.0
            ),
        }
    }

    fn reopen(&mut self) {
        match self.transcriber.open_stream(self.config.params) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.stream_start = self.position();
                self.retained = Retained::at(self.stream_start);
            }
            Err(error) => log::warn!(
                "lane {}: cannot reopen the stream ({error}); transcribing in batches",
                self.config.label
            ),
        }
    }

    /// Finalizes and reopens a long-running stream at a silence.
    pub(super) fn rotate_if_due(&mut self) {
        let streamed = self.position().saturating_sub(self.stream_start);
        let at_silence = self.assembler.open().is_empty();
        let due = streamed >= self.config.rotate_after && at_silence;
        if !due && streamed < self.config.rotate_limit {
            return;
        }
        log::debug!(
            "lane {}: rotating the stream after {} ms",
            self.config.label,
            streamed.0
        );
        self.close_stream();
        self.reopen();
    }

    /// Finalizes the stream; a failed finalize falls back to a batch run.
    fn close_stream(&mut self) {
        let finalized = match self.stream.take() {
            Some(stream) => stream.finalize(),
            None => Ok(None),
        };
        match finalized {
            Ok(Some(delta)) => {
                self.apply(&delta, Millis::ZERO, Duration::ZERO);
                if let Some(closed) = self.assembler.flush() {
                    self.emit(&closed, SegmentOrigin::Stream);
                }
                self.retained = Retained::at(self.position());
            }
            Ok(None) | Err(SttError::StreamLost) => self.recover(false),
            Err(error) => {
                log::warn!("lane {}: finalize failed ({error})", self.config.label);
                self.recover(false);
            }
        }
    }

    pub(super) fn finish(mut self) -> PerfReport {
        self.close_stream();
        if self.retained.duration() > Millis::ZERO {
            self.recover(false);
        }
        self.publish_live(Some(""));
        self.perf.log_summary();
        self.perf.report()
    }
}
