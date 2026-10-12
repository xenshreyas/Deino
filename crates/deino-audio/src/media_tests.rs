use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use deino_testkit::Fixtures;
use deino_testkit::fixture::{Channel, MeetingKind};

use super::*;
use crate::units::{Frames, STORAGE_RATE};

/// The standup fixture's system track (AAC in M4A, about 53 s).
fn fixture_m4a() -> PathBuf {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::Standup).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    fixtures.audio_path(audio, Channel::System).unwrap()
}

/// Writes a 16-bit PCM WAV of `frames` frames per channel: the left channel
/// holds `left`, the right `right`.
fn write_wav(path: &Path, rate: u32, frames: usize, (left, right): (i16, i16)) {
    let data_len = u32::try_from(frames * 4).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&2_u16.to_le_bytes()); // stereo
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for _ in 0..frames {
        bytes.extend_from_slice(&left.to_le_bytes());
        bytes.extend_from_slice(&right.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

/// Every sample of `path` through `decoder`.
fn decode_all(decoder: &dyn MediaDecoder, path: &Path) -> Vec<f32> {
    let mut stream = decoder.open(path).unwrap();
    let mut samples = Vec::new();
    while let Some(block) = stream.next_block().unwrap() {
        samples.extend(block);
    }
    samples
}

#[test]
fn symphonia_reads_aac_in_m4a_at_48k() {
    let samples = decode_all(&SymphoniaDecoder::new(), &fixture_m4a());

    let seconds = samples.len() / STORAGE_RATE as usize;
    assert!((50..60).contains(&seconds), "{seconds} s");
}

#[test]
fn symphonia_mixes_stereo_down_and_resamples_to_48k() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("call.wav");
    write_wav(&wav, 16_000, 32_000, (16_384, 0));

    let samples = decode_all(&SymphoniaDecoder::new(), &wav);

    // Two seconds at 48 kHz, less the resampler's last partial chunk.
    assert!(samples.len().abs_diff(96_000) < 1_000, "{}", samples.len());
    // The left channel at half scale, averaged with silence: a quarter.
    let middle = samples[48_000];
    assert!((middle - 0.25).abs() < 0.01, "{middle}");
}

#[test]
fn a_file_that_is_not_media_is_an_unsupported_format() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("notes.mp3");
    std::fs::write(&fake, "not audio at all, only text").unwrap();

    let result = SymphoniaDecoder::new().open(&fake).map(|_| ());

    assert!(
        matches!(result, Err(MediaError::UnsupportedFormat)),
        "{result:?}"
    );
}

#[test]
fn a_missing_file_is_an_io_error() {
    let result = SymphoniaDecoder::new()
        .open(Path::new("/nonexistent/a.wav"))
        .map(|_| ());

    assert!(matches!(result, Err(MediaError::Io(_))), "{result:?}");
}

/// A stream of fixed blocks.
struct Blocks(Vec<Vec<f32>>);

impl PcmStream for Blocks {
    fn next_block(&mut self) -> Result<Option<Vec<f32>>, MediaError> {
        Ok((!self.0.is_empty()).then(|| self.0.remove(0)))
    }
}

#[test]
fn pump_writes_every_block_and_counts_frames() {
    let mut stream = Blocks(vec![vec![0.1; 10], vec![0.2; 5]]);
    let mut written = Vec::new();

    let length = pump(
        &mut stream,
        &mut |block| {
            written.extend_from_slice(block);
            Ok(())
        },
        &AtomicBool::new(false),
    )
    .unwrap();

    assert_eq!(length, Frames(15));
    assert_eq!(written.len(), 15);
}

#[test]
fn pump_stops_when_cancelled_and_refuses_an_empty_stream() {
    let cancelled = pump(
        &mut Blocks(vec![vec![0.0; 4]]),
        &mut |_| Ok(()),
        &AtomicBool::new(true),
    );
    let empty = pump(
        &mut Blocks(Vec::new()),
        &mut |_| Ok(()),
        &AtomicBool::new(false),
    );

    assert!(
        matches!(cancelled, Err(MediaError::Cancelled)),
        "{cancelled:?}"
    );
    assert!(matches!(empty, Err(MediaError::NoAudioTrack)), "{empty:?}");
}

/// A decoder that can open nothing.
struct Refusing(fn() -> MediaError);

impl MediaDecoder for Refusing {
    fn name(&self) -> &'static str {
        "refusing"
    }

    fn open(&self, _path: &Path) -> Result<Box<dyn PcmStream>, MediaError> {
        Err((self.0)())
    }
}

#[test]
fn when_every_decoder_fails_the_first_error_is_returned_and_nothing_is_left() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("audio.m4a");
    let first = Refusing(|| MediaError::NoAudioTrack);
    let second = Refusing(|| MediaError::UnsupportedFormat);

    let result = transcode(
        &[&first, &second],
        &fixture_m4a(),
        &target,
        &AtomicBool::new(false),
    );

    assert!(
        matches!(result, Err(MediaError::NoAudioTrack)),
        "{result:?}"
    );
    assert!(!target.exists());
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;

    #[test]
    fn the_asset_reader_reads_aac_in_m4a_at_48k() {
        let samples = decode_all(&AssetReaderDecoder, &fixture_m4a());

        let seconds = samples.len() / STORAGE_RATE as usize;
        assert!((50..60).contains(&seconds), "{seconds} s");
    }

    #[test]
    fn the_asset_reader_reports_a_text_file_as_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("notes.m4a");
        std::fs::write(&fake, "not audio").unwrap();

        let result = AssetReaderDecoder.open(&fake).map(|_| ());

        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn transcode_falls_back_and_writes_an_m4a_the_stages_can_read() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("audio.m4a");
        let broken = Refusing(|| MediaError::UnsupportedFormat);

        let done = transcode(
            &[&broken, &AssetReaderDecoder],
            &fixture_m4a(),
            &target,
            &AtomicBool::new(false),
        )
        .unwrap();

        assert_eq!(done.decoder, "AVAssetReader");
        let reread = crate::decode::read_mono(&target, STORAGE_RATE).unwrap();
        let frames = usize::try_from(done.length.0).unwrap();
        assert!(
            reread.len().abs_diff(frames) < 4_800,
            "{} vs {frames}",
            reread.len()
        );
    }

    #[test]
    fn transcode_uses_symphonia_first() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("call.wav");
        write_wav(&wav, 44_100, 44_100, (8_000, 8_000));
        let target = dir.path().join("audio.m4a");
        let symphonia = SymphoniaDecoder::new();

        let done = transcode(
            &[&symphonia, &AssetReaderDecoder],
            &wav,
            &target,
            &AtomicBool::new(false),
        )
        .unwrap();

        assert_eq!(done.decoder, "symphonia");
        assert!(done.length.0.abs_diff(48_000) < 1_000, "{:?}", done.length);
    }
}
