//! [`NativeIngest`]: the import's file work (CAP-11), `deino-core`'s
//! [`MediaIngest`] port over `deino-audio`'s decoders.
//!
//! The file's audio is decoded by symphonia, or by `AVAssetReader` when
//! symphonia cannot read it, and written as `audio.m4a` (48 kHz mono AAC)
//! in the meeting's folder: the same format as a live recording, so
//! `final_transcription`, diarization, the player and clips read it
//! through `ExtAudioFile` with no special case. A video file is copied
//! beside it as `video.<ext>` (a copy-on-write clone on APFS) for the
//! player. Decoding runs on a blocking thread; a two-hour file streams
//! through in blocks and never sits in memory whole.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_audio::media::{MediaDecoder, MediaError, SymphoniaDecoder, transcode};
use deino_core::import::{ImportError, IngestedMedia, MediaFacts, MediaIngest, MediaKind};

/// The converted audio's file name in the meeting's folder.
pub const AUDIO_FILE: &str = "audio.m4a";

/// The kept video's file stem; its extension is the source's.
pub const VIDEO_STEM: &str = "video";

/// Converts imported files with symphonia, then `AVAssetReader`.
#[derive(Clone)]
pub struct NativeIngest {
    decoders: Arc<[Arc<dyn MediaDecoder>]>,
}

impl std::fmt::Debug for NativeIngest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.decoders.iter().map(|decoder| decoder.name()).collect();
        f.debug_struct("NativeIngest")
            .field("decoders", &names)
            .finish()
    }
}

impl Default for NativeIngest {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeIngest {
    /// symphonia first; on macOS, `AVAssetReader` when symphonia fails.
    pub fn new() -> Self {
        let mut decoders: Vec<Arc<dyn MediaDecoder>> = vec![Arc::new(SymphoniaDecoder::new())];
        #[cfg(target_os = "macos")]
        decoders.push(Arc::new(deino_audio::media::AssetReaderDecoder));
        Self::with_decoders(decoders)
    }

    /// An ingest trying `decoders` in order.
    pub fn with_decoders(decoders: Vec<Arc<dyn MediaDecoder>>) -> Self {
        Self {
            decoders: decoders.into(),
        }
    }

    /// Writes the meeting's files into `folder`, removing it on failure.
    fn write(
        &self,
        source: &Path,
        kind: MediaKind,
        folder: &Path,
    ) -> Result<IngestedMedia, ImportError> {
        std::fs::create_dir_all(folder).map_err(files)?;
        let written = self.convert(source, kind, folder);
        if written.is_err() {
            remove_folder(folder);
        }
        written
    }

    /// The audio, then the video for a video file.
    fn convert(
        &self,
        source: &Path,
        kind: MediaKind,
        folder: &Path,
    ) -> Result<IngestedMedia, ImportError> {
        let audio = folder.join(AUDIO_FILE);
        let decoders: Vec<&dyn MediaDecoder> = self.decoders.iter().map(AsRef::as_ref).collect();
        let done =
            transcode(&decoders, source, &audio, &AtomicBool::new(false)).map_err(import_error)?;
        log::info!(target: "import", "converted with {} ({} frames)", done.decoder, done.length.0);
        let video = match kind {
            MediaKind::Audio => None,
            MediaKind::Video => Some(keep_video(source, folder)?),
        };
        Ok(IngestedMedia {
            audio,
            video,
            duration: done.length.as_duration(),
        })
    }
}

#[async_trait]
impl MediaIngest for NativeIngest {
    async fn facts(&self, path: &Path) -> Result<MediaFacts, ImportError> {
        let metadata = match tokio::fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ImportError::Missing);
            }
            Err(error) => return Err(files(error)),
        };
        if !metadata.is_file() {
            return Err(ImportError::UnsupportedFile);
        }
        Ok(MediaFacts {
            size_bytes: metadata.len(),
            modified: metadata.modified().ok().map(DateTime::<Utc>::from),
        })
    }

    async fn ingest(
        &self,
        source: &Path,
        kind: MediaKind,
        folder: &Path,
    ) -> Result<IngestedMedia, ImportError> {
        let (this, source, folder) = (self.clone(), source.to_path_buf(), folder.to_path_buf());
        tokio::task::spawn_blocking(move || this.write(&source, kind, &folder))
            .await
            .map_err(files)?
    }

    async fn discard(&self, folder: &Path) {
        let folder = folder.to_path_buf();
        // A panic in `remove_folder` only means the folder stays behind.
        tokio::task::spawn_blocking(move || remove_folder(&folder))
            .await
            .ok();
    }
}

/// Copies the video into `folder` as `video.<ext>`.
fn keep_video(source: &Path, folder: &Path) -> Result<PathBuf, ImportError> {
    let extension = source
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let target = folder.join(format!("{VIDEO_STEM}.{extension}"));
    std::fs::copy(source, &target).map_err(files)?;
    Ok(target)
}

/// Removes a meeting folder an import gave up on.
fn remove_folder(folder: &Path) {
    if let Err(error) = std::fs::remove_dir_all(folder) {
        log::warn!(target: "import", "could not remove a failed import's folder: {error}");
    }
}

/// A conversion failure as the import reports it.
fn import_error(error: MediaError) -> ImportError {
    match error {
        MediaError::NoAudioTrack => ImportError::NoAudio,
        MediaError::UnsupportedFormat => ImportError::UnsupportedFormat,
        MediaError::Decode(source) => ImportError::Damaged(source),
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ImportError::Missing
        }
        MediaError::Unsupported => ImportError::Unavailable,
        other => files(other),
    }
}

/// A file-system or encoder failure while writing the meeting's files.
fn files(error: impl std::error::Error + Send + Sync + 'static) -> ImportError {
    ImportError::Files(Box::new(error))
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
