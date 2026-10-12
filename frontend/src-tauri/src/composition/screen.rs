//! Wiring for screen capture (CAP-09).
//!
//! [`attach_screen`] manages `Arc<ScreenCapture>` over this Mac's
//! `ScreenCaptureKit` ports (`None` elsewhere), the recorder's playhead
//! ([`RecorderClock`]) and the `snapshots` table ([`DatabaseStore`]), and
//! `Arc<ScreenService>` (the per-meeting off switch) over the same rows and
//! the capture. The recorder starts a capture for each recording through
//! `features::screen::CaptureOnStart`.

use std::path::Path;
use std::sync::Arc;

use deino_core::redaction::{
    LocalModelCheck, NoRedaction, Redactor, SettingRedactor, TextRedactor,
};
use deino_core::screen::{
    CaptureControl, ConsentReminders, GatewayDescriber, NewSnapshot, NoDescriptions, OcrError,
    ScreenContextParts, ScreenContextService, ScreenOcr, ScreenRepo, ScreenService,
    SnapshotDescriber, SqliteScreenRepo,
};
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::{Clock, MeetingId};
use deino_llm::Gateway;
use deino_pipeline::ScreenContextReader;
use deino_recording::{Playhead, RecordingService};
use deino_screen::{
    Position, RecordingClock, ScreenCapture, ScreenCaptureParts, ScreenError, SnapshotRecord,
    SnapshotStore,
};
use tauri::{AppHandle, Manager, Runtime};
use tokio::runtime::Handle;

use super::attach::Phase;
use crate::features::screen::ScreenPicker;
use crate::paths::AppPaths;

super::attach::attach!(
    "screen",
    10,
    Phase::Background(|cx| {
        attach_screen(cx.app, cx.database, cx.clock);
    })
);

/// Builds and manages `Arc<ScreenCapture>` and `Arc<ScreenService>`.
pub(crate) fn attach_screen(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let repo = Arc::new(SqliteScreenRepo::new(database.clone()));
    let store = DatabaseStore {
        repo: repo.clone(),
        clock: Arc::clone(clock),
        runtime: tauri::async_runtime::handle().inner().clone(),
    };
    let capture = Arc::new(ScreenCapture::new(ScreenCaptureParts {
        platform: deino_screen::platform(),
        clock: Arc::new(RecorderClock { app: app.clone() }),
        store: Arc::new(store),
    }));
    let control = Arc::new(Control(Arc::clone(&capture)));
    let recordings = AppPaths::resolve(app)
        .map(|paths| paths.recordings())
        .unwrap_or_default();
    app.manage(Arc::new(ScreenService::new(
        repo,
        control,
        Arc::clone(clock),
        recordings,
    )));
    app.manage(capture);
    if let Some(settings) = app.try_state::<Arc<SettingsService>>() {
        let settings: Arc<SettingsService> = Arc::clone(&settings);
        let screen = Arc::clone(&*app.state::<Arc<ScreenService>>());
        let capture = Arc::clone(&*app.state::<Arc<ScreenCapture>>());
        app.manage(Arc::new(ScreenPicker::new(
            capture,
            screen,
            Arc::clone(&settings),
        )));
        app.manage(Arc::new(ConsentReminders::new(settings)));
    }
}

/// The `screen_context` stage's reader: Apple Vision, `vision.describe`
/// over the gateway, and NOT-09's redactor; without the settings or the
/// gateway, snapshots are only read, never described.
pub(super) fn context_reader(
    app: &AppHandle,
    database: &Database,
    recordings: &Path,
) -> Arc<dyn ScreenContextReader> {
    let settings = app
        .try_state::<Arc<SettingsService>>()
        .map(|s| Arc::clone(&*s));
    let gateway = app.try_state::<Arc<Gateway>>().map(|g| Arc::clone(&*g));
    let (describer, redactor): (Arc<dyn SnapshotDescriber>, Arc<dyn TextRedactor>) =
        match (settings, gateway) {
            (Some(settings), Some(gateway)) => {
                let check = Arc::new(LocalModelCheck::new(Arc::clone(&gateway)));
                let redactor = Redactor::with_check(check);
                (
                    Arc::new(GatewayDescriber::new(gateway, settings.clone())),
                    Arc::new(SettingRedactor::new(settings, Arc::new(redactor))),
                )
            }
            _ => (Arc::new(NoDescriptions), Arc::new(NoRedaction)),
        };
    Arc::new(ScreenContextService::new(ScreenContextParts {
        repo: Arc::new(SqliteScreenRepo::new(database.clone())),
        ocr: Arc::new(VisionOcr),
        describer,
        redactor,
        recordings: recordings.to_path_buf(),
    }))
}

