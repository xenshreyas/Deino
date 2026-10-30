//! Compares the Rust word aligner with a reference (the Python engine's
//! answer on the same lines), word by word.
//!
//! ```bash
//! cargo run -p deino-diarize --release --features bench --bin align-bench -- \
//!     <model.onnx> <track.f32> <lines.json> <reference.json>
//! ```
//!
//! `<track.f32>` is the track as 16 kHz mono little-endian f32 (what the
//! Python engine reads); `<lines.json>` holds `[{start_ms, end_ms, text}]`;
//! `<reference.json>` holds `lines: [{words: [[start_ms, end_ms], …]}]`.
//! Prints one JSON line: how far the start and end times are from the
//! reference (median, 90th percentile, share within 20/50/100 ms) and the
//! time taken.
#![allow(
    clippy::print_stdout,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    reason = "a command-line benchmark prints its results, measures wall time and stops on bad input"
)]

use std::path::PathBuf;
use std::time::Instant;

use deino_core::diarization::AlignLine;
use deino_diarize::RuntimeConfig;
use deino_diarize::align::align_line;
use deino_diarize::align::model::Aligner;
use deino_domain::{Millis, SegmentId};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct LineIn {
    start_ms: u32,
    end_ms: u32,
    text: String,
}

#[derive(Deserialize)]
struct Reference {
    lines: Vec<RefLine>,
}

#[derive(Deserialize)]
struct RefLine {
    words: Vec<(u32, u32)>,
}

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [model, pcm, lines, reference] = args.as_slice() else {
        panic!("usage: align-bench <model.onnx> <track.f32> <lines.json> <reference.json>");
    };
    let bytes = std::fs::read(pcm).expect("read the track");
    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let lines: Vec<LineIn> = serde_json::from_slice(&std::fs::read(lines).unwrap()).unwrap();
    let reference: Reference = serde_json::from_slice(&std::fs::read(reference).unwrap()).unwrap();
    let mut aligner = Aligner::load(model, &RuntimeConfig::default()).expect("load the model");
    let started = Instant::now();
    let (mut starts, mut ends) = (Vec::new(), Vec::new());
    for (line, expected) in lines.iter().zip(&reference.lines) {
        let asked = AlignLine {
            segment: SegmentId::generate(&deino_domain::SystemClock),
            start: Millis(line.start_ms),
            end: Millis(line.end_ms),
            text: line.text.clone(),
        };
        let words = align_line(&mut aligner, &samples, &asked).expect("align");
        for (word, &(start, end)) in words.iter().zip(&expected.words) {
            starts.push(word.start.0.abs_diff(start));
            ends.push(word.end.0.abs_diff(end));
        }
    }
    println!(
        "{}",
        json!({
            "words": starts.len(),
            "start": summary(&mut starts),
            "end": summary(&mut ends),
            "seconds": started.elapsed().as_secs_f64(),
        })
    );
}

fn summary(diffs: &mut [u32]) -> serde_json::Value {
    diffs.sort_unstable();
    let share =
        |ms: u32| diffs.iter().filter(|&&d| d <= ms).count() as f64 / diffs.len().max(1) as f64;
    json!({
        "median_ms": diffs.get(diffs.len() / 2),
        "p90_ms": diffs.get(diffs.len() * 9 / 10),
        "within_20ms": share(20),
        "within_50ms": share(50),
        "within_100ms": share(100),
    })
}
