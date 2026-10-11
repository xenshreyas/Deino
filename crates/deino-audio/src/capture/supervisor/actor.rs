//! The supervisor thread: its inputs, its state, and what it does on each
//! command, system event and tick (the table in [`super`]).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender};

use chrono::{DateTime, Utc};
use deino_domain::Clock;

use super::tracks::TrackSet;
use super::{CaptureParts, DEVICE_DEBOUNCE, TICK, TICK_GAP};
use crate::capture::devices::MicChoice;
use crate::capture::events::{DeviceEvent, PowerEvent, SystemEvent};
use crate::capture::meter::PauseLedger;
use crate::capture::route::{OutputRoute, ROUTE_POLL, RouteProbe, RouteTracker};
use crate::clock::HostClock;
use crate::health::{HealthEvent, HealthSender};
use crate::units::{HostTime, Track};

/// A supervisor command, with its reply.
pub(super) enum Command {
    Pause(SyncSender<()>),
    Resume(SyncSender<()>),
    Stop(SyncSender<()>),
    /// Record from this microphone from now on.
    Microphone(MicChoice, SyncSender<()>),
    /// Turn system audio on or off.
    SystemAudio(bool, SyncSender<()>),
}

/// Everything the supervisor thread receives.
pub(super) enum Input {
    Command(Command),
    System(SystemEvent),
}

/// The supervisor thread's state.
pub(super) struct Supervisor {
    tracks: TrackSet,
    pub(super) pause: Arc<PauseLedger>,
    route: Arc<dyn RouteProbe>,
    routes: Sender<OutputRoute>,
    tracker: Option<RouteTracker>,
    host_clock: Arc<dyn HostClock>,
    wall_clock: Arc<dyn Clock>,
    health: HealthSender,
    paused_at: Option<HostTime>,
    asleep: bool,
    restart_mic_at: Option<HostTime>,
    probe_route_at: HostTime,
    last_tick: DateTime<Utc>,
}

impl Supervisor {
    pub(super) fn new(tracks: TrackSet, parts: &CaptureParts, pause: Arc<PauseLedger>) -> Self {
        Self {
            tracks,
            pause,
            route: Arc::clone(&parts.route),
            routes: parts.routes.clone(),
            tracker: parts.route.route().map(RouteTracker::new),
            host_clock: Arc::clone(&parts.host_clock),
            wall_clock: Arc::clone(&parts.wall_clock),
            health: parts.health.clone(),
            paused_at: None,
            asleep: false,
            restart_mic_at: None,
            probe_route_at: parts.host_clock.now().plus(ROUTE_POLL),
            last_tick: parts.wall_clock.now(),
        }
    }

    /// Handles inputs and ticks until stopped.
    pub(super) fn run(mut self, inputs: &Receiver<Input>) {
        loop {
            match inputs.recv_timeout(TICK) {
                Ok(Input::Command(command)) => {
                    if self.command(command) {
                        break;
                    }
                }
                Ok(Input::System(event)) => self.system_event(event),
                Err(RecvTimeoutError::Timeout) => self.tick(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        self.tracks.finish();
    }

    /// Applies a command; returns true for stop.
    pub(super) fn command(&mut self, command: Command) -> bool {
        let now = self.host_clock.now();
        let (reply, stop) = match command {
            Command::Pause(reply) => (reply, self.pause()),
            Command::Resume(reply) => (reply, self.resume()),
            Command::Stop(reply) => (reply, true),
            Command::Microphone(choice, reply) => {
                if let Err(error) = self.tracks.switch_microphone(choice, now) {
                    log::warn!(target: "deino_audio::capture", "the chosen microphone did not start: {error}");
                }
                (reply, false)
            }
            Command::SystemAudio(enabled, reply) => {
                self.tracks.set_enabled(Track::System, enabled, now);
                (reply, false)
            }
        };
        reply.send(()).ok();
        stop
    }

    pub(super) fn pause(&mut self) -> bool {
        if self.paused_at.is_none() {
            self.tracks.suspend();
            self.paused_at = Some(self.host_clock.now());
        }
        false
    }

    pub(super) fn resume(&mut self) -> bool {
        let now = self.host_clock.now();
        if let Some(paused_at) = self.paused_at.take() {
            self.pause.add(now.since(paused_at));
            self.tracks.resume(now);
        }
        false
    }

    pub(super) fn system_event(&mut self, event: SystemEvent) {
        let now = self.host_clock.now();
        match event {
            SystemEvent::Device(DeviceEvent::DefaultOutputChanged) => {
                self.probe_route_at = now.plus(DEVICE_DEBOUNCE);
            }
            SystemEvent::Device(_) => self.restart_mic_at = Some(now.plus(DEVICE_DEBOUNCE)),
            SystemEvent::Power(PowerEvent::WillSleep) => self.sleep(),
            SystemEvent::Power(PowerEvent::DidWake) => self.wake(now),
        }
    }

    fn sleep(&mut self) {
        if !self.asleep && self.paused_at.is_none() {
            self.tracks.suspend();
            self.health.emit(HealthEvent::Asleep);
        }
        self.asleep = true;
    }

    fn wake(&mut self, now: HostTime) {
        self.asleep = false;
        if self.paused_at.is_none() {
            self.tracks.resume(now);
            self.health.emit(HealthEvent::Awake);
        }
    }

    /// The periodic work: sleep detection, the debounced device restart,
    /// the route probe and the watchdog.
    pub(super) fn tick(&mut self) {
        let now = self.host_clock.now();
        let wall = self.wall_clock.now();
        let gap = (wall - self.last_tick).to_std().unwrap_or_default();
        self.last_tick = wall;
        if gap > TICK_GAP {
            self.wake(now);
        }
        if self.paused_at.is_some() || self.asleep {
            return;
        }
        self.restart_mic_if_due(now);
        self.probe_route_if_due(now);
        self.tracks.tick(now);
    }

    fn restart_mic_if_due(&mut self, now: HostTime) {
        if self.restart_mic_at.is_none_or(|at| now < at) {
            return;
        }
        self.restart_mic_at = None;
        if let Err(error) = self.tracks.restart(Track::Microphone) {
            log::warn!(target: "deino_audio::capture", "the microphone did not follow the device change: {error}");
        }
    }

    fn probe_route_if_due(&mut self, now: HostTime) {
        if now < self.probe_route_at {
            return;
        }
        self.probe_route_at = now.plus(ROUTE_POLL);
        let probe = self.route.route();
        let tracker = self
            .tracker
            .get_or_insert_with(|| RouteTracker::new(probe.unwrap_or(OutputRoute::Speakers)));
        if let Some(route) = tracker.observe(probe) {
            self.routes.send(route).ok();
        }
    }
}
