//! The capture supervisor: one thread that owns both tracks for a recording.
//!
//! It receives commands (pause, resume, stop), hardware and power events
//! (the [`SystemWatch`] port) and a tick every [`TICK`], and turns them into
//! actions on the tracks ([`tracks::TrackSet`]):
//!
//! | Input | Microphone | System audio |
//! |---|---|---|
//! | default input changed, device list changed | restart after a 1 s debounce (it re-picks its device) | – |
//! | default output changed | – | no rebuild: the global tap is upstream of the output device; the route is re-probed for the echo canceller |
//! | stalled or failed (watchdog) | restart with backoff | same |
//! | will sleep / did wake | stop / restart | same |
//! | pause / resume | stop / restart; the pause leaves the timeline | same |
//!
//! A tick that comes more than [`TICK_GAP`] of wall time after the last one
//! means the Mac slept without telling us (openwhispr's tick-gap check);
//! the tracks are restarted as on wake. Host time stops during sleep, so the
//! timeline itself has no gap to fill.

mod actor;
mod tracks;
mod watched;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_domain::Clock;

use self::actor::{Command, Input, Supervisor};
use self::tracks::{TrackSet, TrackSetParts};
use super::conditioner::CaptureShared;
use super::devices::MicChoice;
use super::events::{SystemEvent, SystemWatch};
use super::route::{OutputRoute, RouteProbe};
use super::state::TrackStates;
use crate::clock::HostClock;
use crate::health::HealthSender;
use crate::ring::RtSink;
use crate::source::{SourceError, SourceFactory};
use crate::units::Track;

/// How often the watchdog looks at the tracks.
pub const TICK: Duration = Duration::from_secs(1);

/// Wall time between ticks that means the Mac was asleep.
pub const TICK_GAP: Duration = Duration::from_secs(5);

/// How long a burst of device events is collected before acting (anarlog).
pub const DEVICE_DEBOUNCE: Duration = Duration::from_secs(1);

/// Events queued for the supervisor before new ones are dropped.
const INPUT_CAPACITY: usize = 64;

/// The track writers' rings the capture feeds.
#[derive(Debug)]
pub struct CaptureInputs {
    /// The microphone writer's ring.
    pub microphone: RtSink,
    /// The system writer's ring.
    pub system: RtSink,
}

/// What a recording's capture is built from.
#[derive(Clone)]
pub struct CaptureParts {
    /// Makes the sources.
    pub factory: Arc<dyn SourceFactory>,
    /// Hardware and power events.
    pub watch: Arc<dyn SystemWatch>,
    /// Speakers or headphones.
    pub route: Arc<dyn RouteProbe>,
    /// Receives each accepted route change.
    pub routes: Sender<OutputRoute>,
    /// Monotonic time (stalls, pauses).
    pub host_clock: Arc<dyn HostClock>,
    /// Wall time (the sleep tick-gap check).
    pub wall_clock: Arc<dyn Clock>,
    /// Where capture problems are reported.
    pub health: HealthSender,
}

/// A running recording's capture.
#[derive(Debug)]
pub struct Capture {
    commands: SyncSender<Input>,
    thread: Option<JoinHandle<()>>,
    shared: CaptureShared,
}

impl Capture {
    /// Starts both tracks and the supervisor. Fails when the microphone
    /// cannot start; missing system audio is reported and retried.
    pub fn start(inputs: CaptureInputs, parts: &CaptureParts) -> Result<Self, SourceError> {
        let shared = CaptureShared::default();
        let tracks = TrackSet::start(TrackSetParts {
            factory: Arc::clone(&parts.factory),
            shared: shared.clone(),
            health: parts.health.clone(),
            microphone: inputs.microphone,
            system: inputs.system,
            now: parts.host_clock.now(),
        })?;
        let (commands, inputs) = sync_channel(INPUT_CAPACITY);
        let (events, system_events) = sync_channel(INPUT_CAPACITY);
        let subscription = parts.watch.watch(events);
        forward(system_events, commands.clone());
        let supervisor = Supervisor::new(tracks, parts, Arc::clone(&shared.pause));
        let thread = std::thread::Builder::new()
            .name("deino-audio-supervisor".into())
            .spawn(move || {
                // Listeners stay registered for as long as the supervisor runs.
                let _subscription = subscription;
                supervisor.run(&inputs);
            })
            .map_err(|source| SourceError::Spawn {
                track: Track::Microphone,
                source,
            })?;
        Ok(Self {
            commands,
            thread: Some(thread),
            shared,
        })
    }

    /// The paused time the timeline skips.
    pub fn paused(&self) -> Duration {
        self.shared.pause.total()
    }

    /// Stops both sources; the timeline skips the pause.
    pub fn pause(&self) {
        self.request(Command::Pause);
    }

    /// Restarts both sources after a pause.
    pub fn resume(&self) {
        self.request(Command::Resume);
    }

    /// Each track's meter and status, for the live pipeline, recording
    /// health and the record panel.
    pub fn tracks(&self) -> Arc<TrackStates> {
        Arc::clone(&self.shared.tracks)
    }

    /// Records from the microphone `choice` names from now on; returns
    /// once the switch was attempted (a device that fails to start is
    /// reported through health and retried like any other).
    pub fn set_microphone(&self, choice: MicChoice) {
        self.request(|reply| Command::Microphone(choice, reply));
    }

    /// Turns system audio on or off; off records silence on its track.
    pub fn set_system_audio(&self, enabled: bool) {
        self.request(|reply| Command::SystemAudio(enabled, reply));
    }

    /// Stops both tracks and their conditioners; returns once their audio
    /// has reached the writers' rings.
    pub fn stop(mut self) {
        self.shut_down();
    }

    fn request(&self, command: impl FnOnce(SyncSender<()>) -> Command) {
        let (reply, done) = sync_channel(1);
        if self.commands.send(Input::Command(command(reply))).is_ok() {
            done.recv().ok();
        }
    }

    fn shut_down(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.request(Command::Stop);
        if thread.join().is_err() {
            log::error!(target: "deino_audio::capture", "the capture supervisor panicked");
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// Moves system events into the supervisor's input, on a helper thread.
fn forward(events: Receiver<SystemEvent>, into: SyncSender<Input>) {
    let spawned = std::thread::Builder::new()
        .name("deino-audio-events".into())
        .spawn(move || {
            for event in events {
                if into.try_send(Input::System(event)).is_err() {
                    log::debug!(target: "deino_audio::capture", "dropped a system event");
                }
            }
        });
    if let Err(error) = spawned {
        log::error!(target: "deino_audio::capture", "device events are not watched: {error}");
    }
}

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
