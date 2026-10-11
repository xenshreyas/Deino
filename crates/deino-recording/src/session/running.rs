//! A recording in progress: pause, resume, device switches, what the
//! watchers read, and the ordered stop.

use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use deino_audio::capture::{Capture, MicChoice, OutputRoute, TrackStates};
use deino_audio::{Frames, HealthEvent, HostClock, PlacedAudio, Recorder, Track};
use deino_domain::{MeetingId, Millis};

use super::{Prepared, lane_start};
use crate::activity::{LaneReading, LiveActivity};
use crate::deps::RecordingDeps;
use crate::echo::SonoraAec;
use crate::engine::LaneTranscriber;
use crate::error::RecordingError;
use crate::events::{AudioLevels, HealthNotice, RecordingEvents};
use crate::lane::{Lane, PerLane};
use crate::live::{LaneInput, LiveParts, LivePipeline, PlacedReader};
use crate::monitor::watch::{WatchParts, Watcher};
use crate::state::StateCell;
use crate::transcript::{Router, RouterHandle};

/// A running recording.
pub(crate) struct Session {
    meeting: MeetingId,
    start: Frames,
    recorder: Recorder,
    capture: Capture,
    tracks: Arc<TrackStates>,
    activity: Arc<LiveActivity>,
    /// The live transcription; `None` when the recording is record-only.
    pipeline: Option<LivePipeline>,
    router: Router,
    watcher: Option<Watcher>,
    health: Option<JoinHandle<()>>,
}

impl Prepared {
    /// Starts transcribing (the meeting row exists now), forwarding health
    /// events and watching the recording's health. A record-only recording
    /// starts no live pipeline: its placed audio is left unread, which costs
    /// nothing (CAP-08).
    pub fn go(self, deps: &RecordingDeps, state: StateCell) -> Result<Session, RecordingError> {
        let watcher = self.watch(deps, state)?;
        let start = self.plan.take.start();
        let tracks = self.capture.tracks();
        let pipeline = match self.transcribers {
            Some(transcribers) => Some(live_pipeline(LiveStart {
                placed: self.placed,
                transcribers,
                router: self.router.handle(),
                uses_headphones: self.plan.preferences.uses_headphones,
                route: self.route,
                routes: self.routes,
                start,
                tracks: Arc::clone(&tracks),
            })?),
            None => None,
        };
        let health = forward_health(
            self.plan.meeting.clone(),
            self.health,
            Arc::clone(&deps.events),
        )?;
        Ok(Session {
            meeting: self.plan.meeting,
            start,
            recorder: self.recorder,
            capture: self.capture,
            tracks,
            activity: self.activity,
            pipeline,
            router: self.router,
            watcher: Some(watcher),
            health: Some(health),
        })
    }
}

impl Prepared {
    /// Starts the health watcher over this take's tracks, lanes and folder.
    fn watch(&self, deps: &RecordingDeps, state: StateCell) -> std::io::Result<Watcher> {
        Watcher::start(WatchParts {
            meeting: self.plan.meeting.clone(),
            tracks: self.capture.tracks(),
            activity: Arc::clone(&self.activity),
            engines: PerLane {
                me: self
                    .transcribers
                    .as_ref()
                    .and_then(|lanes| lanes.me.status()),
                them: self
                    .transcribers
                    .as_ref()
                    .and_then(|lanes| lanes.them.status()),
            },
            disk: Arc::clone(&deps.recorder.disk),
            dir: self.plan.dir.clone(),
            clock: Arc::clone(&deps.recorder.host_clock),
            state,
        })
    }
}

impl Session {
    /// The meeting being recorded.
    pub fn meeting(&self) -> &MeetingId {
        &self.meeting
    }

