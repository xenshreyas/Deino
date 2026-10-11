//! `TrackPump`: one track's path from the ring to the encoder.
//!
//! Each poll drains whole blocks from the real-time ring, places them on the
//! timeline (silence for gaps, trimmed overlaps), queues them in the backlog
//! and feeds the encoder while it is ready. It runs on the writer thread
//! only; the real-time side never touches the encoder.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::backlog::{BACKLOG_LIMIT, Backlog};
use super::stall::StallWatch;
use crate::clock::HostClock;
use crate::encoder::{EncodeError, TrackEncoder};
use crate::health::{HealthEvent, HealthSender};
use crate::ring::{BlockStamp, RtSource, TimelineSink};
use crate::timeline::{Placement, TrackTimeline};
use crate::units::{Frames, HostTime, Track};

/// Largest run handed to the encoder in one append (100 ms).
const APPEND_BLOCK: Frames = Frames(4_800);

/// Everything a pump needs; see [`TrackPump::new`].
pub(crate) struct PumpParts {
    /// The track being written.
    pub track: Track,
    /// The writer's end of the track's ring.
    pub source: RtSource,
    /// Where placed audio goes for live transcription (never blocks; a full
    /// ring drops the block, and the reader fills the hole with silence).
    pub placed: TimelineSink,
    /// The recording epoch the timeline is anchored to.
    pub epoch: HostTime,
    /// The track's encoder.
    pub encoder: Box<dyn TrackEncoder>,
    /// The clock stalls are measured on.
    pub clock: Arc<dyn HostClock>,
    /// Where stalls and drops are reported.
    pub health: HealthSender,
    /// Frames handed to the encoder so far, shared with the recorder.
    pub appended: Arc<AtomicU64>,
}

/// What a finished writer produced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WriterReport {
    /// Frames handed to the encoder: the track's length on the timeline.
    pub frames: Frames,
    /// Frames lost to a full ring or backlog (replaced by silence).
    pub dropped: Frames,
}

/// One track's ring → timeline → backlog → encoder path.
pub(crate) struct TrackPump {
    parts: PumpParts,
    timeline: TrackTimeline,
    backlog: Backlog,
    stall: StallWatch,
    block: Vec<f32>,
    chunk: Vec<f32>,
    ring_drops_seen: Frames,
    dropped: Frames,
}

impl std::fmt::Debug for TrackPump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackPump")
            .field("track", &self.parts.track)
            .field("timeline", &self.timeline)
            .field("queued", &self.backlog.queued())
            .finish_non_exhaustive()
    }
}

impl TrackPump {
    /// A pump with an empty timeline anchored at `parts.epoch`.
    pub(crate) fn new(parts: PumpParts) -> Self {
        Self {
            timeline: TrackTimeline::new(parts.epoch),
            backlog: Backlog::new(BACKLOG_LIMIT),
            stall: StallWatch::new(parts.track),
            block: Vec::with_capacity(APPEND_BLOCK.as_len()),
            chunk: Vec::with_capacity(APPEND_BLOCK.as_len()),
            ring_drops_seen: Frames::ZERO,
            dropped: Frames::ZERO,
            parts,
        }
    }

    /// One poll: drain the ring, feed the encoder, report stalls and drops.
    pub(crate) fn poll(&mut self) -> Result<(), EncodeError> {
        self.drain_ring();
        self.settle()
    }

    /// Feeds the encoder and reports stalls and drops, without taking more
    /// audio from the ring (used while draining after a stop).
    pub(crate) fn settle(&mut self) -> Result<(), EncodeError> {
        self.feed()?;
        self.watch();
        Ok(())
    }

    /// Drains the ring, then extends the track with silence up to `stop`,
    /// so both tracks end together. Blocks captured after `stop` are ignored.
    pub(crate) fn pad_to(&mut self, stop: HostTime) {
        self.drain_ring();
        let tail = self.timeline.pad_to(stop);
        self.backlog.push_silence(tail);
    }

    /// Hands queued audio to the encoder for as long as it is ready.
    pub(crate) fn feed(&mut self) -> Result<(), EncodeError> {
        while self.parts.encoder.is_ready() && self.backlog.pop_into(&mut self.chunk, APPEND_BLOCK)
        {
            let at = Frames(self.parts.appended.load(Ordering::Acquire));
            self.parts.encoder.append(&self.chunk, at)?;
            let len = Frames::of_len(self.chunk.len()).0;
            self.parts.appended.fetch_add(len, Ordering::AcqRel);
        }
        Ok(())
    }

    /// Whether everything queued has reached the encoder.
    pub(crate) fn is_drained(&self) -> bool {
        self.backlog.is_empty()
    }

    /// How long queued audio has been waiting for the encoder.
    pub(crate) fn waiting_for(&self) -> Duration {
        self.stall.waiting_for(self.parts.clock.now())
    }

    /// The current host time.
    pub(crate) fn now(&self) -> HostTime {
        self.parts.clock.now()
    }

    /// Gives up on queued audio (the encoder never came back) and reports it.
    pub(crate) fn abandon_backlog(&mut self) {
        let lost = self.backlog.clear();
        self.report_drop(lost);
    }

    /// Finishes the encoder: the last partial segment reaches the sink.
    pub(crate) fn finish(self) -> Result<WriterReport, EncodeError> {
        let frames = Frames(self.parts.appended.load(Ordering::Acquire));
        self.parts.encoder.finish()?;
        Ok(WriterReport {
            frames,
            dropped: self.dropped,
        })
    }

    /// Moves every complete block from the ring onto the timeline.
    fn drain_ring(&mut self) {
        while let Some(stamp) = self.parts.source.pop_block(&mut self.block) {
            self.place(stamp);
        }
    }

    /// Places one block and queues what the timeline keeps.
    fn place(&mut self, stamp: BlockStamp) {
        let skip = match self.timeline.place(stamp.at, Frames(u64::from(stamp.len))) {
            Placement::Contiguous => Frames::ZERO,
            Placement::Gap(gap) => {
                self.backlog.push_silence(gap);
                Frames::ZERO
            }
            Placement::Overlap(overlap) => overlap,
        };
        let kept = self.block.get(skip.as_len()..).unwrap_or_default();
        let position = self.timeline.written() - Frames::of_len(kept.len());
        self.parts.placed.push(kept, position);
        let lost = self.backlog.push_samples(kept);
        self.report_drop(lost);
    }

    /// Reports stalls, and frames the real-time side dropped since last time.
    fn watch(&mut self) {
        let waiting = !self.backlog.is_empty();
        if let Some(event) = self.stall.observe(self.parts.clock.now(), waiting) {
            self.parts.health.emit(event);
        }
        let ring_drops = self.parts.source.counters().dropped();
        let fresh = ring_drops - self.ring_drops_seen;
        self.ring_drops_seen = ring_drops;
        self.report_drop(fresh);
    }

    /// Counts and reports `frames` of lost audio (nothing when zero).
    fn report_drop(&mut self, frames: Frames) {
        if frames == Frames::ZERO {
            return;
        }
        self.dropped += frames;
        self.parts.health.emit(HealthEvent::FramesDropped {
            track: self.parts.track,
            frames,
        });
    }
}

#[cfg(test)]
#[path = "pump_tests.rs"]
mod tests;
