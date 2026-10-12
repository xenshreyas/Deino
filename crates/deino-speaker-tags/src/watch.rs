//! [`Watch`]: one recording's speaker tags, one poll at a time.
//!
//! Each [`tick`](Watch::tick) asks the recording clock where the recording
//! is, then looks at the meeting apps: the app that held the call at the
//! last poll first, then Zoom, Teams and Meet. The first call window an
//! adapter reads decides who speaks. Every failure is a [`TagError`],
//! logged at debug level when it first appears (never per poll) and never
//! raised: the poll simply names nobody, and voice matching names that
//! stretch later. Finished spans are saved every
//! [`FLUSH_EVERY`] of recording time and when the recording ends.

use std::sync::Arc;

use deino_domain::{CallApp, MeetingId, Millis, TagSpan};

use crate::adapter::{SpeakerAdapter, adapter_for};
use crate::error::TagError;
use crate::ports::{Pid, Platform, Position, RecordingClock, TagSink};
use crate::reading::{ReadError, Reading};
use crate::status::LayoutHealth;
use crate::timeline::{Sighting, Timeline};

/// How much recording time passes between saves of finished spans.
pub const FLUSH_EVERY: Millis = Millis(10_000);

/// The log target.
const TARGET: &str = "deino_speaker_tags";

/// What a watch runs on.
#[derive(Clone)]
pub struct WatchParts {
    /// The meeting being recorded.
    pub meeting: MeetingId,
    /// Accessibility, running apps and windows.
    pub platform: Platform,
    /// The recording's clock.
    pub clock: Arc<dyn RecordingClock>,
    /// Where finished spans go.
    pub sink: Arc<dyn TagSink>,
    /// Shared with Settings: whether each app's window was recognised.
    pub health: Arc<LayoutHealth>,
}

/// Whether to keep polling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Poll again after the interval.
    Continue,
    /// The recording ended; everything is saved.
    Done,
}

/// One recording's watch (see the module docs).
pub struct Watch {
    parts: WatchParts,
    timeline: Timeline,
    unsaved: Vec<TagSpan>,
    flushed_at: Millis,
    holder: Option<CallApp>,
    prepared: Vec<Pid>,
    degraded: Option<TagError>,
}

impl Watch {
    /// A watch that has not polled yet.
    pub fn new(parts: WatchParts) -> Self {
        Self {
            parts,
            timeline: Timeline::default(),
            unsaved: Vec::new(),
            flushed_at: Millis(0),
            holder: None,
            prepared: Vec::new(),
            degraded: None,
        }
    }

    /// Polls once.
    pub fn tick(&mut self) -> Step {
        match self.parts.clock.position(&self.parts.meeting) {
            Position::Over => {
                self.finish();
                Step::Done
            }
            Position::Paused => {
                self.timeline.interrupt();
                Step::Continue
            }
            Position::At(at) => {
                let sighting = self.look();
                self.timeline.observe(at, sighting);
                if at.0.saturating_sub(self.flushed_at.0) >= FLUSH_EVERY.0 {
                    self.flush(at);
                }
                Step::Continue
            }
        }
    }

    /// Closes the open span and saves everything; the watch is over.
    pub fn finish(&mut self) {
        self.timeline.interrupt();
        self.flush(self.flushed_at);
    }

    /// Who is speaking, if a remote participant is and a window says so.
    fn look(&mut self) -> Option<Sighting> {
        if !self.parts.platform.permission.granted() {
            self.degrade(TagError::PermissionDenied);
            return None;
        }
        match self.sighting() {
            Ok(sighting) => {
                self.recover();
                sighting
            }
            Err(error) => {
                self.degrade(error);
                None
            }
        }
    }

    /// The first call window's speaker, the last holder's app first.
    fn sighting(&mut self) -> Result<Option<Sighting>, TagError> {
        let mut failure = None;
        for app in self.order() {
            let adapter = adapter_for(app);
            for pid in self.parts.platform.apps.pids(adapter.bundle_ids()) {
                match self.read(adapter, pid) {
                    Ok(Some(reading)) => {
                        self.holder = Some(app);
                        return Ok(reading.active.remote().map(|name| (app, name.clone())));
                    }
                    Ok(None) => {}
                    Err(error) => failure = Some(error),
                }
            }
        }
        self.holder = None;
        failure.map_or(Ok(None), Err)
    }

    /// The apps in the order to look at them.
    fn order(&self) -> Vec<CallApp> {
        let mut order: Vec<CallApp> = self.holder.into_iter().collect();
        order.extend(CallApp::ALL.iter().filter(|app| Some(**app) != self.holder));
        order
    }

    /// The reading of `pid`'s call window, if it has one.
    fn read(
        &mut self,
        adapter: &dyn SpeakerAdapter,
        pid: Pid,
    ) -> Result<Option<Reading>, TagError> {
        let app = adapter.app();
        if !self.prepared.contains(&pid) {
            self.parts.platform.windows.prepare(app, pid);
            self.prepared.push(pid);
        }
        let pick = |window: &_| adapter.is_call_window(window);
        let windows = self.parts.platform.windows.windows(pid, &pick);
        let windows = windows.map_err(|source| TagError::Unreadable { app, source })?;
        for window in &windows {
            match adapter.read(window) {
                Ok(reading) => {
                    self.parts.health.record(app, true);
                    return Ok(Some(reading));
                }
                Err(ReadError::NotInCall) => {}
                Err(source @ ReadError::UnrecognisedLayout) => {
                    self.parts.health.record(app, false);
                    return Err(TagError::Layout { app, source });
                }
            }
        }
        Ok(None)
    }

    /// Notes a failure, logging it when it is new.
    fn degrade(&mut self, error: TagError) {
        if self.degraded != Some(error) {
            log::debug!(
                target: TARGET,
                "meeting {}: {error}; speakers fall back to voice matching",
                self.parts.meeting
            );
        }
        self.degraded = Some(error);
    }

    /// Notes that a poll worked again.
    fn recover(&mut self) {
        if self.degraded.take().is_some() {
            log::debug!(target: TARGET, "meeting {}: speaker tags read again", self.parts.meeting);
        }
    }

    /// Saves the finished spans; keeps them for the next try on failure.
    fn flush(&mut self, at: Millis) {
        self.flushed_at = at;
        self.unsaved.extend(self.timeline.take_closed());
        if self.unsaved.is_empty() {
            return;
        }
        match self.parts.sink.save(&self.parts.meeting, &self.unsaved) {
            Ok(()) => self.unsaved.clear(),
            Err(error) => log::warn!(
                target: TARGET,
                "meeting {}: {} speaker tags kept for the next save: {error}",
                self.parts.meeting,
                self.unsaved.len()
            ),
        }
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
