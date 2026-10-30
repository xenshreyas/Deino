//! The engine behind `deino-core`'s `TrackDiarizer` port.
//!
//! One run: make sure the pinned models are in the store (downloading what
//! is missing through the [`ByteSource`] port, verified), decode the finalized track at
//! 16 kHz with `deino-audio` (`ExtAudioFile`, so the AAC priming is trimmed
//! and timestamps stay on the recording's timeline), choose the parameters
//! for its length ([`policy`]), diarize with the request's known voiceprints
//! (fragments of one known voice merge), then prune phantom speakers. All
//! of it runs on a blocking thread, and the models are loaded per run so a
//! finished meeting holds no memory.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::diarization::{
    KnownVoice, SpeakerHint, TrackDiarization, TrackDiarizer, TrackError, TrackRequest, Voice,
    VoiceTurn,
};
use deino_core::models::ByteSource;

use crate::config::{DiarizationConfig, RuntimeConfig, SpeakerBounds};
use crate::error::DiarizeError;
use crate::models::{EMBEDDING_MODEL_ID, ModelStore};
use crate::pipeline::{Diarization, DiarizeRequest, Diarizer, KnownSpeaker, ModelPaths};
use crate::policy::{self, ExpectedSpeakers};
use crate::prune;
use crate::units::{SAMPLE_RATE, samples_to_duration};
use crate::vbx::Plda;
use crate::vector::Embedding;

/// The ONNX diarizer as a `TrackDiarizer`.
///
/// Runs are serialised: two meetings processed at once would otherwise
/// download into the same `.partial` files and compile the Core ML model into
/// the same cache, and each run holds a whole decoded track in memory.
pub struct OnnxTrackDiarizer {
    store: ModelStore,
    source: Arc<dyn ByteSource>,
    runtime: RuntimeConfig,
    vbx: Option<Arc<Plda>>,
    one_at_a_time: tokio::sync::Mutex<()>,
}

impl OnnxTrackDiarizer {
    /// A diarizer over the models in `store`, fetched through `source` when
    /// missing.
    pub fn new(store: ModelStore, source: Arc<dyn ByteSource>, runtime: RuntimeConfig) -> Self {
        Self {
            store,
            source,
            runtime,
            vbx: None,
            one_at_a_time: tokio::sync::Mutex::new(()),
        }
    }

    /// Clusters with Community-1's VBx over `plda` instead of centroid
    /// linkage.
    #[must_use]
    pub fn with_vbx(mut self, plda: Arc<Plda>) -> Self {
        self.vbx = Some(plda);
        self
    }
}

#[async_trait]
impl TrackDiarizer for OnnxTrackDiarizer {
    fn embedding_model(&self) -> &str {
        EMBEDDING_MODEL_ID
    }

    async fn diarize(&self, request: TrackRequest) -> Result<TrackDiarization, TrackError> {
        let _running = self.one_at_a_time.lock().await;
        let models = self
            .store
            .ensure(self.source.as_ref())
            .await
            .map_err(|error| TrackError::ModelsUnavailable(Box::new(error)))?;
        let (runtime, vbx) = (self.runtime.clone(), self.vbx.clone());
        let run =
            tokio::task::spawn_blocking(move || diarize_file(&models, runtime, vbx, &request));
        run.await
            .map_err(|error| TrackError::Failed(Box::new(error)))?
    }
}

/// Decodes and diarizes one track file with the models at `models`
/// (blocking): the whole engine run of [`OnnxTrackDiarizer`], without the
/// store. Benchmarks and model tests call it with local model files.
pub fn diarize_file(
    models: &ModelPaths,
    runtime: RuntimeConfig,
    vbx: Option<Arc<Plda>>,
    request: &TrackRequest,
) -> Result<TrackDiarization, TrackError> {
    let samples = deino_audio::read_mono(&request.audio, SAMPLE_RATE)
        .map_err(|error| TrackError::Failed(Box::new(error)))?;
    let duration = samples_to_duration(samples.len());
    let bounds = policy::bounds_for(duration, expected(request.speakers));
    let config = DiarizationConfig {
        vbx,
        ..policy::config_for(duration, runtime)
    };
    let mut diarizer = Diarizer::new(config, models).map_err(engine)?;
    let known = known_speakers(&request.known);
    let run = DiarizeRequest {
        bounds,
        known_speakers: &known,
        ..DiarizeRequest::default()
    };
    let diarization = diarizer.diarize(samples.as_slice(), &run).map_err(engine)?;
    log::info!(
        target: "deino_diarize",
        "diarized {:.0} s of audio into {} speakers",
        duration.as_secs_f64(),
        diarization.speakers.len()
    );
    Ok(to_track(post_process(diarization, bounds)))
}

/// Collapse detection, then phantom pruning (OpenWhispr's order: a
/// collapse is judged on the raw clusters). A run limited to one speaker is
/// never "collapsed".
pub fn post_process(diarization: Diarization, bounds: SpeakerBounds) -> Diarization {
    let (_, max) = bounds.range();
    if max > 1 && prune::is_collapsed(&diarization) {
        return prune::collapse_to_one(diarization);
    }
    prune::prune_phantoms(diarization)
}

