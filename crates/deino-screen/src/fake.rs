//! Fakes of the screen ports for tests (here and in the app): a scripted
//! Mac ([`FakeScreen`]), a scripted recording clock ([`ScriptedClock`]),
//! an in-memory video writer ([`FakeEncoders`]) and snapshot store
//! ([`MemoryStore`]). None of them touches the real screen.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use deino_domain::{MeetingId, Millis};

use crate::error::ScreenError;
use crate::frame::{Frame, FrameRate};
use crate::ports::{
    FrameFeed, Platform, Position, RecordingClock, ScreenPermission, ScreenSource, SnapshotRecord,
    SnapshotStore, VideoEncoder, VideoEncoders, Windows,
};
use crate::target::WindowInfo;

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A scripted Mac: a permission switch, a window list and frames to feed.
#[derive(Debug, Default)]
pub struct FakeScreen {
    granted: AtomicBool,
    windows: Mutex<Vec<WindowInfo>>,
    frames: Arc<Mutex<VecDeque<Frame>>>,
    opened: Mutex<Vec<(u32, (u32, u32))>>,
}

impl FakeScreen {
    /// A Mac where Screen Recording is granted and `windows` are shared.
    pub fn granted(windows: Vec<WindowInfo>) -> Arc<Self> {
        let screen = Self::default();
        screen.granted.store(true, Ordering::SeqCst);
        *locked(&screen.windows) = windows;
        Arc::new(screen)
    }

    /// Grants or revokes Screen Recording.
    pub fn set_granted(&self, granted: bool) {
        self.granted.store(granted, Ordering::SeqCst);
    }

    /// Replaces the shared windows (a window minimised, closed, opened).
    pub fn set_windows(&self, windows: Vec<WindowInfo>) {
        *locked(&self.windows) = windows;
    }

    /// Queues frames for the next feed to deliver, one per read.
    pub fn push_frames(&self, frames: impl IntoIterator<Item = Frame>) {
        locked(&self.frames).extend(frames);
    }

    /// The windows opened so far, with their capture sizes.
    pub fn opened(&self) -> Vec<(u32, (u32, u32))> {
        locked(&self.opened).clone()
    }

    /// The platform over this fake and `encoders`.
    pub fn platform(self: &Arc<Self>, encoders: &Arc<FakeEncoders>) -> Platform {
        Platform {
            permission: Arc::clone(self) as Arc<dyn ScreenPermission>,
            windows: Arc::clone(self) as Arc<dyn Windows>,
            source: Arc::clone(self) as Arc<dyn ScreenSource>,
            encoders: Arc::clone(encoders) as Arc<dyn VideoEncoders>,
        }
    }
}

impl ScreenPermission for FakeScreen {
    fn granted(&self) -> bool {
        self.granted.load(Ordering::SeqCst)
    }
}

impl Windows for FakeScreen {
    fn list(&self) -> Result<Vec<WindowInfo>, ScreenError> {
        Ok(locked(&self.windows).clone())
    }

    fn scale(&self, _id: u32) -> f64 {
        2.0
    }
}

impl ScreenSource for FakeScreen {
    fn open(
        &self,
        id: u32,
        size: (u32, u32),
        _rate: FrameRate,
    ) -> Result<Box<dyn FrameFeed>, ScreenError> {
        locked(&self.opened).push((id, size));
        Ok(Box::new(FakeFeed(Arc::clone(&self.frames))))
    }
}

/// Delivers the queued frames, one per read, without waiting.
struct FakeFeed(Arc<Mutex<VecDeque<Frame>>>);

impl FrameFeed for FakeFeed {
    fn next(&mut self, _wait: Duration) -> Option<Frame> {
        locked(&self.0).pop_front()
    }
}

/// A recording clock that plays positions in order, then says `Over`.
#[derive(Debug, Default)]
pub struct ScriptedClock(Mutex<VecDeque<Position>>);

impl ScriptedClock {
    /// A clock that answers `positions`, then `Over` forever.
    pub fn new(positions: impl IntoIterator<Item = Position>) -> Arc<Self> {
        Arc::new(Self(Mutex::new(positions.into_iter().collect())))
    }
}

impl RecordingClock for ScriptedClock {
    fn position(&self, _meeting: &MeetingId) -> Position {
        locked(&self.0).pop_front().unwrap_or(Position::Over)
    }
}

/// What a fake video file saw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FakeVideo {
    /// Where it was opened.
    pub path: PathBuf,
    /// Its size.
    pub size: (u32, u32),
    /// The time of each appended frame.
    pub frames: Vec<Millis>,
    /// Where it was closed, once it was.
    pub finished_at: Option<Millis>,
}

/// Video writers that record calls and write an empty file, so the
/// capture's rename to `video.mp4` can be checked.
#[derive(Debug, Default)]
pub struct FakeEncoders {
    videos: Arc<Mutex<Vec<FakeVideo>>>,
    fail_open: AtomicBool,
}

impl FakeEncoders {
    /// Writers that work.
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    /// Makes the next opens fail.
    pub fn fail_open(&self) {
        self.fail_open.store(true, Ordering::SeqCst);
    }

    /// Every video opened so far.
    pub fn videos(&self) -> Vec<FakeVideo> {
        locked(&self.videos).clone()
    }
}

impl VideoEncoders for FakeEncoders {
    fn open(
        &self,
        path: &Path,
        size: (u32, u32),
        _rate: FrameRate,
    ) -> Result<Box<dyn VideoEncoder>, ScreenError> {
        if self.fail_open.load(Ordering::SeqCst) {
            return Err(ScreenError::Native { call: "fake open" });
        }
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(path, b"")?;
        let mut videos = locked(&self.videos);
        videos.push(FakeVideo {
            path: path.to_owned(),
            size,
            ..FakeVideo::default()
        });
        Ok(Box::new(FakeEncoder {
            videos: Arc::clone(&self.videos),
            index: videos.len() - 1,
        }))
    }
}

struct FakeEncoder {
    videos: Arc<Mutex<Vec<FakeVideo>>>,
    index: usize,
}

impl VideoEncoder for FakeEncoder {
    fn append(&mut self, _frame: &Frame, at: Millis) -> Result<(), ScreenError> {
        if let Some(video) = locked(&self.videos).get_mut(self.index) {
            video.frames.push(at);
        }
        Ok(())
    }

    fn finish(self: Box<Self>, end: Millis) -> Result<(), ScreenError> {
        if let Some(video) = locked(&self.videos).get_mut(self.index) {
            video.finished_at = Some(end);
        }
        Ok(())
    }
}

/// Snapshot rows kept in memory.
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<Vec<SnapshotRecord>>);

impl MemoryStore {
    /// The rows recorded so far.
    pub fn records(&self) -> Vec<SnapshotRecord> {
        locked(&self.0).clone()
    }
}

impl SnapshotStore for MemoryStore {
    fn record(&self, snapshot: &SnapshotRecord) -> Result<(), ScreenError> {
        locked(&self.0).push(snapshot.clone());
        Ok(())
    }
}
