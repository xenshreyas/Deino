//! Passthrough export: a fragmented `.m4a` segment to a flat `.m4a`.
//!
//! `AVAssetExportSession` with the Passthrough preset copies the AAC packets
//! and the priming edit list into a regular (non-fragmented) file without
//! re-encoding: about 1.3 s per hour of audio (spike §2.2). cidre a9587fa
//! does not bind the export session, so this uses objc2.

use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::time::Duration;

use block2::RcBlock;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAssetExportPresetPassthrough, AVAssetExportSession, AVAssetExportSessionStatus,
    AVFileTypeAppleM4A, AVURLAsset,
};
use objc2_foundation::{NSString, NSURL};

use super::FinalizeError;

/// How long one export may take (a multi-hour meeting takes seconds).
const EXPORT_TIMEOUT: Duration = Duration::from_mins(10);

/// Copies the audio of `source` into a new flat `.m4a` at `destination`.
pub(super) fn export_passthrough(source: &Path, destination: &Path) -> Result<(), FinalizeError> {
    let session = session(source, destination)?;
    let (done, finished) = sync_channel(1);
    let handler = RcBlock::new(move || {
        // The receiver may have timed out and gone; nothing else to do then.
        done.try_send(()).ok();
    });
    start(&session, &handler);
    finished
        .recv_timeout(EXPORT_TIMEOUT)
        .map_err(|_| FinalizeError::ExportTimedOut)?;
    match status(&session) {
        AVAssetExportSessionStatus::Completed => Ok(()),
        other => Err(FinalizeError::ExportFailed(other.0)),
    }
}

/// A passthrough export session from `source` to `destination`.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn session(
    source: &Path,
    destination: &Path,
) -> Result<Retained<AVAssetExportSession>, FinalizeError> {
    // SAFETY: plain AVFoundation object construction and property setters,
    // with valid, retained arguments; the preset and file type are
    // framework constants.
    unsafe {
        let asset = AVURLAsset::URLAssetWithURL_options(&*file_url(source)?, None);
        let session = AVAssetExportSession::initWithAsset_presetName(
            AVAssetExportSession::alloc(),
            &asset,
            AVAssetExportPresetPassthrough,
        )
        .ok_or(FinalizeError::ExportUnavailable)?;
        session.setOutputURL(Some(&*file_url(destination)?));
        session.setOutputFileType(AVFileTypeAppleM4A);
        Ok(session)
    }
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
fn file_url(path: &Path) -> Result<Retained<NSURL>, FinalizeError> {
    let path = path.to_str().ok_or(FinalizeError::InvalidPath)?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}
