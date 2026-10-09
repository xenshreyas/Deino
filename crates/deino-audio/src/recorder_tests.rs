//! Recorder tests over fake encoders and clocks: alignment, files, failures.

#![allow(
    clippy::float_cmp,
    reason = "samples are copied or generated exactly, so they compare exactly"
)]

use std::time::Duration;

use chrono::{TimeZone, Utc};
use deino_domain::FakeClock;

use super::*;
use crate::clock::FakeHostClock;
use crate::encoder::{EncodeError, FakeEncoderFactory, TrackEncoder};
use crate::health::FakeDiskSpace;
use crate::manifest::{RecordingStatus, SegmentState};
use crate::units::{Bytes, HostTime};

/// 10 ms of 48 kHz audio.
const BLOCK: Duration = Duration::from_millis(10);

fn deps(encoders: Arc<dyn EncoderFactory>) -> (RecorderDeps, Arc<FakeHostClock>) {
    let host_clock = Arc::new(FakeHostClock::new(HostTime(5_000_000_000)));
    let started_at = Utc.with_ymd_and_hms(2026, 10, 9, 14, 0, 0).unwrap();
    let deps = RecorderDeps {
        encoders,
        host_clock: host_clock.clone(),
        wall_clock: Arc::new(FakeClock::new(started_at)),
        disk: Arc::new(FakeDiskSpace::new(Bytes::gib(100))),
    };
    (deps, host_clock)
}

#[test]
fn both_tracks_share_the_epoch_and_end_together() {
    let encoders = FakeEncoderFactory::default();
    let (deps, clock) = deps(Arc::new(encoders.clone()));
    let dir = tempfile::tempdir().unwrap();
    let (recorder, mut inputs) = Recorder::start(dir.path(), &deps).unwrap();

    for block in 0..100 {
        assert!(inputs.microphone.push(&[0.25; 480], clock.now()));
        if block >= 50 {
            assert!(inputs.system.push(&[0.5; 480], clock.now()));
        }
        clock.advance(BLOCK);
    }
    let summary = recorder.stop().unwrap();

    let microphone = encoders.samples(Track::Microphone);
    let system = encoders.samples(Track::System);
    assert_eq!(microphone.len(), 48_000);
    assert_eq!(system.len(), 48_000);
    assert!(microphone.iter().all(|s| *s == 0.25));
    assert!(system[..24_000].iter().all(|s| *s == 0.0));
    assert!(system[24_000..].iter().all(|s| *s == 0.5));
    assert_eq!(summary.tracks[0].1.frames, Frames(48_000));
    assert_eq!(summary.manifest.epoch, HostTime(5_000_000_000));
}

#[test]
fn stopping_closes_the_files_and_the_manifest() {
    let (deps, clock) = deps(Arc::new(FakeEncoderFactory::default()));
    let dir = tempfile::tempdir().unwrap();
    let (recorder, mut inputs) = Recorder::start(dir.path(), &deps).unwrap();
    assert!(inputs.microphone.push(&vec![0.1; 48_000], clock.now()));
    clock.advance(Duration::from_secs(1));

    let summary = recorder.stop().unwrap();

    let on_disk = ManifestStore::new(dir.path()).read().unwrap();
    assert_eq!(on_disk, summary.manifest);
    assert_eq!(on_disk.status, RecordingStatus::Closed);
    let microphone = on_disk.track(Track::Microphone).unwrap();
    assert_eq!(microphone.segments[0].state, SegmentState::Closed);
    assert_eq!(microphone.segments[0].durable_frames, Frames(48_000 - 2112));
    let file = dir.path().join("audio/mic.001.m4a");
    assert_eq!(std::fs::metadata(file).unwrap().len(), 48_000 * 4);
}

#[test]
fn the_manifest_exists_as_soon_as_recording_starts() {
    let (deps, _clock) = deps(Arc::new(FakeEncoderFactory::default()));
    let dir = tempfile::tempdir().unwrap();

    let (recorder, _inputs) = Recorder::start(dir.path(), &deps).unwrap();

    let on_disk = ManifestStore::new(dir.path()).read().unwrap();
    assert_eq!(on_disk.status, RecordingStatus::Recording);
    assert!(dir.path().join("audio/system.001.m4a.part").exists());
    recorder.stop().unwrap();
}

#[derive(Debug)]
struct NoEncoder;

impl EncoderFactory for NoEncoder {
    fn open(
        &self,
        _track: Track,
        _output: SegmentOutput,
    ) -> Result<Box<dyn TrackEncoder>, EncodeError> {
        Err(EncodeError::Unsupported)
    }
}

#[test]
fn an_encoder_that_cannot_open_fails_the_start() {
    let (deps, _clock) = deps(Arc::new(NoEncoder));
    let dir = tempfile::tempdir().unwrap();

    let error = Recorder::start(dir.path(), &deps).unwrap_err();

    assert!(matches!(
        error,
        RecordError::Writer(WriterError::Encode {
            track: Track::Microphone,
            source: EncodeError::Unsupported,
        })
    ));
}
