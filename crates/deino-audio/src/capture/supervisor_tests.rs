//! Supervisor tests: the thread-free `Supervisor` driven with fake clocks
//! and scripted sources, so device changes, sleep, pauses and stalls are
//! checked without hardware and without waiting.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};

use deino_domain::FakeClock;

use super::*;
use crate::capture::devices::MicChoice;
use crate::capture::events::NoSystemWatch;
use crate::capture::events::{DeviceEvent, PowerEvent};
use crate::capture::route::FixedRoute;
use crate::capture::state::{TrackStates, TrackStatus};
use crate::clock::FakeHostClock;
use crate::health::HealthEvent;
use crate::health::health_channel;
use crate::ring::{capture_ring, track_ring};
use crate::source::{AudioSource, SourceStream};
use crate::units::HostTime;
use crate::units::SampleRate;

/// What a scripted source has been asked to do, shared with the test.
#[derive(Debug, Default)]
struct Script {
    starts: AtomicUsize,
    refuse: AtomicBool,
    dead: AtomicBool,
}

/// A source that delivers nothing and does what its script says.
#[derive(Debug)]
struct ScriptedSource {
    track: Track,
    script: Arc<Script>,
}

impl AudioSource for ScriptedSource {
    fn track(&self) -> Track {
        self.track
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        if self.script.refuse.load(Ordering::SeqCst) {
            return Err(SourceError::NoDevice(self.track));
        }
        self.script.starts.fetch_add(1, Ordering::SeqCst);
        let (_sink, blocks) = capture_ring(48_000);
        Ok(SourceStream {
            rate: SampleRate::STORAGE,
            blocks,
        })
    }

    fn stop(&mut self) {}

    fn is_alive(&self) -> bool {
        !self.script.dead.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Default)]
struct ScriptedFactory {
    microphone: Arc<Script>,
    system: Arc<Script>,
    /// The microphone picked mid-recording.
    chosen: Arc<Script>,
}

impl SourceFactory for ScriptedFactory {
    fn microphone(&self) -> Box<dyn AudioSource> {
        Box::new(ScriptedSource {
            track: Track::Microphone,
            script: Arc::clone(&self.microphone),
        })
    }

    fn system(&self) -> Box<dyn AudioSource> {
        Box::new(ScriptedSource {
            track: Track::System,
            script: Arc::clone(&self.system),
        })
    }

    fn microphone_for(&self, _choice: &MicChoice) -> Box<dyn AudioSource> {
        Box::new(ScriptedSource {
            track: Track::Microphone,
            script: Arc::clone(&self.chosen),
        })
    }
}

struct Rig {
    supervisor: Supervisor,
    tracks: Arc<TrackStates>,
    factory: Arc<ScriptedFactory>,
    host: Arc<FakeHostClock>,
    wall: Arc<FakeClock>,
    health: Receiver<HealthEvent>,
}

fn rig_with(factory: ScriptedFactory) -> Rig {
    let factory = Arc::new(factory);
    let host = Arc::new(FakeHostClock::new(HostTime(1_000_000_000)));
    let wall = Arc::new(FakeClock::new(deino_testkit::at("2026-10-09T10:00:00Z")));
    let (health, events) = health_channel();
    let (routes, _route_changes) = channel();
    let parts = CaptureParts {
        factory: factory.clone(),
        watch: Arc::new(NoSystemWatch),
        route: Arc::new(FixedRoute(Some(OutputRoute::Speakers))),
        routes,
        host_clock: host.clone(),
        wall_clock: wall.clone(),
        health: health.clone(),
    };
    let shared = CaptureShared::default();
    let states = Arc::clone(&shared.tracks);
    let tracks = TrackSet::start(TrackSetParts {
        factory: factory.clone(),
        shared,
        health,
        microphone: track_ring().0,
        system: track_ring().0,
        now: host.now(),
    })
    .unwrap();
    let supervisor = Supervisor::new(tracks, &parts, Arc::default());
    Rig {
        supervisor,
        tracks: states,
        factory,
        host,
        wall,
        health: events,
    }
}

impl Rig {
    /// Advances both clocks by one tick and ticks.
    fn tick(&mut self) {
        self.host.advance(TICK);
        self.wall.advance(chrono::TimeDelta::seconds(1));
        self.supervisor.tick();
    }

    fn mic_starts(&self) -> usize {
        self.factory.microphone.starts.load(Ordering::SeqCst)
    }

    fn health(&self) -> Vec<HealthEvent> {
        self.health.try_iter().collect()
    }

    fn status(&self, track: Track) -> TrackStatus {
        self.tracks.get(track).status()
    }

    fn command(&mut self, command: impl FnOnce(std::sync::mpsc::SyncSender<()>) -> Command) {
        let (reply, _done) = std::sync::mpsc::sync_channel(1);
        self.supervisor.command(command(reply));
    }
}

