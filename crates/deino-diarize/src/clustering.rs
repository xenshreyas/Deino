//! Agglomerative clustering of speaker embeddings, following the
//! centroid-linkage recipe of pyannote.audio 3.1.
//!
//! Adapted from anarlog `crates/pyannote-local/src/clustering.rs` (MIT).
//! Embeddings are unit-normalised, so euclidean distance tracks cosine
//! distance; the dendrogram is cut at a fixed threshold unless the speaker
//! bounds say otherwise; clusters too small to be a speaker are absorbed by
//! the nearest large one. Deino's change: the number of large clusters after
//! every possible cut is computed in one pass instead of one cut per count.

pub mod linkage;

use crate::config::SpeakerBounds;
use crate::vector::{euclidean, mean, normalize};
pub use linkage::linkage;
use linkage::{DisjointSets, Merge, cut};

/// One label per embedding, `0..k`. Clusters with fewer than
/// `min_cluster_size` members are absorbed by the nearest large cluster, so
/// the count reflects the speakers the evidence supports.
pub fn cluster(
    embeddings: &[Vec<f32>],
    threshold: f32,
    min_cluster_size: usize,
    bounds: SpeakerBounds,
) -> Vec<usize> {
    let count = embeddings.len();
    if count == 0 {
        return Vec::new();
    }
    let min_cluster_size = min_cluster_size.max(1);
    let merges = linkage(embeddings);
    let large = large_counts(count, &merges, min_cluster_size);
    let chosen = choose_cut(&merges, &large, threshold, bounds);
    if large.get(chosen).copied().unwrap_or(0) == 0 {
        return vec![0; count];
    }
    absorb_small(embeddings, &cut(count, &merges, chosen), min_cluster_size)
}

/// How many merges to apply: the threshold cut, or when that breaks the
/// bounds, the cut whose large-cluster count is closest to them (ties go to
/// the cut nearest the threshold).
fn choose_cut(merges: &[Merge], large: &[usize], threshold: f32, bounds: SpeakerBounds) -> usize {
    let threshold_cut = merges
        .iter()
        .position(|merge| merge.distance > threshold)
        .unwrap_or(merges.len());
    let count = large.get(threshold_cut).copied().unwrap_or(0);
    let (min, max) = bounds.range();
    if (min..=max).contains(&count) {
        return threshold_cut;
    }
    let target = count.clamp(min, max);
    large
        .iter()
        .enumerate()
        .min_by_key(|(merge_count, large)| {
            (large.abs_diff(target), merge_count.abs_diff(threshold_cut))
        })
        .map_or(threshold_cut, |(merge_count, _)| merge_count)
}

/// `result[m]`: clusters of at least `min_size` members after `m` merges.
fn large_counts(count: usize, merges: &[Merge], min_size: usize) -> Vec<usize> {
    let mut sets = DisjointSets::new(count);
    let mut large = if min_size <= 1 { count } else { 0 };
    let mut counts = Vec::with_capacity(merges.len() + 1);
    counts.push(large);
    for merge in merges {
        if let Some((left, right)) = sets.union(merge.left, merge.right) {
            large -= usize::from(left >= min_size) + usize::from(right >= min_size);
            large += usize::from(left + right >= min_size);
        }
        counts.push(large);
    }
    counts
}

/// Relabels so large clusters are `0..k` (in label order) and every member
/// of a small cluster joins the nearest large centroid.
fn absorb_small(embeddings: &[Vec<f32>], labels: &[usize], min_size: usize) -> Vec<usize> {
    let sizes = cluster_sizes(labels);
    let large: Vec<usize> = (0..sizes.len())
        .filter(|label| sizes.get(*label).copied().unwrap_or(0) >= min_size)
        .collect();
    let all = centroids(embeddings, labels);
    let large_centroids: Vec<Vec<f32>> = large
        .iter()
        .filter_map(|label| all.get(*label).cloned())
        .collect();
    labels
        .iter()
        .zip(embeddings)
        .map(
            |(label, embedding)| match large.iter().position(|l| l == label) {
                Some(index) => index,
                None => nearest_centroid(embedding, &large_centroids).unwrap_or(0),
            },
        )
        .collect()
}

