//! `recording.json`: the typed, versioned manifest of a recording on disk.
//!
//! The manifest says which segment files make up each track, where each one
//! starts on the timeline and how many of its frames are **durable** (synced
//! to disk). The persister rewrites it atomically every few seconds and on
//! events, always *after* syncing the audio it describes, so it never claims
//! audio that is not on disk. Recovery and finalize read it back.
//!
//! ```text
//! <recording>/recording.json
//! <recording>/audio/mic.001.m4a.part     while recording (fragmented MP4)
//! <recording>/audio/mic.001.m4a          once closed or recovered
//! <recording>/mic.m4a system.m4a audio.mp4   after finalize
//! ```

mod store;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use store::{ManifestError, ManifestStore};

use crate::fmp4::AAC_PRIMING;
use crate::units::{Frames, HostTime, STORAGE_RATE, Track};

/// The manifest file's name inside a recording directory.
pub const MANIFEST_FILE: &str = "recording.json";

/// The directory holding the live segment files.
pub const AUDIO_DIR: &str = "audio";

/// Suffix of a segment file that is still being written.
const PART_SUFFIX: &str = ".m4a.part";

/// Suffix of a closed segment file.
const CLOSED_SUFFIX: &str = ".m4a";

/// The manifest format version. Bump it when a field changes meaning; readers
/// refuse versions newer than [`ManifestVersion::CURRENT`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ManifestVersion(pub u32);

impl ManifestVersion {
    /// The version this build writes.
    pub const CURRENT: Self = Self(1);
}

/// Where a recording is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    /// Being written. Found at launch, it means the app died mid-recording.
    Recording,
    /// Stopped cleanly: every segment is closed and synced.
    Closed,
    /// Rebuilt by launch recovery after a crash.
    Recovered,
    /// `mic.m4a`, `system.m4a` and `audio.mp4` exist.
    Finalized,
}

impl RecordingStatus {
    /// Whether launch recovery must repair the recording.
    pub const fn needs_recovery(self) -> bool {
        matches!(self, Self::Recording)
    }
}

/// The state of one segment file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentState {
    /// Still being appended to (`.m4a.part`).
    Open,
    /// Finished and renamed to `.m4a`.
    Closed,
    /// Recovery found no complete init segment; it holds no audio and keeps
    /// its `.m4a.part` name.
    Unplayable,
}

/// One segment file of a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentManifest {
    /// The segment's number, from 1 (`mic.001.m4a`).
    pub index: u16,
    /// Where the segment's first frame sits on the recording timeline.
    pub first_frame: Frames,
    /// Playable frames known to be synced to disk.
    pub durable_frames: Frames,
    /// Whether the file is open, closed or unplayable.
    pub state: SegmentState,
}

impl SegmentManifest {
    /// A new, empty, open segment starting at `first_frame`.
    pub const fn open(index: u16, first_frame: Frames) -> Self {
        Self {
            index,
            first_frame,
            durable_frames: Frames::ZERO,
            state: SegmentState::Open,
        }
    }

    /// The segment's path relative to the recording directory.
    pub fn relative_path(&self, track: Track) -> PathBuf {
        let suffix = match self.state {
            SegmentState::Closed => CLOSED_SUFFIX,
            SegmentState::Open | SegmentState::Unplayable => PART_SUFFIX,
        };
        Path::new(AUDIO_DIR).join(format!("{}.{:03}{suffix}", track.file_stem(), self.index))
    }
}

/// The segments of one track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackManifest {
    /// Which track this is.
    pub track: Track,
    /// Its segment files, in timeline order.
    pub segments: Vec<SegmentManifest>,
}

impl TrackManifest {
    /// Durable frames over all segments, measured to the end of the last one.
    pub fn durable_end(&self) -> Frames {
        self.segments
            .iter()
            .map(|segment| segment.first_frame + segment.durable_frames)
            .max()
            .unwrap_or_default()
    }
}

