//! The diarizer's tuning knobs and the speaker-count bounds of one run.
//!
//! Defaults are anarlog's AMI-tuned values (`pyannote-local/src/pipeline.rs`),
//! which the Deino spike kept (`docs/references/spike-diarization.md`).
//! [`crate::policy`] picks the per-meeting values.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

/// Centroid-linkage cut on unit-normalised embeddings (euclidean distance),
/// tuned by anarlog on AMI. Re-tune on real meetings, never on TTS voices.
pub const DEFAULT_THRESHOLD: f32 = 0.7045;

/// Cosine similarity a cluster needs against a known voiceprint before the
/// voiceprint may merge clusters (anarlog's `MIN_UNIQUE_SCORE`).
pub const DEFAULT_KNOWN_SPEAKER_SCORE: f32 = 0.62;

/// How the diarizer clusters and smooths one recording.
#[derive(Clone, Debug, PartialEq)]
pub struct DiarizationConfig {
    /// Hop between consecutive 10 s segmentation windows. 2 s keeps every
    /// frame under about five windows, so majority voting suppresses
    /// per-window false speech; 5 s breaks the vote.
    pub step: Duration,
    /// Clean (non-overlapped) speech a local speaker needs in a window before
    /// its embedding becomes a clustering anchor.
    pub min_embedding_speech: Duration,
    /// Speech a local speaker needs in a window to be embedded at all;
    /// shorter blips are attached to the nearest cluster.
    pub min_activity: Duration,
    /// Centroid-linkage cut on unit-normalised embeddings.
    pub threshold: f32,
    /// Clusters backed by less anchor speech than this are absorbed by their
    /// nearest neighbour. In seconds of windows, so it holds across steps.
    pub min_cluster_speech: Duration,
    /// Speaker turns shorter than this are dropped after reconstruction.
    pub min_duration_on: Duration,
    /// Same-speaker gaps shorter than this are bridged after reconstruction.
    pub min_duration_off: Duration,
    /// The step grows on long recordings so at most this many windows run.
    pub max_windows: NonZeroUsize,
    /// See [`DEFAULT_KNOWN_SPEAKER_SCORE`].
    pub known_speaker_min_score: f32,
    /// How the models run.
    pub runtime: RuntimeConfig,
    /// Community-1's clustering (`crate::vbx`) with this PLDA, instead of
    /// the centroid-linkage clustering under `threshold`.
    pub vbx: Option<std::sync::Arc<crate::vbx::Plda>>,
    /// One speaker per frame (Community-1's exclusive diarization) instead
    /// of overlapping turns.
    pub exclusive: bool,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            step: Duration::from_secs(2),
            min_embedding_speech: Duration::from_secs(1),
            min_activity: Duration::from_millis(300),
            threshold: DEFAULT_THRESHOLD,
            min_cluster_speech: Duration::from_secs(12),
            min_duration_on: Duration::ZERO,
            min_duration_off: Duration::ZERO,
            max_windows: non_zero(1_800),
            known_speaker_min_score: DEFAULT_KNOWN_SPEAKER_SCORE,
            runtime: RuntimeConfig::default(),
            vbx: None,
            exclusive: false,
        }
    }
}

/// How ONNX Runtime executes the two models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    /// Intra-op threads per model session (anarlog pinned 1; 4 measured best).
    pub intra_threads: NonZeroUsize,
    /// Run the embedding model through Core ML (CPU + Neural Engine) on
    /// macOS. Segmentation always stays on the CPU.
    pub accelerate_embedding: bool,
    /// Where Core ML keeps compiled models; `None` compiles on every load.
    pub accelerator_cache: Option<PathBuf>,
    /// Windows segmented per model run (one batched `ort` call).
    pub batch_windows: NonZeroUsize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            intra_threads: non_zero(4),
            accelerate_embedding: true,
            accelerator_cache: None,
            batch_windows: non_zero(16),
        }
    }
}

/// `value` as a [`NonZeroUsize`], or 1 for zero (for constants).
pub const fn non_zero(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    }
}

/// How many speakers a recording may hold. With no bounds the threshold
/// alone decides; bounds override it (always pass calendar bounds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeakerBounds {
    /// Exactly this many, when known.
    pub exact: Option<usize>,
    /// At least this many.
    pub min: Option<usize>,
    /// At most this many.
    pub max: Option<usize>,
}

impl SpeakerBounds {
    /// Exactly `count` speakers.
    pub fn exact(count: usize) -> Self {
        Self {
            exact: Some(count),
            ..Self::default()
        }
    }

    /// At most `count` speakers.
    pub fn at_most(count: usize) -> Self {
        Self {
            max: Some(count),
            ..Self::default()
        }
    }

    /// The inclusive `(min, max)` cluster counts these bounds allow, each at
    /// least 1.
    pub fn range(self) -> (usize, usize) {
        if let Some(count) = self.exact {
            return (count.max(1), count.max(1));
        }
        let min = self.min.unwrap_or(1).max(1);
        let max = self.max.unwrap_or(usize::MAX).max(min);
        (min, max)
    }

    /// The fewest clusters merging may leave: `exact`, else `min`, else 1.
    pub fn floor(self) -> usize {
        self.exact.or(self.min).unwrap_or(1).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_bounds_pin_both_ends() {
        assert_eq!(SpeakerBounds::exact(3).range(), (3, 3));
        assert_eq!(SpeakerBounds::exact(0).range(), (1, 1));
    }

    #[test]
    fn open_bounds_allow_any_count_from_one() {
        assert_eq!(SpeakerBounds::default().range(), (1, usize::MAX));
        assert_eq!(SpeakerBounds::at_most(4).range(), (1, 4));
        assert_eq!(SpeakerBounds::default().floor(), 1);
    }

    #[test]
    fn a_max_below_the_min_is_raised_to_it() {
        let bounds = SpeakerBounds {
            min: Some(3),
            max: Some(2),
            exact: None,
        };
        assert_eq!(bounds.range(), (3, 3));
        assert_eq!(bounds.floor(), 3);
    }
}
