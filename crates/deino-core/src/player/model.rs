//! What the player plays: a meeting's recording files by source.

use serde::{Deserialize, Serialize};

/// One recording file of a meeting (ADR 0026: `audio.mp4`, `mic.m4a`,
/// `system.m4a`; CAP-09's screen recording).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaSource {
    /// Microphone and system audio mixed: "Mixed".
    Mixed,
    /// The microphone track: "Me".
    Microphone,
    /// The system audio track: "Them".
    System,
    /// The screen recording.
    Video,
}

impl MediaSource {
    /// Every source, in the order the player offers them.
    pub const ALL: [Self; 4] = [Self::Mixed, Self::Microphone, Self::System, Self::Video];

    /// What a downloaded copy is called after the meeting's title.
    pub fn label(self) -> &'static str {
        match self {
            Self::Mixed => "Recording",
            Self::Microphone => "Me",
            Self::System => "Them",
            Self::Video => "Video",
        }
    }
}

/// A playable file of a meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MediaFile {
    /// Which recording it is.
    pub source: MediaSource,
    /// Its absolute path inside the recordings folder, for the asset protocol.
    pub path: String,
}

/// Everything the player can play for a meeting, in [`MediaSource::ALL`]
/// order; empty when the meeting has no recording on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMedia {
    /// The files that exist.
    pub files: Vec<MediaFile>,
}

impl MeetingMedia {
    /// The file of `source`, if the meeting has it.
    pub fn file(&self, source: MediaSource) -> Option<&MediaFile> {
        self.files.iter().find(|file| file.source == source)
    }
}

/// How a download ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DownloadOutcome {
    /// The copy was written where the user chose.
    Saved,
    /// The user closed the save dialog.
    Cancelled,
}
