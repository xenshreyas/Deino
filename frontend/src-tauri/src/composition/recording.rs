//! Wiring for the live recorder (FND-05b, CAP-02).
//!
//! Builds `deino_recording::RecordingService` over the app's adapters: the
//! SQLite repositories, the lifecycle service that starts the pipeline, the
//! STT lanes, the Mac's devices (or `DEINO_MOCK_AUDIO` mocks in debug
//! builds), the `AVAssetWriter` track encoder and the Tauri event adapter;
//! each start links the nearest calendar event (ORG-03's `LinkOnStart`).
//! Then it recovers meetings a crash left in `recording`, in the background.

use std::sync::Arc;

use async_trait::async_trait;
use deino_audio::capture::MicChoice;
use deino_audio::{
    EncoderFactory, HostClock, MockAudio, RecorderDeps, SystemDiskSpace, SystemHostClock,
    TestSignal,
};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::vocabulary::VocabularyService;
use deino_db::Database;
use deino_domain::SystemClock;
use deino_recording::{
    CaptureBackend, DeviceCapture, MockCapture, Preferences, RecordingDeps, RecordingPreferences,
    RecordingService, recover_interrupted,
};
use tauri::{AppHandle, Manager};

use crate::features::calendar::LinkOnStart;
use crate::features::recording::TauriRecordingEvents;
use crate::paths::AppPaths;
use crate::stt::{SttLanes, SttRuntime};

/// The services the recorder is built over, managed before it.
struct Managed {
    settings: Arc<SettingsService>,
    stt: Arc<SttRuntime>,
    lifecycle: Arc<MeetingLifecycleService>,
    vocabulary: Arc<VocabularyService>,
}

impl Managed {
    fn find(app: &AppHandle) -> Option<Self> {
        Some(Self {
            settings: Arc::clone(&*app.try_state::<Arc<SettingsService>>()?),
            stt: Arc::clone(&*app.try_state::<Arc<SttRuntime>>()?),
            lifecycle: Arc::clone(&*app.try_state::<Arc<MeetingLifecycleService>>()?),
            vocabulary: Arc::clone(&*app.try_state::<Arc<VocabularyService>>()?),
        })
    }
}

/// Builds and manages `Arc<RecordingService>`, and recovers interrupted
/// recordings. Needs the settings, the STT runtime and the pipeline's
/// lifecycle service to be managed first.
pub(crate) fn attach_recording(app: &AppHandle, database: &Database) {
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            return log::error!(target: "recording", "app folders unavailable; the recorder is off: {error}");
        }
    };
    let Some(managed) = Managed::find(app) else {
        return log::error!(target: "recording", "settings, transcription, the vocabulary or the pipeline are not ready; recording is off");
    };
    let deps = recording_deps(app, database, managed, &paths);
    spawn_recovery(deps.clone());
    let link_on_start = Arc::new(LinkOnStart::new(app.clone()));
    app.manage(Arc::new(
        RecordingService::new(deps).observing_starts(link_on_start),
    ));
}

/// The recorder's ports over the app's adapters.
fn recording_deps(
    app: &AppHandle,
    database: &Database,
    managed: Managed,
    paths: &AppPaths,
) -> RecordingDeps {
    let host_clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
    let runtime = tauri::async_runtime::handle().inner().clone();
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let Managed {
        settings,
        stt,
        lifecycle,
        vocabulary,
    } = managed;
    RecordingDeps {
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        lifecycle,
        segments: segments.clone(),
        speakers: segments,
        people: Arc::new(SqlitePeopleRepo::new(database.clone())),
        engines: Arc::new(SttLanes::new(
            app.clone(),
            stt,
            Arc::clone(&settings),
            runtime,
        )),
        capture: capture(&host_clock),
        preferences: Arc::new(SettingsPreferences(settings)),
        recorder: RecorderDeps {
            encoders: encoders(),
            host_clock,
            wall_clock: Arc::new(SystemClock),
            disk: Arc::new(SystemDiskSpace),
        },
        events: Arc::new(TauriRecordingEvents::new(app.clone())),
        vocabulary,
        recordings: paths.recordings(),
    }
}

