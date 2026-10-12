//! Cutting a clip (TRN-04): `[start, end)` of a finished recording copied
//! into a new file, without FFmpeg (PARITY_PLAN D11).
//!
//! On macOS, [`cut_clip`] runs an `AVAssetExportSession` with the
//! Passthrough preset and a `timeRange`: the AAC (and video) packets are
//! copied, not re-encoded, and the edit list trims the cut to the sample
//! (the native-recording spike measured a sample-exact cut in 11 ms).
//! Elsewhere it is a stub that always fails with [`ClipError::Unsupported`].

#[cfg(target_os = "macos")]
mod native;

use std::path::Path;

use deino_domain::{ErrorClass, ErrorCode};

use crate::Millis;

/// The file a clip is written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipContainer {
    /// MPEG-4 audio (`.m4a`), for a clip of the audio.
    M4a,
    /// MPEG-4 (`.mp4`), for a clip of a screen recording.
    Mp4,
}

impl ClipContainer {
    /// The file extension, without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::M4a => "m4a",
            Self::Mp4 => "mp4",
        }
    }
}

/// What to cut, from where to where.
#[derive(Clone, Copy, Debug)]
pub struct ClipRequest<'a> {
    /// The finished recording file.
    pub source: &'a Path,
    /// The clip file to write; replaced if it exists.
    pub destination: &'a Path,
    /// Where the clip starts on the source's timeline.
    pub start: Millis,
    /// Where it ends (exclusive); past the end of the source, the clip stops there.
    pub end: Millis,
    /// The clip file's type.
    pub container: ClipContainer,
}

/// Why a clip could not be cut.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClipError {
    /// The range is empty or reversed.
    #[error("the clip's range is empty")]
    EmptyRange,
    /// A path is not valid UTF-8, so `AVFoundation` cannot take it.
    #[error("the clip's path is not valid")]
    InvalidPath,
    /// `AVFoundation` offered no passthrough export for the source.
    #[error("this recording cannot be clipped")]
    Unavailable,
    /// The export did not finish in time.
    #[error("cutting the clip took too long")]
    TimedOut,
    /// The export ended with this `AVAssetExportSessionStatus`.
    #[error("cutting the clip failed (status {0})")]
    Failed(isize),
    /// Cutting clips is only available on macOS.
    #[error("clips are only available on macOS")]
    Unsupported,
}

impl ErrorClass for ClipError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EmptyRange | Self::InvalidPath => ErrorCode::Invalid,
            Self::Unavailable | Self::Unsupported => ErrorCode::Unavailable,
            Self::TimedOut | Self::Failed(_) => ErrorCode::Internal,
        }
    }
}

/// Cuts `request.start..request.end` of the source into the destination.
/// Blocks until the export ends; call it off the async runtime.
pub fn cut_clip(request: &ClipRequest<'_>) -> Result<(), ClipError> {
    if request.end <= request.start {
        return Err(ClipError::EmptyRange);
    }
    cut(request)
}

#[cfg(target_os = "macos")]
fn cut(request: &ClipRequest<'_>) -> Result<(), ClipError> {
    native::cut(request)
}

#[cfg(not(target_os = "macos"))]
fn cut(_request: &ClipRequest<'_>) -> Result<(), ClipError> {
    Err(ClipError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_range_is_refused_before_any_export() {
        let request = ClipRequest {
            source: Path::new("missing.m4a"),
            destination: Path::new("clip.m4a"),
            start: Millis(2_000),
            end: Millis(2_000),
            container: ClipContainer::M4a,
        };

        assert!(matches!(cut_clip(&request), Err(ClipError::EmptyRange)));
    }

    #[test]
    fn containers_name_their_extension() {
        assert_eq!(ClipContainer::M4a.extension(), "m4a");
        assert_eq!(ClipContainer::Mp4.extension(), "mp4");
    }
}
