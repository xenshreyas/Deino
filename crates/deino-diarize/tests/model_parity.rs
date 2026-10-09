//! Checks on the real models that pin the CAP-03 model decision
//! (`docs/perf/diarization.md`). They need the ONNX files, so they are
//! ignored by default:
//!
//! ```bash
//! DEINO_DIARIZE_MODELS=<dir> cargo test -p deino-diarize --release \
//!     --test model_parity -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `<dir>` holds `pyannote-segmentation-3.0.onnx` and
//! `wespeaker-voxceleb-resnet34-lm.onnx` (the model store's names), plus,
//! for the parity check, anarlog's bundled `anarlog-embedding.onnx` and
//! sherpa-onnx's `wespeaker-sherpa.onnx`.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies"
)]

use std::path::{Path, PathBuf};

use deino_diarize::embedding::{AudioFeatures, EmbeddingExtractor};
use deino_diarize::models::{EMBEDDING, SEGMENTATION};
use deino_diarize::segmentation::Segmenter;
use deino_diarize::units::WINDOW_SAMPLES;
use deino_diarize::{Embedding, RuntimeConfig};
use deino_testkit::Fixtures;
use deino_testkit::fixture::{Channel, MeetingKind};

fn models() -> PathBuf {
    PathBuf::from(std::env::var("DEINO_DIARIZE_MODELS").expect("set DEINO_DIARIZE_MODELS"))
}

fn cpu() -> RuntimeConfig {
    RuntimeConfig {
        accelerate_embedding: false,
        ..RuntimeConfig::default()
    }
}

/// Every spoken segment of the standup fixture's clip, as 16 kHz samples.
fn utterances() -> Vec<Vec<f32>> {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::Standup).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let mut spans = Vec::new();
    for channel in [Channel::Mic, Channel::System] {
        let samples =
            deino_audio::read_mono(&fixtures.audio_path(audio, channel).unwrap(), 16_000).unwrap();
        for segment in meeting.segments.iter().filter(|s| s.channel == channel) {
            let (start, end) = (
                segment.start_ms.get() as usize * 16,
                segment.end_ms.get() as usize * 16,
            );
            if end <= samples.len() {
                spans.push(samples[start..end].to_vec());
            }
        }
    }
    spans
}

fn embed_all(model: &Path, runtime: &RuntimeConfig, spans: &[Vec<f32>]) -> Vec<Embedding> {
    let mut extractor = EmbeddingExtractor::new(model, runtime).unwrap();
    spans
        .iter()
        .map(|span| extractor.embed_samples(span).unwrap().unwrap())
        .collect()
}

fn min_cosine(left: &[Embedding], right: &[Embedding]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(l, r)| l.cosine(r).unwrap())
        .fold(1.0, f32::min)
}

#[test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_MODELS)"]
fn the_pinned_embedding_model_matches_anarlogs_bundled_one() {
    let spans = utterances();
    assert!(spans.len() >= 8, "{} utterances", spans.len());
    let anarlog = embed_all(&models().join("anarlog-embedding.onnx"), &cpu(), &spans);
    let pinned = embed_all(&models().join(EMBEDDING.file_name), &cpu(), &spans);
    let sherpa = embed_all(&models().join("wespeaker-sherpa.onnx"), &cpu(), &spans);
    let (pinned_vs_anarlog, sherpa_vs_anarlog) =
        (min_cosine(&pinned, &anarlog), min_cosine(&sherpa, &anarlog));
    println!(
        "min cosine over {} utterances: pinned {pinned_vs_anarlog:.6}, sherpa {sherpa_vs_anarlog:.6}",
        spans.len()
    );
    assert!(pinned_vs_anarlog >= 0.999);
    assert!(sherpa_vs_anarlog >= 0.999);
}

#[test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_MODELS)"]
fn core_ml_embeddings_match_the_cpu() {
    let spans = utterances();
    let model = models().join(EMBEDDING.file_name);
    let on_cpu = embed_all(&model, &cpu(), &spans);
    let on_core_ml = embed_all(&model, &RuntimeConfig::default(), &spans);
    let similarity = min_cosine(&on_cpu, &on_core_ml);
    println!("min cosine Core ML vs CPU: {similarity:.6}");
    assert!(similarity >= 0.999);
}

#[test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_MODELS)"]
fn a_batch_of_equal_length_inputs_embeds_like_single_runs() {
    let spans = utterances();
    let shortest = spans.iter().map(Vec::len).min().unwrap();
    let inputs: Vec<_> = spans
        .iter()
        .map(|span| {
            AudioFeatures::compute(&span[..shortest])
                .unwrap()
                .unwrap()
                .all()
        })
        .collect();
    let mut extractor =
        EmbeddingExtractor::new(&models().join(EMBEDDING.file_name), &cpu()).unwrap();
    let batched = extractor.embed_all(&inputs).unwrap();
    for (input, batched) in inputs.iter().zip(batched) {
        let single = extractor
            .embed_all(std::slice::from_ref(input))
            .unwrap()
            .remove(0)
            .unwrap();
        assert!(single.cosine(&batched.unwrap()).unwrap() > 0.9999);
    }
}

#[test]
#[ignore = "needs the ONNX models (DEINO_DIARIZE_MODELS)"]
fn a_batch_of_windows_segments_like_single_windows() {
    let spans = utterances();
    let audio: Vec<f32> = spans.concat();
    let windows: Vec<f32> = audio
        .iter()
        .copied()
        .cycle()
        .take(4 * WINDOW_SAMPLES)
        .collect();
    let mut segmenter = Segmenter::new(&models().join(SEGMENTATION.file_name), &cpu()).unwrap();
    let batched = segmenter.run(&windows).unwrap();
    for (window, batched) in windows.chunks_exact(WINDOW_SAMPLES).zip(&batched) {
        assert_eq!(&segmenter.run(window).unwrap()[0], batched);
    }
}
