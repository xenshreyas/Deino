//! `Recorder`: one crash-safe two-track recording, start to stop.
//!
//! `start` takes the epoch, writes the initial `recording.json`, creates one
//! ring, `.part` sink and writer thread per track, and starts the persister.
//! It returns the rings' real-time ends ([`RtSink`]) for the capture side,
//! the placed-audio rings ([`TimelineSource`]) for live transcription and the
//! health channel. `stop` pads both tracks to the stop time, finishes the
//! encoders, renames the files and writes the closed manifest.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use deino_domain::{Clock, ErrorClass, ErrorCode};

use crate::clock::HostClock;
use crate::encoder::{EncoderFactory, SegmentOutput};
use crate::health::{DiskSpace, HealthEvent, HealthSender, health_channel};
use crate::manifest::{AUDIO_DIR, ManifestError, ManifestStore, RecordingManifest};
use crate::persister::{
    PersistError, PersistedTrack, Persister, PersisterParts, PersisterThread, SYNC_INTERVAL,
};
use crate::ring::{RtSink, TimelineSource, timeline_ring, track_ring};
use crate::sink::PartFileSink;
use crate::units::{Frames, HostTime, Track};
use crate::writer::{TrackWriter, TrackWriterParts, WriterContext, WriterError, WriterReport};

/// Why a recording could not start or stop cleanly.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecordError {
    /// The recording directory or a segment file could not be created.
    #[error("could not create the recording files")]
    Io(#[from] std::io::Error),
    /// The manifest could not be written.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// A writer thread failed.
    #[error(transparent)]
    Writer(#[from] WriterError),
    /// Syncing or closing failed.
    #[error(transparent)]
    Persist(#[from] PersistError),
}

impl ErrorClass for RecordError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}

/// The adapters a recording runs on, chosen by the composition root.
#[derive(Clone, Debug)]
pub struct RecorderDeps {
    /// Opens the track encoders (`CmafEncoderFactory` in the app).
    pub encoders: Arc<dyn EncoderFactory>,
    /// Monotonic time for the epoch, stop time and stalls.
    pub host_clock: Arc<dyn HostClock>,
    /// Wall time for the manifest's `started_at`.
    pub wall_clock: Arc<dyn Clock>,
    /// Free space, for `DiskLow`.
    pub disk: Arc<dyn DiskSpace>,
}

/// What a started recording hands to the capture side.
#[derive(Debug)]
pub struct RecordingInputs {
    /// The microphone ring's real-time end.
    pub microphone: RtSink,
    /// The system-audio ring's real-time end.
    pub system: RtSink,
    /// Stalls, drops and disk warnings.
    pub health: Receiver<HealthEvent>,
    /// Reports more health events on the same channel (capture problems).
    pub reporter: HealthSender,
    /// Each track's audio as it was placed on the timeline, for live
    /// transcription. Undrained rings drop blocks; recording is unaffected.
    pub placed: PlacedAudio,
}

/// Both tracks' placed audio: 48 kHz blocks stamped with timeline positions.
#[derive(Debug)]
pub struct PlacedAudio {
    /// The microphone track.
    pub microphone: TimelineSource,
    /// The system track.
    pub system: TimelineSource,
}

/// What a stopped recording produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingSummary {
    /// The closed manifest.
    pub manifest: RecordingManifest,
    /// Each writer's report, microphone first.
    pub tracks: Vec<(Track, WriterReport)>,
}

/// A running recording.
#[derive(Debug)]
pub struct Recorder {
    dir: PathBuf,
    epoch: HostTime,
    writers: Vec<TrackWriter>,
    persister: PersisterThread,
    host_clock: Arc<dyn HostClock>,
}