/// The port's voiceprints as the engine's known speakers; a vector the
/// model could not have produced (not finite) is dropped.
pub fn known_speakers(known: &[KnownVoice]) -> Vec<KnownSpeaker> {
    known
        .iter()
        .filter_map(|voice| {
            Some(KnownSpeaker {
                person: voice.person,
                embedding: Embedding::new(voice.embedding.clone())?,
            })
        })
        .collect()
}

/// The port's speaker hint as the policy's expectation.
pub fn expected(hint: SpeakerHint) -> ExpectedSpeakers {
    match hint {
        SpeakerHint::Unknown => ExpectedSpeakers::Unknown,
        SpeakerHint::AtMost(count) => ExpectedSpeakers::AtMost(usize::from(count)),
        SpeakerHint::Exactly(count) => ExpectedSpeakers::Exactly(usize::from(count)),
    }
}

/// The engine's result in the port's terms.
pub fn to_track(diarization: Diarization) -> TrackDiarization {
    TrackDiarization {
        turns: diarization
            .turns
            .iter()
            .map(|turn| VoiceTurn {
                start: turn.start,
                end: turn.end,
                voice: turn.speaker,
            })
            .collect(),
        voices: diarization
            .speakers
            .into_iter()
            .map(|speaker| Voice {
                centroid: speaker.centroid.into_vec(),
                speech: speaker.speech,
            })
            .collect(),
    }
}

fn engine(error: DiarizeError) -> TrackError {
    match error {
        DiarizeError::Cancelled => TrackError::Cancelled,
        error => TrackError::Failed(Box::new(error)),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::Millis;

    use super::*;
    use crate::pipeline::{DiarizedSpeaker, SpeakerTurn};
    use crate::vector::Embedding;

    fn diarization(turns: &[(usize, u32, u32)]) -> Diarization {
        let speakers = turns.iter().map(|t| t.0 + 1).max().unwrap_or(0);
        Diarization {
            turns: turns
                .iter()
                .map(|&(speaker, start, end)| SpeakerTurn {
                    start: Millis(start),
                    end: Millis(end),
                    speaker,
                })
                .collect(),
            speakers: (0..speakers)
                .map(|index| DiarizedSpeaker {
                    index,
                    centroid: Embedding::new(vec![1.0, [0.0, 1.0, 2.0][index]]).unwrap(),
                    speech: Millis(1),
                    identity: None,
                })
                .collect(),
        }
    }

    #[test]
    fn known_voices_become_known_speakers_unless_unusable() {
        let clock = deino_domain::FakeClock::new(deino_testkit::at("2026-10-11T09:00:00Z"));
        let person = deino_domain::PersonId::generate(&clock);
        let voice = |embedding: Vec<f32>| KnownVoice { person, embedding };

        let known = known_speakers(&[voice(vec![0.6, 0.8]), voice(vec![f32::NAN, 1.0])]);

        assert_eq!(known.len(), 1);
        assert_eq!(known[0].person, person);
        assert_eq!(known[0].embedding.as_slice(), [0.6, 0.8]);
    }

    #[test]
    fn hints_map_to_expectations() {
        assert_eq!(expected(SpeakerHint::Unknown), ExpectedSpeakers::Unknown);
        assert_eq!(
            expected(SpeakerHint::AtMost(3)),
            ExpectedSpeakers::AtMost(3)
        );
        assert_eq!(
            expected(SpeakerHint::Exactly(2)),
            ExpectedSpeakers::Exactly(2)
        );
    }

    #[test]
    fn results_map_turn_for_turn_and_voice_for_speaker() {
        let track = to_track(diarization(&[(0, 0, 4_000), (1, 4_000, 9_000)]));
        assert_eq!(
            track.turns[1],
            VoiceTurn {
                start: Millis(4_000),
                end: Millis(9_000),
                voice: 1
            }
        );
        assert_eq!(track.voices[1].centroid, vec![1.0, 1.0]);
    }

    #[test]
    fn a_collapsed_run_becomes_one_speaker_unless_only_one_was_allowed() {
        let mut turns = vec![(0, 0, 300_000)];
        turns.extend((0..5).map(|i| (1, 300_000 + i * 3_000, 301_500 + i * 3_000)));
        let collapsed = post_process(diarization(&turns), SpeakerBounds::at_most(4));
        assert_eq!(collapsed.speakers.len(), 1);
        let capped = post_process(diarization(&turns), SpeakerBounds::exact(1));
        assert_eq!(
            capped.speakers.len(),
            2,
            "a one-speaker cap is the clustering's job"
        );
    }

    #[test]
    fn phantoms_are_pruned_after_the_collapse_check() {
        let mut turns = vec![(0, 0, 100_000), (1, 100_000, 200_000)];
        turns.extend((0..5).map(|i| (2, 200_000 + i * 3_000, 201_500 + i * 3_000)));
        let pruned = post_process(diarization(&turns), SpeakerBounds::default());
        assert_eq!(pruned.speakers.len(), 2);
    }
}
