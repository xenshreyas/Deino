//! The two capture backends: the Mac's devices, and mock audio.
//!
//! [`DeviceCapture`] is what the app records with: the cpal microphone, the
//! Core Audio process tap, Core Audio and `NSWorkspace` listeners and the
//! output-route probe (macOS; other platforms get the microphone only).
//! [`MockCapture`] plays two signals instead (generated voices or a fixture
//! meeting's tracks), selected by `DEINO_MOCK_AUDIO` in debug builds, so the
//! whole pipeline runs without hardware.

use std::sync::Arc;

use deino_audio::capture::{
    DeviceSources, FixedRoute, MicChoice, NoSystemWatch, OutputRoute, RouteProbe, SystemWatch,
};
use deino_audio::source::SourceFactory;
use deino_audio::{HostClock, MockSources, TestSignal};

use crate::deps::CaptureBackend;

/// The real devices.
#[derive(Debug)]
pub struct DeviceCapture {
    clock: Arc<dyn HostClock>,
}

impl DeviceCapture {
    /// Devices stamped on `clock` (the recorder's host clock).
    pub fn new(clock: Arc<dyn HostClock>) -> Self {
        Self { clock }
    }
}

impl CaptureBackend for DeviceCapture {
    fn sources(&self, microphone: &MicChoice) -> Arc<dyn SourceFactory> {
        Arc::new(DeviceSources::new(
            microphone.clone(),
            Arc::clone(&self.clock),
        ))
    }

    #[cfg(target_os = "macos")]
    fn watch(&self) -> Arc<dyn SystemWatch> {
        Arc::new(deino_audio::capture::macos::MacSystemWatch)
    }

    #[cfg(not(target_os = "macos"))]
    fn watch(&self) -> Arc<dyn SystemWatch> {
        Arc::new(NoSystemWatch)
    }

    #[cfg(target_os = "macos")]
    fn route(&self) -> Arc<dyn RouteProbe> {
        Arc::new(deino_audio::capture::macos::CoreAudioRoute)
    }

    #[cfg(not(target_os = "macos"))]
    fn route(&self) -> Arc<dyn RouteProbe> {
        Arc::new(FixedRoute(None))
    }
}

/// Mock audio: two signals played in real time, as if through speakers.
#[derive(Debug)]
pub struct MockCapture {
    sources: Arc<MockSources>,
}

impl MockCapture {
    /// Plays `microphone` and `system` (the system voice starts a moment
    /// late, like a tap attaching), stamped on `clock`.
    pub fn new(clock: Arc<dyn HostClock>, microphone: TestSignal, system: TestSignal) -> Self {
        Self {
            sources: Arc::new(MockSources::new(clock, microphone, system)),
        }
    }
}

impl CaptureBackend for MockCapture {
    fn sources(&self, _microphone: &MicChoice) -> Arc<dyn SourceFactory> {
        Arc::clone(&self.sources) as Arc<dyn SourceFactory>
    }

    fn watch(&self) -> Arc<dyn SystemWatch> {
        Arc::new(NoSystemWatch)
    }

    fn route(&self) -> Arc<dyn RouteProbe> {
        Arc::new(FixedRoute(Some(OutputRoute::Speakers)))
    }
}
