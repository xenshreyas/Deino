//! The writer's bounded in-memory backlog.
//!
//! When the encoder is not ready (`isReadyForMoreMediaData` is false, or the
//! disk is slow), placed audio waits here instead of being dropped. Samples
//! are bounded by [`BACKLOG_LIMIT`] (anarlog's 64 MiB, about 5.8 minutes of
//! one 48 kHz mono track). Silence costs nothing, so timeline gaps are queued
//! as a frame count. Past the limit, new audio is replaced by silence of the
//! same length: the timeline stays continuous and the loss is reported.

use std::collections::VecDeque;

use crate::units::{Bytes, Frames};

/// Most sample bytes the backlog holds before new audio becomes silence.
pub const BACKLOG_LIMIT: Bytes = Bytes::mib(64);

/// Bytes one queued sample costs.
const SAMPLE_BYTES: u64 = 4;

/// A run of queued audio.
#[derive(Debug)]
enum Chunk {
    /// Captured samples.
    Samples(Vec<f32>),
    /// A gap the timeline filled with silence.
    Silence(Frames),
}

/// Placed audio waiting for the encoder, in timeline order.
#[derive(Debug)]
pub(crate) struct Backlog {
    chunks: VecDeque<Chunk>,
    sample_bytes: u64,
    queued: Frames,
    limit: Bytes,
}

impl Backlog {
    /// An empty backlog that holds at most `limit` bytes of samples.
    pub(crate) fn new(limit: Bytes) -> Self {
        Self {
            chunks: VecDeque::new(),
            sample_bytes: 0,
            queued: Frames::ZERO,
            limit,
        }
    }

    /// Queues captured samples. Returns the frames that did not fit and were
    /// queued as silence instead (zero when everything fit).
    pub(crate) fn push_samples(&mut self, samples: &[f32]) -> Frames {
        let len = Frames::of_len(samples.len());
        let bytes = len.0 * SAMPLE_BYTES;
        if self.sample_bytes + bytes > self.limit.0 {
            self.push_silence(len);
            return len;
        }
        self.sample_bytes += bytes;
        self.queued += len;
        self.chunks.push_back(Chunk::Samples(samples.to_vec()));
        Frames::ZERO
    }

    /// Queues `frames` of silence.
    pub(crate) fn push_silence(&mut self, frames: Frames) {
        if frames == Frames::ZERO {
            return;
        }
        self.queued += frames;
        match self.chunks.back_mut() {
            Some(Chunk::Silence(run)) => *run += frames,
            _ => self.chunks.push_back(Chunk::Silence(frames)),
        }
    }

    /// Frames queued, silence included.
    pub(crate) fn queued(&self) -> Frames {
        self.queued
    }

    /// Whether nothing is queued.
    pub(crate) fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Moves up to `max` frames from the front into `out` (cleared first).
    /// Returns false when the backlog is empty.
    pub(crate) fn pop_into(&mut self, out: &mut Vec<f32>, max: Frames) -> bool {
        out.clear();
        let Some(front) = self.chunks.front_mut() else {
            return false;
        };
        let emptied = match front {
            Chunk::Samples(samples) => {
                let n = samples.len().min(max.as_len());
                out.extend(samples.drain(..n));
                self.sample_bytes -= Frames::of_len(n).0 * SAMPLE_BYTES;
                samples.is_empty()
            }
            Chunk::Silence(run) => {
                let n = (*run).min(max);
                out.resize(n.as_len(), 0.0);
                *run = *run - n;
                *run == Frames::ZERO
            }
        };
        if emptied {
            self.chunks.pop_front();
        }
        self.queued = self.queued - Frames::of_len(out.len());
        true
    }

    /// Drops everything queued and returns how many frames that was.
    pub(crate) fn clear(&mut self) -> Frames {
        let dropped = self.queued;
        self.chunks.clear();
        self.sample_bytes = 0;
        self.queued = Frames::ZERO;
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_comes_out_in_order_with_gaps_as_zeros() {
        let mut backlog = Backlog::new(BACKLOG_LIMIT);
        backlog.push_samples(&[1.0, 2.0, 3.0]);
        backlog.push_silence(Frames(2));
        backlog.push_samples(&[4.0]);
        let mut out = Vec::new();
        let mut all = Vec::new();

        while backlog.pop_into(&mut out, Frames(2)) {
            all.extend_from_slice(&out);
        }

        assert_eq!(all, vec![1.0, 2.0, 3.0, 0.0, 0.0, 4.0]);
        assert!(backlog.is_empty());
        assert_eq!(backlog.queued(), Frames::ZERO);
    }

    #[test]
    fn audio_past_the_limit_becomes_silence_of_the_same_length() {
        let mut backlog = Backlog::new(Bytes(16));
        assert_eq!(backlog.push_samples(&[0.5; 4]), Frames::ZERO);

        let dropped = backlog.push_samples(&[0.5; 3]);

        assert_eq!(dropped, Frames(3));
        assert_eq!(backlog.queued(), Frames(7));
        let mut out = Vec::new();
        backlog.pop_into(&mut out, Frames(100));
        assert_eq!(out, vec![0.5; 4]);
        assert_eq!(
            backlog.push_samples(&[0.25; 4]),
            Frames::ZERO,
            "popping frees the budget"
        );
    }

    #[test]
    fn adjacent_silence_merges_and_huge_gaps_cost_no_memory() {
        let mut backlog = Backlog::new(Bytes(0));
        backlog.push_silence(Frames(48_000 * 3_600));
        backlog.push_silence(Frames(5));

        assert_eq!(backlog.chunks.len(), 1);
        assert_eq!(backlog.queued(), Frames(48_000 * 3_600 + 5));
        assert_eq!(backlog.clear(), Frames(48_000 * 3_600 + 5));
        assert!(backlog.is_empty());
    }
}
