//! [`SignalReader`]: reads every sensor once and attributes what it finds,
//! producing the [`SignalSnapshot`] the resolver works on.
//!
//! - Processes whose own bundle is denied (Apple's `com.apple.CoreSpeech`,
//!   which "also processes ordinary playback") and Deino's own process are
//!   dropped before attribution.
//! - Processes are attributed to apps once per pid and cached; an app's
//!   capture and playback helpers merge, so a browser in a call is full duplex.
//! - Browser windows are read through Accessibility only while something
//!   captures audio, and only for capturing or frontmost browsers.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use deino_domain::{BundleId, Clock};

use crate::attribution::attribute;
use crate::calendar::CalendarLookup;
use crate::classify::{AppRole, role_of};
use crate::model::{AppIdentity, AudioProcess, MicClient, Pid};
use crate::ports::{RunningApp, Sensors};
use crate::snapshot::{BrowserMeeting, SignalSnapshot};
use crate::url::normalize;

/// Reads the sensors into snapshots. Lives on the detector thread.
pub struct SignalReader {
    calendar: Arc<dyn CalendarLookup>,
    clock: Arc<dyn Clock>,
    apps: HashMap<Pid, Option<AppIdentity>>,
}

impl SignalReader {
    /// A reader that links calendar events through `calendar`.
    pub fn new(calendar: Arc<dyn CalendarLookup>, clock: Arc<dyn Clock>) -> Self {
        Self {
            calendar,
            clock,
            apps: HashMap::new(),
        }
    }

    /// Everything the sensors report now. A sensor that fails reads as idle.
    pub fn read(&mut self, sensors: &Sensors) -> SignalSnapshot {
        let processes = read_or(sensors.processes.read(), "audio processes", Vec::new());
        let clients = self.clients(&processes, sensors);
        let microphone_running = read_or(sensors.microphone.read(), "the microphone", false);
        let frontmost = sensors.apps.frontmost();
        let capturing = microphone_running || !clients.is_empty();
        let browser_meetings = if capturing {
            rooms(sensors, &clients, frontmost.as_ref())
        } else {
            Vec::new()
        };
        SignalSnapshot {
            clients,
            microphone_running,
            camera_running: read_or(sensors.camera.read(), "the camera", false),
            frontmost: frontmost.map(|front| front.app),
            browser_meetings,
            event: self.calendar.linked_event(self.clock.now()),
        }
    }

    /// The apps capturing audio, each once, with full duplex when any of the
    /// app's processes also plays.
    fn clients(&mut self, processes: &[AudioProcess], sensors: &Sensors) -> Vec<MicClient> {
        self.apps
            .retain(|pid, _| processes.iter().any(|process| process.pid == *pid));
        let mut by_app: BTreeMap<BundleId, MicClient> = BTreeMap::new();
        let mut playing: Vec<BundleId> = Vec::new();
        for process in processes
            .iter()
            .filter(|process| counts(process, sensors.own_pid))
        {
            let Some(app) = self.app_of(process, sensors) else {
                continue;
            };
            if process.running_output {
                playing.push(app.bundle_id.clone());
            }
            if process.running_input {
                by_app.entry(app.bundle_id.clone()).or_insert(MicClient {
                    app,
                    pid: process.pid,
                    full_duplex: false,
                });
            }
        }
        by_app
            .into_values()
            .map(|client| MicClient {
                full_duplex: playing.contains(&client.app.bundle_id),
                ..client
            })
            .collect()
    }

    fn app_of(&mut self, process: &AudioProcess, sensors: &Sensors) -> Option<AppIdentity> {
        self.apps
            .entry(process.pid)
            .or_insert_with(|| attribute(process, sensors.apps.as_ref()))
            .clone()
    }
}

/// A process that may be part of a meeting: active, not Deino, and not a
/// system service that is never one.
fn counts(process: &AudioProcess, own_pid: Pid) -> bool {
    let active = process.running_input || process.running_output;
    let denied = process
        .bundle_id
        .as_ref()
        .is_some_and(|id| role_of(id) == AppRole::Denied);
    active && process.pid != own_pid && !denied
}

/// Meeting rooms in the capturing and the frontmost browsers.
fn rooms(
    sensors: &Sensors,
    clients: &[MicClient],
    frontmost: Option<&RunningApp>,
) -> Vec<BrowserMeeting> {
    if !sensors.tabs.available() {
        return Vec::new();
    }
    let capturing = clients
        .iter()
        .filter(|client| role_of(&client.app.bundle_id) == AppRole::Browser)
        .filter_map(|client| sensors.apps.running_app(&client.app.bundle_id));
    let front = frontmost
        .filter(|app| role_of(&app.app.bundle_id) == AppRole::Browser)
        .cloned();
    let mut seen = HashSet::new();
    capturing
        .chain(front)
        .filter(|browser| seen.insert(browser.app.bundle_id.clone()))
        .filter_map(|browser| room_in(sensors, browser))
        .collect()
}

/// The first meeting room among a browser's windows, focused window first.
fn room_in(sensors: &Sensors, browser: RunningApp) -> Option<BrowserMeeting> {
    let url = sensors
        .tabs
        .window_urls(browser.pid)
        .iter()
        .find_map(|raw| normalize(raw))?;
    Some(BrowserMeeting {
        focused: browser.active,
        browser: browser.app,
        url,
    })
}

fn read_or<T>(reading: Result<T, crate::DetectError>, what: &str, idle: T) -> T {
    reading.unwrap_or_else(|error| {
        log::debug!(target: "detect", "could not read {what}: {error}");
        idle
    })
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
