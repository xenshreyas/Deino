//! Recovery fault tests: torn tails, truncated `moov`, missing manifests,
//! interrupted recoveries and clean recordings.

use std::path::Path;

use chrono::{TimeZone, Utc};

use super::*;
use crate::fmp4_fixture::{init_segment, media_segment, track_file};
use crate::units::HostTime;

/// Packets in a one-second media segment.
const SECOND: u32 = 47;

/// Playable frames of `packets` AAC packets.
fn playable(packets: u64) -> Frames {
    Frames(packets * 1024 - 2112)
}

/// A recording directory with an unclosed manifest and the given part files.
fn crashed_recording(mic: &[u8], system: &[u8]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(AUDIO_DIR)).unwrap();
    std::fs::write(dir.path().join("audio/mic.001.m4a.part"), mic).unwrap();
    std::fs::write(dir.path().join("audio/system.001.m4a.part"), system).unwrap();
    let started_at = Utc.with_ymd_and_hms(2026, 10, 9, 14, 0, 0).unwrap();
    let manifest = RecordingManifest::start(HostTime(9), started_at);
    ManifestStore::new(dir.path()).write(&manifest).unwrap();
    dir
}

fn recovered(outcome: Recovery) -> (RecordingManifest, bool, Vec<(Track, TrackRecovery)>) {
    match outcome {
        Recovery::Recovered {
            manifest,
            rebuilt,
            tracks,
        } => (manifest, rebuilt, tracks),
        Recovery::NotNeeded(_) => panic!("expected a recovery"),
    }
}

fn segment(manifest: &RecordingManifest, track: Track) -> SegmentManifest {
    manifest.track(track).unwrap().segments[0]
}

#[test]
fn a_killed_recording_is_closed_with_every_whole_segment() {
    let dir = crashed_recording(&track_file(&[SECOND; 5]), &track_file(&[SECOND; 4]));

    let (manifest, rebuilt, tracks) = recovered(recover_recording(dir.path()).unwrap());

    assert!(!rebuilt);
    assert_eq!(manifest.status, RecordingStatus::Recovered);
    assert_eq!(
        segment(&manifest, Track::Microphone).durable_frames,
        playable(5 * 47)
    );
    assert_eq!(
        segment(&manifest, Track::System).state,
        SegmentState::Closed
    );
    assert_eq!(tracks[1].1.playable, playable(4 * 47));
    assert!(dir.path().join("audio/mic.001.m4a").exists());
    assert!(!dir.path().join("audio/mic.001.m4a.part").exists());
    assert_eq!(ManifestStore::new(dir.path()).read().unwrap(), manifest);
}

#[test]
fn a_torn_tail_is_cut_and_counted() {
    let mut torn = track_file(&[SECOND; 3]);
    let whole_len = torn.len() as u64;
    torn.extend_from_slice(&media_segment(SECOND)[..1_000]);
    let dir = crashed_recording(&torn, &track_file(&[SECOND]));

    let (manifest, _, tracks) = recovered(recover_recording(dir.path()).unwrap());

    assert_eq!(tracks[0].1.torn, Bytes(1_000));
    assert_eq!(
        segment(&manifest, Track::Microphone).durable_frames,
        playable(3 * 47)
    );
    let file = dir.path().join("audio/mic.001.m4a");
    assert_eq!(std::fs::metadata(file).unwrap().len(), whole_len);
}

#[test]
fn a_truncated_moov_marks_the_segment_unplayable_and_keeps_the_other_track() {
    let init = init_segment();
    let dir = crashed_recording(&track_file(&[SECOND; 2]), &init[..init.len() - 20]);

    let (manifest, _, tracks) = recovered(recover_recording(dir.path()).unwrap());

    let system = segment(&manifest, Track::System);
    assert_eq!(system.state, SegmentState::Unplayable);
    assert_eq!(system.durable_frames, Frames::ZERO);
    assert_eq!(tracks[1].1.unplayable_segments, 1);
    assert!(dir.path().join("audio/system.001.m4a.part").exists());
    assert_eq!(
        segment(&manifest, Track::Microphone).durable_frames,
        playable(2 * 47)
    );
}

#[test]
fn a_missing_manifest_is_rebuilt_from_the_segment_files() {
    let dir = crashed_recording(&track_file(&[SECOND; 2]), &track_file(&[SECOND]));
    std::fs::remove_file(dir.path().join("recording.json")).unwrap();
    std::fs::write(
        dir.path().join("audio/mic.002.m4a.part"),
        track_file(&[SECOND]),
    )
    .unwrap();

    let (manifest, rebuilt, tracks) = recovered(recover_recording(dir.path()).unwrap());

    assert!(rebuilt);
    let mic = manifest.track(Track::Microphone).unwrap();
    assert_eq!(mic.segments.len(), 2);
    assert_eq!(mic.segments[1].first_frame, playable(2 * 47));
    assert_eq!(tracks[0].1.playable, playable(2 * 47) + playable(47));
    assert!(dir.path().join("audio/mic.002.m4a").exists());
}

#[test]
fn recovery_interrupted_after_renaming_can_run_again() {
    let dir = crashed_recording(&track_file(&[SECOND; 2]), &track_file(&[SECOND; 2]));
    let audio = dir.path().join(AUDIO_DIR);
    std::fs::rename(audio.join("mic.001.m4a.part"), audio.join("mic.001.m4a")).unwrap();

    let (manifest, _, _) = recovered(recover_recording(dir.path()).unwrap());

    assert_eq!(
        segment(&manifest, Track::Microphone).durable_frames,
        playable(2 * 47)
    );
    assert_eq!(
        segment(&manifest, Track::Microphone).state,
        SegmentState::Closed
    );
}

#[test]
fn a_cleanly_closed_recording_is_left_alone() {
    let dir = crashed_recording(&track_file(&[SECOND]), &track_file(&[SECOND]));
    let store = ManifestStore::new(dir.path());
    let mut manifest = store.read().unwrap();
    manifest.status = RecordingStatus::Closed;
    store.write(&manifest).unwrap();

    let outcome = recover_recording(dir.path()).unwrap();

    assert_eq!(outcome, Recovery::NotNeeded(manifest));
    assert!(dir.path().join("audio/mic.001.m4a.part").exists());
}

#[test]
fn an_empty_folder_has_nothing_to_recover() {
    let dir = tempfile::tempdir().unwrap();
    let error = recover_recording(dir.path()).unwrap_err();
    assert!(matches!(error, RecoveryError::NothingToRecover));
}

#[test]
fn unclosed_recordings_are_found_under_the_meetings_root() {
    let root = tempfile::tempdir().unwrap();
    let crashed = crashed_recording(&track_file(&[SECOND]), &track_file(&[SECOND]));
    let orphan = crashed_recording(&track_file(&[SECOND]), &track_file(&[SECOND]));
    std::fs::remove_file(orphan.path().join("recording.json")).unwrap();
    let closed = crashed_recording(&[], &[]);
    let store = ManifestStore::new(closed.path());
    let mut manifest = store.read().unwrap();
    manifest.status = RecordingStatus::Closed;
    store.write(&manifest).unwrap();
    for (name, from) in [
        ("a", crashed.path()),
        ("b", orphan.path()),
        ("c", closed.path()),
    ] {
        std::fs::rename(from, root.path().join(name)).unwrap();
    }
    std::fs::create_dir(root.path().join("d")).unwrap();

    let found = find_unclosed(root.path()).unwrap();

    let names: Vec<_> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert!(Path::new(&found[0]).is_dir());
}
