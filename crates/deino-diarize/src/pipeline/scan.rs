//! Stage 1: segment every window and embed every local speaker in it.
//!
//! Windows run in batches of `runtime.batch_windows`: one segmentation call
//! per batch, then one filterbank per window shared by its local speakers,
//! then one embedding call per distinct input length in the batch. Only a
//! batch of windows is in memory at a time.

use super::{AudioSource, DiarizeRequest};
use crate::config::DiarizationConfig;
use crate::embedding::{AudioFeatures, EmbeddingExtractor, Features};
use crate::error::DiarizeError;
use crate::segmentation::{LOCAL_SPEAKERS, Segmenter, WindowActivity, is_active, is_clean};
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

/// Runs stage 1 over every window of `grid`.
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
    let thresholds = Thresholds::new(config);
    let mut scanned = Scan {
        activities: Vec::with_capacity(grid.count),
        embeddings: Vec::new(),
    };
    let batch = config.runtime.batch_windows.get();
    let mut buffer = Vec::new();
    for first in (0..grid.count).step_by(batch) {
        if request.is_cancelled.is_some_and(|cancelled| cancelled()) {
            return Err(DiarizeError::Cancelled);
        }
        let windows = first..(first + batch).min(grid.count);
        read_windows(audio, grid, windows.clone(), &mut buffer)?;
        let activities = segment(segmenter, &buffer, grid, windows.clone())?;
        let mut jobs = Vec::new();
        for (offset, (samples, activity)) in buffer
            .chunks_exact(WINDOW_SAMPLES)
            .zip(&activities)
            .enumerate()
        {
            jobs.extend(window_jobs(
                samples,
                activity,
                windows.start + offset,
                thresholds,
            )?);
        }
        embed(embedder, jobs, &mut scanned.embeddings)?;
        scanned.activities.extend(activities);
        if let Some(progress) = request.on_progress {
            progress(as_f32(windows.end) / as_f32(grid.count));
        }
    }
    Ok(scanned)
}

/// Fills `buffer` with the windows' samples, zero-padded past the end.
fn read_windows(
    audio: &(impl AudioSource + ?Sized),
    grid: &WindowGrid,
    windows: std::ops::Range<usize>,
    buffer: &mut Vec<f32>,
) -> Result<(), DiarizeError> {
    buffer.resize(windows.len() * WINDOW_SAMPLES, 0.0);
    for (index, out) in windows.zip(buffer.chunks_exact_mut(WINDOW_SAMPLES)) {
        audio.read(grid.start(index), out)?;
    }
    Ok(())
}

/// Segments the batch and drops frames past the end of the audio.
fn segment(
    segmenter: &mut Segmenter,
    buffer: &[f32],
    grid: &WindowGrid,
    windows: std::ops::Range<usize>,
) -> Result<Vec<WindowActivity>, DiarizeError> {
    let mut activities = segmenter.run(buffer)?;
    for (index, activity) in windows.zip(&mut activities) {
        let valid = WINDOW_SAMPLES.min(grid.total.saturating_sub(grid.start(index)));
        let frames = frames_for_samples(valid).min(activity.frames.len());
        activity.frames.truncate(frames);
    }
    Ok(activities)
}

/// Speech, in segmentation frames, a local speaker needs to be embedded
/// and to anchor the clustering.
#[derive(Clone, Copy)]
struct Thresholds {
    activity: usize,
    anchor: usize,
}

impl Thresholds {
    fn new(config: &DiarizationConfig) -> Self {
        Self {
            activity: duration_to_frames(config.min_activity).max(1),
            anchor: duration_to_frames(config.min_embedding_speech).max(1),
        }
    }
}

/// The local speakers of one window worth embedding, with their features.
/// An anchor is embedded from its clean frames only, anyone else from all
/// their active frames.
fn window_jobs(
    samples: &[f32],
    activity: &WindowActivity,
    window: usize,
    thresholds: Thresholds,
) -> Result<Vec<Job>, DiarizeError> {
    let speakers: Vec<usize> = (0..LOCAL_SPEAKERS)
        .filter(|speaker| activity.active_frames(*speaker) >= thresholds.activity)
        .collect();
    if speakers.is_empty() {
        return Ok(Vec::new());
    }
    let Some(features) = AudioFeatures::compute(samples)? else {
        return Ok(Vec::new());
    };
    let jobs = speakers
        .into_iter()
        .filter_map(|local_speaker| {
            let anchor = activity.clean_frames(local_speaker) >= thresholds.anchor;
            let heard = |frame: &[bool; LOCAL_SPEAKERS]| {
                if anchor {
                    is_clean(frame, local_speaker)
                } else {
                    is_active(frame, local_speaker)
                }
            };
            let features =
                features.masked(|frame| activity.frames.get(frame).is_some_and(heard))?;
            Some(Job {
                window,
                local_speaker,
                anchor,
                features,
            })
        })
        .collect();
    Ok(jobs)
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
