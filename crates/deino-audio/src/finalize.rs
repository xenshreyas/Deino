//! Finalize: turn a closed (or recovered) recording into its three files.
//!
//! - `mic.m4a`, `system.m4a`: one flat AAC file per track. A track with one
//!   segment is passthrough-exported (no re-encode); several segments are
//!   decoded and spliced on the timeline, with silence between them.
//! - `audio.mp4`: both tracks decoded with `ExtAudioFile` (priming trimmed, so
//!   they stay sample-aligned), summed at −6 dB through a limiter, encoded.
//!
//! The mix is a pure function of the two tracks, which is how "checkpoint
//! recovery for all three files" holds without a third live encoder
//! (ADR 0014). Each output is written to a temporary name, synced and
//! renamed, so a crash never leaves a half-written file under the final name.
//! Running it again after a crash simply redoes the work.

#[cfg(target_os = "macos")]
mod export;
#[cfg(target_os = "macos")]
pub(crate) mod ext_audio;
mod mix;
#[cfg(target_os = "macos")]
mod native;

use std::path::{Path, PathBuf};

use deino_domain::{ErrorClass, ErrorCode};

pub use mix::{LIMIT_THRESHOLD, MIX_GAIN, limit, mix_blocks, mix_sample};

use crate::manifest::{ManifestError, RecordingStatus};
use crate::units::{Frames, Track};

/// The flat microphone track, next to `recording.json`.
pub const MICROPHONE_FILE: &str = "mic.m4a";

/// The flat system track.
pub const SYSTEM_FILE: &str = "system.m4a";

/// The mixed track used for playback.
pub const MIXED_FILE: &str = "audio.mp4";

/// The finalized file name of `track`.
pub const fn track_file(track: Track) -> &'static str {
    match track {
        Track::Microphone => MICROPHONE_FILE,
        Track::System => SYSTEM_FILE,
    }
}

/// Why finalize failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FinalizeError {
    /// The manifest could not be read or written.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The recording is still being written (or needs recovery first).
    #[error("the recording is still {0:?}; stop or recover it first")]
    NotStopped(RecordingStatus),
    /// A path is not valid UTF-8 or not a file URL.
    #[error("the recording path cannot be used")]
    InvalidPath,
    /// An `AudioToolbox` call failed.
    #[error("{call} failed with OSStatus {status}")]
    Audio {
        /// The failing call.
        call: &'static str,
        /// Its status.
        status: i32,
    },
    /// `AVFoundation` refused to create a passthrough export session.
    #[error("the track cannot be exported without re-encoding")]
    ExportUnavailable,
    /// The export did not finish in time.
    #[error("exporting the track took too long")]
    ExportTimedOut,
    /// The export ended with a non-completed `AVAssetExportSessionStatus`.
    #[error("exporting the track failed with status {0}")]
    ExportFailed(isize),
    /// Syncing or renaming an output file failed.
    #[error("could not save the finalized audio")]
    Io(#[from] std::io::Error),
    /// Native audio encoding is only available on macOS.
    #[error("finalizing audio is only available on macOS")]
    Unsupported,
}

impl ErrorClass for FinalizeError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Manifest(error) => error.code(),
            Self::NotStopped(_) => ErrorCode::Conflict,
            Self::Unsupported => ErrorCode::Unavailable,
            _ => ErrorCode::Internal,
        }
    }
}

/// The finalized files of a recording.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalizedAudio {
    /// `mic.m4a`.
    pub microphone: PathBuf,
    /// `system.m4a`.
    pub system: PathBuf,
    /// `audio.mp4`.
    pub mixed: PathBuf,
    /// Frames in the mix (the longer track's length).
    pub frames: Frames,
}

/// Writes `mic.m4a`, `system.m4a` and `audio.mp4` for the recording in `dir`
/// and marks its manifest finalized.
pub fn finalize_recording(dir: &Path) -> Result<FinalizedAudio, FinalizeError> {
    #[cfg(target_os = "macos")]
    {
        native::finalize(dir)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = dir;
        Err(FinalizeError::Unsupported)
    }
}