/// Recovers meetings a crash left in `recording`, off the startup path.
fn spawn_recovery(deps: RecordingDeps) {
    tauri::async_runtime::spawn(async move {
        match recover_interrupted(&deps).await {
            Ok(report) if report.recovered.is_empty() && report.failed.is_empty() => {}
            Ok(report) => log::info!(
                target: "recording",
                "recovered {} interrupted recordings ({} without audio)",
                report.recovered.len(),
                report.failed.len()
            ),
            Err(error) => log::error!(target: "recording", "launch recovery failed: {error}"),
        }
    });
}

/// The Mac's devices, or mock audio when `DEINO_MOCK_AUDIO` asks (debug builds).
fn capture(clock: &Arc<dyn HostClock>) -> Arc<dyn CaptureBackend> {
    let Some(mock) = MockAudio::from_env() else {
        return Arc::new(DeviceCapture::new(Arc::clone(clock)));
    };
    let (microphone, system) = mock_voices(&mock);
    log::info!(target: "recording", "recording mock audio ({mock:?}) instead of the devices");
    Arc::new(MockCapture::new(Arc::clone(clock), microphone, system))
}

/// The voices a mock plays: generated, or a fixture meeting's tracks
/// (`DEINO_MOCK_AUDIO=one-on-one`); generated when the fixture cannot load.
fn mock_voices(mock: &MockAudio) -> (TestSignal, TestSignal) {
    let generated = (TestSignal::microphone(), TestSignal::system());
    let MockAudio::Fixture(name) = mock else {
        return generated;
    };
    fixture_voices(name).unwrap_or_else(|| {
        log::warn!(target: "recording", "no fixture audio named {name:?}; playing generated voices");
        generated
    })
}

#[cfg(debug_assertions)]
fn fixture_voices(name: &str) -> Option<(TestSignal, TestSignal)> {
    use deino_domain::Channel;
    use deino_testkit::fixture::{Fixtures, PcmTrack};

    let fixtures = Fixtures::load().ok()?;
    let meeting = fixtures
        .meetings()
        .iter()
        .find(|m| m.id.as_str().ends_with(name))?;
    let audio = meeting.audio.as_ref()?;
    let decode = |channel| {
        let track = PcmTrack::decode(&fixtures.audio_path(audio, channel)?).ok()?;
        Some(TestSignal::from_pcm(&track.samples, track.sample_rate))
    };
    Some((decode(Channel::Mic)?, decode(Channel::System)?))
}

#[cfg(not(debug_assertions))]
fn fixture_voices(_name: &str) -> Option<(TestSignal, TestSignal)> {
    None
}

/// The native track encoder (`AVAssetWriter`, CMAF segments; ADR 0014).
#[cfg(target_os = "macos")]
fn encoders() -> Arc<dyn EncoderFactory> {
    Arc::new(deino_audio::CmafEncoderFactory)
}

/// No native encoder off macOS: recording reports it cannot start.
#[cfg(not(target_os = "macos"))]
fn encoders() -> Arc<dyn EncoderFactory> {
    Arc::new(NoEncoders)
}

/// The encoder factory of platforms without native encoding.
#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
struct NoEncoders;

#[cfg(not(target_os = "macos"))]
impl EncoderFactory for NoEncoders {
    fn open(
        &self,
        _track: deino_audio::Track,
        _output: deino_audio::encoder::SegmentOutput,
    ) -> Result<Box<dyn deino_audio::TrackEncoder>, deino_audio::EncodeError> {
        Err(deino_audio::EncodeError::Rejected(
            "recording on this platform",
        ))
    }
}

/// The microphone, headphones and live transcription settings, read at
/// every start.
struct SettingsPreferences(Arc<SettingsService>);

#[async_trait]
impl RecordingPreferences for SettingsPreferences {
    async fn current(&self) -> Preferences {
        let settings = self.0.get().await;
        let audio = settings.audio;
        Preferences {
            microphone: MicChoice {
                preferred: audio
                    .preferred_mic_device
                    .filter(|name| !name.trim().is_empty()),
            },
            uses_headphones: audio.uses_headphones,
            record_only: !settings.transcription.live_transcription,
        }
    }
}
