//! [`Capturer`]: a [`Capture`] on its own thread.
//!
//! The thread finds the meeting window, opens its frame feed and steps the
//! capture with each frame (or every [`POLL`] without one) until the
//! recording ends or the handle is dropped, which closes the video. Every
//! blocking call (`ScreenCaptureKit`'s completion handlers, the H.264 writer,
//! PNG encoding, the snapshot row) runs here, never on an audio, runtime
//! or `ScreenCaptureKit` queue thread (§3C).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::capture::{Capture, CaptureParts, Step};
use crate::error::ScreenError;
use crate::frame::FrameRate;
use crate::plan::CapturePlan;
use crate::ports::{FrameFeed, Platform, RecordingClock, SnapshotStore};
use crate::status::StatusCell;
use crate::target::{pick_window, stream_size};

/// The longest wait for a frame before the capture checks the clock.
pub const POLL: Duration = Duration::from_millis(500);
/// The log target.
const TARGET: &str = "deino_screen";

/// What a capture thread starts from.
#[derive(Clone)]
pub struct Start {
    /// What to capture.
    pub plan: CapturePlan,
    /// This Mac's screen ports.
    pub platform: Platform,
    /// The recorder's clock.
    pub clock: Arc<dyn RecordingClock>,
    /// Where snapshot rows go.
    pub store: Arc<dyn SnapshotStore>,
    /// Shared with the UI.
    pub status: Arc<StatusCell>,
}

/// A running capture. Dropping it stops the capture, which closes the
/// video on its own thread.
#[derive(Debug)]
pub struct Capturer {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Capturer {
    /// Starts capturing on a new thread.
    pub fn spawn(start: Start) -> std::io::Result<Self> {
        let (stop, stopped) = channel();
        let thread = std::thread::Builder::new()
            .name("deino-screen".into())
            .spawn(move || run(&start, &stopped))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    /// Stops capturing and waits until the video is closed.
    pub fn stop(mut self) {
        // The thread may already be gone (the recording ended).
        self.stop.send(()).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }

    /// Whether the thread has ended.
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

/// Opens the capture, then steps it until the recording ends or a stop.
fn run(start: &Start, stopped: &Receiver<()>) {
    let meeting = &start.plan.meeting;
    let (mut capture, mut feed) = match open(start) {
        Ok(opened) => opened,
        Err(error) => {
            log::info!(target: TARGET, "screen capture did not start: {error}");
            return start.status.end(meeting);
        }
    };
    log::info!(target: TARGET, "capturing the meeting window of {meeting}");
    loop {
        match stopped.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => return capture.finish(),
            Err(TryRecvError::Empty) => {}
        }
        if capture.step(feed.next(POLL)) == Step::Done {
            return;
        }
    }
}

/// Finds the meeting window and opens its frames.
fn open(start: &Start) -> Result<(Capture, Box<dyn FrameFeed>), ScreenError> {
    let platform = &start.platform;
    let windows = platform.windows.list()?;
    let window = pick_window(&windows, &start.plan.app).ok_or(ScreenError::WindowGone)?;
    let size = stream_size(
        window.width,
        window.height,
        platform.windows.scale(window.id),
    );
    // Snapshots alone need a frame every 10 s; one a second is plenty.
    let rate = start.plan.video.unwrap_or(FrameRate::DEFAULT);
    let feed = platform.source.open(window.id, size, rate)?;
    let capture = Capture::new(CaptureParts {
        plan: start.plan.clone(),
        window: window.id,
        clock: Arc::clone(&start.clock),
        windows: Arc::clone(&platform.windows),
        encoders: Arc::clone(&platform.encoders),
        store: Arc::clone(&start.store),
        status: Arc::clone(&start.status),
    });
    Ok((capture, feed))
}