/// The mean of each label's unit-normalised embeddings, indexed by label.
pub fn centroids(embeddings: &[Vec<f32>], labels: &[usize]) -> Vec<Vec<f32>> {
    let count = labels.iter().max().map_or(0, |max| max + 1);
    (0..count)
        .map(|label| {
            let members = embeddings
                .iter()
                .zip(labels)
                .filter(|(_, l)| **l == label)
                .map(|(embedding, _)| embedding.as_slice());
            mean(members, true)
        })
        .collect()
}

/// The centroid closest to `embedding` (normalised), if any.
pub fn nearest_centroid(embedding: &[f32], centroids: &[Vec<f32>]) -> Option<usize> {
    let unit = normalize(embedding);
    centroids
        .iter()
        .enumerate()
        .map(|(index, centroid)| (index, euclidean(&unit, centroid)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

fn cluster_sizes(labels: &[usize]) -> Vec<usize> {
    let count = labels.iter().max().map_or(0, |max| max + 1);
    let mut sizes = vec![0; count];
    for label in labels {
        if let Some(size) = sizes.get_mut(*label) {
            *size += 1;
        }
    }
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `per` noisy copies of each direction in `directions`.
    fn voices(directions: &[[f32; 3]], per: usize) -> Vec<Vec<f32>> {
        directions
            .iter()
            .flat_map(|direction| {
                std::iter::successors(Some(0.0_f32), |jitter| Some(jitter + 0.01))
                    .take(per)
                    .map(move |jitter| direction.iter().map(|v| v + jitter).collect())
            })
            .collect()
    }

    const THREE: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    #[test]
    fn distinct_voices_form_their_own_clusters() {
        let labels = cluster(&voices(&THREE, 5), 0.7, 1, SpeakerBounds::default());
        assert_eq!(labels, [[0; 5], [1; 5], [2; 5]].concat());
    }

    #[test]
    fn exact_bounds_override_the_threshold() {
        let embeddings = voices(&THREE, 5);
        let two = cluster(&embeddings, 0.7, 1, SpeakerBounds::exact(2));
        assert_eq!(two.iter().max(), Some(&1));
        let one = cluster(&embeddings, 0.7, 1, SpeakerBounds::at_most(1));
        assert!(one.iter().all(|label| *label == 0));
    }

    #[test]
    fn small_clusters_join_their_nearest_large_neighbour() {
        let mut embeddings = voices(&THREE[..2], 6);
        embeddings.push(vec![0.9, 0.1, 0.8]);
        let labels = cluster(&embeddings, 0.5, 3, SpeakerBounds::default());
        assert_eq!(labels.iter().max(), Some(&1), "the stray item is absorbed");
        assert_eq!(labels.last(), Some(&0));
    }

    #[test]
    fn large_counts_track_every_cut() {
        let merges = [
            Merge {
                left: 0,
                right: 1,
                distance: 0.1,
            },
            Merge {
                left: 2,
                right: 3,
                distance: 0.2,
            },
            Merge {
                left: 0,
                right: 2,
                distance: 0.9,
            },
        ];
        assert_eq!(large_counts(4, &merges, 1), vec![4, 3, 2, 1]);
        assert_eq!(large_counts(4, &merges, 2), vec![0, 1, 2, 1]);
        assert_eq!(large_counts(4, &merges, 4), vec![0, 0, 0, 1]);
    }

    #[test]
    fn centroids_average_unit_vectors_and_nearest_finds_them() {
        let embeddings = vec![vec![2.0, 0.0], vec![4.0, 0.0], vec![0.0, 3.0]];
        let centroids = centroids(&embeddings, &[0, 0, 1]);
        assert_eq!(centroids, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        assert_eq!(nearest_centroid(&[0.1, 5.0], &centroids), Some(1));
        assert_eq!(nearest_centroid(&[0.1, 5.0], &[]), None);
    }

    #[test]
    fn no_embeddings_have_no_labels() {
        assert!(cluster(&[], 0.7, 1, SpeakerBounds::default()).is_empty());
    }
}
