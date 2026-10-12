//! A manual check of the macOS adapters (not run in CI).
//!
//! Reads Screen Recording with the preflight call, which never prompts, and
//! stops there unless it is already granted. When it is, it lists the
//! on-screen windows (bundle and size, never titles) and records the
//! largest on-screen window of the bundle given as the first argument for
//! about 12 s into a temporary folder: `video.mp4` and the snapshots.
//!
//! ```sh
//! cargo run -p deino-screen --example capture_probe -- com.apple.finder
//! ```

#![allow(clippy::print_stdout, reason = "a manual probe reports on stdout")]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use deino_domain::{BundleId, MeetingId, Millis};
use deino_screen::fake::MemoryStore;
use deino_screen::{
    CapturePlan, FrameRate, Platform, Position, RecordingClock, ScreenCapture, ScreenCaptureParts,
    SnapshotPolicy, VIDEO_FILE,
};

/// A clock that advances 500 ms per read and ends after `limit` reads.
struct Ticking {
    reads: AtomicU32,
    limit: u32,
}

impl RecordingClock for Ticking {
    fn position(&self, _meeting: &MeetingId) -> Position {
        let read = self.reads.fetch_add(1, Ordering::SeqCst);
        if read >= self.limit {
            Position::Over
        } else {
            Position::At(Millis(read * 500))
        }
    }
}

fn main() {
    let Some(platform) = deino_screen::platform() else {
        return println!("no screen platform here");
    };
    let granted = platform.permission.granted();
    println!("Screen Recording granted: {granted}");
    if !granted {
        return;
    }
    list_windows(&platform);
    let bundle = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "com.apple.finder".to_owned());
    match BundleId::try_from(bundle.as_str()) {
        Ok(app) => capture(platform, app),
        Err(_) => println!("not a bundle id: {bundle}"),
    }
}

/// Prints the ordinary on-screen windows: owner and size only.
fn list_windows(platform: &Platform) {
    let windows = match platform.windows.list() {
        Ok(windows) => windows,
        Err(error) => return println!("could not list windows: {error}"),
    };
    for window in windows
        .iter()
        .filter(|w| w.on_screen && w.layer == 0 && w.width >= 200.0)
    {
        let app = window.bundle_id.as_ref().map_or("?", BundleId::as_str);
        println!(
            "window {} of {app}: {}×{} pt",
            window.id, window.width, window.height
        );
    }
}

/// Captures `app`'s window for about 12 s and reports what was written.
fn capture(platform: Platform, app: BundleId) {
    let folder = std::env::temp_dir().join("deino-capture-probe");
    std::fs::remove_dir_all(&folder).ok();
    let store = Arc::new(MemoryStore::default());
    let capture = ScreenCapture::new(ScreenCaptureParts {
        platform: Some(platform),
        clock: Arc::new(Ticking {
            reads: AtomicU32::new(0),
            limit: 24,
        }),
        store: store.clone(),
    });
    capture.start(CapturePlan {
        meeting: MeetingId::new("probe"),
        folder: folder.clone(),
        app,
        video: Some(FrameRate::new(2)),
        snapshots: Some(SnapshotPolicy {
            every: Millis(2_000),
            threshold: 10,
        }),
    });
    wait_for(&folder.join(VIDEO_FILE), &capture);
    println!("folder: {}", folder.display());
    println!("snapshots: {}", store.records().len());
    println!("video: {}", folder.join(VIDEO_FILE).exists());
}

/// Waits until `video` exists or the capture has ended.
fn wait_for(video: &Path, capture: &ScreenCapture) {
    let (_keep, wait) = std::sync::mpsc::channel::<()>();
    while !video.exists() && capture.status().meeting.is_some() {
        wait.recv_timeout(Duration::from_millis(500)).ok();
    }
}
