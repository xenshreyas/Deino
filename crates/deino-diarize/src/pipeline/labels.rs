//! Stage 2: cluster the anchors and label every window embedding.
//!
//! Only anchors (embedded from enough clean speech) shape the clusters;
//! every other embedding joins the nearest centroid. Without any anchor,
//! all embeddings anchor.

use super::SpeakerIdentity;
use super::scan::{WindowEmbedding, WindowGrid};
use crate::clustering;
use crate::config::{DiarizationConfig, SpeakerBounds};
use crate::units::{ceil_ratio, samples_to_duration};

/// Each window embedding's cluster and the clusters' centroids.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Clusters {
    /// One cluster per window embedding.
    pub(super) labels: Vec<usize>,
    /// The mean unit embedding of each cluster's anchors.
    pub(super) centroids: Vec<Vec<f32>>,
    /// The known person each cluster matched (stage 3), by cluster.
    pub(super) identities: Vec<Option<SpeakerIdentity>>,
}

/// Clusters `embeddings` within `bounds`.
pub(super) fn label(
    embeddings: &[WindowEmbedding],
    config: &DiarizationConfig,
    grid: &WindowGrid,
    bounds: SpeakerBounds,
) -> Clusters {
    let anchors: Vec<usize> = anchor_indices(embeddings);
    let vectors: Vec<Vec<f32>> = anchors
        .iter()
        .filter_map(|index| embeddings.get(*index))
        .map(|item| item.embedding.clone())
        .collect();
    let min_size = min_cluster_size(config, grid);
    let anchor_labels = clustering::cluster(&vectors, config.threshold, min_size, bounds);
    let centroids = clustering::centroids(&vectors, &anchor_labels);
    let mut labels: Vec<Option<usize>> = vec![None; embeddings.len()];
    for (anchor, label) in anchors.iter().zip(&anchor_labels) {
        if let Some(slot) = labels.get_mut(*anchor) {
            *slot = Some(*label);
        }
    }
    let labels = labels
        .into_iter()
        .zip(embeddings)
        .map(|(label, item)| {
            label.unwrap_or_else(|| {
                clustering::nearest_centroid(&item.embedding, &centroids).unwrap_or(0)
            })
        })
        .collect();
    let identities = vec![None; centroids.len()];
    Clusters {
        labels,
        centroids,
        identities,
    }
}

/// The anchors' positions, or every position when there is no anchor.
fn anchor_indices(embeddings: &[WindowEmbedding]) -> Vec<usize> {
    let anchors: Vec<usize> = embeddings
        .iter()
        .enumerate()
        .filter(|(_, item)| item.anchor)
        .map(|(index, _)| index)
        .collect();
    if anchors.is_empty() {
        (0..embeddings.len()).collect()
    } else {
        anchors
    }
}

/// `min_cluster_speech` in anchors: one anchor per window step (pyannote's
/// 12 anchors at a 1 s step).
fn min_cluster_size(config: &DiarizationConfig, grid: &WindowGrid) -> usize {
    let step = samples_to_duration(grid.step).as_secs_f32();
    ceil_ratio(config.min_cluster_speech.as_secs_f32(), step)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn embedding(window: usize, direction: [f32; 2], anchor: bool) -> WindowEmbedding {
        WindowEmbedding {
            window,
            local_speaker: 0,
            embedding: direction.to_vec(),
            anchor,
        }
    }

    fn grid() -> WindowGrid {
        WindowGrid::new(60 * 16_000, &DiarizationConfig::default()).unwrap()
    }

    fn config() -> DiarizationConfig {
        DiarizationConfig {
            min_cluster_speech: Duration::from_secs(4),
            ..DiarizationConfig::default()
        }
    }

    #[test]
    fn non_anchors_join_the_nearest_anchor_cluster() {
        let mut embeddings: Vec<WindowEmbedding> = (0..4)
            .map(|w| embedding(w, [1.0, 0.0], true))
            .chain((4..8).map(|w| embedding(w, [0.0, 1.0], true)))
            .collect();
        embeddings.push(embedding(8, [0.1, 1.0], false));
        let clusters = label(&embeddings, &config(), &grid(), SpeakerBounds::default());
        assert_eq!(clusters.labels, vec![0, 0, 0, 0, 1, 1, 1, 1, 1]);
        assert_eq!(clusters.centroids.len(), 2);
    }

    #[test]
    fn without_anchors_everything_anchors() {
        let embeddings: Vec<WindowEmbedding> =
            (0..4).map(|w| embedding(w, [1.0, 0.0], false)).collect();
        let clusters = label(&embeddings, &config(), &grid(), SpeakerBounds::default());
        assert_eq!(clusters.labels, vec![0; 4]);
    }

    #[test]
    fn the_minimum_cluster_size_scales_with_the_step() {
        // 12 s of anchors at a ~2 s step is 6 anchors.
        let size = min_cluster_size(&DiarizationConfig::default(), &grid());
        assert_eq!(size, 6);
    }
}
