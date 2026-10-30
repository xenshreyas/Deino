//! Stage 1: segment every window and embed every local speaker in it.
//!
//! Windows run in batches of `runtime.batch_windows`: one segmentation call
//! per batch, then one filterbank per window shared by its local speakers,
//! then one embedding call per distinct input length in the batch. Only a
//! batch of windows is in memory at a time.

mod segmenting;

use segmenting::segment_all;

use super::scan_cache::{BatchKey, ScanCache, ScannedBatch};
use super::{AudioSource, DiarizeRequest};
use crate::config::DiarizationConfig;
use crate::embedding::{EmbeddingExtractor, Features};
use crate::error::DiarizeError;
use crate::segmentation::{Segmenter, WindowActivity};
use crate::units::{FRAME_SIZE, WINDOW_SAMPLES, as_f32, duration_to_frames, frames_for_samples};

/// The models a scan runs.
pub(super) struct Models<'a> {
    pub(super) segmenter: &'a mut Segmenter,
    pub(super) embedder: &'a mut EmbeddingExtractor,
    pub(super) config: &'a DiarizationConfig,
}

/// Where the windows of one recording start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowGrid {
    /// Samples in the recording.
    pub(super) total: usize,
    /// Samples between window starts (a whole number of frames).
    pub(super) step: usize,
    /// Windows to run.
    pub(super) count: usize,
}

impl WindowGrid {
    /// The grid for `total` samples; `None` for empty audio. The step grows
    /// past the configured one when more than `max_windows` would run.
    pub(super) fn new(total: usize, config: &DiarizationConfig) -> Option<Self> {
        if total == 0 {
            return None;
        }
        let requested = duration_to_frames(config.step).max(1);
        let spans = config.max_windows.get().saturating_sub(1).max(1) * FRAME_SIZE;
        let needed = total.saturating_sub(WINDOW_SAMPLES).div_ceil(spans);
        let frames = requested
            .max(needed)
            .min(frames_for_samples(WINDOW_SAMPLES));
        let step = frames * FRAME_SIZE;
        let count = total.saturating_sub(WINDOW_SAMPLES).div_ceil(step) + 1;
        Some(Self { total, step, count })
    }

    /// The first sample of window `index`.
    pub(super) fn start(&self, index: usize) -> usize {
        index * self.step
    }
}

/// A local speaker's embedding in one window.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WindowEmbedding {
    pub(super) window: usize,
    pub(super) local_speaker: usize,
    pub(super) embedding: Vec<f32>,
    /// Embedded from enough clean speech to anchor the clustering.
    pub(super) anchor: bool,
}

/// Everything the later stages need from the audio.
pub(super) struct Scan {
    pub(super) activities: Vec<WindowActivity>,
    pub(super) embeddings: Vec<WindowEmbedding>,
}

/// One local speaker to embed.
struct Job {
    window: usize,
    local_speaker: usize,
    anchor: bool,
    features: Features,
}

/// One batch of windows, segmented, with the local speakers to embed, or
/// its embeddings when an earlier pass scanned the same batch.
struct Segmented {
    windows: std::ops::Range<usize>,
    activities: Vec<WindowActivity>,
    jobs: Vec<Job>,
    key: BatchKey,
    embedded: Option<Vec<WindowEmbedding>>,
}

/// Runs stage 1 over every window of `grid`. Segmentation (CPU) runs one
/// batch ahead of the embeddings (Core ML) on its own thread, so the two
/// models overlap; every model call is the same as one after the other.
pub(super) fn scan(
    models: Models<'_>,
    audio: &(impl AudioSource + ?Sized),
    grid: &WindowGrid,
    request: &DiarizeRequest<'_>,
) -> Result<Scan, DiarizeError> {
    let Models {
        segmenter,
        embedder,
        config,
    } = models;
    let mut scanned = Scan {
        activities: Vec::with_capacity(grid.count),
        embeddings: Vec::new(),
    };
    std::thread::scope(|scope| {
        let (send, batches) = std::sync::mpsc::sync_channel(1);
        let cache = request.scan_cache;
        scope.spawn(move || segment_all(segmenter, (audio, cache, grid), config, &send));
        for segmented in batches {
            if request.is_cancelled.is_some_and(|cancelled| cancelled()) {
                return Err(DiarizeError::Cancelled);
            }
            let (windows, batch) = embedded(embedder, segmented?, cache)?;
            scanned.embeddings.extend(batch.embeddings);
            scanned.activities.extend(batch.activities);
            if let Some(progress) = request.on_progress {
                progress(as_f32(windows.end) / as_f32(grid.count));
            }
        }
        Ok(())
    })?;
    Ok(scanned)
}

/// The batch's windows, activities and embeddings: embedded now (and kept
/// in `cache`) unless an earlier pass scanned it.
fn embedded(
    embedder: &mut EmbeddingExtractor,
    segmented: Segmented,
    cache: Option<&ScanCache>,
) -> Result<(std::ops::Range<usize>, ScannedBatch), DiarizeError> {
    if let Some(embeddings) = segmented.embedded {
        let batch = ScannedBatch {
            activities: segmented.activities,
            embeddings,
        };
        return Ok((segmented.windows, batch));
    }
    let mut embeddings = Vec::new();
    embed(embedder, segmented.jobs, &mut embeddings)?;
    let batch = ScannedBatch {
        activities: segmented.activities,
        embeddings,
    };
    if let Some(cache) = cache {
        cache.put(segmented.key, batch.clone());
    }
    Ok((segmented.windows, batch))
}

/// Embeds the batch's jobs and keeps the finite results, in job order.
fn embed(
    embedder: &mut EmbeddingExtractor,
    jobs: Vec<Job>,
    out: &mut Vec<WindowEmbedding>,
) -> Result<(), DiarizeError> {
    let inputs: Vec<Features> = jobs.iter().map(|job| job.features.clone()).collect();
    let embeddings = embedder.embed_all(&inputs)?;
    for (job, embedding) in jobs.into_iter().zip(embeddings) {
        if let Some(embedding) = embedding {
            out.push(WindowEmbedding {
                window: job.window,
                local_speaker: job.local_speaker,
                embedding: embedding.into_vec(),
                anchor: job.anchor,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::non_zero;

    #[test]
    fn a_short_recording_is_one_window() {
        let grid = WindowGrid::new(16_000, &DiarizationConfig::default()).unwrap();
        assert_eq!((grid.count, grid.step), (1, 119 * FRAME_SIZE));
        assert!(WindowGrid::new(0, &DiarizationConfig::default()).is_none());
    }

    #[test]
    fn windows_cover_the_whole_recording() {
        let total = 60 * 16_000;
        let grid = WindowGrid::new(total, &DiarizationConfig::default()).unwrap();
        assert!(grid.start(grid.count - 1) + WINDOW_SAMPLES >= total);
        assert!(grid.start(grid.count - 2) + WINDOW_SAMPLES < total);
    }

    #[test]
    fn long_recordings_grow_the_step_to_cap_the_window_count() {
        let config = DiarizationConfig {
            max_windows: non_zero(500),
            step: Duration::from_secs(2),
            ..DiarizationConfig::default()
        };
        let grid = WindowGrid::new(3_600 * 16_000, &config).unwrap();
        assert!(grid.count <= 500, "{}", grid.count);
        assert!(grid.step > 2 * 16_000);
        assert!(grid.step <= WINDOW_SAMPLES);
    }
}
