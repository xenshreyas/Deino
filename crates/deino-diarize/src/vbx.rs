//! Community-1's clustering in Rust (`pyannote.audio`'s `VBxClustering`):
//! the speakers of a meeting from its window embeddings.
//!
//! 1. The training embeddings (enough clean speech) are clustered roughly:
//!    centroid-linkage AHC on unit vectors, cut at [`THRESHOLD`] ([`ahc`]).
//! 2. They move into the PLDA space ([`plda`]) and VBx refines the rough
//!    clusters, dropping the speakers nothing needs ([`vb`]).
//! 3. Each kept speaker's centroid is the responsibility-weighted mean of
//!    the raw training embeddings.
//! 4. Every embedding takes the centroid it is most similar to (cosine),
//!    and two speakers heard in one window never take the same centroid
//!    ([`assign`]).
//!
//! Checked against pyannote's own output on a synthetic case
//! (`tests/vbx_reference.rs`, made by `scripts/vbx-reference.py`).

pub mod ahc;
pub mod plda;
pub mod vb;

pub use plda::{Plda, PldaError};
pub use vb::{Vbx, VbxParams, vbx};

/// Where Community-1 cuts the rough first pass (unit vectors, euclidean).
pub const THRESHOLD: f64 = 0.6;

/// A speaker's prior below which VBx has dropped it.
const DROPPED: f64 = 1e-7;

/// One speaker VBx kept: its centroid in the raw embedding space, and its
/// share of the training embeddings.
#[derive(Clone, Debug, PartialEq)]
pub struct Speaker {
    /// The responsibility-weighted mean of the raw embeddings.
    pub centroid: Vec<f32>,
    /// The speaker's prior.
    pub weight: f64,
}

/// The speakers in `train` (raw embeddings).
pub fn speakers(train: &[&[f32]], plda: &Plda, params: VbxParams) -> Vec<Speaker> {
    if train.len() < 2 {
        let mean = weighted_mean(train, &vec![1.0; train.len()]);
        return mean
            .into_iter()
            .map(|centroid| Speaker {
                centroid,
                weight: 1.0,
            })
            .collect();
    }
    let unit: Vec<Vec<f64>> = train.iter().map(|x| unit_f64(x)).collect();
    let rough = ahc::clusters(&unit, THRESHOLD);
    let fea: Vec<Vec<f64>> = train.iter().map(|x| plda.transform(x)).collect();
    let refined = vbx(&fea, plda.phi(), &rough, params);
    refined
        .pi
        .iter()
        .enumerate()
        .filter(|(_, prior)| **prior > DROPPED)
        .filter_map(|(s, prior)| {
            let weights: Vec<f64> = refined
                .gamma
                .iter()
                .map(|row| row.get(s).copied().unwrap_or(0.0))
                .collect();
            weighted_mean(train, &weights).map(|centroid| Speaker {
                centroid,
                weight: *prior,
            })
        })
        .collect()
}

/// Each embedding's speaker: the most similar centroid, with the speakers
/// of one window (`window` per embedding) on distinct centroids where there
/// are enough of them.
pub fn assign(embeddings: &[&[f32]], windows: &[usize], centroids: &[Vec<f32>]) -> Vec<usize> {
    let scores: Vec<Vec<f64>> = embeddings
        .iter()
        .map(|x| centroids.iter().map(|c| 1.0 + cosine(x, c)).collect())
        .collect();
    let mut labels: Vec<usize> = scores.iter().map(|row| best(row)).collect();
    let mut start = 0;
    while start < windows.len() {
        let window = windows.get(start).copied();
        let end = start
            + windows
                .iter()
                .skip(start)
                .take_while(|w| Some(**w) == window)
                .count();
        let chosen = scores.get(start..end).and_then(distinct);
        if let (Some(chosen), Some(slots)) = (chosen, labels.get_mut(start..end)) {
            slots.copy_from_slice(&chosen);
        }
        start = end.max(start + 1);
    }
    labels
}

/// The highest-scoring assignment of `rows` to distinct centroids, when
/// there are at least as many centroids as rows (a window has at most
/// three speakers, so every choice is tried).
fn distinct(rows: &[Vec<f64>]) -> Option<Vec<usize>> {
    let centroids = rows.first().map_or(0, Vec::len);
    if rows.len() < 2 || rows.len() > centroids {
        return None;
    }
    let score = |labels: &[usize]| -> f64 {
        rows.iter()
            .zip(labels)
            .map(|(row, k)| row.get(*k).copied().unwrap_or(0.0))
            .sum()
    };
    choices(rows.len(), centroids)
        .into_iter()
        .max_by(|a, b| score(a).total_cmp(&score(b)))
}

/// Every way to give `rows` rows distinct labels below `centroids`.
fn choices(rows: usize, centroids: usize) -> Vec<Vec<usize>> {
    if rows == 0 {
        return vec![Vec::new()];
    }
    choices(rows - 1, centroids)
        .into_iter()
        .flat_map(|prefix| {
            (0..centroids)
                .filter(|k| !prefix.contains(k))
                .map(|k| [prefix.as_slice(), &[k]].concat())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn best(row: &[f64]) -> usize {
    row.iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(index, _)| index)
}

fn weighted_mean(vectors: &[&[f32]], weights: &[f64]) -> Option<Vec<f32>> {
    let dims = vectors.first()?.len();
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, reason = "embedding values are f32")]
    let mean = (0..dims)
        .map(|d| {
            let sum: f64 = vectors
                .iter()
                .zip(weights)
                .map(|(v, w)| w * f64::from(v.get(d).copied().unwrap_or(0.0)))
                .sum();
            (sum / total) as f32
        })
        .collect();
    Some(mean)
}

fn unit_f64(x: &[f32]) -> Vec<f64> {
    let values: Vec<f64> = x.iter().map(|v| f64::from(*v)).collect();
    let norm = values.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm == 0.0 {
        return values;
    }
    values.iter().map(|v| v / norm).collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let norm = |v: &[f32]| v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    dot / (norm(a) * norm(b)).max(f64::MIN_POSITIVE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speakers_of_one_window_take_distinct_centroids() {
        let centroids = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        // Both lean towards the first centroid; the second leans less.
        let a: &[f32] = &[1.0, 0.1];
        let b: &[f32] = &[1.0, 0.6];
        let lone: &[f32] = &[1.0, 0.6];

        let labels = assign(&[a, b, lone], &[0, 0, 1], &centroids);

        assert_eq!(labels, vec![0, 1, 0]);
    }

    #[test]
    fn more_speakers_than_centroids_take_the_nearest() {
        let centroids = vec![vec![1.0, 0.0]];
        let a: &[f32] = &[1.0, 0.0];
        let b: &[f32] = &[0.9, 0.1];

        assert_eq!(assign(&[a, b], &[3, 3], &centroids), vec![0, 0]);
    }

    #[test]
    fn every_distinct_choice_is_tried() {
        assert_eq!(choices(2, 3).len(), 6);
        assert!(choices(2, 3).iter().all(|c| c[0] != c[1]));
    }
}
