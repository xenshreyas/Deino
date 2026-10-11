//! An in-memory encoder for tests of the writer, the persister and the recorder.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::{EncodeError, EncoderFactory, SegmentOutput, TrackEncoder};
use crate::units::{Frames, Track};

/// What the fake encoders of one factory have received.
#[derive(Debug, Default)]
struct Received {
    samples: BTreeMap<Track, Vec<f32>>,
    finished: BTreeMap<Track, bool>,
}

/// Opens encoders that keep every sample, emit one "segment" per append
/// (the raw PCM bytes) and can be told to refuse input, like a stalled
/// `AVAssetWriterInput`.
#[derive(Clone, Debug)]
pub struct FakeEncoderFactory {
    received: Arc<Mutex<Received>>,
    ready: Arc<AtomicBool>,
}

impl Default for FakeEncoderFactory {
    fn default() -> Self {
        Self {
            received: Arc::default(),
            ready: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl FakeEncoderFactory {
    /// Makes every encoder accept (`true`) or refuse (`false`) input.
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    /// Every sample `track`'s encoder received, in timeline order.
    pub fn samples(&self, track: Track) -> Vec<f32> {
        let received = self.received.lock().unwrap_or_else(PoisonError::into_inner);
        received.samples.get(&track).cloned().unwrap_or_default()
    }

    /// Whether `track`'s encoder was finished.
    pub fn finished(&self, track: Track) -> bool {
        let received = self.received.lock().unwrap_or_else(PoisonError::into_inner);
        received.finished.get(&track).copied().unwrap_or(false)
    }
}

impl EncoderFactory for FakeEncoderFactory {
    fn open(
        &self,
        track: Track,
        output: SegmentOutput,
    ) -> Result<Box<dyn TrackEncoder>, EncodeError> {
        Ok(Box::new(FakeEncoder {
            track,
            output,
            factory: self.clone(),
            written: Frames::ZERO,
        }))
    }
}

/// One fake track encoder.
#[derive(Debug)]
struct FakeEncoder {
    track: Track,
    output: SegmentOutput,
    factory: FakeEncoderFactory,
    /// Frames this encoder (one segment) has taken; timestamps are relative
    /// to the segment, like `AVAssetWriter`'s.
    written: Frames,
}

impl TrackEncoder for FakeEncoder {
    fn is_ready(&self) -> bool {
        self.factory.ready.load(Ordering::SeqCst)
    }

    fn append(&mut self, samples: &[f32], at: Frames) -> Result<(), EncodeError> {
        let mut received = self
            .factory
            .received
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if self.written != at {
            return Err(EncodeError::Rejected("a non-contiguous timestamp"));
        }
        self.written += Frames::of_len(samples.len());
        let track = received.samples.entry(self.track).or_default();
        track.extend_from_slice(samples);
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        if self.output.sink.append(&bytes).is_ok() {
            self.output.ledger.record(Frames::of_len(samples.len()));
        }
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), EncodeError> {
        let mut received = self
            .factory
            .received
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        received.finished.insert(self.track, true);
        Ok(())
    }
}
