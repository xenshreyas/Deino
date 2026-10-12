//! The two tracks the supervisor owns, and what it does to them.
//!
//! The microphone must start for a recording to start. System audio may be
//! unavailable (no permission, an older macOS); its slot then keeps the
//! writer's ring, the track records silence, and the watchdog keeps trying
//! with backoff. Mid-recording the user can switch the microphone (the new
//! device feeds the same ring, so the timeline goes on) and turn system
//! audio off and on again (off records silence). Every change and every
//! tick publishes each track's [`TrackStatus`].

use std::sync::Arc;

use crate::capture::conditioner::CaptureShared;
use crate::capture::devices::MicChoice;
use crate::capture::track::TrackCapture;
use crate::capture::watchdog::WatchAction;
use crate::health::{HealthEvent, HealthSender};
use crate::ring::RtSink;
use crate::source::{AudioSource, SourceError, SourceFactory};
use crate::units::{HostTime, Track};

use super::watched::{Slot, Watched, observe, status_of, watched};

/// Both tracks of a recording.
#[derive(Debug)]
pub(super) struct TrackSet {
    microphone: Watched,
    system: Watched,
    factory: Arc<dyn SourceFactory>,
    choice: Option<MicChoice>,
    shared: CaptureShared,
    health: HealthSender,
    suspended: bool,
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
            choice: None,
            shared: parts.shared,
            health: parts.health,
            suspended: false,
        };
        if set.bring_up(Track::System).is_err() {
            set.health.emit(HealthEvent::SystemAudioUnavailable);
        }
        set.check_hands_free();
        set.publish();
        Ok(set)
    }

    /// One watchdog tick for both tracks.
    pub(super) fn tick(&mut self, now: HostTime) {
        for track in Track::ALL {
            let action = observe(self.get(track), now);
            self.act(track, action);
        }
        self.publish();
    }

    /// Stops both sources (pause, sleep); conditioners keep their rings.
    pub(super) fn suspend(&mut self) {
        self.suspended = true;
        for track in Track::ALL {
            self.stop(track);
        }
        self.publish();
    }

    /// Restarts the wanted sources (resume, wake) and forgets stalls.
    pub(super) fn resume(&mut self, now: HostTime) {
        self.suspended = false;
        for track in Track::ALL {
            if !self.get(track).enabled {
                continue;
            }
            if let Err(error) = self.restart(track) {
                log::warn!(target: "deino_audio::capture", "the {track} source did not restart: {error}");
            }
            self.get(track).watch.reset(now);
        }
        self.check_hands_free();
        self.publish();
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

    /// Records from the microphone `choice` names from now on. The new
    /// device feeds the same ring, so the track goes on without a gap
    /// beyond the switch itself (the timeline fills it with silence).
    pub(super) fn switch_microphone(
        &mut self,
        choice: MicChoice,
        now: HostTime,
    ) -> Result<(), SourceError> {
        let source = self.factory.microphone_for(&choice);
        self.choice = Some(choice);
        let result = match &mut self.microphone.slot {
            Slot::Running(capture) if !self.suspended => capture.replace(source),
            Slot::Running(capture) => {
                capture.adopt(source);
                Ok(())
            }
            Slot::Absent(_) | Slot::Empty => self.bring_up(Track::Microphone),
        };
        self.microphone.watch.reset(now);
        self.check_hands_free();
        self.publish();
        result
    }

    /// Turns a track on or off. Off stops its source and records silence;
    /// on starts it again (immediately, unless the recording is paused).
    pub(super) fn set_enabled(&mut self, track: Track, enabled: bool, now: HostTime) {
        if self.get(track).enabled == enabled {
            return;
        }
        self.get(track).enabled = enabled;
        if !enabled {
            self.stop(track);
        } else if !self.suspended {
            if let Err(error) = self.restart(track) {
                log::warn!(target: "deino_audio::capture", "the {track} source did not start: {error}");
            }
            self.get(track).watch.reset(now);
        }
        self.publish();
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

    /// Stops a running source, keeping its conditioner.
    fn stop(&mut self, track: Track) {
        if let Slot::Running(capture) = &mut self.get(track).slot {
            capture.suspend();
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
        let mut source = self.new_source(track);
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

    /// A new source for `track`, following the microphone switch if any.
    fn new_source(&self, track: Track) -> Box<dyn AudioSource> {
        match (track, &self.choice) {
            (Track::Microphone, Some(choice)) => self.factory.microphone_for(choice),
            (Track::Microphone, None) => self.factory.microphone(),
            (Track::System, _) => self.factory.system(),
        }
    }

    /// Writes each track's status for the pipeline, health and the panel.
    fn publish(&self) {
        for (track, watched) in [
            (Track::Microphone, &self.microphone),
            (Track::System, &self.system),
        ] {
            let status = status_of(watched, self.suspended);
            self.shared.tracks.get(track).set_status(status);
        }
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
