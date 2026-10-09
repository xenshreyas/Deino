//! The persister: owns durability and the manifest while a recording runs.
//!
//! Every [`SYNC_INTERVAL`] it reads how many frames each track's sink has
//! accepted, **then** `F_FULLFSYNC`s the sinks, **then** rewrites
//! `recording.json` atomically with those counts. Reading before syncing
//! means the manifest can lag the file by one interval but can never claim
//! audio that is not on disk. It also checks free space and reports
//! [`HealthEvent::DiskLow`]. On stop, [`Persister::close`] renames the
//! `.part` files and marks the recording closed.
//!
//! Loss bounds: `kill -9` keeps every whole segment the sink wrote (≤ 1 s
//! lost); a power loss keeps what the last sync covered (≤ 5 s lost).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_domain::{ErrorClass, ErrorCode};

use crate::durability::rename_durably;
use crate::health::{DISK_LOW, DiskSpace, HealthEvent, HealthSender};
use crate::manifest::{
    ManifestError, ManifestStore, RecordingManifest, RecordingStatus, SegmentManifest, SegmentState,
};
use crate::sink::{SegmentLedger, TrackSink};
use crate::units::Track;

/// How often the sinks are synced and the manifest rewritten.
pub const SYNC_INTERVAL: Duration = Duration::from_secs(5);

/// Why persisting failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PersistError {
    /// Syncing or renaming a track's file failed.
    #[error("could not make the {track} audio durable")]
    Storage {
        /// The failing track.
        track: Track,
        /// The OS error.
        #[source]
        source: std::io::Error,
    },
    /// The manifest could not be written.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The persister thread panicked.
    #[error("the recording persister stopped unexpectedly")]
    Panicked,
}

impl ErrorClass for PersistError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}

/// One track as the persister sees it: its sink and what the sink accepted.
#[derive(Clone, Debug)]
pub struct PersistedTrack {
    /// Which track.
    pub track: Track,
    /// The sink to sync.
    pub sink: Arc<dyn TrackSink>,
    /// What the sink accepted.
    pub ledger: Arc<SegmentLedger>,
}

/// Everything a [`Persister`] needs.
#[derive(Debug)]
pub struct PersisterParts {
    /// The recording directory.
    pub dir: PathBuf,
    /// The manifest as written at start.
    pub manifest: RecordingManifest,
    /// The tracks to sync.
    pub tracks: Vec<PersistedTrack>,
    /// Reports free space.
    pub disk: Arc<dyn DiskSpace>,
    /// Receives `DiskLow` and `StorageFailed`.
    pub health: HealthSender,
}

/// Durability and manifest bookkeeping for one recording.
#[derive(Debug)]
pub struct Persister {
    dir: PathBuf,
    store: ManifestStore,
    manifest: RecordingManifest,
    tracks: Vec<PersistedTrack>,
    disk: Arc<dyn DiskSpace>,
    health: HealthSender,
    disk_low: bool,
}

impl Persister {
    /// A persister for the recording in `parts.dir`, starting from its manifest.
    pub fn new(parts: PersisterParts) -> Self {
        Self {
            store: ManifestStore::new(&parts.dir),
            dir: parts.dir,
            manifest: parts.manifest,
            tracks: parts.tracks,
            disk: parts.disk,
            health: parts.health,
            disk_low: false,
        }
    }

    /// The manifest as last written.
    pub fn manifest(&self) -> &RecordingManifest {
        &self.manifest
    }

    /// Syncs every sink and rewrites the manifest with the synced counts.
    pub fn checkpoint(&mut self) -> Result<(), PersistError> {
        self.check_disk();
        for index in 0..self.tracks.len() {
            self.sync_track(index)?;
        }
        self.store.write(&self.manifest)?;
        Ok(())
    }

    /// Final checkpoint, then renames each open `.part` file to `.m4a` and
    /// marks the recording closed.
    pub fn close(mut self) -> Result<RecordingManifest, PersistError> {
        self.checkpoint()?;
        for entry in &mut self.manifest.tracks {
            let open = entry
                .segments
                .iter_mut()
                .filter(|segment| segment.state == SegmentState::Open);
            for segment in open {
                close_segment(&self.dir, entry.track, segment)?;
            }
        }
        self.manifest.status = RecordingStatus::Closed;
        self.store.write(&self.manifest)?;
        Ok(self.manifest)
    }

    /// Snapshot, sync, then record one track's durable frames.
    fn sync_track(&mut self, index: usize) -> Result<(), PersistError> {
        let Some(track) = self.tracks.get(index) else {
            return Ok(());
        };
        let encoded = track.ledger.encoded_frames();
        if let Err(source) = track.sink.sync() {
            self.health
                .emit(HealthEvent::StorageFailed { track: track.track });
            return Err(PersistError::Storage {
                track: track.track,
                source,
            });
        }
        let durable = encoded - self.manifest.priming_frames;
        let track = track.track;
        if let Some(segment) = self.manifest.open_segment_mut(track) {
            segment.durable_frames = durable;
        }
        Ok(())
    }

    /// Raises `DiskLow` when free space first drops below [`DISK_LOW`].
    fn check_disk(&mut self) {
        let Ok(free) = self.disk.free(&self.dir) else {
            return;
        };
        let low = free < DISK_LOW;
        if low && !self.disk_low {
            self.health.emit(HealthEvent::DiskLow { free });
        }
        self.disk_low = low;
    }
}

/// Renames an open segment's `.part` file to `.m4a` and marks it closed.
fn close_segment(
    dir: &Path,
    track: Track,
    segment: &mut SegmentManifest,
) -> Result<(), PersistError> {
    let part = dir.join(segment.relative_path(track));
    segment.state = SegmentState::Closed;
    let closed = dir.join(segment.relative_path(track));
    rename_durably(&part, &closed).map_err(|source| PersistError::Storage { track, source })
}

/// A persister running on its own thread, checkpointing every [`SYNC_INTERVAL`].
#[derive(Debug)]
pub struct PersisterThread {
    stop: Sender<()>,
    thread: JoinHandle<Persister>,
}

impl PersisterThread {
    /// Starts checkpointing `persister` every `interval`.
    pub fn spawn(mut persister: Persister, interval: Duration) -> std::io::Result<Self> {
        let (stop, stops) = channel();
        let thread = std::thread::Builder::new()
            .name("deino-audio-persister".into())
            .spawn(move || {
                while let Err(RecvTimeoutError::Timeout) = stops.recv_timeout(interval) {
                    if let Err(error) = persister.checkpoint() {
                        log::error!(target: "deino_audio::persister", "checkpoint failed: {error}");
                    }
                }
                persister
            })?;
        Ok(Self { stop, thread })
    }

    /// Stops the thread and hands the persister back for [`Persister::close`].
    pub fn stop(self) -> Result<Persister, PersistError> {
        // A send fails only if the thread already ended; join reports why.
        self.stop.send(()).ok();
        self.thread.join().map_err(|_| PersistError::Panicked)
    }
}

#[cfg(test)]
#[path = "persister_tests.rs"]
mod tests;
