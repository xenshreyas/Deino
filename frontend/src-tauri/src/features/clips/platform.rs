//! The clip service's macOS adapters: [`AvClipCutter`] over
//! `deino_audio::clip` (`AVFoundation` passthrough export, run off the async
//! runtime) and [`OpenFinder`] (`open -R`).

use std::io;
use std::path::Path;

use async_trait::async_trait;
use deino_audio::clip::{ClipContainer, ClipRequest, cut_clip};
use deino_core::clips::{ClipCutter, ClipMedia, CutFailed, CutRequest, Finder};

/// Cuts clips with `AVAssetExportSession` on a blocking thread.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AvClipCutter;

#[async_trait]
impl ClipCutter for AvClipCutter {
    async fn cut(&self, request: &CutRequest) -> Result<(), CutFailed> {
        let request = request.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            cut_clip(&ClipRequest {
                source: &request.source,
                destination: &request.destination,
                start: request.start,
                end: request.end,
                container: container(request.media),
            })
        })
        .await;
        match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                log::warn!(target: "clips", "cutting a clip failed: {error}");
                Err(CutFailed)
            }
            Err(error) => {
                log::warn!(target: "clips", "the clip cutter stopped: {error}");
                Err(CutFailed)
            }
        }
    }
}

/// The file type a clip of `media` is written as.
fn container(media: ClipMedia) -> ClipContainer {
    match media {
        ClipMedia::Audio => ClipContainer::M4a,
        ClipMedia::Video => ClipContainer::Mp4,
    }
}

/// Reveals files with `open -R`; the clip service checked the path first.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OpenFinder;

#[async_trait]
impl Finder for OpenFinder {
    #[cfg(target_os = "macos")]
    async fn reveal(&self, path: &Path) -> io::Result<()> {
        let status = tokio::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("open -R exited with {status}")))
        }
    }

    #[cfg(not(target_os = "macos"))]
    async fn reveal(&self, _path: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
}
