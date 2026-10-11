//! A recording service over real SQLite, mock capture playing fixture
//! audio, fake encoders and the scripted fake STT engine, with a host clock
//! that runs several times faster than real time ([`FastClock`]), so a
//! minute of fixture audio records in a few seconds.
#![allow(dead_code, reason = "each test crate uses part of the shared rig")]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use deino_audio::capture::{
    FixedRoute, MicChoice, NoSystemWatch, OutputRoute, RouteProbe, SystemWatch,
};
use deino_audio::source::{AudioSource, SourceFactory};
use deino_audio::{
    Bytes, FakeDiskSpace, FakeEncoderFactory, HostClock, HostTime, MockAudioSource, RecorderDeps,
    SystemHostClock, TestSignal, Track,
};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_db::Database;
use deino_domain::FakeClock;
use deino_recording::events::RecordedEvents;
use deino_recording::testing::ScriptedEngines;
use deino_recording::{CaptureBackend, LaneEngines, Preferences, RecordingDeps, RecordingService};

/// Host time that runs `speed` times faster than the real clock.
#[derive(Debug)]
pub struct FastClock {
    real: SystemHostClock,
    origin: HostTime,
    speed: u32,
}

impl FastClock {
    pub fn new(speed: u32) -> Self {
        let real = SystemHostClock::new();
        Self {
            origin: real.now(),
            real,
            speed,
        }
    }

    /// Waits until `audio` of recording time has passed, without sleeping.
    pub fn wait_for(&self, audio: Duration) {
        let (_keep, timer) = channel::<()>();
        timer.recv_timeout(audio / self.speed).ok();
    }
}

impl HostClock for FastClock {
    fn now(&self) -> HostTime {
        let elapsed = self.real.now().since(self.origin) * self.speed;
        self.origin.plus(elapsed)
    }
}

/// Mock capture playing two signals from the start of the recording.
pub struct FixtureCapture {
    pub clock: Arc<FastClock>,
    pub microphone: TestSignal,
    pub system: TestSignal,
    pub route: OutputRoute,
}

#[derive(Debug)]
struct Sources {
    clock: Arc<dyn HostClock>,
    microphone: TestSignal,
    system: TestSignal,
}

impl SourceFactory for Sources {
    fn microphone(&self) -> Box<dyn AudioSource> {
        Box::new(MockAudioSource::new(
            Track::Microphone,
            self.microphone.clone(),
            Arc::clone(&self.clock),
        ))
    }

    fn system(&self) -> Box<dyn AudioSource> {
        Box::new(MockAudioSource::new(
            Track::System,
            self.system.clone(),
            Arc::clone(&self.clock),
        ))
    }
}

impl CaptureBackend for FixtureCapture {
    fn sources(&self, _microphone: &MicChoice) -> Arc<dyn SourceFactory> {
        Arc::new(Sources {
            clock: self.clock.clone(),
            microphone: self.microphone.clone(),
            system: self.system.clone(),
        })
    }

    fn watch(&self) -> Arc<dyn SystemWatch> {
        Arc::new(NoSystemWatch)
    }

    fn route(&self) -> Arc<dyn RouteProbe> {
        Arc::new(FixedRoute(Some(self.route)))
    }
}

/// The service and what the test inspects.
pub struct Rig {
    pub service: RecordingService,
    /// What the service was built over (launch recovery takes it).
    pub deps: RecordingDeps,
    pub clock: Arc<FastClock>,
    /// The wall clock: meeting times, pauses.
    pub wall: Arc<FakeClock>,
    pub database: Database,
    pub events: Arc<RecordedEvents>,
    pub recordings: tempfile::TempDir,
}

/// A service recording `capture` into a fresh database, transcribing with
/// the scripted fake engine.
pub async fn rig(capture: FixtureCapture) -> Rig {
    rig_with(capture, Arc::new(ScriptedEngines)).await
}

/// A service recording `capture` and transcribing with `engines`.
pub async fn rig_with(capture: FixtureCapture, engines: Arc<dyn LaneEngines>) -> Rig {
    let database = deino_testkit::test_db().await;
    let capture_clock = Arc::clone(&capture.clock);
    let recordings = tempfile::tempdir().unwrap();
    let wall = Arc::new(FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap(),
    ));
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let events = Arc::new(RecordedEvents::default());
    let deps = RecordingDeps {
        meetings: meetings.clone(),
        lifecycle: Arc::new(MeetingLifecycleService::new(meetings, wall.clone())),
        segments: segments.clone(),
        speakers: segments,
        people: Arc::new(SqlitePeopleRepo::new(database.clone())),
        engines,
        recorder: RecorderDeps {
            encoders: Arc::new(FakeEncoderFactory::default()),
            host_clock: capture.clock.clone(),
            wall_clock: wall.clone(),
            disk: Arc::new(FakeDiskSpace::new(Bytes::gib(100))),
        },
        capture: Arc::new(capture),
        preferences: Arc::new(Preferences::default()),
        events: events.clone(),
        recordings: PathBuf::from(recordings.path()),
    };
    Rig {
        clock: capture_clock,
        service: RecordingService::new(deps.clone()),
        deps,
        wall,
        database,
        events,
        recordings,
    }
}
