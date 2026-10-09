//! The ports detection reads the Mac through. The macOS adapters live in
//! `macos/`; tests use the fakes behind `test-support`.
//!
//! Every signal is a [`Sensor`]: a current reading plus a [`Watch`] that
//! wakes the detector when the reading may have changed. OS callbacks only
//! call [`Waker::wake`]; the detector thread then re-reads everything, so no
//! state is shared with a callback (the OpenWhispr listener model).

use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

use deino_domain::BundleId;

use crate::error::DetectError;
use crate::model::{AppIdentity, Pid};

/// A signal the detector reads: the audio process list, whether the
/// default input device runs, whether a camera runs.
pub trait Sensor<T>: Send {
    /// The current reading.
    fn read(&self) -> Result<T, DetectError>;
    /// Starts change notifications that call `waker`; dropping the returned
    /// [`Watch`] stops them.
    fn watch(&self, waker: &Waker) -> Result<Box<dyn Watch>, DetectError>;
}

/// A live subscription to OS change notifications. Dropping it unsubscribes.
///
/// Created, refreshed and dropped on the detector thread only.
pub trait Watch {
    /// Re-attaches per-object listeners after a change: new audio process
    /// objects, a plugged-in camera, a new default input device.
    fn refresh(&mut self);
}

/// Wakes the detector thread. Cheap, never blocks, safe from any thread:
/// wakes that arrive while one is pending merge into it.
#[derive(Clone, Debug)]
pub struct Waker(SyncSender<()>);

impl Waker {
    /// A waker and the receiver the detector waits on.
    pub fn channel() -> (Self, Receiver<()>) {
        let (sender, receiver) = sync_channel(1);
        (Self(sender), receiver)
    }

    /// Asks the detector to re-read its sensors.
    pub fn wake(&self) {
        // `Full` means a wake is already pending, which covers this one.
        if let Err(TrySendError::Disconnected(())) = self.0.try_send(()) {
            log::trace!(target: "detect", "wake after the detector stopped");
        }
    }
}

/// A running app, as Launch Services knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningApp {
    /// The app.
    pub app: AppIdentity,
    /// Its main process.
    pub pid: Pid,
    /// It is the frontmost app.
    pub active: bool,
}

/// The apps that are running (`NSWorkspace`, `NSRunningApplication`).
pub trait AppDirectory: Send {
    /// The app `pid` belongs to: the outermost `.app` around its executable,
    /// or a `process.<name>` identity outside any bundle. `None` once the
    /// process has exited.
    fn app_for_pid(&self, pid: Pid) -> Option<AppIdentity>;
    /// The running app with `bundle_id`, if there is one.
    fn running_app(&self, bundle_id: &BundleId) -> Option<RunningApp>;
    /// The frontmost app.
    fn frontmost(&self) -> Option<RunningApp>;
}

/// The page addresses of a browser's windows, read through Accessibility
/// (`kAXDocumentAttribute`). Never logged: a URL can be private.
pub trait BrowserTabs: Send {
    /// Whether addresses can be read: the user granted Accessibility.
    fn available(&self) -> bool;
    /// The addresses of the windows of the browser whose main process is
    /// `pid`, focused window first. Empty when none can be read.
    fn window_urls(&self, pid: Pid) -> Vec<String>;
}

/// Every OS port the detector reads, for one platform.
pub struct Sensors {
    /// Core Audio process objects: who captures and who plays audio.
    pub processes: Box<dyn Sensor<Vec<crate::model::AudioProcess>>>,
    /// The default input device is running somewhere.
    pub microphone: Box<dyn Sensor<bool>>,
    /// Any camera is running somewhere.
    pub camera: Box<dyn Sensor<bool>>,
    /// Running apps.
    pub apps: Box<dyn AppDirectory>,
    /// Browser window addresses.
    pub tabs: Box<dyn BrowserTabs>,
    /// This app's own process, never a meeting.
    pub own_pid: Pid,
}
