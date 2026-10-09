//! The offline diarizer: who spoke when on one recorded track.
//!
//! Adapted from anarlog `crates/pyannote-local/src/pipeline.rs` (MIT), a CPU
//! port of pyannote.audio 3.1, split by stage:
//!
//! 1. [`scan`]: slide 10 s windows over the audio; segment each (batched),
//!    then embed every local speaker with enough speech (batched by length);
//! 2. [`labels`]: cluster the clean anchors, attach the rest to the nearest
//!    centroid;
//! 3. [`known`]: merge clusters that match the same known voiceprint and
//!    name the survivors;
//! 4. [`reconstruct`]: vote every window's activity onto the global frame
//!    grid and turn the majority into speaker turns, numbered by first
//!    appearance.

mod known;
mod labels;
mod reconstruct;
mod scan;

use std::path::PathBuf;

use deino_domain::{Millis, PersonId};

use crate::config::{DiarizationConfig, SpeakerBounds};
use crate::embedding::EmbeddingExtractor;
use crate::error::DiarizeError;
use crate::segmentation::Segmenter;
use crate::vector::Embedding;

/// Where the two ONNX models are on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelPaths {
    /// pyannote segmentation-3.0.
    pub segmentation: PathBuf,
    /// WeSpeaker ResNet34-LM.
    pub embedding: PathBuf,
}

/// Random-access mono 16 kHz audio, so a long recording need not be held in
/// memory by every implementation.
pub trait AudioSource {
    /// Total samples.
    fn sample_count(&self) -> usize;

    /// Fills `out` with the samples from `start`, zero-padding past the end.
    fn read(&self, start: usize, out: &mut [f32]) -> Result<(), DiarizeError>;
}

impl AudioSource for [f32] {
    fn sample_count(&self) -> usize {
        self.len()
    }

    fn read(&self, start: usize, out: &mut [f32]) -> Result<(), DiarizeError> {
        let available = self.get(start..).unwrap_or_default();
        let copied = available.len().min(out.len());
        let (head, tail) = out.split_at_mut(copied);
        head.copy_from_slice(available.get(..copied).unwrap_or_default());
        tail.fill(0.0);
        Ok(())
    }
}

/// A voiceprint the caller already trusts (a confirmed exemplar of a
/// meeting attendee): it merges fragmented clusters of the same voice and
/// names the speaker it matches.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownSpeaker {
    /// Whose voice it is.
    pub person: PersonId,
    /// One exemplar; a person may have several.
    pub embedding: Embedding,
}

/// One diarization run's inputs besides the audio.
#[derive(Default)]
pub struct DiarizeRequest<'a> {
    /// How many speakers there may be (always pass calendar bounds).
    pub bounds: SpeakerBounds,
    /// Voiceprints of the people expected on the track.
    pub known_speakers: &'a [KnownSpeaker],
    /// Polled between batches; `true` stops the run with `Cancelled`.
    pub is_cancelled: Option<&'a dyn Fn() -> bool>,
    /// Called after each batch with the fraction of windows done.
    pub on_progress: Option<&'a dyn Fn(f32)>,
}

/// A stretch of one speaker's speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeakerTurn {
    /// Start, from the start of the audio.
    pub start: Millis,
    /// End; after `start`.
    pub end: Millis,
    /// Index into [`Diarization::speakers`].
    pub speaker: usize,
}

/// A known voiceprint a speaker matched uniquely.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeakerIdentity {
    /// Who it is.
    pub person: PersonId,
    /// The cosine similarity that decided it.
    pub score: f32,
}

/// One speaker of the result.
#[derive(Clone, Debug, PartialEq)]
pub struct DiarizedSpeaker {
    /// 0-based, in order of first appearance.
    pub index: usize,
    /// The mean of the speaker's anchor embeddings, L2-normalised (256-d).
    pub centroid: Embedding,
    /// Total length of the speaker's turns.
    pub speech: Millis,
    /// The known voiceprint it matched, if any.
    pub identity: Option<SpeakerIdentity>,
}

/// Who spoke when.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diarization {
    /// Turns sorted by start, then speaker.
    pub turns: Vec<SpeakerTurn>,
    /// Every speaker with at least one turn, by index.
    pub speakers: Vec<DiarizedSpeaker>,
}

/// The diarizer: both models loaded, ready for any number of runs.
pub struct Diarizer {
    segmenter: Segmenter,
    embedder: EmbeddingExtractor,
    config: DiarizationConfig,
}

impl Diarizer {
    /// Loads the models (segmentation on the CPU, embeddings on Core ML when
    /// `config.runtime` asks for it).
    pub fn new(config: DiarizationConfig, models: &ModelPaths) -> Result<Self, DiarizeError> {
        Ok(Self {
            segmenter: Segmenter::new(&models.segmentation, &config.runtime)?,
            embedder: EmbeddingExtractor::new(&models.embedding, &config.runtime)?,
            config,
        })
    }

    /// The configuration runs use.
    pub fn config(&self) -> &DiarizationConfig {
        &self.config
    }

    /// Replaces the configuration for later runs (the models stay loaded).
    pub fn set_config(&mut self, config: DiarizationConfig) {
        self.config = config;
    }

    /// The embedding model, for voiceprints and live clustering.
    pub fn embedder(&mut self) -> &mut EmbeddingExtractor {
        &mut self.embedder
    }

    /// Diarizes `audio`.
    pub fn diarize(
        &mut self,
        audio: &(impl AudioSource + ?Sized),
        request: &DiarizeRequest<'_>,
    ) -> Result<Diarization, DiarizeError> {
        let grid = scan::WindowGrid::new(audio.sample_count(), &self.config);
        let Some(grid) = grid else {
            return Ok(Diarization::default());
        };
        let models = scan::Models {
            segmenter: &mut self.segmenter,
            embedder: &mut self.embedder,
            config: &self.config,
        };
        let scanned = scan::scan(models, audio, &grid, request)?;
        if scanned.embeddings.is_empty() {
            return Ok(Diarization::default());
        }
        let mut clusters = labels::label(&scanned.embeddings, &self.config, &grid, request.bounds);
        known::apply(&scanned.embeddings, &mut clusters, request, &self.config);
        Ok(reconstruct::reconstruct(
            &scanned,
            &grid,
            &clusters,
            &self.config,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slice_source_zero_pads_past_its_end() {
        let audio = [1.0_f32, 2.0, 3.0];
        let mut out = [9.0; 4];
        audio.as_slice().read(1, &mut out).unwrap();
        assert_eq!(
            out.map(f32::to_bits),
            [2.0_f32, 3.0, 0.0, 0.0].map(f32::to_bits)
        );
        audio.as_slice().read(10, &mut out).unwrap();
        assert!(out.iter().all(|sample| sample.to_bits() == 0));
        assert_eq!(audio.as_slice().sample_count(), 3);
    }
}
