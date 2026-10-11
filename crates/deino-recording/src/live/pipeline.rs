//! `LivePipeline`: the thread between the track writers and the lanes.
//!
//! Every [`POLL`] it reads both tracks' placed audio ([`PlacedReader`]),
//! pairs their 16 kHz frames by timeline index, cancels the system audio's
//! echo out of the microphone frame (system track as the reference, in
//! 10 ms frames, [`crate::echo`]) and hands each lane its frame. A track
//! that falls more than [`MAX_LAG`] behind the other (a tap that has not
//! attached yet, a stalled device) is padded with silence, so the other
//! lane never waits for it.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_audio::Frames;
use deino_audio::capture::OutputRoute;

use super::lanes::LaneInput;
use super::placed::PlacedReader;
use crate::echo::{EchoCanceller, EchoMode, FRAME};
use crate::lane::{Lane, PerLane};
use crate::transcript::RouterHandle;

/// How often the pipeline reads the tracks.
pub const POLL: Duration = Duration::from_millis(10);

/// How far one track may run ahead of the other before the late one is
/// padded with silence (0.5 s of 48 kHz frames).
pub const MAX_LAG: Frames = Frames(24_000);

/// What the pipeline is built from.
pub struct LiveParts {
    /// Each track's placed audio.
    pub readers: PerLane<PlacedReader>,
    /// Each lane's input side.
    pub lanes: PerLane<LaneInput>,
    /// The echo canceller for the Me lane.
    pub echo: Box<dyn EchoCanceller>,
    /// The user's "I use headphones" setting.
    pub uses_headphones: bool,
    /// The output route at start, if known.
    pub route: Option<OutputRoute>,
    /// Later route changes from the capture supervisor.
    pub routes: Receiver<OutputRoute>,
    /// Where the echo mode is reported.
    pub router: RouterHandle,
}

enum Control {
    Finish { end: Frames, done: SyncSender<()> },
}

/// The running pipeline.
pub struct LivePipeline {
    control: Sender<Control>,
    thread: Option<JoinHandle<()>>,
}

impl LivePipeline {
    /// Starts the pipeline thread.
    pub fn start(parts: LiveParts) -> std::io::Result<Self> {
        let (control, controls) = channel();
        let mut worker = Worker::new(parts);
        let thread = std::thread::Builder::new()
            .name("deino-live-pipeline".into())
            .spawn(move || worker.run(&controls))?;
        Ok(Self {
            control,
            thread: Some(thread),
        })
    }

    /// Processes the tracks up to `end` (the recording's length), finishes
    /// both lanes and joins the thread. Blocks until the last segments are out.
    pub fn finish(mut self, end: Frames) {
        let (done, finished) = sync_channel(1);
        if self.control.send(Control::Finish { end, done }).is_ok() {
            finished.recv().ok();
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!(target: "recording", "the live pipeline panicked");
        }
    }
}

/// The pipeline thread's state.
struct Worker {
    readers: PerLane<PlacedReader>,
    lanes: Option<PerLane<LaneInput>>,
    echo: Box<dyn EchoCanceller>,
    uses_headphones: bool,
    mode: EchoMode,
    routes: Receiver<OutputRoute>,
    router: RouterHandle,
    frames: PerLane<[f32; FRAME]>,
}

impl Worker {
    fn new(parts: LiveParts) -> Self {
        let mode = EchoMode::choose(parts.uses_headphones, parts.route);
        parts.router.echo_mode(mode);
        Self {
            readers: parts.readers,
            lanes: Some(parts.lanes),
            echo: parts.echo,
            uses_headphones: parts.uses_headphones,
            mode,
            routes: parts.routes,
            router: parts.router,
            frames: PerLane {
                me: [0.0; FRAME],
                them: [0.0; FRAME],
            },
        }
    }

    fn run(&mut self, controls: &Receiver<Control>) {
        loop {
            match controls.recv_timeout(POLL) {
                Err(RecvTimeoutError::Timeout) => self.step(),
                Ok(Control::Finish { end, done }) => {
                    self.finish(end);
                    done.send(()).ok();
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Reads both tracks and processes every complete frame pair.
    fn step(&mut self) {
        self.follow_route();
        for lane in Lane::ALL {
            self.readers.get_mut(lane).drain();
        }
        self.keep_up();
        while self.take_pair() {
            self.process_pair();
        }
    }

    /// Switches echo handling when the output route changes.
    fn follow_route(&mut self) {
        let Some(route) = self.routes.try_iter().last() else {
            return;
        };
        let mode = EchoMode::choose(self.uses_headphones, Some(route));
        if mode != self.mode {
            log::info!(target: "recording", "echo handling is now {mode:?}");
            self.mode = mode;
            self.router.echo_mode(mode);
        }
    }

    /// Pads a track that fell more than [`MAX_LAG`] behind the other.
    fn keep_up(&mut self) {
        let (me, them) = (self.readers.me.position(), self.readers.them.position());
        if me > them + MAX_LAG {
            self.readers.them.pad_to(me - MAX_LAG);
        } else if them > me + MAX_LAG {
            self.readers.me.pad_to(them - MAX_LAG);
        }
    }

    fn take_pair(&mut self) -> bool {
        let ready = Lane::ALL
            .iter()
            .all(|lane| self.readers.get(*lane).available() >= FRAME);
        ready
            && self.readers.me.take(&mut self.frames.me)
            && self.readers.them.take(&mut self.frames.them)
    }

    /// Cancels echo out of the Me frame and feeds both lanes.
    fn process_pair(&mut self) {
        if self.mode == EchoMode::Cancel {
            self.echo.process(&self.frames.them, &mut self.frames.me);
        }
        if let Some(lanes) = self.lanes.as_mut() {
            lanes.me.push(&self.frames.me);
            lanes.them.push(&self.frames.them);
        }
    }

    /// Pads both tracks to `end`, processes what is left (the last frame
    /// padded with silence) and finishes both lanes.
    fn finish(&mut self, end: Frames) {
        for lane in Lane::ALL {
            let reader = self.readers.get_mut(lane);
            reader.drain();
            reader.pad_to(end.max(reader.position()));
        }
        while self.take_pair() {
            self.process_pair();
        }
        let Some(lanes) = self.lanes.take() else {
            return;
        };
        // Both lanes flush at once: each may wait on its engine.
        let PerLane { me, them } = lanes;
        std::thread::scope(|scope| {
            scope.spawn(move || me.finish());
            scope.spawn(move || them.finish());
        });
    }
}
