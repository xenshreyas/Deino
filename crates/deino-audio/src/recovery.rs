//! Launch recovery for recordings the app never closed (a crash, `kill -9`,
//! a power loss).
//!
//! [`recover_recording`] is a pure function of a recording directory, safe to
//! run again if it is itself interrupted:
//!
//! 1. Read `recording.json`, or rebuild it from the segment files when it is
//!    missing (a crash before the first manifest write).
//! 2. For each open segment, cut any torn tail with
//!    [`truncate_to_playable`] and record its playable frames
//!    (`packets × 1024 − 2112`). A segment without a complete `moov` holds
//!    no audio and is marked unplayable.
//! 3. Rename `.m4a.part` to `.m4a`, sync the directory, and write the
//!    manifest with status `recovered`.
//!
//! Setting the meeting to `processing` and enqueueing `finalize_audio` is the
//! caller's job (FND-05b, FND-08).

mod rebuild;

use std::path::{Path, PathBuf};

use deino_domain::{ErrorClass, ErrorCode};

use crate::durability::rename_durably;
use crate::fmp4::truncate_to_playable;
use crate::manifest::{
    AUDIO_DIR, ManifestError, ManifestStore, RecordingManifest, RecordingStatus, SegmentManifest,
    SegmentState,
};
use crate::units::{Bytes, Frames, Track};

/// Why a recording could not be recovered.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecoveryError {
    /// The manifest is unreadable for a reason other than being missing.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// There is neither a manifest nor any segment file: nothing to recover.
    #[error("the folder holds no recording")]
    NothingToRecover,
    /// Scanning, truncating or renaming a segment file failed.
    #[error("could not repair the {track} audio")]
    Segment {
        /// The track being repaired.
        track: Track,
        /// The OS error.
        #[source]
        source: std::io::Error,
    },
    /// Listing the segment files failed.
    #[error("could not list the recording's audio files")]
    Io(#[from] std::io::Error),
}

impl ErrorClass for RecoveryError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Manifest(error) => error.code(),
            Self::NothingToRecover => ErrorCode::NotFound,
            Self::Segment { .. } | Self::Io(_) => ErrorCode::Internal,
        }
    }
}

/// What recovery did to one track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackRecovery {
    /// Playable frames across the track's segments after repair.
    pub playable: Frames,
    /// Torn bytes cut from the end of its files.
    pub torn: Bytes,
    /// Segments found without a complete init segment.
    pub unplayable_segments: u32,
}

/// The outcome of [`recover_recording`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// The recording was closed cleanly; nothing was touched.
    NotNeeded(RecordingManifest),
    /// The recording was repaired.
    Recovered {
        /// The repaired manifest (status `recovered`).
        manifest: RecordingManifest,
        /// Whether the manifest had to be rebuilt from the files.
        rebuilt: bool,
        /// What happened to each track, microphone first.
        tracks: Vec<(Track, TrackRecovery)>,
    },
}

/// The folder a meeting's recording lives in: `<root>/<meeting id>`. The
/// recorder, launch recovery and the `finalize_audio` stage all find it here.
pub fn recording_dir(root: &Path, meeting: &deino_domain::MeetingId) -> PathBuf {
    root.join(meeting.as_str())
}

/// Repairs the recording in `dir` if it was never closed.
pub fn recover_recording(dir: &Path) -> Result<Recovery, RecoveryError> {
    let store = ManifestStore::new(dir);
    let (mut manifest, rebuilt) = match store.read() {
        Ok(manifest) => (manifest, false),
        Err(ManifestError::Missing) => (rebuild::from_segment_files(dir)?, true),
        Err(error) => return Err(error.into()),
    };
    if !manifest.status.needs_recovery() {
        return Ok(Recovery::NotNeeded(manifest));
    }
    let mut tracks = Vec::with_capacity(manifest.tracks.len());
    for entry in &mut manifest.tracks {
        let mut report = TrackRecovery::default();
        for segment in &mut entry.segments {
            repair_segment(dir, entry.track, segment, &mut report)?;
        }
        tracks.push((entry.track, report));
    }
    manifest.status = RecordingStatus::Recovered;
    store.write(&manifest)?;
    Ok(Recovery::Recovered {
        manifest,
        rebuilt,
        tracks,
    })
}

/// Recording directories under `root` that need [`recover_recording`]:
/// a manifest still `recording`, or segment files without any manifest.
pub fn find_unclosed(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let dir = entry?.path();
        if dir.is_dir() && needs_recovery(&dir) {
            found.push(dir);
        }
    }
    found.sort();
    Ok(found)
}

/// Whether one directory holds an unclosed recording.
fn needs_recovery(dir: &Path) -> bool {
    match ManifestStore::new(dir).read() {
        Ok(manifest) => manifest.status.needs_recovery(),
        Err(ManifestError::Missing) => rebuild::has_segment_files(&dir.join(AUDIO_DIR)),
        // A damaged or newer manifest is left for the user; recovery would fail.
        Err(_) => false,
    }
}

/// Truncates, measures and closes one open segment.
fn repair_segment(
    dir: &Path,
    track: Track,
    segment: &mut SegmentManifest,
    report: &mut TrackRecovery,
) -> Result<(), RecoveryError> {
    let failed = |source| RecoveryError::Segment { track, source };
    let part = dir.join(segment.relative_path(track));
    let closed = dir.join(
        SegmentManifest {
            state: SegmentState::Closed,
            ..*segment
        }
        .relative_path(track),
    );
    let file = match segment.state {
        SegmentState::Open if part.exists() => part.clone(),
        SegmentState::Open | SegmentState::Closed if closed.exists() => closed.clone(),
        _ => {
            mark_unplayable(segment, report);
            return Ok(());
        }
    };
    let scanned = truncate_to_playable(&file).map_err(failed)?;
    report.torn.0 += scanned.torn_bytes().0;
    if !scanned.has_moov {
        mark_unplayable(segment, report);
        return Ok(());
    }
    if file == part {
        rename_durably(&part, &closed).map_err(failed)?;
    }
    segment.durable_frames = scanned.playable_frames();
    segment.state = SegmentState::Closed;
    report.playable += segment.durable_frames;
    Ok(())
}

/// Records a segment that holds no audio.
fn mark_unplayable(segment: &mut SegmentManifest, report: &mut TrackRecovery) {
    segment.durable_frames = Frames::ZERO;
    segment.state = SegmentState::Unplayable;
    report.unplayable_segments += 1;
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
