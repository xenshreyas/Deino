//! A recording in progress: pause, resume, and the ordered stop.

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use deino_audio::capture::Capture;
use deino_audio::{Frames, HealthEvent, HostClock, Recorder};
use deino_domain::MeetingId;

use super::Prepared;
use crate::echo::SonoraAec;
use crate::error::RecordingError;
use crate::events::{HealthNotice, RecordingEvents};
use crate::lane::{Lane, PerLane};
use crate::live::{LaneInput, LiveParts, LivePipeline, PlacedReader};
use crate::transcript::Router;

/// A running recording.
pub(crate) struct Session {
    meeting: MeetingId,
    recorder: Recorder,
    capture: Capture,
    pipeline: LivePipeline,
    router: Router,
    health: Option<JoinHandle<()>>,
}

impl Prepared {
    /// Starts transcribing (the meeting row exists now) and forwarding health.
    pub fn go(self, events: Arc<dyn RecordingEvents>) -> Result<Session, RecordingError> {
        let handle = self.router.handle();
        let readers = PerLane {
            me: PlacedReader::new(self.placed.microphone).map_err(std::io::Error::other)?,
            them: PlacedReader::new(self.placed.system).map_err(std::io::Error::other)?,
        };
        let PerLane { me, them } = self.transcribers;
        let pipeline = LivePipeline::start(LiveParts {
            readers,
            lanes: PerLane {
                me: LaneInput::new(Lane::Me, me, handle.clone()),
                them: LaneInput::new(Lane::Them, them, handle.clone()),
            },
            echo: Box::new(SonoraAec::new()),
            uses_headphones: self.plan.preferences.uses_headphones,
            route: self.route,
            routes: self.routes,
            router: handle,
        })?;
        let health = forward_health(self.plan.meeting.clone(), self.health, events)?;
        Ok(Session {
            meeting: self.plan.meeting,
            recorder: self.recorder,
            capture: self.capture,
            pipeline,
            router: self.router,
            health: Some(health),
        })
    }
}

impl Session {
    /// The meeting being recorded.
    pub fn meeting(&self) -> &MeetingId {
        &self.meeting
    }

    /// Stops the sources; the timeline skips the pause.
    pub fn pause(&self) {
        self.capture.pause();
    }

    /// Restarts the sources.
    pub fn resume(&self) {
        self.capture.resume();
    }

    /// Stops capture, closes both tracks at the recording's end, finishes
    /// both lanes and saves the last segments, in that order. Returns the
    /// recorded length (pauses excluded).
    pub fn stop(mut self, host_clock: &dyn HostClock) -> Result<Frames, RecordingError> {
        let paused = self.capture.paused();
        self.capture.stop();
        let end = host_clock.now();
        let end = deino_audio::HostTime(
            end.0
                .saturating_sub(u64::try_from(paused.as_nanos()).unwrap_or(u64::MAX)),
        );
        let summary = self.recorder.stop_at(end);
        let length = summary
            .as_ref()
            .map(|summary| {
                summary
                    .tracks
                    .iter()
                    .map(|(_, report)| report.frames)
                    .max()
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        self.pipeline.finish(length);
        self.router.finish();
        if let Some(health) = self.health.take() {
            health.join().ok();
        }
        summary?;
        Ok(length)
    }
}

/// Forwards the recording's health events to the UI until the recording's
/// writers and capture are gone.
fn forward_health(
    meeting: MeetingId,
    health: Receiver<HealthEvent>,
    events: Arc<dyn RecordingEvents>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("deino-recording-health".into())
        .spawn(move || {
            for event in health {
                log::info!(target: "recording", "health: {event:?}");
                if let Some(notice) = HealthNotice::from_event(&meeting, event) {
                    events.health(&notice);
                }
            }
        })
}
