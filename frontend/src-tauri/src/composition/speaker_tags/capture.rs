//! [`LogFolderCapture`]: where speaker tags' call-window dumps go (CAP-05).
//!
//! With Settings → Advanced → Developer → "Save the call window when
//! speaker names can't be read" on (`developer.captureCallWindows`), a
//! watch that cannot read a meeting app's call window hands this port one
//! full Accessibility dump of the app's windows, once per meeting. It is
//! written beside Deino's log (`~/Library/Logs/com.xenshreyas.deino`, the
//! folder `logging.rs` resolves) as
//! `call-window-<app>-<meeting id>-<UTC time>.json`, a JSON array of
//! windows in the format of `crates/deino-speaker-tags/fixtures/`.
//!
//! A dump holds participants' names, so only its file name is logged; the
//! owner anonymises it before it becomes a fixture (`fixtures/README.md`).
//! Both calls run on the watcher's own thread, never a runtime thread.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

use chrono::{DateTime, Utc};
use deino_core::settings::SettingsService;
use deino_domain::{CallApp, Clock, MeetingId};
use deino_speaker_tags::{AxNode, WindowCapture};
use tokio::runtime::Handle;

/// The log target.
const TARGET: &str = "speaker_tags";

/// Writes call-window dumps to a folder while the setting is on.
pub(crate) struct LogFolderCapture {
    /// The setting, read when a dump would be taken.
    pub(crate) settings: Arc<SettingsService>,
    /// Blocks the watcher's thread on the settings read.
    pub(crate) runtime: Handle,
    /// Names the files.
    pub(crate) clock: Arc<dyn Clock>,
    /// Where dumps go; `None` where there is no log folder (never wanted).
    pub(crate) folder: Option<PathBuf>,
}

impl WindowCapture for LogFolderCapture {
    fn wanted(&self) -> bool {
        self.folder.is_some()
            && self
                .runtime
                .block_on(self.settings.get())
                .developer
                .capture_call_windows
    }

    fn save(&self, meeting: &MeetingId, app: CallApp, windows: &[AxNode]) {
        let Some(folder) = &self.folder else {
            return;
        };
        let name = file_name(app, meeting, self.clock.now());
        match write_dump(folder, &name, windows) {
            Ok(()) => log::info!(target: TARGET, "saved the call window to {name}"),
            Err(error) => {
                log::warn!(target: TARGET, "could not save the call window to {name}: {error}");
            }
        }
    }
}

/// `call-window-<app>-<meeting id>-<UTC time>.json`; the meeting ID keeps
/// only characters that are safe in a file name.
fn file_name(app: CallApp, meeting: &MeetingId, at: DateTime<Utc>) -> String {
    let id: String = meeting
        .as_str()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let time = at.format("%Y%m%dT%H%M%SZ");
    format!("call-window-{app}-{id}-{time}.json")
}

/// Writes `windows` as pretty fixture JSON to `folder/name`.
fn write_dump(folder: &Path, name: &str, windows: &[AxNode]) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let json = serde_json::to_string_pretty(windows).map_err(io::Error::other)?;
    fs::write(folder.join(name), json + "\n")
}

#[cfg(test)]
mod tests {
    use deino_core::settings::{FakeSettingsStore, RecordedSettingsEvents};
    use deino_testkit::{FakeClock, at};

    use super::*;

    fn window() -> AxNode {
        AxNode {
            role: "AXWindow".into(),
            title: Some("Zoom Meeting".into()),
            children: vec![AxNode {
                role: "AXGroup".into(),
                description: Some("Alex Chen, Computer audio unmuted, Video on".into()),
                ..AxNode::default()
            }],
            ..AxNode::default()
        }
    }

    fn capture(runtime: &tokio::runtime::Runtime, folder: Option<PathBuf>) -> LogFolderCapture {
        let settings = runtime.block_on(SettingsService::load(
            Arc::new(FakeSettingsStore::default()),
            Arc::new(RecordedSettingsEvents::default()),
        ));
        LogFolderCapture {
            settings: Arc::new(settings.unwrap()),
            runtime: runtime.handle().clone(),
            clock: Arc::new(FakeClock::new(at("2026-10-19T14:30:05Z"))),
            folder,
        }
    }

    #[test]
    fn a_dump_is_fixture_json_named_by_app_meeting_and_time() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let logs = folder.path().join("logs");
        let capture = capture(&runtime, Some(logs.clone()));
        let meeting = MeetingId::new("m-1/x");

        capture.save(&meeting, CallApp::Zoom, &[window()]);

        let path = logs.join("call-window-zoom-m-1_x-20261019T143005Z.json");
        let json = fs::read_to_string(path).unwrap();
        let read: Vec<AxNode> = serde_json::from_str(&json).unwrap();
        assert_eq!(read, [window()]);
        assert!(json.contains(r#""role": "AXWindow""#));
        assert!(json.ends_with("]\n"));
    }

    #[test]
    fn dumps_are_wanted_only_with_the_setting_on_and_a_folder() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let with_folder = capture(&runtime, Some(folder.path().to_owned()));
        let without = capture(&runtime, None);
        let on = |capture: &LogFolderCapture| {
            runtime
                .block_on(capture.settings.update(|settings| {
                    settings.developer.capture_call_windows = true;
                }))
                .unwrap();
        };

        assert!(!with_folder.wanted());
        on(&with_folder);
        on(&without);
        assert!(with_folder.wanted());
        assert!(!without.wanted());
    }
}
