//! An A/B of the speaker engines on one real call: the same transcript lines
//! attributed by the stage's own rule (`deino_core::diarization::attribute`)
//! once with the Rust engine (ONNX diarizer and word aligner, the bundled
//! models) and once with the Python engine's saved answer. Voices are named
//! from saved voiceprints by the voiceprint rule's thresholds. Reads private
//! local files named by environment variables, so it is ignored by default:
//!
//! ```bash
//! AB_AUDIO=<system.m4a> AB_LINES=<lines.json> AB_PY_DIARIZED=<diarized.json> \
//! AB_PY_ALIGNED=<aligned.json> AB_VOICEPRINTS=<voiceprints.json> \
//! AB_SPEAKERS=<at most N> AB_OUT=<out.json> \
//! cargo test -p deino-diarize --release --test speaker_ab -- --ignored --nocapture
//! ```
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::disallowed_methods,
    clippy::panic,
    clippy::too_many_lines,
    reason = "an integration-test crate run by hand: it panics on bad input and prints a report"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deino_core::diarization::{
    AlignLine, KnownVoice, SpeakerHint, TrackDiarization, TrackRequest, Voice, VoiceTurn, attribute,
};
use deino_core::segments::{Segment, WordTiming};
use deino_core::voiceprints::matching::{MIN_UNIQUE_MARGIN, MIN_UNIQUE_SCORE};
use deino_diarize::RuntimeConfig;
use deino_diarize::align::align_file;
use deino_diarize::models::bundled::{ALIGNER, FOLDER};
use deino_diarize::models::{EMBEDDING, SEGMENTATION};
use deino_diarize::pipeline::ModelPaths;
use deino_diarize::track::diarize_file;
use deino_domain::{Channel, MeetingId, Millis, PersonId, SegmentId, SystemClock};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, Clone)]
struct Line {
    start_ms: u32,
    end_ms: u32,
    text: String,
}

#[derive(Deserialize)]
struct PyDiarized {
    turns: Vec<PyTurn>,
    voices: BTreeMap<String, Vec<f32>>,
}

#[derive(Deserialize)]
struct PyTurn {
    start_ms: u32,
    end_ms: u32,
    speaker: String,
}

#[derive(Deserialize)]
struct PyAligned {
    lines: Vec<Vec<PyWord>>,
}

#[derive(Deserialize)]
struct PyWord {
    text: String,
    start_ms: u32,
    end_ms: u32,
}

#[derive(Deserialize)]
struct Voiceprint {
    person: String,
    name: String,
    embedding: Vec<f32>,
}

