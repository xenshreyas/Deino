//! The macOS muxer: an `AVMutableComposition` with the video file's video
//! track and the audio file's audio track, each inserted at its own time
//! range, exported with the Passthrough preset (no re-encode). cidre
//! a9587fa binds neither the composition nor the export session, so this
//! uses objc2, like `clip::native`.

#![allow(
    deprecated,
    reason = "tracksWithMediaType: is the synchronous call; the async loader needs a run loop this blocking step does not have"
)]

use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::time::Duration;

use block2::RcBlock;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAsset, AVAssetExportPresetPassthrough, AVAssetExportSession, AVAssetExportSessionStatus,
    AVAssetTrack, AVFileTypeMPEG4, AVMediaType, AVMediaTypeAudio, AVMediaTypeVideo,
    AVMutableComposition, AVURLAsset,
};
use objc2_foundation::{NSString, NSURL};

use super::{MuxError, MuxRequest};

/// How long one export may take (a passthrough copy of an hour takes seconds).
const EXPORT_TIMEOUT: Duration = Duration::from_mins(10);
/// "Pick any free track ID" (`kCMPersistentTrackID_Invalid`).
const ANY_TRACK_ID: i32 = 0;

/// Writes the composition of the request's video and audio.
pub(super) fn add_audio(request: &MuxRequest<'_>) -> Result<(), MuxError> {
    // AVFoundation refuses to overwrite; a stale file from a failed run goes.
    std::fs::remove_file(request.destination).ok();
    let video =
        first_track(request.video, video_type()?)?.ok_or(MuxError::MissingTrack("video"))?;
    let audio =
        first_track(request.audio, audio_type()?)?.ok_or(MuxError::MissingTrack("audio"))?;
    let composition = compose(&video, &audio)?;
    export(&composition, request.destination)
}

/// Whether `path` has an audio track.
pub(super) fn has_audio(path: &Path) -> Result<bool, MuxError> {
    Ok(first_track(path, audio_type()?)?.is_some())
}

#[allow(unsafe_code, reason = "reading an AVFoundation extern constant")]
fn video_type() -> Result<&'static AVMediaType, MuxError> {
    // SAFETY: reading a framework constant.
    unsafe { AVMediaTypeVideo }.ok_or(MuxError::Unavailable)
}

#[allow(unsafe_code, reason = "reading an AVFoundation extern constant")]
fn audio_type() -> Result<&'static AVMediaType, MuxError> {
    // SAFETY: reading a framework constant.
    unsafe { AVMediaTypeAudio }.ok_or(MuxError::Unavailable)
}

/// The first track of `kind` in the file at `path`.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn first_track(
    path: &Path,
    kind: &AVMediaType,
) -> Result<Option<Retained<AVAssetTrack>>, MuxError> {
    let url = file_url(path)?;
    // SAFETY: plain AVFoundation construction and a synchronous track
    // query on a file URL.
    let tracks =
        unsafe { AVURLAsset::URLAssetWithURL_options(&url, None).tracksWithMediaType(kind) };
    Ok(tracks.firstObject())
}

/// A composition of `video` and `audio`, each at its own time range.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn compose(
    video: &AVAssetTrack,
    audio: &AVAssetTrack,
) -> Result<Retained<AVMutableComposition>, MuxError> {
    // SAFETY: a new composition, tracks added and filled from live source
    // tracks with their own valid time ranges.
    unsafe {
        let composition = AVMutableComposition::composition();
        for (track, kind) in [(video, video_type()?), (audio, audio_type()?)] {
            let target = composition
                .addMutableTrackWithMediaType_preferredTrackID(kind, ANY_TRACK_ID)
                .ok_or(MuxError::Compose)?;
            let range = track.timeRange();
            target
                .insertTimeRange_ofTrack_atTime_error(range, track, range.start)
                .map_err(|_| MuxError::Compose)?;
        }
        Ok(composition)
    }
}

/// Exports `asset` as an MP4 at `destination`, waiting for the result.
#[allow(unsafe_code, reason = "objc2 marks every AVFoundation call unsafe")]
fn export(asset: &AVAsset, destination: &Path) -> Result<(), MuxError> {
    let url = file_url(destination)?;
    // SAFETY: a passthrough session over a live asset, configured with
    // retained arguments; AVFoundation copies the completion block.
    let session = unsafe {
        let session = AVAssetExportSession::initWithAsset_presetName(
            AVAssetExportSession::alloc(),
            asset,
            AVAssetExportPresetPassthrough,
        )
        .ok_or(MuxError::Unavailable)?;
        session.setOutputURL(Some(&url));
        session.setOutputFileType(AVFileTypeMPEG4);
        session
    };
    let (done, finished) = sync_channel(1);
    let handler = RcBlock::new(move || {
        // The receiver may have timed out and gone; nothing else to do then.
        done.try_send(()).ok();
    });
    // SAFETY: the session is configured; AVFoundation copies the block.
    unsafe { session.exportAsynchronouslyWithCompletionHandler(&handler) };
    finished
        .recv_timeout(EXPORT_TIMEOUT)
        .map_err(|_| MuxError::TimedOut)?;
    // SAFETY: a property read on a live session.
    match unsafe { session.status() } {
        AVAssetExportSessionStatus::Completed => Ok(()),
        other => Err(MuxError::Failed(other.0)),
    }
}

/// A file URL for `path`.
fn file_url(path: &Path) -> Result<Retained<NSURL>, MuxError> {
    let path = path.to_str().ok_or(MuxError::InvalidPath)?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}
