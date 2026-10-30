//! Scanned batches kept across passes over a growing recording. A batch's segmentation and embeddings depend only on its
//! windows' samples and how much of each window is audio, so a batch a pass
//! during the meeting scanned is the same batch after it when those match
//! exactly (the key); anything else is scanned again.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::scan::WindowEmbedding;
use crate::segmentation::WindowActivity;

/// A batch: its first window, and a hash of its samples and valid lengths.
pub(super) type BatchKey = (usize, u64);

/// One scanned batch.
#[derive(Clone)]
pub(super) struct ScannedBatch {
    pub(super) activities: Vec<WindowActivity>,
    pub(super) embeddings: Vec<WindowEmbedding>,
}

/// Scanned batches, shared between passes over one recording.
#[derive(Default)]
pub struct ScanCache {
    batches: Mutex<HashMap<BatchKey, ScannedBatch>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl ScanCache {
    /// Forgets every batch (a new recording).
    pub fn clear(&self) {
        if let Ok(mut batches) = self.batches.lock() {
            batches.clear();
        }
    }

    /// Batches reused and scanned since the last call, resetting both.
    pub fn take_counts(&self) -> (usize, usize) {
        (
            self.hits.swap(0, Ordering::Relaxed),
            self.misses.swap(0, Ordering::Relaxed),
        )
    }

    pub(super) fn get(&self, key: &BatchKey) -> Option<ScannedBatch> {
        let found = self.batches.lock().ok()?.get(key).cloned();
        let counter = if found.is_some() {
            &self.hits
        } else {
            &self.misses
        };
        counter.fetch_add(1, Ordering::Relaxed);
        found
    }

    pub(super) fn put(&self, key: BatchKey, batch: ScannedBatch) {
        if let Ok(mut batches) = self.batches.lock() {
            batches.insert(key, batch);
        }
    }
}

/// The key of the batch starting at window `first` whose windows' samples
/// are `buffer` and whose windows hold `valid` samples of audio each.
pub(super) fn key(first: usize, buffer: &[f32], valid: impl Iterator<Item = usize>) -> BatchKey {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |word: u64| {
        hash = (hash ^ word).wrapping_mul(0x0000_0100_0000_01b3);
    };
    for sample in buffer {
        mix(u64::from(sample.to_bits()));
    }
    valid.for_each(|samples| mix(samples as u64 | 1 << 63));
    mix(buffer.len() as u64);
    (first, hash ^ (hash >> 29))
}
