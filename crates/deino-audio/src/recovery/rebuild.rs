//! Rebuilding a lost `recording.json` from the segment files on disk.
//!
//! The recorder writes the manifest before any audio, so a missing manifest
//! means it was lost or deleted. The segment names say which track and
//! segment each file is (`mic.002.m4a.part`); each segment is assumed to
//! follow the previous one directly on the timeline, and the epoch is
//! unknown (zero). The recovery pass then repairs the files as usual.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::RecoveryError;
use crate::fmp4::scan;
use crate::manifest::{AUDIO_DIR, RecordingManifest, SegmentManifest, SegmentState, TrackManifest};
use crate::units::{Frames, HostTime, Track};

/// Suffix of an open segment file.
const PART_SUFFIX: &str = ".m4a.part";

/// Suffix of a closed segment file.
const CLOSED_SUFFIX: &str = ".m4a";

/// A segment file found on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoundSegment {
    track: Track,
    index: u16,
    state: SegmentState,
}

/// A manifest describing the segment files in `dir/audio`.
pub(super) fn from_segment_files(dir: &Path) -> Result<RecordingManifest, RecoveryError> {
    let found = segment_files(&dir.join(AUDIO_DIR))?;
    if found.is_empty() {
        return Err(RecoveryError::NothingToRecover);
    }
    let mut manifest = RecordingManifest::start(HostTime::default(), created_at(dir));
    for entry in &mut manifest.tracks {
        *entry = track_from_files(dir, entry.track, &found);
    }
    Ok(manifest)
}

/// Whether `audio` holds any segment file.
pub(super) fn has_segment_files(audio: &Path) -> bool {
    segment_files(audio).is_ok_and(|found| !found.is_empty())
}

/// One track's segments, each placed right after the previous one.
fn track_from_files(dir: &Path, track: Track, found: &[FoundSegment]) -> TrackManifest {
    let mut next_frame = Frames::ZERO;
    let segments = found
        .iter()
        .filter(|segment| segment.track == track)
        .map(|found| {
            let segment = SegmentManifest {
                state: found.state,
                ..SegmentManifest::open(found.index, next_frame)
            };
            let playable = scan(&dir.join(segment.relative_path(track)))
                .map(|scanned| scanned.playable_frames())
                .unwrap_or_default();
            next_frame += playable;
            segment
        })
        .collect();
    TrackManifest { track, segments }
}

/// Segment files in `audio`, sorted by track then index. A missing folder
/// holds none.
fn segment_files(audio: &Path) -> std::io::Result<Vec<FoundSegment>> {
    let entries = match std::fs::read_dir(audio) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        entries => entries?,
    };
    let mut found = Vec::new();
    for entry in entries {
        if let Some(segment) = entry?.file_name().to_str().and_then(parse_name) {
            found.push(segment);
        }
    }
    found.sort_by_key(|segment| (segment.track, segment.index));
    Ok(found)
}

/// Parses `mic.001.m4a.part` or `system.002.m4a`.
fn parse_name(name: &str) -> Option<FoundSegment> {
    let (stem, state) = match name.strip_suffix(PART_SUFFIX) {
        Some(stem) => (stem, SegmentState::Open),
        None => (name.strip_suffix(CLOSED_SUFFIX)?, SegmentState::Closed),
    };
    let (track, index) = stem.split_once('.')?;
    let track = Track::ALL.into_iter().find(|t| t.file_stem() == track)?;
    Some(FoundSegment {
        track,
        index: index.parse().ok()?,
        state,
    })
}

/// When the recording folder was created (or last changed), as the best
/// available start time.
fn created_at(dir: &Path) -> DateTime<Utc> {
    std::fs::metadata(dir)
        .and_then(|meta| meta.created().or_else(|_| meta.modified()))
        .map(DateTime::<Utc>::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_names_parse_into_track_index_and_state() {
        assert_eq!(
            parse_name("mic.001.m4a.part"),
            Some(FoundSegment {
                track: Track::Microphone,
                index: 1,
                state: SegmentState::Open,
            })
        );
        assert_eq!(
            parse_name("system.012.m4a"),
            Some(FoundSegment {
                track: Track::System,
                index: 12,
                state: SegmentState::Closed,
            })
        );
    }

    #[test]
    fn unrelated_names_are_ignored() {
        for name in [
            "recording.json",
            "mic.m4a",
            "camera.001.m4a",
            "mic.x.m4a.part",
            ".DS_Store",
        ] {
            assert_eq!(parse_name(name), None, "{name}");
        }
    }
}
