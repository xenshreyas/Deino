//! One recording's parts, started and stopped in a safe order.
//!
//! Starting ([`prepare`]) opens what can fail before anything is stored:
//! the transcript router, both lanes' engines (a missing model fails here;
//! a record-only recording opens none, CAP-08), the track writers and the
//! capture. Audio is already being recorded when
//! the service then creates the meeting row, and only after that does
//! [`Prepared::go`] start the live pipeline, so no segment is saved for a
//! meeting that does not exist. Every step here blocks; the service runs it
//! off the async runtime.

mod running;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use deino_audio::capture::{Capture, CaptureInputs, CaptureParts, OutputRoute};
use deino_audio::{HealthEvent, PlacedAudio, Recorder};
use deino_core::vocabulary::Corrector;
use deino_domain::{MeetingId, SpeakerId};
pub(crate) use running::Session;
use tokio::runtime::Handle;

use crate::bleed::HoldbackPolicy;
use crate::deps::{Preferences, RecordingDeps};
use crate::engine::LaneTranscriber;
use crate::error::RecordingError;
use crate::events::RecordingEvents;
use crate::lane::{Lane, PerLane};
use crate::transcript::vocabulary::{CorrectedEvents, CorrectedStore, CorrectedStoreParts};
use crate::transcript::{Router, RouterParts, SegmentStore, WriterStore};

/// What a recording will be.
#[derive(Clone, Debug)]
pub(crate) struct Plan {
    pub meeting: MeetingId,
    pub speakers: PerLane<Option<SpeakerId>>,
    pub dir: PathBuf,
    pub preferences: Preferences,
    pub runtime: Handle,
}

/// A recording that captures audio but does not transcribe yet.
pub(crate) struct Prepared {
    plan: Plan,
    router: Router,
    /// Each lane's engine; `None` when the recording is record-only.
    transcribers: Option<PerLane<Box<dyn LaneTranscriber>>>,
    recorder: Recorder,
    capture: Capture,
    placed: PlacedAudio,
    health: Receiver<HealthEvent>,
    route: Option<OutputRoute>,
    routes: Receiver<OutputRoute>,
}

/// Opens the router, the engines, the writers and the capture, in that order.
pub(crate) fn prepare(deps: &RecordingDeps, plan: Plan) -> Result<Prepared, RecordingError> {
    let router = start_router(deps, &plan)?;
    let transcribers = if plan.preferences.record_only {
        None
    } else {
        Some(open_lanes(deps, &router)?)
    };
    let (recorder, inputs) = Recorder::start(&plan.dir, &deps.recorder)?;
    let (route_changes, route_updates) = channel();
    let route_probe = deps.capture.route();
    let parts = CaptureParts {
        factory: deps.capture.sources(&plan.preferences.microphone),
        watch: deps.capture.watch(),
        route: Arc::clone(&route_probe),
        routes: route_changes,
        host_clock: Arc::clone(&deps.recorder.host_clock),
        wall_clock: Arc::clone(&deps.recorder.wall_clock),
        health: inputs.reporter,
    };
    let rings = CaptureInputs {
        microphone: inputs.microphone,
        system: inputs.system,
    };
    let capture = match Capture::start(rings, &parts) {
        Ok(capture) => capture,
        Err(error) => {
            discard_recorder(recorder, &plan.dir);
            return Err(error.into());
        }
    };
    Ok(Prepared {
        route: route_probe.route(),
        plan,
        router,
        transcribers,
        recorder,
        capture,
        placed: inputs.placed,
        health: inputs.health,
        routes: route_updates,
    })
}

impl Prepared {
    /// Gives up before the meeting exists: stops everything and deletes the
    /// recording folder.
    pub fn abort(self) {
        self.capture.stop();
        discard_recorder(self.recorder, &self.plan.dir);
        if let Some(PerLane { me, them }) = self.transcribers {
            me.finish();
            them.finish();
        }
        self.router.finish();
    }
}

/// Starts the router, saving and showing segments corrected by the
/// vocabulary (TRN-05).
fn start_router(deps: &RecordingDeps, plan: &Plan) -> Result<Router, RecordingError> {
    let corrector = Arc::new(live_corrector(deps, plan));
    let store = WriterStore::new(Arc::clone(&deps.segments), plan.runtime.clone());
    let store: Arc<dyn SegmentStore> = Arc::new(CorrectedStore::new(CorrectedStoreParts {
        inner: Arc::new(store),
        corrector: Arc::clone(&corrector),
        vocabulary: Arc::clone(&deps.vocabulary),
        runtime: plan.runtime.clone(),
    }));
    let events: Arc<dyn RecordingEvents> =
        Arc::new(CorrectedEvents::new(Arc::clone(&deps.events), corrector));
    Ok(Router::start(RouterParts {
        meeting: plan.meeting.clone(),
        speakers: plan.speakers,
        store,
        events,
        clock: Arc::clone(&deps.recorder.wall_clock),
        host_clock: Arc::clone(&deps.recorder.host_clock),
        policy: HoldbackPolicy::default(),
    })?)
}

/// The vocabulary's corrector for the live segments: none when nothing is
/// transcribed live, or when the vocabulary cannot be read (the recording
/// matters more than its spelling, and `vocab_fixups` corrects afterwards).
fn live_corrector(deps: &RecordingDeps, plan: &Plan) -> Corrector {
    if plan.preferences.record_only {
        return Corrector::default();
    }
    plan.runtime
        .block_on(deps.vocabulary.corrector())
        .unwrap_or_else(|error| {
            log::warn!(target: "recording", "{error}; live segments are saved as heard");
            Corrector::default()
        })
}

/// Opens both lanes' engines at once (each may load a model).
fn open_lanes(
    deps: &RecordingDeps,
    router: &Router,
) -> Result<PerLane<Box<dyn LaneTranscriber>>, RecordingError> {
    let open = |lane: Lane| deps.engines.open(lane, router.handle().sink(lane));
    let (me, them) = std::thread::scope(|scope| {
        let me = scope.spawn(|| open(Lane::Me));
        let them = open(Lane::Them);
        (me.join(), them)
    });
    let me = me.map_err(|_| std::io::Error::other("the Me engine panicked while opening"))?;
    Ok(PerLane {
        me: me?,
        them: them?,
    })
}

/// Stops the writers and deletes a recording that never became a meeting.
fn discard_recorder(recorder: Recorder, dir: &std::path::Path) {
    if let Err(error) = recorder.stop() {
        log::warn!(target: "recording", "closing an abandoned recording failed: {error}");
    }
    if let Err(error) = std::fs::remove_dir_all(dir) {
        log::warn!(target: "recording", "could not delete an abandoned recording folder: {error}");
    }
}
