//! The ports screen capture runs on. `macos/` implements the platform ones
//! with `ScreenCaptureKit` and `AVAssetWriter`; the app implements the clock
//! (the recorder's playhead) and the store (the `snapshots` table); `fake`
//! has scripted versions of each for tests. Nothing here may prompt for a
//! permission: the app checks [`ScreenPermission::granted`] first.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use deino_domain::{MeetingId, Millis};

use crate::error::ScreenError;
use crate::frame::{Frame, FrameRate};
use crate::phash::PHash;
use crate::pick::DisplayInfo;
use crate::target::WindowInfo;

/// Where a recording is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Recording, this far into its timeline (pauses skipped).
    At(Millis),
    /// Paused: the timeline stands still.
    Paused,
    /// Stopped, or another meeting is recording.
    Over,
}

/// The recorder's clock.
pub trait RecordingClock: Send + Sync {
    /// Where the recording of `meeting` is now.
    fn position(&self, meeting: &MeetingId) -> Position;
}

/// Screen Recording, read without ever showing the macOS prompt.
pub trait ScreenPermission: Send + Sync {
    /// Whether Deino may capture the screen now.
    fn granted(&self) -> bool;
}

/// The windows macOS shares (only called once permission is granted).
pub trait Windows: Send + Sync {
    /// Every shareable window, on screen or not, with its display scale.
    fn list(&self) -> Result<Vec<WindowInfo>, ScreenError>;

    /// The backing scale (pixels per point) of the display showing window
    /// `id`; 2.0 on Retina displays.
    fn scale(&self, id: u32) -> f64;

    /// Every display macOS shares, with its scale. None by default.
    fn displays(&self) -> Result<Vec<DisplayInfo>, ScreenError> {
        Ok(Vec::new())
    }
}

/// Opens a stream of one window's frames.
pub trait ScreenSource: Send + Sync {
    /// Starts capturing window `id` at `size` pixels, about `rate` frames a
    /// second. Frames stop when the feed is dropped.
    fn open(
        &self,
        id: u32,
        size: (u32, u32),
        rate: FrameRate,
    ) -> Result<Box<dyn FrameFeed>, ScreenError>;

    /// Starts capturing the whole display `id` (Deino's own windows
    /// left out); unsupported by default.
    fn open_display(
        &self,
        _id: u32,
        _size: (u32, u32),
        _rate: FrameRate,
    ) -> Result<Box<dyn FrameFeed>, ScreenError> {
        Err(ScreenError::Unsupported)
    }
}

/// A running window capture.
pub trait FrameFeed: Send {
    /// The newest frame, waiting at most `wait`; `None` when none came
    /// (an unchanged or hidden window sends nothing).
    fn next(&mut self, wait: Duration) -> Option<Frame>;
}

/// Opens video files.
pub trait VideoEncoders: Send + Sync {
    /// A new H.264 MP4 at `path`, `size` pixels, about `rate` frames a second.
    fn open(
        &self,
        path: &Path,
        size: (u32, u32),
        rate: FrameRate,
    ) -> Result<Box<dyn VideoEncoder>, ScreenError>;
}

/// One video file being written.
pub trait VideoEncoder: Send {
    /// Appends `frame`, shown from `at` on the recording's timeline.
    fn append(&mut self, frame: &Frame, at: Millis) -> Result<(), ScreenError>;

    /// Closes the file, its last frame shown until `end`.
    fn finish(self: Box<Self>, end: Millis) -> Result<(), ScreenError>;
}

/// A kept snapshot, already written as a PNG.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotRecord {
    /// The meeting.
    pub meeting: MeetingId,
    /// Where on the recording's timeline it was taken.
    pub at: Millis,
    /// The PNG's file name inside the meeting's `snapshots/` folder.
    pub file_name: String,
    /// Its size in pixels.
    pub width: u32,
    /// Its size in pixels.
    pub height: u32,
    /// Its perceptual hash.
    pub hash: PHash,
}

/// Where kept snapshots are recorded (the `snapshots` table in the app).
pub trait SnapshotStore: Send + Sync {
    /// Records `snapshot`, whose file exists.
    fn record(&self, snapshot: &SnapshotRecord) -> Result<(), ScreenError>;
}

/// This Mac's screen ports; `None` off macOS.
#[derive(Clone)]
pub struct Platform {
    /// Screen Recording.
    pub permission: Arc<dyn ScreenPermission>,
    /// The shareable windows.
    pub windows: Arc<dyn Windows>,
    /// Window frames.
    pub source: Arc<dyn ScreenSource>,
    /// The H.264 writer.
    pub encoders: Arc<dyn VideoEncoders>,
}
