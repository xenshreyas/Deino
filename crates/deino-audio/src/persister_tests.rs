//! Persister tests: durable counts, failures, disk space and closing.

use std::io;
use std::sync::mpsc::Receiver;

use chrono::{TimeZone, Utc};

use super::*;
use crate::health::{FakeDiskSpace, health_channel};
use crate::manifest::{AUDIO_DIR, SegmentManifest};
use crate::sink::PartFileSink;
use crate::units::{Bytes, Frames, HostTime};

struct Rig {
    dir: tempfile::TempDir,
    persister: Persister,
    ledgers: Vec<Arc<SegmentLedger>>,
    disk: Arc<FakeDiskSpace>,
    events: Receiver<HealthEvent>,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(AUDIO_DIR)).unwrap();
    let started_at = Utc.with_ymd_and_hms(2026, 10, 9, 14, 0, 0).unwrap();
    let manifest = RecordingManifest::start(HostTime(1), started_at);
    let tracks: Vec<PersistedTrack> = Track::ALL
        .into_iter()
        .map(|track| {
            let path = dir
                .path()
                .join(SegmentManifest::open(1, Frames::ZERO).relative_path(track));
            PersistedTrack {
                track,
                sink: Arc::new(PartFileSink::create(&path).unwrap()),
                ledger: Arc::default(),
            }
        })
        .collect();
    let ledgers = tracks.iter().map(|t| Arc::clone(&t.ledger)).collect();
    let disk = Arc::new(FakeDiskSpace::new(Bytes::gib(100)));
    let (health, events) = health_channel();
    let persister = Persister::new(PersisterParts {
        dir: dir.path().to_owned(),
        manifest,
        tracks,
        disk: disk.clone(),
        health,
    });
    Rig {
        dir,
        persister,
        ledgers,
        disk,
        events,
    }
}

fn durable(manifest: &RecordingManifest, track: Track) -> Frames {
    manifest.track(track).unwrap().segments[0].durable_frames
}

#[test]
fn a_checkpoint_writes_the_synced_playable_frames_to_disk() {
    let mut rig = rig();
    rig.ledgers[0].record(Frames(10 * 1024));

    rig.persister.checkpoint().unwrap();

    let on_disk = ManifestStore::new(rig.dir.path()).read().unwrap();
    assert_eq!(
        durable(&on_disk, Track::Microphone),
        Frames(10 * 1024 - 2112)
    );
    assert_eq!(durable(&on_disk, Track::System), Frames::ZERO);
    assert_eq!(on_disk.status, RecordingStatus::Recording);
}

#[derive(Debug)]
struct BrokenDisk;

impl TrackSink for BrokenDisk {
    fn append(&self, _segment: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn sync(&self) -> io::Result<()> {
        Err(io::ErrorKind::Other.into())
    }
}

#[test]
fn a_failed_sync_is_reported_and_never_counted_as_durable() {
    let mut rig = rig();
    rig.persister.tracks[1].sink = Arc::new(BrokenDisk);
    rig.ledgers[1].record(Frames(48_000));

    let error = rig.persister.checkpoint().unwrap_err();

    assert!(matches!(
        error,
        PersistError::Storage {
            track: Track::System,
            ..
        }
    ));
    assert_eq!(
        durable(rig.persister.manifest(), Track::System),
        Frames::ZERO
    );
    let events: Vec<_> = rig.events.try_iter().collect();
    assert_eq!(
        events,
        vec![HealthEvent::StorageFailed {
            track: Track::System
        }]
    );
}

#[test]
fn low_disk_space_is_reported_once_until_it_recovers() {
    let mut rig = rig();
    rig.disk.set_free(Bytes::mib(500));

    rig.persister.checkpoint().unwrap();
    rig.persister.checkpoint().unwrap();
    rig.disk.set_free(Bytes::gib(50));
    rig.persister.checkpoint().unwrap();
    rig.disk.set_free(Bytes::mib(10));
    rig.persister.checkpoint().unwrap();

    let events: Vec<_> = rig.events.try_iter().collect();
    assert_eq!(
        events,
        vec![
            HealthEvent::DiskLow {
                free: Bytes::mib(500)
            },
            HealthEvent::DiskLow {
                free: Bytes::mib(10)
            },
        ]
    );
}

#[test]
fn closing_renames_the_part_files_and_marks_the_recording_closed() {
    let rig = rig();
    let dir = rig.dir.path().to_owned();

    let manifest = rig.persister.close().unwrap();

    assert_eq!(manifest.status, RecordingStatus::Closed);
    assert!(dir.join("audio/mic.001.m4a").exists());
    assert!(dir.join("audio/system.001.m4a").exists());
    assert!(!dir.join("audio/mic.001.m4a.part").exists());
    assert_eq!(ManifestStore::new(&dir).read().unwrap(), manifest);
}

#[test]
fn the_thread_hands_the_persister_back_when_stopped() {
    let rig = rig();
    let thread = PersisterThread::spawn(rig.persister, Duration::from_hours(1)).unwrap();

    let persister = thread.stop().unwrap();

    assert_eq!(persister.manifest().status, RecordingStatus::Recording);
}
