//! Speaker embeddings: WeSpeaker ResNet34-LM over filterbank features.
//!
//! Adapted from anarlog `crates/embedding/src/onnx/mod.rs` (MIT). Deino's
//! changes: the model comes from the model store, the filterbank is shared
//! per window ([`features`]), and inputs with the same frame count run as one
//! batch per `ort` call. Batching is exact: the model pools statistics over
//! each item's own frames, so only equal-length inputs can share a call
//! (padding would change the result).

pub mod features;

use std::collections::BTreeMap;
use std::path::Path;

use knf_rs::MEL_BINS;
use ort::session::Session;
use ort::value::TensorRef;

use crate::config::RuntimeConfig;
use crate::error::DiarizeError;
use crate::onnx;
use crate::vector::{EMBEDDING_DIM, Embedding};
pub use features::{AudioFeatures, Features};

/// The model's input and output names.
const INPUT: &str = "feats";
const OUTPUT: &str = "embs";

/// The speaker-embedding model.
pub struct EmbeddingExtractor {
    session: Session,
}

impl EmbeddingExtractor {
    /// Loads the model at `path`, on Core ML when `runtime` asks for it.
    pub fn new(path: &Path, runtime: &RuntimeConfig) -> Result<Self, DiarizeError> {
        Ok(Self {
            session: onnx::accelerated_session(path, runtime)?,
        })
    }

    /// The embedding of a whole utterance of 16 kHz `samples`; `None` when it
    /// is too short or the model cannot embed it.
    pub fn embed_samples(&mut self, samples: &[f32]) -> Result<Option<Embedding>, DiarizeError> {
        let Some(features) = AudioFeatures::compute(samples)? else {
            return Ok(None);
        };
        let mut embeddings = self.embed_all(&[features.all()])?;
        Ok(embeddings.pop().flatten())
    }

    /// One embedding per input, in input order (`None` where the model
    /// returned non-finite values). Inputs of equal length share a batch.
    pub fn embed_all(
        &mut self,
        inputs: &[Features],
    ) -> Result<Vec<Option<Embedding>>, DiarizeError> {
        let mut by_length: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (index, input) in inputs.iter().enumerate() {
            by_length.entry(input.frames()).or_default().push(index);
        }
        let mut results: Vec<Option<Embedding>> = vec![None; inputs.len()];
        for (frames, indices) in by_length {
            let batch: Vec<&Features> = indices.iter().filter_map(|i| inputs.get(*i)).collect();
            let embeddings = self.run(frames, &batch)?;
            for (index, embedding) in indices.into_iter().zip(embeddings) {
                if let Some(slot) = results.get_mut(index) {
                    *slot = embedding;
                }
            }
        }
        Ok(results)
    }

    /// Runs `batch` inputs of `frames` frames each in one call.
    fn run(
        &mut self,
        frames: usize,
        batch: &[&Features],
    ) -> Result<Vec<Option<Embedding>>, DiarizeError> {
        let values: Vec<f32> = batch
            .iter()
            .flat_map(|input| input.as_slice())
            .copied()
            .collect();
        let input =
            TensorRef::from_array_view(([batch.len(), frames, MEL_BINS], values.as_slice()))?;
        let outputs = self.session.run(ort::inputs![INPUT => input])?;
        let output = outputs.get(OUTPUT).ok_or(DiarizeError::OutputShape {
            expected: batch.len() * EMBEDDING_DIM,
            actual: 0,
        })?;
        let (_, embeddings) = output.try_extract_tensor::<f32>()?;
        if embeddings.len() != batch.len() * EMBEDDING_DIM {
            return Err(DiarizeError::OutputShape {
                expected: batch.len() * EMBEDDING_DIM,
                actual: embeddings.len(),
            });
        }
        Ok(embeddings
            .chunks_exact(EMBEDDING_DIM)
            .map(|values| Embedding::new(values.to_vec()))
            .collect())
    }
}
