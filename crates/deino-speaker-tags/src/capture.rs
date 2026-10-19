//! [`WindowCapture`]: an opt-in diagnostic for when a meeting app's call
//! window cannot be read. The watch saves one full Accessibility dump of
//! that app's windows, so the owner's real Zoom, Teams or Meet window can
//! become a fixture (`fixtures/README.md`) and the adapter can be fixed.
//!
//! A dump is taken at most once per meeting and app ([`Captures`]), on the
//! watcher's thread, with one full read per process (every attribute, as
//! the capture tool reads), at the first of:
//! - a poll where the app's call window has a layout its adapter does not
//!   know ([`Captures::unrecognised`]);
//! - the first poll [`CAPTURE_AFTER`] into the recording at which the app
//!   runs and no call window has been found yet ([`Captures::no_call_yet`]).
//!
//! Only windows that may hold a call are read for Meet (a browser's other
//! pages are never dumped); Zoom and Teams give every window, since their
//! call window is the one that was not recognised. The port decides whether
//! a dump is wanted (the user's setting) and where it goes; nothing read
//! here is logged.

use deino_domain::{CallApp, MeetingId, Millis};

use crate::adapter::adapter_for;
use crate::tree::{AxNode, WindowSummary};
use crate::watch::WatchParts;

/// How far into a recording a running app without a call window is dumped.
pub const CAPTURE_AFTER: Millis = Millis(20_000);

/// After this many failed reads in one meeting, no more dumps are tried.
const MAX_FAILED_READS: usize = 3;

/// The log target.
const TARGET: &str = "deino_speaker_tags";

/// Where call-window dumps go. The default does nothing: no dump is wanted.
pub trait WindowCapture: Send + Sync {
    /// Whether a dump is wanted now. Asked only when one would be taken.
    fn wanted(&self) -> bool {
        false
    }

    /// Keeps `windows`, the full trees of `app`'s windows while `meeting`
    /// records. They hold participants' names: never log them.
    fn save(&self, _meeting: &MeetingId, _app: CallApp, _windows: &[AxNode]) {}
}

/// No dumps: the watch's port when nobody asked for them.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoCapture;

impl WindowCapture for NoCapture {}

/// One meeting's dumps: which apps were saved or looked at.
#[derive(Debug, Default)]
pub(crate) struct Captures {
    /// Apps whose windows were saved.
    saved: Vec<CallApp>,
    /// Apps already looked at for the no-call-window dump.
    looked: Vec<CallApp>,
    /// Reads that failed so far.
    failed: usize,
}

impl Captures {
    /// After `app`'s call window was not recognised: dumps it once.
    pub(crate) fn unrecognised(&mut self, parts: &WatchParts, app: CallApp) {
        if self.may_save(app) && parts.capture.wanted() {
            self.take(parts, app);
        }
    }

    /// After a poll at `at` that has found no call window in this meeting
    /// yet: from [`CAPTURE_AFTER`] on, dumps each running app once.
    pub(crate) fn no_call_yet(&mut self, parts: &WatchParts, at: Millis) {
        if at.0 < CAPTURE_AFTER.0 || !parts.platform.permission.granted() {
            return;
        }
        for app in CallApp::ALL {
            if !self.may_save(*app) || self.looked.contains(app) {
                continue;
            }
            let running = !parts
                .platform
                .apps
                .pids(adapter_for(*app).bundle_ids())
                .is_empty();
            if running {
                self.looked.push(*app);
                if parts.capture.wanted() {
                    self.take(parts, *app);
                }
            }
        }
    }

    /// Whether `app` may still be dumped in this meeting.
    fn may_save(&self, app: CallApp) -> bool {
        !self.saved.contains(&app) && self.failed < MAX_FAILED_READS
    }

    /// Reads `app`'s windows in full, once per process, and saves them.
    fn take(&mut self, parts: &WatchParts, app: CallApp) {
        let adapter = adapter_for(app);
        let pick = |window: &WindowSummary| app != CallApp::Meet || adapter.is_call_window(window);
        let mut windows = Vec::new();
        for pid in parts.platform.apps.pids(adapter.bundle_ids()) {
            match parts.platform.windows.full_windows(pid, &pick) {
                Ok(read) => windows.extend(read),
                Err(error) => {
                    self.failed += 1;
                    let meeting = &parts.meeting;
                    log::debug!(target: TARGET, "meeting {meeting}: the {app} windows could not be saved: {error}");
                    return;
                }
            }
        }
        if !windows.is_empty() {
            parts.capture.save(&parts.meeting, app, &windows);
            self.saved.push(app);
        }
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
