//! The macOS clip cutter: `AVAssetExportSession`, Passthrough preset, with a
//! `timeRange` (the spike's `deino-spikes/native-recording/src/clip.rs`).
//! cidre a9587fa does not bind the export session, so this uses objc2, like
//! `finalize::export`.

use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::time::Duration;

use block2::RcBlock;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAssetExportPresetPassthrough, AVAssetExportSession, AVAssetExportSessionStatus,
    AVFileTypeAppleM4A, AVFileTypeMPEG4, AVURLAsset,
};
use objc2_core_media::{CMTime, CMTimeRange};
use objc2_foundation::{NSString, NSURL};

use super::{ClipContainer, ClipError, ClipRequest};

/// How long one export may take (a passthrough cut of an hour takes about a second).
const EXPORT_TIMEOUT: Duration = Duration::from_mins(5);

/// The time scale of the range: milliseconds, the unit clips are stored in.
const MILLIS_SCALE: i32 = 1_000;

/// Cuts the request's range of its source into its destination.
pub(super) fn cut(request: &ClipRequest<'_>) -> Result<(), ClipError> {
    // AVFoundation refuses to overwrite; a stale file from a failed cut goes.
    std::fs::remove_file(request.destination).ok();
    let session = session(request)?;
    let (done, finished) = sync_channel(1);
    let handler = RcBlock::new(move || {
        // The receiver may have timed out and gone; nothing else to do then.
        done.try_send(()).ok();
    });
    start(&session, &handler);
    finished
        .recv_timeout(EXPORT_TIMEOUT)
        .map_err(|_| ClipError::TimedOut)?;
    match status(&session) {
        AVAssetExportSessionStatus::Completed => Ok(()),
        other => Err(ClipError::Failed(other.0)),
    }
}

/// A passthrough export session of the request's range.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn session(request: &ClipRequest<'_>) -> Result<Retained<AVAssetExportSession>, ClipError> {
    let range = CMTimeRange {
        start: millis(request.start.0),
        duration: millis(request.end.0.saturating_sub(request.start.0)),
    };
    // SAFETY: plain AVFoundation object construction and property setters,
    // with valid, retained arguments; the preset and file types are
    // framework constants.
    unsafe {
        let asset = AVURLAsset::URLAssetWithURL_options(&*file_url(request.source)?, None);
        let session = AVAssetExportSession::initWithAsset_presetName(
            AVAssetExportSession::alloc(),
            &asset,
            AVAssetExportPresetPassthrough,
        )
        .ok_or(ClipError::Unavailable)?;
        session.setOutputURL(Some(&*file_url(request.destination)?));
        session.setOutputFileType(match request.container {
            ClipContainer::M4a => AVFileTypeAppleM4A,
            ClipContainer::Mp4 => AVFileTypeMPEG4,
        });
        session.setTimeRange(range);
        Ok(session)
    }
}

/// `ms` milliseconds as a `CMTime`.
#[allow(unsafe_code, reason = "objc2 marks every Core Media call unsafe")]
fn millis(ms: u32) -> CMTime {
    // SAFETY: a pure constructor of a plain C struct.
    unsafe { CMTime::new(i64::from(ms), MILLIS_SCALE) }
}

/// Starts the export; `handler` runs when it ends.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn start(session: &AVAssetExportSession, handler: &RcBlock<dyn Fn()>) {
    // SAFETY: the session is configured, and AVFoundation copies the block.
    unsafe { session.exportAsynchronouslyWithCompletionHandler(handler) };
}

/// The session's final status.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn status(session: &AVAssetExportSession) -> AVAssetExportSessionStatus {
    // SAFETY: a property read on a live session.
    unsafe { session.status() }
}

/// A file URL for `path`.
fn file_url(path: &Path) -> Result<Retained<NSURL>, ClipError> {
    let path = path.to_str().ok_or(ClipError::InvalidPath)?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}