/// Apple Vision through `deino-screen`.
struct VisionOcr;

impl ScreenOcr for VisionOcr {
    fn read(&self, png: &Path) -> Result<String, OcrError> {
        deino_screen::recognize_text(png).map_err(|error| match error {
            ScreenError::Unsupported => OcrError::Unsupported,
            other => OcrError::Failed(other.to_string()),
        })
    }
}

/// The recording clock over the live recorder's playhead, looked up on each
/// read (the recorder is managed after screen capture).
struct RecorderClock<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> RecordingClock for RecorderClock<R> {
    fn position(&self, meeting: &MeetingId) -> Position {
        let Some(recorder) = self.app.try_state::<Arc<RecordingService>>() else {
            return Position::Over;
        };
        position(&recorder.playhead(), meeting)
    }
}

/// Where `meeting` is, from the recorder's playhead: any other meeting
/// recording, or none, means its recording is over.
fn position(playhead: &Playhead, meeting: &MeetingId) -> Position {
    match playhead {
        Playhead::Recording { meeting: now, at } if now == meeting => Position::At(*at),
        Playhead::Paused { meeting: now } if now == meeting => Position::Paused,
        Playhead::Recording { .. } | Playhead::Paused { .. } | Playhead::Idle => Position::Over,
    }
}

/// Stores snapshot rows in `snapshots`, blocking the capture's thread
/// (never a runtime thread) on the write.
struct DatabaseStore {
    repo: Arc<SqliteScreenRepo>,
    clock: Arc<dyn Clock>,
    runtime: Handle,
}

impl SnapshotStore for DatabaseStore {
    fn record(&self, snapshot: &SnapshotRecord) -> Result<(), ScreenError> {
        let row = NewSnapshot {
            meeting: snapshot.meeting.clone(),
            at: snapshot.at,
            file_name: snapshot.file_name.clone(),
            width: snapshot.width,
            height: snapshot.height,
            phash: snapshot.hash.to_i64(),
        };
        self.runtime
            .block_on(self.repo.add_snapshot(&row, self.clock.now()))
            .map_err(|error| ScreenError::Store(error.to_string()))
    }
}

/// The running capture, as the service's port.
struct Control(Arc<ScreenCapture>);

impl CaptureControl for Control {
    fn turn_off(&self, meeting: &MeetingId) -> bool {
        self.0.turn_off(meeting)
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::Millis;

    use super::*;

    #[test]
    fn the_core_and_the_capture_name_the_snapshot_folder_alike() {
        assert_eq!(deino_core::screen::SNAPSHOTS_DIR, deino_screen::SNAPSHOTS);
    }

    #[test]
    fn the_playhead_places_only_its_own_meeting() {
        let meeting = MeetingId::new("meeting");
        let other = MeetingId::new("other");
        let at = Playhead::Recording {
            meeting: meeting.clone(),
            at: Millis(1_500),
        };

        assert_eq!(position(&at, &meeting), Position::At(Millis(1_500)));
        assert_eq!(position(&at, &other), Position::Over);
        let paused = Playhead::Paused {
            meeting: meeting.clone(),
        };
        assert_eq!(position(&paused, &meeting), Position::Paused);
        assert_eq!(position(&Playhead::Idle, &meeting), Position::Over);
    }
}
