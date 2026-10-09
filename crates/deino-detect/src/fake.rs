//! Fakes of every port, for tests here and in the app (`test-support`).
//!
//! Each fake is a cheap handle over shared state: clone it, hand one clone
//! to the code under test and drive or inspect it through the other.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use deino_domain::BundleId;

use crate::error::DetectError;
use crate::event::DetectionEvent;
use crate::model::{AppIdentity, AudioProcess, Pid};
use crate::outputs::{
    ConfigSource, DetectionConfig, DetectionSink, IgnoreList, RecordingRequest, RecordingStarter,
    RecordingStatus,
};
use crate::ports::{AppDirectory, BrowserTabs, RunningApp, Sensor, Sensors, Waker, Watch};

/// Shared, lockable test state.
#[derive(Debug, Default)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Shared<T> {
    /// State starting at `value`.
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    /// Locks the state.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Replaces the state.
    pub fn set(&self, value: T) {
        *self.lock() = value;
    }
}

impl<T: Clone> Shared<T> {
    /// A copy of the state.
    pub fn get(&self) -> T {
        self.lock().clone()
    }
}

/// A sensor whose reading the test sets; its watch counts refreshes.
#[derive(Clone, Debug, Default)]
pub struct FakeSensor<T> {
    /// The reading, or `None` to fail the read.
    pub reading: Shared<Option<T>>,
    /// The waker the detector registered, to simulate a notification.
    pub waker: Shared<Option<Waker>>,
    /// How often the detector refreshed the watch.
    pub refreshes: Shared<usize>,
    /// Fail to register a listener.
    pub unwatchable: bool,
}

impl<T: Clone + Send + 'static> Sensor<T> for FakeSensor<T> {
    fn read(&self) -> Result<T, DetectError> {
        self.reading
            .get()
            .ok_or(DetectError::os("a fake sensor", -1))
    }

    fn watch(&self, waker: &Waker) -> Result<Box<dyn Watch>, DetectError> {
        if self.unwatchable {
            return Err(DetectError::os("a fake listener", -1));
        }
        self.waker.set(Some(waker.clone()));
        Ok(Box::new(FakeWatch(self.refreshes.clone())))
    }
}

struct FakeWatch(Shared<usize>);

impl Watch for FakeWatch {
    fn refresh(&mut self) {
        *self.0.lock() += 1;
    }
}

/// Running apps the test declares.
#[derive(Clone, Debug, Default)]
pub struct FakeDirectory {
    /// The app each pid belongs to.
    pub pids: Shared<HashMap<Pid, AppIdentity>>,
    /// Running apps by bundle.
    pub running: Shared<Vec<RunningApp>>,
}

impl AppDirectory for FakeDirectory {
    fn app_for_pid(&self, pid: Pid) -> Option<AppIdentity> {
        self.pids.lock().get(&pid).cloned()
    }

    fn running_app(&self, bundle_id: &BundleId) -> Option<RunningApp> {
        self.running
            .lock()
            .iter()
            .find(|app| &app.app.bundle_id == bundle_id)
            .cloned()
    }

    fn frontmost(&self) -> Option<RunningApp> {
        self.running.lock().iter().find(|app| app.active).cloned()
    }
}

/// Browser windows the test declares, by browser pid.
#[derive(Clone, Debug, Default)]
pub struct FakeTabs {
    /// Accessibility is granted.
    pub available: Shared<bool>,
    /// Window addresses by browser pid.
    pub urls: Shared<HashMap<Pid, Vec<String>>>,
}

impl BrowserTabs for FakeTabs {
    fn available(&self) -> bool {
        self.available.get()
    }

    fn window_urls(&self, pid: Pid) -> Vec<String> {
        self.urls.lock().get(&pid).cloned().unwrap_or_default()
    }
}

/// A whole fake Mac: every sensor plus handles to drive them.
#[derive(Clone, Debug, Default)]
pub struct FakeMac {
    /// Core Audio processes.
    pub processes: FakeSensor<Vec<AudioProcess>>,
    /// The default input device.
    pub microphone: FakeSensor<bool>,
    /// Cameras.
    pub camera: FakeSensor<bool>,
    /// Running apps.
    pub apps: FakeDirectory,
    /// Browser windows.
    pub tabs: FakeTabs,
}

impl FakeMac {
    /// An idle Mac: nothing captures, nothing runs.
    pub fn idle() -> Self {
        let mac = Self::default();
        mac.processes.reading.set(Some(Vec::new()));
        mac.microphone.reading.set(Some(false));
        mac.camera.reading.set(Some(false));
        mac
    }

    /// Sensors over this Mac; `own_pid` is Deino's process.
    pub fn sensors(&self, own_pid: Pid) -> Sensors {
        Sensors {
            processes: Box::new(self.processes.clone()),
            microphone: Box::new(self.microphone.clone()),
            camera: Box::new(self.camera.clone()),
            apps: Box::new(self.apps.clone()),
            tabs: Box::new(self.tabs.clone()),
            own_pid,
        }
    }
}

/// Records every published event.
#[derive(Clone, Debug, Default)]
pub struct RecordedSink(pub Shared<Vec<DetectionEvent>>);

impl DetectionSink for RecordedSink {
    fn publish(&self, event: DetectionEvent) {
        self.0.lock().push(event);
    }
}

/// Records recording requests, and can report that Deino is recording.
#[derive(Clone, Debug, Default)]
pub struct FakeRecorder {
    /// Every request, oldest first.
    pub requests: Shared<Vec<RecordingRequest>>,
    /// What [`RecordingStatus::is_recording`] answers.
    pub recording: Shared<bool>,
}

impl RecordingStarter for FakeRecorder {
    fn start(&self, request: &RecordingRequest) -> Result<(), DetectError> {
        self.requests.lock().push(request.clone());
        Ok(())
    }
}

impl RecordingStatus for FakeRecorder {
    fn is_recording(&self) -> bool {
        self.recording.get()
    }
}

/// Settings the test controls; "Always ignore" writes into them.
#[derive(Clone, Debug, Default)]
pub struct FakeSettings(pub Shared<DetectionConfig>);

impl ConfigSource for FakeSettings {
    fn config(&self) -> DetectionConfig {
        self.0.get()
    }
}

#[async_trait]
impl IgnoreList for FakeSettings {
    async fn ignore(&self, app: &AppIdentity) -> Result<(), DetectError> {
        self.0.lock().ignored.insert(app.bundle_id.clone());
        Ok(())
    }
}
