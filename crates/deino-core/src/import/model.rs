//! What an import reads and produces.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, Seconds};
use serde::Serialize;

/// The largest file an import accepts: 20 GB, a long screen recording.
pub const MAX_IMPORT_BYTES: u64 = 20 * 1024 * 1024 * 1024;

/// Audio file extensions an import accepts (lower case, no dot).
pub const AUDIO_EXTENSIONS: [&str; 11] = [
    "m4a", "mp3", "wav", "aac", "aif", "aiff", "caf", "flac", "ogg", "oga", "opus",
];

/// Video file extensions an import accepts; the video is kept for playback.
pub const VIDEO_EXTENSIONS: [&str; 5] = ["mp4", "mov", "m4v", "mkv", "webm"];

/// Whether a file is audio only or a video with an audio track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    /// Audio only.
    Audio,
    /// A video; its file is kept beside the converted audio.
    Video,
}

impl MediaKind {
    /// The kind of the file at `path`, by extension; `None` when imports do
    /// not accept it.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            Some(Self::Audio)
        } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
            Some(Self::Video)
        } else {
            None
        }
    }
}

/// A file the user chose, checked and ready to import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    /// Its absolute path.
    pub path: String,
    /// Its name, as the import dialog shows it.
    pub file_name: String,
    /// Audio, or a video whose file is kept.
    pub kind: MediaKind,
}

/// A request to import one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportRequest {
    /// The file.
    pub source: PathBuf,
    /// The meeting's title; the file's name when `None` or blank.
    pub title: Option<String>,
}

/// What the file system says about a file before it is imported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaFacts {
    /// Its size in bytes.
    pub size_bytes: u64,
    /// When it last changed: about when the recording ended.
    pub modified: Option<DateTime<Utc>>,
}

/// The files an import wrote into the meeting's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestedMedia {
    /// The converted audio (`audio.m4a`, 48 kHz mono AAC).
    pub audio: PathBuf,
    /// The video kept for playback, for a video file.
    pub video: Option<PathBuf>,
    /// The audio's length.
    pub duration: std::time::Duration,
}

/// The meeting an import created.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMeeting {
    /// The new meeting, now `processing`.
    pub meeting_id: MeetingId,
    /// Its title.
    pub title: String,
    /// How long the recording is.
    pub duration: Seconds,
    /// Whether a video was kept for playback.
    pub has_video: bool,
}