fn env(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
#[ignore = "reads a private recording named by AB_* variables"]
fn the_engines_side_by_side() {
    let lines: Vec<Line> = read(&env("AB_LINES"));
    let prints: Vec<Voiceprint> = read(&env("AB_VOICEPRINTS"));
    let rust = rust_engine(&env("AB_AUDIO"), &lines, &prints);
    let python = python_engine(&lines);
    let out = json!({
        "rust": report(&lines, &rust.0, &rust.1, &prints),
        "python": report(&lines, &python.0, &python.1, &prints),
    });
    std::fs::write(env("AB_OUT"), serde_json::to_vec(&out).unwrap()).unwrap();
    for engine in ["rust", "python"] {
        println!("{engine}: {}", out[engine]["voices"]);
    }
}

/// The Rust engine's diarization and word times, as the stage gets them.
fn rust_engine(
    audio: &Path,
    lines: &[Line],
    prints: &[Voiceprint],
) -> (TrackDiarization, Vec<Vec<WordTiming>>) {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/src-tauri/resources")
        .join(FOLDER);
    let models = ModelPaths {
        segmentation: folder.join(SEGMENTATION.file_name),
        embedding: folder.join(EMBEDDING.file_name),
    };
    let speakers: u16 = std::env::var("AB_SPEAKERS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let request = TrackRequest {
        audio: audio.to_owned(),
        speakers: if speakers == 0 {
            SpeakerHint::Unknown
        } else {
            SpeakerHint::AtMost(speakers)
        },
        known: if std::env::var_os("AB_NO_KNOWN").is_some() {
            Vec::new()
        } else {
            prints
                .iter()
                .map(|print| KnownVoice {
                    person: print.person.parse::<PersonId>().unwrap(),
                    embedding: print.embedding.clone(),
                })
                .collect()
        },
    };
    let started = std::time::Instant::now();
    let vbx = std::env::var_os("AB_VBX").map(|_| {
        std::sync::Arc::new(
            deino_diarize::vbx::Plda::load(
                &folder.join(deino_diarize::models::bundled::VBX.file_name),
            )
            .unwrap(),
        )
    });
    let diarization = diarize_file(&models, RuntimeConfig::default(), vbx, &request).unwrap();
    println!("rust diarization: {:.0} s", started.elapsed().as_secs_f64());
    let asked: Vec<AlignLine> = lines
        .iter()
        .map(|line| AlignLine {
            segment: SegmentId::generate(&SystemClock),
            start: Millis(line.start_ms),
            end: Millis(line.end_ms),
            text: line.text.clone(),
        })
        .collect();
    let words = align_file(
        &folder.join(ALIGNER.file_name),
        &RuntimeConfig::default(),
        audio,
        &asked,
    )
    .unwrap();
    println!("rust total: {:.0} s", started.elapsed().as_secs_f64());
    (diarization, words)
}

/// The Python engine's saved answer in the same terms.
fn python_engine(lines: &[Line]) -> (TrackDiarization, Vec<Vec<WordTiming>>) {
    let diarized: PyDiarized = read(&env("AB_PY_DIARIZED"));
    let aligned: PyAligned = read(&env("AB_PY_ALIGNED"));
    let names: Vec<&String> = diarized.voices.keys().collect();
    let turns: Vec<VoiceTurn> = diarized
        .turns
        .iter()
        .map(|turn| VoiceTurn {
            start: Millis(turn.start_ms),
            end: Millis(turn.end_ms),
            voice: names
                .iter()
                .position(|name| **name == turn.speaker)
                .unwrap(),
        })
        .collect();
    let voices = names
        .iter()
        .map(|name| Voice {
            centroid: diarized.voices[*name].clone(),
            speech: Millis(speech(
                &turns,
                names.iter().position(|n| n == name).unwrap(),
            )),
        })
        .collect();
    assert_eq!(aligned.lines.len(), lines.len());
    let words = aligned
        .lines
        .into_iter()
        .map(|line| {
            line.into_iter()
                .map(|w| WordTiming {
                    text: w.text,
                    start: Millis(w.start_ms),
                    end: Millis(w.end_ms.max(w.start_ms)),
                })
                .collect()
        })
        .collect();
    (TrackDiarization { turns, voices }, words)
}

fn speech(turns: &[VoiceTurn], voice: usize) -> u32 {
    turns
        .iter()
        .filter(|t| t.voice == voice)
        .map(|t| t.end.get() - t.start.get())
        .sum()
}

/// The lines attributed by the stage's rule, and each voice's name.
fn report(
    lines: &[Line],
    diarization: &TrackDiarization,
    words: &[Vec<WordTiming>],
    prints: &[Voiceprint],
) -> Value {
    let names = name_voices(diarization, prints);
    let meeting = MeetingId::new("meeting-ab");
    let mut pieces = Vec::new();
    for (line, words) in lines.iter().zip(words) {
        let segment = Segment {
            id: SegmentId::generate(&SystemClock),
            meeting_id: meeting.clone(),
            channel: Channel::System,
            speaker_id: None,
            start: Millis(line.start_ms),
            end: Millis(line.end_ms),
            text: line.text.clone(),
            words: words.clone(),
            is_deleted: false,
            edited_at: None,
            original_text: None,
        };
        for piece in attribute(&segment, &diarization.turns) {
            let who = piece
                .voice
                .map_or("?".to_owned(), |voice| names[voice].clone());
            pieces.push(
                json!({"s": piece.start.get(), "e": piece.end.get(), "who": who, "t": piece.text}),
            );
        }
    }
    let voices: Vec<Value> = diarization
        .voices
        .iter()
        .zip(&names)
        .map(|(voice, name)| json!({"name": name, "speech_s": voice.speech.get() / 1000}))
        .collect();
    json!({"voices": voices, "pieces": pieces, "splits": pieces.len() - lines.len()})
}

/// Each voice's name: a voiceprint takes the voice it matches best when the
/// score and the margin over the next voice pass the voiceprint rule;
/// the rest are "Speaker N" by speech.
fn name_voices(diarization: &TrackDiarization, prints: &[Voiceprint]) -> Vec<String> {
    let mut names: Vec<Option<String>> = vec![None; diarization.voices.len()];
    for print in prints {
        let mut scores: Vec<(f32, usize)> = diarization
            .voices
            .iter()
            .enumerate()
            .map(|(index, voice)| (cosine(&voice.centroid, &print.embedding), index))
            .collect();
        scores.sort_by(|a, b| b.0.total_cmp(&a.0));
        let best = scores[0];
        let margin = best.0 - scores.get(1).map_or(0.0, |s| s.0);
        println!("{}: best {:.3} margin {:.3}", print.name, best.0, margin);
        if best.0 >= MIN_UNIQUE_SCORE && margin >= MIN_UNIQUE_MARGIN && names[best.1].is_none() {
            names[best.1] = Some(print.name.clone());
        }
    }
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(diarization.voices[i].speech.get()));
    let mut next = 1;
    for index in order {
        if names[index].is_none() {
            names[index] = Some(format!("Speaker {next}"));
            next += 1;
        }
    }
    names.into_iter().map(Option::unwrap).collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm(a) * norm(b)).max(f32::EPSILON)
}
