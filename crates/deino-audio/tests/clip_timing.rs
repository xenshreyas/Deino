//! TRN-04 acceptance on macOS: a clip cut from fixture audio holds exactly
//! the selected range, within ±250 ms.
//!
//! The clip and its source are decoded with `ExtAudioFile` (edit lists
//! honoured) at 8 kHz. The clip's start is found by sliding its first half
//! second along the source around the requested start and taking the
//! offset with the least squared error; its duration is its decoded length.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies; the measured error is printed as test evidence"
)]

use std::path::PathBuf;

use deino_audio::clip::{ClipContainer, ClipRequest, cut_clip};
use deino_audio::{Millis, read_mono};

/// The decode rate: plenty for matching speech, cheap to search.
const RATE: u32 = 8_000;
/// Samples per millisecond at [`RATE`].
const PER_MS: i64 = 8;
/// The acceptance: ±250 ms.
const TOLERANCE_MS: i64 = 250;
/// How far around the requested start the clip's audio is searched for.
const SEARCH_MS: i64 = 1_000;
/// How much of the clip is matched against the source (half a second).
const PROBE: usize = 4_000;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/audio/standup")
        .join(name)
}

/// The offset (in samples) in `source` where `probe` matches best, searched
/// over `from..to`.
fn best_offset(source: &[f32], probe: &[f32], from: usize, to: usize) -> usize {
    let error = |offset: usize| -> f32 {
        let window = &source[offset..offset + probe.len()];
        window
            .iter()
            .zip(probe)
            .map(|(a, b)| (a - b) * (a - b))
            .sum()
    };
    (from..to)
        .min_by(|a, b| error(*a).total_cmp(&error(*b)))
        .unwrap()
}

/// Cuts `start..end` of the fixture's microphone track and returns the
/// errors of the clip's start and duration, in milliseconds.
fn measure(start: u32, end: u32) -> (i64, i64) {
    let source = fixture("mic.m4a");
    let scratch = tempfile::tempdir().unwrap();
    let destination = scratch.path().join("clip.m4a");
    cut_clip(&ClipRequest {
        source: &source,
        destination: &destination,
        start: Millis(start),
        end: Millis(end),
        container: ClipContainer::M4a,
    })
    .unwrap();
    let whole = read_mono(&source, RATE).unwrap();
    let clip = read_mono(&destination, RATE).unwrap();
    let expected = i64::from(start) * PER_MS;
    let from = usize::try_from((expected - SEARCH_MS * PER_MS).max(0)).unwrap();
    let to = usize::try_from(expected + SEARCH_MS * PER_MS).unwrap();
    let probe = &clip[..PROBE];
    // A silent probe would match anywhere: the ranges are chosen in speech.
    let energy = probe.iter().map(|sample| sample * sample).sum::<f32>() / 4_000.0;
    assert!(energy > 1e-4, "the probe at {start} ms is silent");
    let found = i64::try_from(best_offset(&whole, probe, from, to)).unwrap();
    let length = i64::try_from(clip.len()).unwrap();
    let start_error = (found - expected) / PER_MS;
    let duration_error = (length - i64::from(end - start) * PER_MS) / PER_MS;
    (start_error, duration_error)
}

#[test]
fn a_clip_holds_the_selected_range_within_250_ms() {
    for (start, end) in [(3_000, 7_500), (26_120, 28_870)] {
        let (start_error, duration_error) = measure(start, end);

        println!(
            "clip {start}..{end} ms: start off by {start_error} ms, duration by {duration_error} ms"
        );
        assert!(
            start_error.abs() <= TOLERANCE_MS,
            "start off by {start_error} ms"
        );
        assert!(
            duration_error.abs() <= TOLERANCE_MS,
            "duration off by {duration_error} ms"
        );
    }
}

#[test]
fn a_clip_of_a_video_container_is_written_as_mp4() {
    let scratch = tempfile::tempdir().unwrap();
    let destination = scratch.path().join("clip.mp4");

    cut_clip(&ClipRequest {
        source: &fixture("system.m4a"),
        destination: &destination,
        start: Millis(1_000),
        end: Millis(2_000),
        container: ClipContainer::Mp4,
    })
    .unwrap();

    let clip = read_mono(&destination, RATE).unwrap();
    let length_ms = i64::try_from(clip.len()).unwrap() / PER_MS;
    assert!((length_ms - 1_000).abs() <= TOLERANCE_MS, "{length_ms} ms");
}
