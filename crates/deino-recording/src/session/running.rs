//! A recording in progress: pause, resume, and the ordered stop.

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use deino_audio::capture::{Capture, OutputRoute};
use deino_audio::{Frames, HealthEvent, HostClock, PlacedAudio, Recorder};
use deino_domain::MeetingId;

use super::Prepared;
use crate::echo::SonoraAec;
use crate::engine::LaneTranscriber;
use crate::error::RecordingError;
use crate::events::{HealthNotice, RecordingEvents};
use crate::lane::{Lane, PerLane};
use crate::live::{LaneInput, LiveParts, LivePipeline, PlacedReader};
use crate::transcript::{Router, RouterHandle};

/// A running recording.
pub(crate) struct Session {
    meeting: MeetingId,
    recorder: Recorder,
    capture: Capture,
    /// The live transcription; `None` when the recording is record-only.
    pipeline: Option<LivePipeline>,
    router: Router,
    health: Option<JoinHandle<()>>,
}

impl Prepared {
    /// Starts transcribing (the meeting row exists now) and forwarding health.
    /// A record-only recording starts no live pipeline: its placed audio is
    /// left unread, which costs nothing (CAP-08).
    pub fn go(self, events: Arc<dyn RecordingEvents>) -> Result<Session, RecordingError> {
        let pipeline = match self.transcribers {
            Some(transcribers) => Some(live_pipeline(LiveStart {
                placed: self.placed,
                transcribers,
                router: self.router.handle(),
                uses_headphones: self.plan.preferences.uses_headphones,
                route: self.route,
                routes: self.routes,
            })?),
            None => None,
        };
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
        if let Some(pipeline) = self.pipeline.take() {
            pipeline.finish(length);
        }
        self.router.finish();
        if let Some(health) = self.health.take() {
            health.join().ok();
        }
        summary?;
        Ok(length)
    }
}

/// What the live pipeline starts from.
struct LiveStart {
    placed: PlacedAudio,
    transcribers: PerLane<Box<dyn LaneTranscriber>>,
    router: RouterHandle,
    uses_headphones: bool,
    route: Option<OutputRoute>,
    routes: Receiver<OutputRoute>,
}

/// Starts the live pipeline: both tracks' placed audio into both lanes.
fn live_pipeline(start: LiveStart) -> Result<LivePipeline, RecordingError> {
    let readers = PerLane {
        me: PlacedReader::new(start.placed.microphone).map_err(std::io::Error::other)?,
        them: PlacedReader::new(start.placed.system).map_err(std::io::Error::other)?,
    };
    let PerLane { me, them } = start.transcribers;
    Ok(LivePipeline::start(LiveParts {
        readers,
        lanes: PerLane {
            me: LaneInput::new(Lane::Me, me, start.router.clone()),
            them: LaneInput::new(Lane::Them, them, start.router.clone()),
        },
        echo: Box::new(SonoraAec::new()),
        uses_headphones: start.uses_headphones,
        route: start.route,
        routes: start.routes,
        router: start.router,
    })?)
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