#[test]
fn missing_system_audio_is_reported_and_the_microphone_still_records() {
    let factory = ScriptedFactory::default();
    factory.system.refuse.store(true, Ordering::SeqCst);
    let rig = rig_with(factory);

    assert_eq!(rig.mic_starts(), 1);
    assert_eq!(rig.health(), [HealthEvent::SystemAudioUnavailable]);
}

#[test]
fn a_default_input_change_restarts_the_microphone_after_the_debounce() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.supervisor
        .system_event(SystemEvent::Device(DeviceEvent::DefaultInputChanged));
    rig.supervisor
        .system_event(SystemEvent::Device(DeviceEvent::DeviceListChanged));
    assert_eq!(rig.mic_starts(), 1, "debounced, not immediate");

    rig.tick();

    assert_eq!(rig.mic_starts(), 2, "one restart for the burst");
}

#[test]
fn a_dead_device_is_restarted_and_reported() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.factory.microphone.dead.store(true, Ordering::SeqCst);

    rig.tick();

    assert_eq!(rig.mic_starts(), 2);
    assert!(rig.health().contains(&HealthEvent::Recovering {
        track: Track::Microphone,
        attempt: 1
    }));
}

#[test]
fn sleep_stops_capture_and_wake_restarts_it() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.supervisor
        .system_event(SystemEvent::Power(PowerEvent::WillSleep));
    rig.tick();
    assert_eq!(rig.mic_starts(), 1, "no watchdog restarts while asleep");

    rig.supervisor
        .system_event(SystemEvent::Power(PowerEvent::DidWake));

    assert_eq!(rig.mic_starts(), 2);
    assert_eq!(rig.health(), [HealthEvent::Asleep, HealthEvent::Awake]);
}

#[test]
fn a_long_wall_clock_gap_between_ticks_is_treated_as_a_wake() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.wall.advance(chrono::TimeDelta::minutes(10));

    rig.tick();

    assert_eq!(rig.mic_starts(), 2);
    assert!(rig.health().contains(&HealthEvent::Awake));
}

#[test]
fn a_pause_is_added_to_the_ledger_on_resume() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.supervisor.pause();
    rig.host.advance(Duration::from_secs(30));
    rig.tick();
    assert_eq!(rig.mic_starts(), 1, "no watchdog while paused");

    rig.supervisor.resume();

    assert_eq!(rig.supervisor.pause.total(), Duration::from_secs(31));
    assert_eq!(rig.mic_starts(), 2);
}

#[test]
fn each_track_publishes_what_its_capture_is_doing() {
    let factory = ScriptedFactory::default();
    factory.system.refuse.store(true, Ordering::SeqCst);
    let mut rig = rig_with(factory);
    assert_eq!(rig.status(Track::Microphone), TrackStatus::Live);
    assert_eq!(rig.status(Track::System), TrackStatus::Absent);

    rig.factory.microphone.dead.store(true, Ordering::SeqCst);
    rig.tick();
    assert_eq!(rig.status(Track::Microphone), TrackStatus::Recovering);

    rig.supervisor.pause();
    assert_eq!(rig.status(Track::Microphone), TrackStatus::Off);
    assert_eq!(rig.status(Track::System), TrackStatus::Off);
}

#[test]
fn switching_the_microphone_starts_the_chosen_device_at_once() {
    let mut rig = rig_with(ScriptedFactory::default());

    rig.command(|reply| Command::Microphone(MicChoice::default(), reply));

    assert_eq!(rig.factory.chosen.starts.load(Ordering::SeqCst), 1);
    assert_eq!(rig.status(Track::Microphone), TrackStatus::Live);
}

#[test]
fn a_microphone_chosen_while_paused_starts_on_resume() {
    let mut rig = rig_with(ScriptedFactory::default());
    rig.supervisor.pause();

    rig.command(|reply| Command::Microphone(MicChoice::default(), reply));
    assert_eq!(rig.factory.chosen.starts.load(Ordering::SeqCst), 0);
    rig.supervisor.resume();

    assert_eq!(rig.factory.chosen.starts.load(Ordering::SeqCst), 1);
    assert_eq!(rig.mic_starts(), 1, "the old device does not come back");
}

#[test]
fn system_audio_switched_off_records_silence_until_switched_on() {
    let mut rig = rig_with(ScriptedFactory::default());
    assert_eq!(rig.factory.system.starts.load(Ordering::SeqCst), 1);

    rig.command(|reply| Command::SystemAudio(false, reply));
    for _ in 0..10 {
        rig.tick();
    }
    assert_eq!(rig.status(Track::System), TrackStatus::Off);
    assert_eq!(
        rig.factory.system.starts.load(Ordering::SeqCst),
        1,
        "the watchdog leaves it off"
    );
    rig.supervisor.pause();
    rig.supervisor.resume();
    assert_eq!(
        rig.factory.system.starts.load(Ordering::SeqCst),
        1,
        "resuming keeps it off"
    );

    rig.command(|reply| Command::SystemAudio(true, reply));
    assert_eq!(rig.factory.system.starts.load(Ordering::SeqCst), 2);
    assert_eq!(rig.status(Track::System), TrackStatus::Live);
}
