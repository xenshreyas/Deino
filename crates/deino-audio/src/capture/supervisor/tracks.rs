//! The two tracks the supervisor owns, and what it does to them.
//!
//! The microphone must start for a recording to start. System audio may be
//! unavailable (no permission, an older macOS); its slot then keeps the
//! writer's ring, the track records silence, and the watchdog keeps trying
//! with backoff.

use std::sync::Arc;

use crate::capture::conditioner::CaptureShared;
use crate::capture::track::TrackCapture;
use crate::capture::watchdog::{TrackSample, TrackWatch, WatchAction, WatchPolicy};
use crate::health::{HealthEvent, HealthSender};
use crate::ring::RtSink;
use crate::source::{SourceError, SourceFactory};
use crate::units::{Frames, HostTime, Track};

/// A track that is capturing, or waiting for its device.
#[derive(Debug)]
enum Slot {
    Running(TrackCapture),
    Absent(RtSink),
    /// Momentarily empty while a start is attempted.
    Empty,
}

/// One track: its slot and its watchdog.
#[derive(Debug)]
struct Watched {
    slot: Slot,
    watch: TrackWatch,
}

/// Both tracks of a recording.
#[derive(Debug)]
pub(super) struct TrackSet {
    microphone: Watched,
    system: Watched,
    factory: Arc<dyn SourceFactory>,
    shared: CaptureShared,
    health: HealthSender,
}

/// What a [`TrackSet`] is built from.
pub(super) struct TrackSetParts {
    pub factory: Arc<dyn SourceFactory>,
    pub shared: CaptureShared,
    pub health: HealthSender,
    pub microphone: RtSink,
    pub system: RtSink,
    pub now: HostTime,
}

impl TrackSet {
    /// Starts both tracks; fails only when the microphone cannot start.
    pub(super) fn start(parts: TrackSetParts) -> Result<Self, SourceError> {
        let microphone =
            TrackCapture::start(parts.factory.microphone(), parts.microphone, &parts.shared)?;
        let mut set = Self {
            microphone: watched(Track::Microphone, Slot::Running(microphone), parts.now),
            system: watched(Track::System, Slot::Absent(parts.system), parts.now),
            factory: parts.factory,
            shared: parts.shared,
            health: parts.health,
        };
        if set.bring_up(Track::System).is_err() {
            set.health.emit(HealthEvent::SystemAudioUnavailable);
        }
        set.check_hands_free();
        Ok(set)
    }

    /// One watchdog tick for both tracks.
    pub(super) fn tick(&mut self, now: HostTime) {
        for track in Track::ALL {
            let action = observe(self.get(track), now);
            self.act(track, action);
        }
    }

    /// Stops both sources (pause, sleep); conditioners keep their rings.
    pub(super) fn suspend(&mut self) {
        for track in Track::ALL {
            if let Slot::Running(capture) = &mut self.get(track).slot {
                capture.suspend();
            }
        }
    }

    /// Restarts both sources (resume, wake) and forgets stalls.
    pub(super) fn resume(&mut self, now: HostTime) {
        for track in Track::ALL {
            if let Err(error) = self.restart(track) {
                log::warn!(target: "deino_audio::capture", "the {track} source did not restart: {error}");
            }
        }
        for track in Track::ALL {
            self.get(track).watch.reset(now);
        }
        self.check_hands_free();
    }

    /// Restarts one track's source (a device change); its stall watch keeps running.
    pub(super) fn restart(&mut self, track: Track) -> Result<(), SourceError> {
        let result = match &mut self.get(track).slot {
            Slot::Running(capture) => capture.restart(),
            Slot::Absent(_) | Slot::Empty => self.bring_up(track),
        };
        if track == Track::Microphone {
            self.check_hands_free();
        }
        result
    }

    /// Stops both tracks and joins their conditioners.
    pub(super) fn finish(self) {
        for watched in [self.microphone, self.system] {
            if let Slot::Running(capture) = watched.slot {
                capture.finish();
            }
        }
    }

    /// The watched slot of `track`.
    fn get(&mut self, track: Track) -> &mut Watched {
        match track {
            Track::Microphone => &mut self.microphone,
            Track::System => &mut self.system,
        }
    }

    fn act(&mut self, track: Track, action: WatchAction) {
        match action {
            WatchAction::None => {}
            WatchAction::Restart { attempt } => {
                self.health.emit(HealthEvent::Recovering { track, attempt });
                if let Err(error) = self.restart(track) {
                    log::warn!(target: "deino_audio::capture", "restarting the {track} source failed: {error}");
                }
            }
            WatchAction::GiveUp => self.health.emit(HealthEvent::DeviceLost { track }),
            WatchAction::Recovered => self.health.emit(HealthEvent::Recovered { track }),
        }
    }

    /// Starts a fresh source for an absent track, keeping its ring on failure.
    fn bring_up(&mut self, track: Track) -> Result<(), SourceError> {
        let Slot::Absent(output) = std::mem::replace(&mut self.get(track).slot, Slot::Empty) else {
            return Ok(());
        };
        let mut source = match track {
            Track::Microphone => self.factory.microphone(),
            Track::System => self.factory.system(),
        };
        let stream = match source.start() {
            Ok(stream) => stream,
            Err(error) => {
                self.get(track).slot = Slot::Absent(output);
                return Err(error);
            }
        };
        let capture = TrackCapture::spawn(source, stream, output, &self.shared)?;
        self.get(track).slot = Slot::Running(capture);
        Ok(())
    }

    /// Warns once per start when the microphone is a headset in call mode.
    fn check_hands_free(&self) {
        if let Slot::Running(capture) = &self.microphone.slot
            && capture.hands_free()
        {
            self.health.emit(HealthEvent::BluetoothCallMode);
        }
    }
}

fn watched(track: Track, slot: Slot, now: HostTime) -> Watched {
    let policy = match track {
        Track::Microphone => WatchPolicy::MICROPHONE,
        Track::System => WatchPolicy::SYSTEM,
    };
    Watched {
        slot,
        watch: TrackWatch::new(policy, now),
    }
}

/// Feeds one tick of `watched`'s activity to its watchdog. A suspended
/// track is left alone; an absent one counts as a dead device.
fn observe(watched: &mut Watched, now: HostTime) -> WatchAction {
    let sample = match &watched.slot {
        Slot::Running(capture) if !capture.is_running() => return WatchAction::None,
        Slot::Running(capture) => TrackSample {
            frames: capture.meter().frames(),
            alive: capture.is_alive(),
        },
        Slot::Absent(_) | Slot::Empty => TrackSample {
            frames: Frames::ZERO,
            alive: false,
        },
    };
    watched.watch.observe(now, sample)
}
