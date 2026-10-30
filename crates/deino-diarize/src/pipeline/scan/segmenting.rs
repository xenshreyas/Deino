//! The segmentation side of a scan: reading each batch of windows,
//! segmenting it and listing the local speakers to embed, or finding the
//! batch an earlier pass scanned.

use super::super::AudioSource;
use super::super::scan_cache::{self, BatchKey, ScanCache, ScannedBatch};
use super::{Job, Segmented, WindowGrid};
use crate::config::DiarizationConfig;
use crate::embedding::AudioFeatures;
use crate::error::DiarizeError;
use crate::segmentation::{LOCAL_SPEAKERS, Segmenter, WindowActivity, is_active, is_clean};
use crate::units::{WINDOW_SAMPLES, duration_to_frames, frames_for_samples};

/// Segments every batch of `grid` in order, sending each; stops at the
/// first error or when the receiver is gone (the scan failed or was
/// cancelled).
pub(super) fn segment_all(
    segmenter: &mut Segmenter,
    (audio, cache, grid): (
        &(impl AudioSource + ?Sized),
        Option<&ScanCache>,
        &WindowGrid,
    ),
    config: &DiarizationConfig,
    send: &std::sync::mpsc::SyncSender<Result<Segmented, DiarizeError>>,
) {
    let thresholds = Thresholds::new(config);
    let batch = config.runtime.batch_windows.get();
    let mut buffer = Vec::new();
    for first in (0..grid.count).step_by(batch) {
        let windows = first..(first + batch).min(grid.count);
        let next = match remembered(cache, (audio, grid), windows.clone(), &mut buffer) {
            Ok((key, Some(known))) => Ok(Segmented {
                windows,
                activities: known.activities,
                jobs: Vec::new(),
                key,
                embedded: Some(known.embeddings),
            }),
            Ok((key, None)) => segment_batch(segmenter, grid, (windows, key, thresholds), &buffer),
            Err(error) => Err(error),
        };
        let failed = next.is_err();
        if send.send(next).is_err() || failed {
            return;
        }
    }
}

/// Reads one batch's windows into `buffer`, and the batch an earlier pass
/// scanned from the same samples, if any.
fn remembered(
    cache: Option<&ScanCache>,
    (audio, grid): (&(impl AudioSource + ?Sized), &WindowGrid),
    windows: std::ops::Range<usize>,
    buffer: &mut Vec<f32>,
) -> Result<(BatchKey, Option<ScannedBatch>), DiarizeError> {
    read_windows(audio, grid, windows.clone(), buffer)?;
    let Some(cache) = cache else {
        return Ok(((windows.start, 0), None));
    };
    let valid = windows
        .clone()
        .map(|index| WINDOW_SAMPLES.min(grid.total.saturating_sub(grid.start(index))));
    let key = scan_cache::key(windows.start, buffer, valid);
    Ok((key, cache.get(&key)))
}

/// Segments one batch of windows (read into `buffer`) and lists its local
/// speakers.
fn segment_batch(
    segmenter: &mut Segmenter,
    grid: &WindowGrid,
    (windows, key, thresholds): (std::ops::Range<usize>, BatchKey, Thresholds),
    buffer: &[f32],
) -> Result<Segmented, DiarizeError> {
    let activities = segment(segmenter, buffer, grid, windows.clone())?;
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
    Ok(Segmented {
        windows,
        activities,
        jobs,
        key,
        embedded: None,
    })
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
