//! Speaker activity inside 10 s windows: pyannote segmentation-3.0.
//!
//! Adapted from anarlog `crates/pyannote-local/src/segmentation.rs` (MIT).
//! Deino's changes: the model is loaded from the model store, and several
//! windows run as one batch per `ort` call.

use std::path::Path;

use ort::session::Session;
use ort::value::TensorRef;

use crate::config::RuntimeConfig;
use crate::error::DiarizeError;
use crate::onnx;
use crate::units::{WINDOW_SAMPLES, frames_for_samples};

/// The model separates at most this many concurrent speakers per window.
pub const LOCAL_SPEAKERS: usize = 3;

/// Output frames per full window.
const FRAMES_PER_WINDOW: usize = frames_for_samples(WINDOW_SAMPLES);

/// segmentation-3.0 predicts a powerset over three local speakers, ordered
/// by cardinality: silence, the three singletons, then the three pairs.
const POWERSET: [[bool; LOCAL_SPEAKERS]; 7] = [
    [false, false, false],
    [true, false, false],
    [false, true, false],
    [false, false, true],
    [true, true, false],
    [true, false, true],
    [false, true, true],
];

/// Which local speakers are active in one segmentation frame.
pub type FrameActivity = [bool; LOCAL_SPEAKERS];

/// Per-frame activity of the local speakers inside one window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowActivity {
    /// One entry per segmentation frame.
    pub frames: Vec<FrameActivity>,
}

impl WindowActivity {
    /// Frames where `speaker` talks.
    pub fn active_frames(&self, speaker: usize) -> usize {
        self.frames
            .iter()
            .filter(|frame| is_active(frame, speaker))
            .count()
    }

    /// Frames where `speaker` talks alone; overlapped speech contaminates
    /// speaker embeddings.
    pub fn clean_frames(&self, speaker: usize) -> usize {
        self.frames
            .iter()
            .filter(|frame| is_clean(frame, speaker))
            .count()
    }
}

/// Whether `speaker` talks in `frame`.
pub fn is_active(frame: &FrameActivity, speaker: usize) -> bool {
    frame.get(speaker).copied().unwrap_or(false)
}

/// Whether `speaker` is the only one talking in `frame`.
pub fn is_clean(frame: &FrameActivity, speaker: usize) -> bool {
    is_active(frame, speaker) && frame.iter().filter(|active| **active).count() == 1
}

/// The segmentation model.
pub struct Segmenter {
    session: Session,
}

impl Segmenter {
    /// Loads the model at `path` on the CPU.
    pub fn new(path: &Path, runtime: &RuntimeConfig) -> Result<Self, DiarizeError> {
        Ok(Self {
            session: onnx::cpu_session(path, runtime)?,
        })
    }

    /// Runs consecutive windows of exactly [`WINDOW_SAMPLES`] samples each
    /// (`windows.len()` is a multiple of it) in one batch. Samples are in
    /// `[-1, 1]`; the model normalises the level itself.
    pub fn run(&mut self, windows: &[f32]) -> Result<Vec<WindowActivity>, DiarizeError> {
        let batch = windows.len() / WINDOW_SAMPLES;
        let input = TensorRef::from_array_view(([batch, 1, WINDOW_SAMPLES], windows))?;
        let outputs = self.session.run(ort::inputs![input])?;
        let value = outputs.values().next().ok_or(DiarizeError::OutputShape {
            expected: 1,
            actual: 0,
        })?;
        let (_, logits) = value.try_extract_tensor::<f32>()?;
        decode(logits, batch)
    }
}

/// Splits `batch` windows of logits into per-frame powerset decisions.
fn decode(logits: &[f32], batch: usize) -> Result<Vec<WindowActivity>, DiarizeError> {
    let per_window = logits.len().checked_div(batch).unwrap_or_default();
    let classes = POWERSET.len();
    if per_window == 0 || !per_window.is_multiple_of(classes) || per_window * batch != logits.len()
    {
        return Err(DiarizeError::OutputShape {
            expected: batch * FRAMES_PER_WINDOW * classes,
            actual: logits.len(),
        });
    }
    let windows = logits
        .chunks_exact(per_window)
        .map(|window| WindowActivity {
            frames: window.chunks_exact(classes).map(most_likely).collect(),
        })
        .collect();
    Ok(windows)
}

/// The powerset class with the highest logit.
fn most_likely(classes: &[f32]) -> FrameActivity {
    let class = classes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map_or(0, |(index, _)| index);
    POWERSET.get(class).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_logits(class: usize) -> Vec<f32> {
        (0..POWERSET.len())
            .map(|index| if index == class { 5.0 } else { -5.0 })
            .collect()
    }

    #[test]
    fn logits_decode_to_the_powerset_class() {
        assert_eq!(most_likely(&frame_logits(0)), [false, false, false]);
        assert_eq!(most_likely(&frame_logits(2)), [false, true, false]);
        assert_eq!(most_likely(&frame_logits(6)), [false, true, true]);
    }

    #[test]
    fn a_batch_splits_into_one_activity_per_window() {
        let window: Vec<f32> = (0..FRAMES_PER_WINDOW)
            .flat_map(|_| frame_logits(1))
            .collect();
        let other: Vec<f32> = (0..FRAMES_PER_WINDOW)
            .flat_map(|_| frame_logits(4))
            .collect();
        let activities = decode(&[window, other].concat(), 2).unwrap();
        assert_eq!(activities.len(), 2);
        assert_eq!(activities[0].active_frames(0), FRAMES_PER_WINDOW);
        assert_eq!(activities[0].clean_frames(0), FRAMES_PER_WINDOW);
        assert_eq!(activities[1].active_frames(1), FRAMES_PER_WINDOW);
        assert_eq!(activities[1].clean_frames(1), 0);
    }

    #[test]
    fn a_short_output_is_a_shape_error() {
        assert!(matches!(
            decode(&[0.0; 10], 1),
            Err(DiarizeError::OutputShape { .. })
        ));
    }
}
