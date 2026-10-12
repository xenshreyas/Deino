//! A manual check of the macOS adapters (not run in CI).
//!
//! Reads Screen Recording with the preflight call, which never prompts, and
//! stops there unless it is already granted. When it is, it records the
//! largest on-screen window of the bundle given as the first argument for
//! a few seconds into a temporary folder: `video.mp4` and the snapshots.
//!
//! ```sh
//! cargo run -p deino-screen --example capture_probe -- com.apple.finder
//! ```

#![allow(clippy::print_stdout, reason = "a manual probe reports on stdout")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use deino_domain::{BundleId, MeetingId, Millis};
use deino_screen::fake::MemoryStore;
use deino_screen::{
    CapturePlan, FrameRate, Position, RecordingClock, ScreenCapture, ScreenCaptureParts,
    SnapshotPolicy,
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
    match platform.windows.list() {
        Ok(windows) => {
            let shown = windows
                .iter()
                .filter(|w| w.on_screen && w.layer == 0 && w.width >= 200.0);
            for window in shown {
                let app = window.bundle_id.as_ref().map_or("?", BundleId::as_str);
                println!(
                    "window {} of {app}: {}×{} pt",
                    window.id, window.width, window.height
                );
            }
        }
        Err(error) => println!("could not list windows: {error}"),
    }
    let bundle = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "com.apple.finder".to_owned());
    let Ok(app) = BundleId::try_from(bundle.as_str()) else {
        return println!("not a bundle id: {bundle}");
    };
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
    let (_keep, wait) = std::sync::mpsc::channel::<()>();
    while !folder.join(deino_screen::VIDEO_FILE).exists() {
        if wait.recv_timeout(std::time::Duration::from_secs(1)).is_ok()
            || capture.status().meeting.is_none()
        {
            break;
        }
    }
    println!("folder: {}", folder.display());
    println!("snapshots: {}", store.records().len());
    println!("video: {}", folder.join(deino_screen::VIDEO_FILE).exists());
}