impl Recorder {
    /// Starts recording into `dir` (created if needed). Start the capture
    /// sources right after, with the returned [`RecordingInputs`].
    pub fn start(dir: &Path, deps: &RecorderDeps) -> Result<(Self, RecordingInputs), RecordError> {
        std::fs::create_dir_all(dir.join(AUDIO_DIR))?;
        let manifest = RecordingManifest::start(deps.host_clock.now(), deps.wall_clock.now());
        ManifestStore::new(dir).write(&manifest)?;
        let (health, events) = health_channel();
        let context = WriterContext {
            encoders: Arc::clone(&deps.encoders),
            clock: Arc::clone(&deps.host_clock),
            health: health.clone(),
            epoch: manifest.epoch,
        };
        let mic = start_track(dir, Track::Microphone, &context)?;
        let system = start_track(dir, Track::System, &context)?;
        let persister = PersisterThread::spawn(
            Persister::new(PersisterParts {
                dir: dir.to_owned(),
                manifest,
                tracks: vec![mic.persisted, system.persisted],
                disk: Arc::clone(&deps.disk),
                health: health.clone(),
            }),
            SYNC_INTERVAL,
        )?;
        let recorder = Self {
            dir: dir.to_owned(),
            epoch: context.epoch,
            writers: vec![mic.writer, system.writer],
            persister,
            host_clock: Arc::clone(&deps.host_clock),
        };
        let inputs = RecordingInputs {
            microphone: mic.input,
            system: system.input,
            health: events,
            reporter: health,
            placed: PlacedAudio {
                microphone: mic.placed,
                system: system.placed,
            },
        };
        Ok((recorder, inputs))
    }

    /// The recording directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The recording epoch (`t0`): the host time both tracks start at, so
    /// a recording's offsets are measured from it.
    pub fn epoch(&self) -> HostTime {
        self.epoch
    }

    /// Frames each track has handed to its encoder so far.
    pub fn appended(&self, track: Track) -> Frames {
        self.writers
            .iter()
            .find(|writer| writer.track() == track)
            .map_or(Frames::ZERO, TrackWriter::appended)
    }

    /// Stops both tracks at the current host time and closes the recording.
    /// Stop the capture sources first.
    pub fn stop(self) -> Result<RecordingSummary, RecordError> {
        let stop_at = self.host_clock.now();
        self.stop_at(stop_at)
    }

    /// Stops both tracks at `stop_at` on the timeline's host clock (the
    /// current time less any paused time) and closes the recording.
    pub fn stop_at(self, stop_at: HostTime) -> Result<RecordingSummary, RecordError> {
        let mut tracks = Vec::with_capacity(self.writers.len());
        let mut first_error = None;
        for writer in self.writers {
            let track = writer.track();
            match writer.stop(stop_at) {
                Ok(report) => tracks.push((track, report)),
                Err(error) => first_error = first_error.or(Some(error)),
            }
        }
        let manifest = self.persister.stop()?.close()?;
        match first_error {
            Some(error) => Err(error.into()),
            None => Ok(RecordingSummary { manifest, tracks }),
        }
    }
}

/// One started track: its rings' outer ends, writer and persisted files.
struct StartedTrack {
    input: RtSink,
    placed: TimelineSource,
    writer: TrackWriter,
    persisted: PersistedTrack,
}

/// Creates one track's rings, `.part` sink and writer thread.
fn start_track(
    dir: &Path,
    track: Track,
    context: &WriterContext,
) -> Result<StartedTrack, RecordError> {
    let (input, source) = track_ring();
    let (placed_sink, placed) = timeline_ring();
    let manifest_path =
        crate::manifest::SegmentManifest::open(1, Frames::ZERO).relative_path(track);
    let sink = Arc::new(PartFileSink::create(&dir.join(manifest_path))?);
    let output = SegmentOutput {
        sink: sink.clone(),
        ledger: Arc::default(),
    };
    let persisted = PersistedTrack {
        track,
        sink,
        ledger: Arc::clone(&output.ledger),
    };
    let writer = TrackWriter::spawn(
        TrackWriterParts {
            track,
            source,
            placed: placed_sink,
            output,
        },
        context,
    )?;
    Ok(StartedTrack {
        input,
        placed,
        writer,
        persisted,
    })
}

#[cfg(test)]
#[path = "recorder_tests.rs"]
mod tests;
