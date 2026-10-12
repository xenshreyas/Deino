//! [`Capture`]: one recording's screen capture, one frame at a time.
//!
//! Each [`step`](Capture::step) asks the recorder's clock where the
//! recording is. Paused or stopped, the frame is dropped (the video and
//! the snapshots stay on the recording's timeline, pauses skipped, like the
//! transcript). Every [`VISIBILITY_EVERY`] it asks whether the meeting
//! window is still on screen; while it is minimised, its app hidden or the
//! window closed, frames are dropped too ("Paused screen capture"). A
//! visible frame goes to the video (`video.part.mp4`, renamed `video.mp4`
//! when the recording ends) and, when the sampler wants it, becomes a PNG
//! snapshot plus a row. Failures are logged and never stop the recording:
//! a video that cannot be written is dropped, snapshots carry on.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use deino_domain::Millis;

use crate::encode::{encode_png, write_atomically};
use crate::error::ScreenError;
use crate::frame::{Frame, FrameRate};
use crate::phash::PHash;
use crate::plan::CapturePlan;
use crate::ports::{
    Position, RecordingClock, SnapshotRecord, SnapshotStore, VideoEncoder, VideoEncoders, Windows,
};
use crate::sampler::{SnapshotSampler, Verdict};
use crate::status::{CaptureState, StatusCell};
use crate::target::is_visible;

/// Recording time between two checks that the window is on screen.
pub const VISIBILITY_EVERY: Millis = Millis(2_000);
/// The video while it is being written.
pub const VIDEO_PART: &str = "video.part.mp4";
/// The finished video, which the `screen_video` stage gives sound.
pub const VIDEO_FILE: &str = "video.mp4";
/// The folder of snapshot PNG files inside the meeting's recording folder.
pub const SNAPSHOTS: &str = "snapshots";
/// The log target.
const TARGET: &str = "deino_screen";

/// What a capture runs on.
#[derive(Clone)]
pub struct CaptureParts {
    /// What to capture.
    pub plan: CapturePlan,
    /// The meeting window.
    pub window: u32,
    /// The recorder's clock.
    pub clock: Arc<dyn RecordingClock>,
    /// The shareable windows, for the visibility check.
    pub windows: Arc<dyn Windows>,
    /// The video writer.
    pub encoders: Arc<dyn VideoEncoders>,
    /// Where snapshot rows go.
    pub store: Arc<dyn SnapshotStore>,
    /// Shared with the UI.
    pub status: Arc<StatusCell>,
}

/// Whether to keep capturing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Read the next frame.
    Continue,
    /// The recording ended; the video is closed.
    Done,
}

/// The video's state.
enum Video {
    /// Wanted; opened at the first visible frame, whose size it takes.
    Wanted(FrameRate),
    /// Being written at this size.
    Writing(Box<dyn VideoEncoder>, (u32, u32)),
    /// Not wanted, failed or finished.
    Off,
}

/// One recording's capture (see the module docs).
pub struct Capture {
    parts: CaptureParts,
    video: Video,
    sampler: Option<SnapshotSampler>,
    visible: bool,
    checked_at: Option<Millis>,
    last_at: Millis,
}

impl Capture {
    /// A capture that has not stepped yet.
    pub fn new(parts: CaptureParts) -> Self {
        let video = parts.plan.video.map_or(Video::Off, Video::Wanted);
        let sampler = parts.plan.snapshots.map(SnapshotSampler::new);
        parts
            .status
            .begin(&parts.plan.meeting, parts.plan.video.is_some());
        Self {
            parts,
            video,
            sampler,
            visible: true,
            checked_at: None,
            last_at: Millis(0),
        }
    }

    /// Handles one frame, or none when the feed had nothing new.
    pub fn step(&mut self, frame: Option<Frame>) -> Step {
        let meeting = &self.parts.plan.meeting;
        let at = match self.parts.clock.position(meeting) {
            Position::Over => {
                self.finish();
                return Step::Done;
            }
            Position::Paused => {
                self.parts.status.set(meeting, CaptureState::Paused);
                return Step::Continue;
            }
            Position::At(at) => at,
        };
        self.last_at = at;
        if !self.window_visible(at) {
            self.parts
                .status
                .set(&self.parts.plan.meeting, CaptureState::Hidden);
            return Step::Continue;
        }
        self.parts
            .status
            .set(&self.parts.plan.meeting, CaptureState::Capturing);
        if let Some(frame) = frame {
            self.record(&frame, at);
            self.snapshot(&frame, at);
        }
        Step::Continue
    }