    /// How far the recording is on its own timeline now: where this take
    /// starts (a resumed meeting's earlier length), plus host time since the
    /// take's epoch, minus the pauses (which the timeline skips). Segment
    /// times are measured on the same timeline.
    pub fn position(&self, host_clock: &dyn HostClock) -> Millis {
        let elapsed = host_clock.now().since(self.recorder.epoch());
        let at = self.start.as_duration() + elapsed.saturating_sub(self.capture.paused());
        Millis(u32::try_from(at.as_millis()).unwrap_or(u32::MAX))
    }

    /// Stops the sources; the timeline skips the pause.
    pub fn pause(&self) {
        self.capture.pause();
    }

    /// Restarts the sources.
    pub fn resume(&self) {
        self.capture.resume();
    }

    /// Records from the microphone `choice` names from now on.
    pub fn set_microphone(&self, choice: MicChoice) {
        self.capture.set_microphone(choice);
    }

    /// Turns system audio on or off.
    pub fn set_system_audio(&self, enabled: bool) {
        self.capture.set_system_audio(enabled);
    }

    /// Each track's loudest sample since the last call (one reader).
    pub fn levels(&self) -> AudioLevels {
        let level = |track: Track| self.tracks.get(track).meter().take_level();
        AudioLevels {
            microphone: level(Track::Microphone),
            system: level(Track::System),
        }
    }

    /// What each lane has heard so far.
    pub fn activity(&self) -> PerLane<LaneReading> {
        PerLane {
            me: self.activity.read(Lane::Me),
            them: self.activity.read(Lane::Them),
        }
    }

    /// Stops capture, closes both tracks at the recording's end, finishes
    /// both lanes and saves the last segments, in that order. Returns the
    /// recording's whole length (pauses excluded, earlier takes included).
    pub fn stop(mut self, host_clock: &dyn HostClock) -> Result<Frames, RecordingError> {
        if let Some(watcher) = self.watcher.take() {
            watcher.stop();
        }
        let paused = self.capture.paused();
        self.capture.stop();
        let end = host_clock.now();
        let end = deino_audio::HostTime(
            end.0
                .saturating_sub(u64::try_from(paused.as_nanos()).unwrap_or(u64::MAX)),
        );
        let summary = self.recorder.stop_at(end);
        let take = summary
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
        let length = self.start + take;
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
    /// Where this take starts on the timeline (a resumed meeting's length).
    start: Frames,
    tracks: Arc<TrackStates>,
}

/// Starts the live pipeline: both tracks' placed audio into both lanes.
fn live_pipeline(start: LiveStart) -> Result<LivePipeline, RecordingError> {
    let readers = readers(start.placed, start.start)?;
    let PerLane { me, them } = start.transcribers;
    let at = lane_start(start.start);
    Ok(LivePipeline::start(LiveParts {
        readers,
        lanes: PerLane {
            me: LaneInput::new(Lane::Me, me, start.router.clone(), at),
            them: LaneInput::new(Lane::Them, them, start.router.clone(), at),
        },
        echo: Box::new(SonoraAec::new()),
        uses_headphones: start.uses_headphones,
        route: start.route,
        routes: start.routes,
        router: start.router,
        tracks: start.tracks,
    })?)
}

/// Readers of both tracks' placed audio from `start` on.
fn readers(placed: PlacedAudio, start: Frames) -> std::io::Result<PerLane<PlacedReader>> {
    let reader = |placed| PlacedReader::starting_at(placed, start).map_err(std::io::Error::other);
    Ok(PerLane {
        me: reader(placed.microphone)?,
        them: reader(placed.system)?,
    })
}

/// Forwards the recording's health events to the UI until the recording's
/// writers and capture are gone.
fn forward_health(
    meeting: MeetingId,
    health: Receiver<HealthEvent>,
    events: Arc<dyn RecordingEvents>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("deino-recording-health-events".into())
        .spawn(move || {
            for event in health {
                log::info!(target: "recording", "health: {event:?}");
                if let Some(notice) = HealthNotice::from_event(&meeting, event) {
                    events.health(&notice);
                }
            }
        })
}
