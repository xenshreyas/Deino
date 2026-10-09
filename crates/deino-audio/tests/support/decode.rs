//! Test-side readers: decode a file with `AVAudioFile`, measure peaks.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail the calling test by panicking"
)]

use std::path::Path;

use cidre::{av, ns};

/// Every frame of `path`, decoded by `AVAudioFile` (priming trimmed) as mono
/// f32. Fails the test when the file cannot be opened.
pub fn decode_with_av_audio_file(path: &Path) -> Vec<f32> {
    let url = ns::Url::with_fs_path_str(path.to_str().unwrap(), false);
    let opened = av::AudioFile::open_read(&url);
    assert!(opened.is_ok(), "AVAudioFile cannot open {}", path.display());
    let mut file = opened.unwrap();
    let format = file.processing_format();
    let mut buffer = av::AudioPcmBuf::with_format(&format, 48_000).unwrap();
    let mut samples = Vec::new();
    // Never read at the end: -readIntoBuffer: then fails without an NSError,
    // which cidre a9587fa does not expect.
    while file.frame_pos() < file.len() {
        file.read(&mut buffer).unwrap();
        if buffer.frame_len() == 0 {
            break;
        }
        samples.extend_from_slice(buffer.data_f32_at(0).unwrap());
    }
    samples
}

/// The frame of the loudest sample within `radius` frames of `around`.
#[allow(dead_code, reason = "shared by several test crates, not all use it")]
pub fn peak_near(samples: &[f32], around: usize, radius: usize) -> usize {
    let start = around.saturating_sub(radius);
    let end = (around + radius).min(samples.len());
    (start..end)
        .max_by(|a, b| samples[*a].abs().total_cmp(&samples[*b].abs()))
        .unwrap()
}

/// Root-mean-square level of `samples`.
#[allow(dead_code, reason = "shared by several test crates, not all use it")]
pub fn rms(samples: &[f32]) -> f32 {
    let energy: f32 = samples.iter().map(|s| s * s).sum();
    #[allow(
        clippy::cast_precision_loss,
        reason = "test clips are far below 2^24 samples"
    )]
    let len = samples.len().max(1) as f32;
    (energy / len).sqrt()
}

/// `frames` at 48 kHz in seconds.
#[allow(dead_code, reason = "shared by several test crates, not all use it")]
pub fn seconds(frames: u64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "test recordings are far below 2^52 frames"
    )]
    let frames = frames as f64;
    frames / 48_000.0
}
