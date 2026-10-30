//! Stage 2 with Community-1's clustering (`crate::vbx`): VBx finds the
//! speakers from the anchors, then every window embedding takes its most
//! similar speaker, the speakers of one window on distinct speakers.
//!
//! The calendar's upper bound still holds: past it, the speakers with the
//! smallest share join their nearest neighbour (pyannote re-clusters with
//! k-means there instead; with a calendar this rarely happens).

use super::labels::Clusters;
use super::scan::WindowEmbedding;
use crate::clustering;
use crate::config::SpeakerBounds;
use crate::vbx::{self, Plda, Speaker, VbxParams};

/// Clusters `embeddings` with VBx within `bounds`.
pub(super) fn label(
    embeddings: &[WindowEmbedding],
    plda: &Plda,
    bounds: SpeakerBounds,
) -> Clusters {
    let anchors: Vec<&[f32]> = train(embeddings);
    let mut speakers = vbx::speakers(&anchors, plda, VbxParams::default());
    let (_, max) = bounds.range();
    while speakers.len() > max {
        fold_smallest(&mut speakers);
    }
    let all: Vec<&[f32]> = embeddings
        .iter()
        .map(|item| item.embedding.as_slice())
        .collect();
    let windows: Vec<usize> = embeddings.iter().map(|item| item.window).collect();
    let centroids: Vec<Vec<f32>> = speakers
        .into_iter()
        .map(|speaker| speaker.centroid)
        .collect();
    let labels = compact(&vbx::assign(&all, &windows, &centroids));
    let centroids = unit_centroids(embeddings, &labels);
    let identities = vec![None; centroids.len()];
    Clusters {
        labels,
        centroids,
        identities,
    }
}

/// The anchors' embeddings, or every embedding when none anchors.
fn train(embeddings: &[WindowEmbedding]) -> Vec<&[f32]> {
    let anchors: Vec<&[f32]> = embeddings
        .iter()
        .filter(|item| item.anchor)
        .map(|item| item.embedding.as_slice())
        .collect();
    if anchors.is_empty() {
        return embeddings
            .iter()
            .map(|item| item.embedding.as_slice())
            .collect();
    }
    anchors
}

/// Joins the speaker with the smallest share to its nearest neighbour.
fn fold_smallest(speakers: &mut Vec<Speaker>) {
    let Some(smallest) = speakers
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.weight.total_cmp(&b.1.weight))
        .map(|(index, _)| index)
    else {
        return;
    };
    let gone = speakers.remove(smallest);
    let nearest = speakers.iter_mut().max_by(|a, b| {
        similarity(&a.centroid, &gone.centroid).total_cmp(&similarity(&b.centroid, &gone.centroid))
    });
    if let Some(nearest) = nearest {
        let total = nearest.weight + gone.weight;
        #[allow(clippy::cast_possible_truncation, reason = "embedding values are f32")]
        for (value, other) in nearest.centroid.iter_mut().zip(&gone.centroid) {
            *value = ((f64::from(*value) * nearest.weight + f64::from(*other) * gone.weight)
                / total) as f32;
        }
        nearest.weight = total;
    }
}

/// Labels renumbered `0..k` in order of first appearance, so every label
/// has members.
fn compact(labels: &[usize]) -> Vec<usize> {
    let mut seen: Vec<usize> = Vec::new();
    labels
        .iter()
        .map(|label| {
            seen.iter().position(|s| s == label).unwrap_or_else(|| {
                seen.push(*label);
                seen.len() - 1
            })
        })
        .collect()
}

/// Each cluster's mean unit embedding over its anchors (all its members
/// when it has none), as the rest of the pipeline expects.
fn unit_centroids(embeddings: &[WindowEmbedding], labels: &[usize]) -> Vec<Vec<f32>> {
    let count = labels.iter().max().map_or(0, |max| max + 1);
    (0..count)
        .map(|label| {
            let members: Vec<&WindowEmbedding> = embeddings
                .iter()
                .zip(labels)
                .filter(|(_, l)| **l == label)
                .map(|(item, _)| item)
                .collect();
            let anchored: Vec<Vec<f32>> = members
                .iter()
                .filter(|m| m.anchor)
                .map(|m| m.embedding.clone())
                .collect();
            let chosen = if anchored.is_empty() {
                members.iter().map(|m| m.embedding.clone()).collect()
            } else {
                anchored
            };
            let zeros = vec![0; chosen.len()];
            clustering::centroids(&chosen, &zeros)
                .into_iter()
                .next()
                .unwrap_or_default()
        })
        .collect()
}

fn similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let norm = |v: &[f32]| v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    dot / (norm(a) * norm(b)).max(f64::MIN_POSITIVE)
}
