//! A recording service over real SQLite, mock capture playing fixture
//! audio, fake encoders and the scripted fake STT engine. The host clock
//! either runs several times faster than real time ([`FastClock`], for the
//! state-machine tests) or is paced by the lanes' intake
//! (`support/paced.rs`, for the transcript tests), so a minute of fixture
//! audio records in a few seconds.
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
    Bytes, EncoderFactory, FakeDiskSpace, FakeEncoderFactory, HostClock, HostTime, MockAudioSource,
    RecorderDeps, SystemHostClock, TestSignal, Track,
};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::vocabulary::{LiveVocabulary, NoLiveVocabulary};
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

/// A test's host clock: it can wait until some recording time has passed.
pub trait TestClock: HostClock {
    /// Returns once `audio` of recording time has passed, without sleeping.
    fn wait_for(&self, audio: Duration);
}

impl TestClock for FastClock {
    fn wait_for(&self, audio: Duration) {
        FastClock::wait_for(self, audio);
    }
}

/// What a rig records with: its clock and capture.
pub struct Harness {
    /// The clock the test waits on.
    pub clock: Arc<dyn TestClock>,
    /// The same clock, as the recorder's host clock.
    pub host: Arc<dyn HostClock>,
    /// The sources.
    pub capture: Arc<dyn CaptureBackend>,
}

impl From<FixtureCapture> for Harness {
    fn from(capture: FixtureCapture) -> Self {
        let clock = Arc::clone(&capture.clock);
        Self {
            clock: clock.clone(),
            host: clock,
            capture: Arc::new(capture),
        }
    }
}

/// What a rig transcribes, corrects and encodes with, and the user's settings.
pub struct RigOptions {
    pub engines: Arc<dyn LaneEngines>,
    pub vocabulary: Arc<dyn LiveVocabulary>,
    pub preferences: Preferences,
    pub encoders: Arc<dyn EncoderFactory>,
}

impl Default for RigOptions {
    fn default() -> Self {
        Self {
            engines: Arc::new(ScriptedEngines),
            vocabulary: Arc::new(NoLiveVocabulary),
            preferences: Preferences::default(),
            encoders: Arc::new(FakeEncoderFactory::default()),
        }
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
    pub clock: Arc<dyn TestClock>,
    /// The wall clock: meeting times, pauses.
    pub wall: Arc<FakeClock>,
    pub database: Database,
    pub events: Arc<RecordedEvents>,
    pub recordings: tempfile::TempDir,
    /// The recording volume's free space.
    pub disk: Arc<FakeDiskSpace>,
}

/// A service recording `capture` into a fresh database, transcribing with
/// the scripted fake engine.
pub async fn rig(capture: FixtureCapture) -> Rig {
    rig_with(capture, Arc::new(ScriptedEngines)).await
}

/// A service recording `capture` and transcribing with `engines`.
pub async fn rig_with(capture: FixtureCapture, engines: Arc<dyn LaneEngines>) -> Rig {
    let options = RigOptions {
        engines,
        ..RigOptions::default()
    };
    rig_over(capture, options).await
}

/// A service recording `capture` with `options`.
pub async fn rig_over(capture: FixtureCapture, options: RigOptions) -> Rig {
    rig_on(capture.into(), options).await
}

/// A service over `harness` with `options`, into a fresh database.
pub async fn rig_on(harness: Harness, options: RigOptions) -> Rig {
    let database = deino_testkit::test_db().await;
    let recordings = tempfile::tempdir().unwrap();
    let wall = Arc::new(FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 9, 10, 0, 0).unwrap(),
    ));
    let events = Arc::new(RecordedEvents::default());
    let disk = Arc::new(FakeDiskSpace::new(Bytes::gib(100)));
    let deps = deps(
        &database,
        &recordings,
        (options, harness.capture),
        Shared {
            host: harness.host,
            wall: wall.clone(),
            disk: disk.clone(),
            events: events.clone(),
        },
    );
    Rig {
        clock: harness.clock,
        service: RecordingService::new(deps.clone()),
        deps,
        wall,
        database,
        events,
        recordings,
        disk,
    }
}

/// The fakes a rig shares with its test.
struct Shared {
    host: Arc<dyn HostClock>,
    wall: Arc<FakeClock>,
    disk: Arc<FakeDiskSpace>,
    events: Arc<RecordedEvents>,
}

/// The service's ports over `database` and the shared fakes.
fn deps(
    database: &Database,
    recordings: &tempfile::TempDir,
    (options, capture): (RigOptions, Arc<dyn CaptureBackend>),
    shared: Shared,
) -> RecordingDeps {
    let RigOptions {
        engines,
        vocabulary,
        preferences,
        encoders,
    } = options;
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    RecordingDeps {
        catalog: meetings.clone(),
        meetings: meetings.clone(),
        lifecycle: Arc::new(MeetingLifecycleService::new(meetings, shared.wall.clone())),
        segments: segments.clone(),
        speakers: segments,
        people: Arc::new(SqlitePeopleRepo::new(database.clone())),
        engines,
        recorder: RecorderDeps {
            encoders,
            host_clock: shared.host,
            wall_clock: shared.wall,
            disk: shared.disk,
        },
        capture,
        preferences: Arc::new(preferences),
        events: shared.events,
        vocabulary,
        recordings: PathBuf::from(recordings.path()),
    }
}
