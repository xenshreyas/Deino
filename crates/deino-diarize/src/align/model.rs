//! wav2vec2-base-960h on ONNX Runtime: a stretch of 16 kHz audio in, a
//! log-probability per vocabulary entry for every 20 ms frame out.

use std::path::Path;

use ort::session::Session;
use ort::value::TensorRef;

use crate::config::RuntimeConfig;
use crate::error::DiarizeError;

/// The model's vocabulary size.
pub const CLASSES: usize = 32;

/// The loaded model (CPU; Core ML does not run its int8 weights).
pub struct Aligner {
    session: Session,
}

impl Aligner {
    /// Loads the model at `path`.
    pub fn load(path: &Path, runtime: &RuntimeConfig) -> Result<Self, DiarizeError> {
        Ok(Self {
            session: crate::onnx::cpu_session(path, runtime)?,
        })
    }

    /// The log-probabilities for `samples`, `frames × CLASSES`, row-major.
    pub fn emission(&mut self, samples: &[f32]) -> Result<Vec<f32>, DiarizeError> {
        let normalised = normalise(samples);
        let input = TensorRef::from_array_view(([1, normalised.len()], normalised.as_slice()))?;
        let outputs = self.session.run(ort::inputs![input])?;
        let value = outputs.values().next().ok_or(DiarizeError::OutputShape {
            expected: 1,
            actual: 0,
        })?;
        let (_, logits) = value.try_extract_tensor::<f32>()?;
        if logits.len() % CLASSES != 0 {
            return Err(DiarizeError::OutputShape {
                expected: CLASSES,
                actual: logits.len() % CLASSES,
            });
        }
        let mut emission = logits.to_vec();
        for row in emission.chunks_mut(CLASSES) {
            log_softmax(row);
        }
        Ok(emission)
    }
}

/// Zero mean, unit variance, as the model's feature extractor does it.
fn normalise(samples: &[f32]) -> Vec<f32> {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a meeting's sample count is far below 2^52"
    )]
    let count = samples.len().max(1) as f64;
    let mean = samples.iter().map(|&x| f64::from(x)).sum::<f64>() / count;
    let variance = samples
        .iter()
        .map(|&x| (f64::from(x) - mean).powi(2))
        .sum::<f64>()
        / count;
    let scale = (variance + 1e-7).sqrt();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the model takes f32; normalised samples are near unit scale"
    )]
    samples
        .iter()
        .map(|&x| ((f64::from(x) - mean) / scale) as f32)
        .collect()
}

/// `row` as log-probabilities.
fn log_softmax(row: &mut [f32]) {
    let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let total: f32 = row.iter().map(|&x| (x - max).exp()).sum();
    let log_total = total.ln() + max;
    for x in row.iter_mut() {
        *x -= log_total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_is_normalised_to_zero_mean_and_unit_variance() {
        let normalised = normalise(&[1.0, 2.0, 3.0, 4.0]);

        let mean: f32 = normalised.iter().sum::<f32>() / 4.0;
        let variance: f32 = normalised.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / 4.0;
        assert!(mean.abs() < 1e-6);
        assert!((variance - 1.0).abs() < 1e-4);
    }

    #[test]
    fn log_softmax_rows_sum_to_one_in_probability() {
        let mut row = [1.0, 2.0, 3.0];

        log_softmax(&mut row);

        let total: f32 = row.iter().map(|x| x.exp()).sum();
        assert!((total - 1.0).abs() < 1e-6);
        assert!(row[2] > row[1] && row[1] > row[0]);
    }
}