    /// Closes the video (renamed `video.mp4`) and ends the status. Called
    /// when the recording ends or the user turns capture off.
    pub fn finish(&mut self) {
        if let Video::Writing(encoder, _) = std::mem::replace(&mut self.video, Video::Off) {
            match encoder.finish(self.last_at) {
                Ok(()) => self.publish_video(),
                Err(error) => {
                    log::warn!(target: TARGET, "the screen recording could not be closed: {error}");
                }
            }
        }
        self.parts.status.end(&self.parts.plan.meeting);
    }

    /// Whether the window is on screen, re-checked every [`VISIBILITY_EVERY`].
    fn window_visible(&mut self, at: Millis) -> bool {
        let due = self.checked_at.is_none_or(|checked| {
            at.0 >= checked.0.saturating_add(VISIBILITY_EVERY.0) || at < checked
        });
        if !due {
            return self.visible;
        }
        self.checked_at = Some(at);
        let visible = match self.parts.windows.list() {
            Ok(windows) => is_visible(&windows, self.parts.window),
            Err(error) => {
                log::debug!(target: TARGET, "could not list windows: {error}");
                self.visible
            }
        };
        if visible != self.visible {
            log::info!(target: TARGET, "meeting window {}: capture {}", self.parts.window, if visible { "resumed" } else { "paused" });
        }
        self.visible = visible;
        visible
    }

    /// Appends `frame` to the video, opening it at the first frame.
    fn record(&mut self, frame: &Frame, at: Millis) {
        let size = (frame.width(), frame.height());
        if let Video::Wanted(rate) = self.video {
            self.video = match self.parts.encoders.open(&self.path(VIDEO_PART), size, rate) {
                Ok(encoder) => Video::Writing(encoder, size),
                Err(error) => self.video_failed(&error),
            };
        }
        let Video::Writing(encoder, writing) = &mut self.video else {
            return;
        };
        if *writing != size {
            return log::debug!(target: TARGET, "skipped a frame of another size");
        }
        if let Err(error) = encoder.append(frame, at) {
            self.video = self.video_failed(&error);
        }
    }

    fn video_failed(&self, error: &ScreenError) -> Video {
        log::warn!(target: TARGET, "the screen recording stopped: {error}");
        self.parts.status.video_stopped(&self.parts.plan.meeting);
        Video::Off
    }

    /// Keeps `frame` as a snapshot when the sampler wants it.
    fn snapshot(&mut self, frame: &Frame, at: Millis) {
        let Some(sampler) = self.sampler.as_mut().filter(|sampler| sampler.due(at)) else {
            return;
        };
        let hash = PHash::of(frame);
        if sampler.judge(at, hash) == Verdict::Unchanged {
            return;
        }
        match self.save(frame, at, hash) {
            Ok(()) => {
                if let Some(sampler) = self.sampler.as_mut() {
                    sampler.kept(hash);
                }
                self.parts.status.snapshot_kept(&self.parts.plan.meeting);
            }
            Err(error) => log::warn!(target: TARGET, "a snapshot could not be saved: {error}"),
        }
    }

    /// Writes the PNG, then its row.
    fn save(&self, frame: &Frame, at: Millis, hash: PHash) -> Result<(), ScreenError> {
        let file_name = snapshot_name(at);
        let path = self.path(SNAPSHOTS).join(&file_name);
        write_atomically(&path, &encode_png(frame)?)?;
        self.parts.store.record(&SnapshotRecord {
            meeting: self.parts.plan.meeting.clone(),
            at,
            file_name,
            width: frame.width(),
            height: frame.height(),
            hash,
        })
    }

    /// Renames the finished part file to `video.mp4`.
    fn publish_video(&self) {
        if let Err(error) = std::fs::rename(self.path(VIDEO_PART), self.path(VIDEO_FILE)) {
            log::warn!(target: TARGET, "the screen recording could not be renamed: {error}");
        }
    }

    fn path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.parts.plan.folder.join(name)
    }
}

/// A snapshot's file name: its time, zero-padded so names sort in order.
pub fn snapshot_name(at: Millis) -> String {
    format!("{:010}.png", at.0)
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
