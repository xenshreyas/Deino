//! Recording health: typed events on a bounded channel, and the disk port.
//!
//! The writer and the persister report trouble as [`HealthEvent`]s. They are
//! sent with `try_send` on a bounded channel, so a slow listener can never
//! stall recording; when the channel is full the event is dropped and logged
//! (the Cap health-bus pattern, `docs/references/audio-cap-zed.md` §5).
//! CAP-10 turns these events into the record pill's warnings.

use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

use crate::units::{Bytes, Frames, Track};

/// Events the channel buffers before new ones are dropped.
pub const HEALTH_CAPACITY: usize = 32;

/// Free space below which [`HealthEvent::DiskLow`] is raised (CAP-10's 2 GB).
pub const DISK_LOW: Bytes = Bytes::gib(2);

/// Something about a recording the user may need to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthEvent {
    /// The encoder has refused audio for `for_at_least`; audio is queuing in
    /// the in-memory backlog.
    Stalled {
        /// The stalled track.
        track: Track,
        /// How long the encoder has not accepted audio.
        for_at_least: Duration,
    },
    /// The encoder accepts audio again after a stall.
    Resumed {
        /// The track that recovered.
        track: Track,
    },
    /// Audio was lost: the real-time ring or the backlog overflowed.
    FramesDropped {
        /// The track that lost audio.
        track: Track,
        /// Frames lost since the previous report.
        frames: Frames,
    },
    /// The recording volume is running out of space.
    DiskLow {
        /// Free space left.
        free: Bytes,
    },
    /// Writing or syncing a track's file failed; audio after this may be lost.
    StorageFailed {
        /// The failing track.
        track: Track,
    },
    /// A track's device failed or stalled; the supervisor is restarting it.
    Recovering {
        /// The track.
        track: Track,
        /// The attempt within this episode, from 1.
        attempt: u8,
    },
    /// A recovering track delivers audio again.
    Recovered {
        /// The track.
        track: Track,
    },
    /// Every attempt failed; the track records silence until a later retry.
    DeviceLost {
        /// The track.
        track: Track,
    },
    /// The microphone is a Bluetooth headset in call (hands-free) mode, so
    /// its quality is reduced.
    BluetoothCallMode,
    /// System audio cannot be captured (no permission, or no process tap on
    /// this macOS); the recording continues with the microphone only.
    SystemAudioUnavailable,
    /// The Mac went to sleep; capture is suspended.
    Asleep,
    /// The Mac woke up; capture restarted.
    Awake,
}

/// The sending half of the health channel. Cloning shares the channel.
#[derive(Clone, Debug)]
pub struct HealthSender {
    sender: SyncSender<HealthEvent>,
}

impl HealthSender {
    /// Sends `event` without blocking. A full or closed channel drops it.
    pub fn emit(&self, event: HealthEvent) {
        match self.sender.try_send(event) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(dropped)) => {
                log::warn!(target: "deino_audio::health", "health channel full; dropped {dropped:?}");
            }
        }
    }
}

/// A bounded health channel of [`HEALTH_CAPACITY`] events.
pub fn health_channel() -> (HealthSender, Receiver<HealthEvent>) {
    let (sender, receiver) = sync_channel(HEALTH_CAPACITY);
    (HealthSender { sender }, receiver)
}

/// Free space on the volume holding a path.
pub trait DiskSpace: std::fmt::Debug + Send + Sync {
    /// Bytes available to this user on the volume that holds `path`.
    fn free(&self, path: &Path) -> std::io::Result<Bytes>;
}

/// The real disk, read with `statvfs`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDiskSpace;

impl DiskSpace for SystemDiskSpace {
    #[cfg(unix)]
    fn free(&self, path: &Path) -> std::io::Result<Bytes> {
        let stats = rustix::fs::statvfs(path)?;
        Ok(Bytes(stats.f_bavail.saturating_mul(stats.f_frsize)))
    }

    #[cfg(not(unix))]
    fn free(&self, _path: &Path) -> std::io::Result<Bytes> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeDiskSpace;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::DiskSpace;
    use crate::units::Bytes;

    /// A disk whose free space a test sets.
    #[derive(Debug)]
    pub struct FakeDiskSpace {
        free: AtomicU64,
    }

    impl FakeDiskSpace {
        /// A disk with `free` bytes available.
        pub fn new(free: Bytes) -> Self {
            Self {
                free: AtomicU64::new(free.0),
            }
        }

        /// Changes the free space.
        pub fn set_free(&self, free: Bytes) {
            self.free.store(free.0, Ordering::SeqCst);
        }
    }

    impl DiskSpace for FakeDiskSpace {
        fn free(&self, _path: &Path) -> std::io::Result<Bytes> {
            Ok(Bytes(self.free.load(Ordering::SeqCst)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_channel_drops_events_instead_of_blocking_the_sender() {
        let (health, events) = health_channel();
        for _ in 0..HEALTH_CAPACITY + 5 {
            health.emit(HealthEvent::DiskLow { free: Bytes(1) });
        }
        assert_eq!(events.try_iter().count(), HEALTH_CAPACITY);
    }

    #[test]
    fn emitting_after_the_listener_left_is_harmless() {
        let (health, events) = health_channel();
        drop(events);
        health.emit(HealthEvent::Resumed {
            track: Track::System,
        });
    }

    #[cfg(unix)]
    #[test]
    fn the_system_disk_reports_some_free_space_for_the_temp_dir() {
        let free = SystemDiskSpace.free(&std::env::temp_dir()).unwrap();
        assert!(free > Bytes(0));
    }
}
