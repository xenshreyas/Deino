//! Reading a finished track back as mono PCM at any sample rate.
//!
//! Post-meeting stages (diarization reads 16 kHz) decode the finalized
//! `mic.m4a`, `system.m4a` or `audio.mp4` here. `ExtAudioFile` honours the
//! edit list, so the 2112 frames of AAC encoder priming are trimmed and
//! sample 0 is the recording epoch; symphonia 0.5 would shift every
//! timestamp late (see [`crate::fmp4::AAC_PRIMING`]).

use std::path::Path;

use deino_domain::{ErrorClass, ErrorCode};

use crate::finalize::FinalizeError;

/// Frames read per `ExtAudioFileRead` call.
#[cfg(target_os = "macos")]
const BLOCK_FRAMES: usize = 16_384;

/// Why a track could not be decoded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DecodeError {
    /// The file could not be opened or decoded.
    #[error("the audio file could not be decoded")]
    Audio(#[source] FinalizeError),
    /// Decoding is only available on macOS.
    #[error("decoding audio is only available on macOS")]
    Unsupported,
}

impl ErrorClass for DecodeError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Audio(_) => ErrorCode::Internal,
            Self::Unsupported => ErrorCode::Unavailable,
        }
    }
}

/// The whole file at `path` as mono f32 samples at `sample_rate`.
pub fn read_mono(path: &Path, sample_rate: u32) -> Result<Vec<f32>, DecodeError> {
    #[cfg(target_os = "macos")]
    {
        native::read_mono(path, sample_rate).map_err(DecodeError::Audio)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, sample_rate);
        Err(DecodeError::Unsupported)
    }
}

#[cfg(target_os = "macos")]
mod native {
    use std::path::Path;

    use super::BLOCK_FRAMES;
    use crate::finalize::FinalizeError;
    use crate::finalize::ext_audio::PcmReader;

    pub(super) fn read_mono(path: &Path, sample_rate: u32) -> Result<Vec<f32>, FinalizeError> {
        let mut reader = PcmReader::open(path, sample_rate)?;
        let mut samples = Vec::new();
        let mut block = vec![0.0; BLOCK_FRAMES];
        loop {
            let read = reader.read(&mut block)?;
            match block.get(..read) {
                Some(chunk) if !chunk.is_empty() => samples.extend_from_slice(chunk),
                _ => return Ok(samples),
            }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use deino_testkit::Fixtures;
    use deino_testkit::fixture::{Channel, MeetingKind};

    use super::*;

    #[test]
    fn a_fixture_track_decodes_at_the_requested_rate_with_priming_trimmed() {
        let fixtures = Fixtures::load().unwrap();
        let meeting = fixtures.meeting(MeetingKind::Standup).unwrap();
        let audio = meeting.audio.as_ref().unwrap();
        let path = fixtures.audio_path(audio, Channel::System).unwrap();

        let at_16k = read_mono(&path, 16_000).unwrap();
        let at_48k = read_mono(&path, 48_000).unwrap();

        // The clip is about 53 s long at both rates (50 ms apart at most).
        assert!(
            (16_000 * 50..16_000 * 60).contains(&at_16k.len()),
            "{}",
            at_16k.len()
        );
        assert!(at_48k.len().abs_diff(3 * at_16k.len()) < 2_400);
        // The system track is silent until the first remote speaker (9.7 s).
        let lead = &at_16k[..16_000 * 9];
        assert!(lead.iter().all(|sample| sample.abs() < 0.01));
    }

    #[test]
    fn a_missing_file_is_an_audio_error() {
        let result = read_mono(Path::new("/nonexistent/track.m4a"), 16_000);
        assert!(matches!(result, Err(DecodeError::Audio(_))), "{result:?}");
    }
}
