//! The ports a recording runs on, chosen by the composition root.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use deino_audio::RecorderDeps;
use deino_audio::capture::{MicChoice, RouteProbe, SystemWatch};
use deino_audio::source::SourceFactory;
use deino_core::meetings::{MeetingLifecycleRepo, MeetingLifecycleService};
use deino_core::people::PersonRepo;
use deino_core::segments::{SegmentWriter, SpeakerRepo};

use crate::engine::LaneEngines;
use crate::events::RecordingEvents;

/// Where audio comes from: real devices, or `DEINO_MOCK_AUDIO` mocks.
pub trait CaptureBackend: Send + Sync {
    /// The sources of a recording that uses `microphone`.
    fn sources(&self, microphone: &MicChoice) -> Arc<dyn SourceFactory>;
    /// Hardware and power events.
    fn watch(&self) -> Arc<dyn SystemWatch>;
    /// Speakers or headphones.
    fn route(&self) -> Arc<dyn RouteProbe>;
}

/// The user's recording settings, read at every start.
#[async_trait]
pub trait RecordingPreferences: Send + Sync {
    /// The settings now.
    async fn current(&self) -> Preferences;
}

/// What the user chose in Settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    /// The microphone setting.
    pub microphone: MicChoice,
    /// The "I use headphones" setting: skip echo cancellation and holdback.
    pub uses_headphones: bool,
}

/// Fixed preferences (tests, and anywhere settings do not apply).
#[async_trait]
impl RecordingPreferences for Preferences {
    async fn current(&self) -> Preferences {
        self.clone()
    }
}

/// Everything the recording service is built over.
#[derive(Clone)]
pub struct RecordingDeps {
    /// Creates meetings and stores their timing.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// Moves meetings to `processing` (which starts the pipeline).
    pub lifecycle: Arc<MeetingLifecycleService>,
    /// Saves the live transcript.
    pub segments: Arc<dyn SegmentWriter>,
    /// Creates the Me and Them speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// Finds the user's own person, for the Me speaker.
    pub people: Arc<dyn PersonRepo>,
    /// Opens each lane's transcription engine.
    pub engines: Arc<dyn LaneEngines>,
    /// Real or mock capture.
    pub capture: Arc<dyn CaptureBackend>,
    /// The microphone and headphones settings.
    pub preferences: Arc<dyn RecordingPreferences>,
    /// The track writers' adapters (encoder, clocks, disk).
    pub recorder: RecorderDeps,
    /// Where state, live segments and health go.
    pub events: Arc<dyn RecordingEvents>,
    /// The folder every recording's folder lives in (under the app data dir).
    pub recordings: PathBuf,
}
