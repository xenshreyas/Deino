//! Giving a screen recording its sound (CAP-09): [`add_audio`] writes a
//! new MP4 holding the video track of a silent `video.mp4` and the audio
//! track of the meeting's mixed `audio.mp4`, both kept at their own times
//! on the recording's timeline, without re-encoding either (no FFmpeg).
//! A clip cut from the result (TRN-04) and a downloaded video then have
//! sound. [`has_audio`] tells whether a file already has an audio track, so
//! the step runs once and never touches an imported video.
//!
//! On macOS both run on `AVFoundation` (`AVMutableComposition` plus a
//! Passthrough `AVAssetExportSession`); elsewhere they fail with
//! [`MuxError::Unsupported`].

#[cfg(target_os = "macos")]
mod native;

use std::path::Path;

use deino_domain::{ErrorClass, ErrorCode};

/// What to combine, and where to write it.
#[derive(Clone, Copy, Debug)]
pub struct MuxRequest<'a> {
    /// The silent screen recording.
    pub video: &'a Path,
    /// The meeting's mixed audio.
    pub audio: &'a Path,
    /// The file to write; replaced if it exists.
    pub destination: &'a Path,
}

/// Why the video could not be given its sound.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MuxError {
    /// A path is not valid UTF-8, so `AVFoundation` cannot take it.
    #[error("a recording path is not valid")]
    InvalidPath,
    /// A file has no track of the kind needed.
    #[error("the {0} file has no {0} track")]
    MissingTrack(&'static str),
    /// A track could not be added to the composition.
    #[error("the tracks could not be combined")]
    Compose,
    /// `AVFoundation` offered no passthrough export.
    #[error("the screen recording cannot be exported")]
    Unavailable,
    /// The export did not finish in time.
    #[error("adding sound to the screen recording took too long")]
    TimedOut,
    /// The export ended with this `AVAssetExportSessionStatus`.
    #[error("adding sound to the screen recording failed (status {0})")]
    Failed(isize),
    /// Only available on macOS.
    #[error("screen recordings are only available on macOS")]
    Unsupported,
}

impl ErrorClass for MuxError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidPath | Self::MissingTrack(_) => ErrorCode::Invalid,
            Self::Unavailable | Self::Unsupported => ErrorCode::Unavailable,
            Self::Compose | Self::TimedOut | Self::Failed(_) => ErrorCode::Internal,
        }
    }
}

/// Writes the request's video with the request's audio into its
/// destination. Blocks until the export ends; call it off the runtime.
pub fn add_audio(request: &MuxRequest<'_>) -> Result<(), MuxError> {
    platform_add_audio(request)
}

/// Whether the file at `path` has an audio track.
pub fn has_audio(path: &Path) -> Result<bool, MuxError> {
    platform_has_audio(path)
}

#[cfg(target_os = "macos")]
fn platform_add_audio(request: &MuxRequest<'_>) -> Result<(), MuxError> {
    native::add_audio(request)
}

#[cfg(not(target_os = "macos"))]
fn platform_add_audio(_request: &MuxRequest<'_>) -> Result<(), MuxError> {
    Err(MuxError::Unsupported)
}

#[cfg(target_os = "macos")]
fn platform_has_audio(path: &Path) -> Result<bool, MuxError> {
    native::has_audio(path)
}

#[cfg(not(target_os = "macos"))]
fn platform_has_audio(_path: &Path) -> Result<bool, MuxError> {
    Err(MuxError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_have_stable_codes() {
        assert_eq!(MuxError::MissingTrack("video").code(), ErrorCode::Invalid);
        assert_eq!(MuxError::Unsupported.code(), ErrorCode::Unavailable);
        assert_eq!(MuxError::Failed(4).code(), ErrorCode::Internal);
        assert_eq!(
            MuxError::MissingTrack("audio").to_string(),
            "the audio file has no audio track"
        );
    }
}
