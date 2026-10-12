//! Reading any audio or video file the user imports (CAP-11), without FFmpeg.
//!
//! A [`MediaDecoder`] opens a file and streams its first audio track as mono
//! f32 blocks at 48 kHz ([`STORAGE_RATE`]), so a two-hour file never sits in
//! memory whole. Two decoders implement it, tried in this order
//! (`docs/references/spike-native-recording.md` §4):
//!
//! | Decoder | Reads |
//! |---|---|
//! | [`SymphoniaDecoder`] | MP3, WAV, FLAC, Vorbis, AIFF, CAF, ALAC, AAC in MP4/M4A/MOV, MKV and `WebM`, Opus through libopus |
//! | [`AssetReaderDecoder`] (macOS) | Everything `AVFoundation` plays: Apple ADTS, AAC-in-CAF, MP4/MOV video |
//!
//! [`transcode`] runs the first decoder that reads the whole file and
//! writes it as Deino's own 48 kHz mono AAC `.m4a`, which every
//! post-meeting stage reads through `ExtAudioFile` ([`crate::decode`]). A
//! decoder that fails partway is abandoned and the next starts again from
//! the beginning, so a file is either converted whole or not at all.

#[cfg(target_os = "macos")]
mod asset_reader;
mod fallback;
mod symphonia_decoder;

use std::path::Path;

use deino_domain::{ErrorClass, ErrorCode};

#[cfg(target_os = "macos")]
pub use asset_reader::AssetReaderDecoder;
pub use fallback::{Transcoded, pump, transcode};
pub use symphonia_decoder::SymphoniaDecoder;

#[cfg(doc)]
use crate::units::STORAGE_RATE;

/// An open file's audio as mono f32 blocks at 48 kHz.
pub trait PcmStream {
    /// The next block of samples, or `None` once the file has ended.
    fn next_block(&mut self) -> Result<Option<Vec<f32>>, MediaError>;
}

/// Opens audio and video files as a [`PcmStream`].
///
/// Contract (checked for every implementation by the tests): the stream
/// yields the file's first audio track, every channel mixed down to mono,
/// at 48 kHz, from its first sample to its last.
pub trait MediaDecoder: Send + Sync {
    /// A short name for logs (`symphonia`, `AVAssetReader`).
    fn name(&self) -> &'static str;

    /// Opens `path` for reading.
    fn open(&self, path: &Path) -> Result<Box<dyn PcmStream>, MediaError>;
}

/// Why a file could not be read or converted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MediaError {
    /// The file could not be opened or read.
    #[error("the file could not be read")]
    Io(#[from] std::io::Error),
    /// The file holds no audio track (a silent screen recording, an image).
    #[error("the file has no audio track")]
    NoAudioTrack,
    /// No decoder understands the container or the codec.
    #[error("the file's format is not supported")]
    UnsupportedFormat,
    /// The audio is damaged beyond what the decoder skips over.
    #[error("the file's audio could not be decoded")]
    Decode(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The conversion was cancelled.
    #[error("the import was cancelled")]
    Cancelled,
    /// The converted audio could not be written.
    #[error("the converted audio could not be written")]
    Encode(#[source] crate::FinalizeError),
    /// Converting needs the native encoder (macOS).
    #[error("importing audio is only available on macOS")]
    Unsupported,
}

impl MediaError {
    /// A decode failure carrying `source` for the logs.
    pub fn decode(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Decode(source.into())
    }
}

impl ErrorClass for MediaError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NoAudioTrack | Self::UnsupportedFormat | Self::Decode(_) => ErrorCode::Invalid,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Unsupported => ErrorCode::Unavailable,
            Self::Io(_) | Self::Encode(_) => ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
