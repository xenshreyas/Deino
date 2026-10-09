//! Speaker embeddings and the vector maths on them.
//!
//! WeSpeaker ResNet34-LM embeds a voice as 256 numbers. Clustering compares
//! unit-normalised embeddings by euclidean distance (a monotone function of
//! cosine distance on the unit sphere); voiceprints compare by cosine
//! similarity, which ignores scale.

/// Dimensions of a WeSpeaker ResNet34-LM embedding.
pub const EMBEDDING_DIM: usize = 256;

/// A speaker embedding with finite values.
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    /// Wraps `values`, or `None` when any is NaN or infinite (the model's
    /// answer to audio it cannot embed).
    pub fn new(values: Vec<f32>) -> Option<Self> {
        values
            .iter()
            .all(|value| value.is_finite())
            .then_some(Self(values))
    }

    /// The values.
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// The values, consuming the embedding.
    pub fn into_vec(self) -> Vec<f32> {
        self.0
    }

    /// This embedding scaled to unit length (unchanged if it is all zeros).
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self(normalize(&self.0))
    }

    /// Cosine similarity with `other`; `None` for mismatched or zero vectors.
    pub fn cosine(&self, other: &Self) -> Option<f32> {
        cosine_similarity(&self.0, &other.0)
    }
}

/// `vector` scaled to unit length; zero or non-finite norms leave it as is.
pub fn normalize(vector: &[f32]) -> Vec<f32> {
    let norm = norm(vector);
    if norm == 0.0 || !norm.is_finite() {
        return vector.to_vec();
    }
    vector.iter().map(|value| value / norm).collect()
}

/// Euclidean length.
pub fn norm(vector: &[f32]) -> f32 {
    vector.iter().map(|value| value * value).sum::<f32>().sqrt()
}

/// Euclidean distance.
pub fn euclidean(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(l, r)| (l - r) * (l - r))
        .sum::<f32>()
        .sqrt()
}

/// Cosine similarity in `[-1, 1]`; `None` for mismatched lengths or a zero
/// vector.
pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let dot: f32 = left.iter().zip(right).map(|(l, r)| l * r).sum();
    let denominator = norm(left) * norm(right);
    (denominator > 0.0).then(|| (dot / denominator).clamp(-1.0, 1.0))
}

/// The element-wise mean of `vectors` (each normalised first when
/// `unit` is set); empty when there are none.
pub fn mean<'a>(vectors: impl IntoIterator<Item = &'a [f32]>, unit: bool) -> Vec<f32> {
    let mut sum: Vec<f32> = Vec::new();
    let mut count = 0_u32;
    for vector in vectors {
        let vector = if unit {
            normalize(vector)
        } else {
            vector.to_vec()
        };
        if sum.is_empty() {
            sum = vec![0.0; vector.len()];
        }
        sum.iter_mut()
            .zip(&vector)
            .for_each(|(total, value)| *total += value);
        count += 1;
    }
    #[allow(clippy::cast_precision_loss, reason = "vector counts stay below 2^24")]
    let count = count as f32;
    for total in &mut sum {
        *total /= count.max(1.0);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_values_are_not_an_embedding() {
        assert!(Embedding::new(vec![1.0, f32::NAN]).is_none());
        assert!(Embedding::new(vec![1.0, f32::INFINITY]).is_none());
        assert!(Embedding::new(vec![1.0, 2.0]).is_some());
    }

    #[test]
    fn normalizing_yields_unit_length_and_keeps_zero_vectors() {
        let unit = normalize(&[3.0, 4.0]);
        assert!((norm(&unit) - 1.0).abs() < 1e-6);
        assert_eq!(normalize(&[0.0, 0.0]), vec![0.0, 0.0]);
    }

    #[test]
    fn cosine_ignores_scale_and_rejects_mismatches() {
        let similarity = cosine_similarity(&[1.0, 0.0], &[5.0, 0.0]).unwrap();
        assert!((similarity - 1.0).abs() < 1e-6);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), Some(0.0));
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 0.0]), None);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]), None);
    }

    #[test]
    fn mean_of_unit_vectors_averages_directions() {
        let vectors = [vec![2.0, 0.0], vec![0.0, 4.0]];
        let mean = mean(vectors.iter().map(Vec::as_slice), true);
        assert_eq!(mean, vec![0.5, 0.5]);
        assert!(super::mean(std::iter::empty(), true).is_empty());
    }

    #[test]
    fn euclidean_distance_of_unit_vectors_tracks_cosine() {
        let near = euclidean(&normalize(&[1.0, 0.1]), &[1.0, 0.0]);
        let far = euclidean(&normalize(&[0.1, 1.0]), &[1.0, 0.0]);
        assert!(near < far);
    }
}