/// The whole manifest (`recording.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingManifest {
    /// Format version; see [`ManifestVersion`].
    pub version: ManifestVersion,
    /// Frames per second of every track (always [`STORAGE_RATE`]).
    pub sample_rate: u32,
    /// Encoder priming at the start of each segment, trimmed by readers.
    pub priming_frames: Frames,
    /// Host time of timeline frame zero, shared by both tracks.
    pub epoch: HostTime,
    /// Wall-clock start of the recording.
    pub started_at: DateTime<Utc>,
    /// Where the recording is in its life.
    pub status: RecordingStatus,
    /// One entry per track, each with at least one segment.
    pub tracks: Vec<TrackManifest>,
}

impl RecordingManifest {
    /// A new recording: both tracks with one open segment at frame zero.
    pub fn start(epoch: HostTime, started_at: DateTime<Utc>) -> Self {
        let tracks = Track::ALL
            .into_iter()
            .map(|track| TrackManifest {
                track,
                segments: vec![SegmentManifest::open(1, Frames::ZERO)],
            })
            .collect();
        Self {
            version: ManifestVersion::CURRENT,
            sample_rate: STORAGE_RATE,
            priming_frames: AAC_PRIMING,
            epoch,
            started_at,
            status: RecordingStatus::Recording,
            tracks,
        }
    }

    /// The entry for `track`, if the manifest has one.
    pub fn track(&self, track: Track) -> Option<&TrackManifest> {
        self.tracks.iter().find(|entry| entry.track == track)
    }

    /// The mutable entry for `track`, if the manifest has one.
    pub fn track_mut(&mut self, track: Track) -> Option<&mut TrackManifest> {
        self.tracks.iter_mut().find(|entry| entry.track == track)
    }

    /// The open segment `track` is currently writing, if any.
    pub fn open_segment_mut(&mut self, track: Track) -> Option<&mut SegmentManifest> {
        self.track_mut(track)?
            .segments
            .iter_mut()
            .rfind(|segment| segment.state == SegmentState::Open)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn started_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 9, 14, 0, 0).unwrap()
    }

    #[test]
    fn a_new_recording_has_one_open_segment_per_track() {
        let manifest = RecordingManifest::start(HostTime(5), started_at());

        assert_eq!(manifest.version, ManifestVersion::CURRENT);
        assert_eq!(manifest.status, RecordingStatus::Recording);
        for track in Track::ALL {
            let entry = manifest.track(track).unwrap();
            assert_eq!(entry.segments, vec![SegmentManifest::open(1, Frames::ZERO)]);
        }
    }

    #[test]
    fn segment_paths_follow_the_state() {
        let mut segment = SegmentManifest::open(1, Frames::ZERO);
        assert_eq!(
            segment.relative_path(Track::Microphone),
            Path::new("audio/mic.001.m4a.part")
        );
        segment.state = SegmentState::Closed;
        assert_eq!(
            segment.relative_path(Track::System),
            Path::new("audio/system.001.m4a")
        );
    }

    #[test]
    fn the_json_shape_is_stable() {
        let manifest = RecordingManifest::start(HostTime(5), started_at());
        let json = serde_json::to_value(&manifest).unwrap();

        assert_eq!(json["version"], 1);
        assert_eq!(json["sample_rate"], 48_000);
        assert_eq!(json["priming_frames"], 2112);
        assert_eq!(json["status"], "recording");
        assert_eq!(json["tracks"][1]["track"], "system");
        assert_eq!(json["tracks"][0]["segments"][0]["state"], "open");
    }

    #[test]
    fn durable_end_spans_every_segment() {
        let mut entry = TrackManifest {
            track: Track::Microphone,
            segments: vec![SegmentManifest::open(1, Frames::ZERO)],
        };
        entry.segments[0].durable_frames = Frames(100);
        entry.segments.push(SegmentManifest::open(2, Frames(500)));
        entry.segments[1].durable_frames = Frames(20);

        assert_eq!(entry.durable_end(), Frames(520));
    }
}
