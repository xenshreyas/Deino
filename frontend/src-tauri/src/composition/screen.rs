//! Wiring for screen capture (CAP-09).
//!
//! [`attach_screen`] manages `Arc<ScreenCapture>` over this Mac's
//! `ScreenCaptureKit` ports (`None` elsewhere), the recorder's playhead
//! ([`RecorderClock`]) and the `snapshots` table ([`DatabaseStore`]), and
//! `Arc<ScreenService>` (the per-meeting off switch) over the same rows and
//! the capture. The recorder starts a capture for each recording through
//! `features::screen::CaptureOnStart`.

use std::sync::Arc;

use deino_core::screen::{
    CaptureControl, NewSnapshot, ScreenRepo, ScreenService, SqliteScreenRepo,
};
use deino_db::Database;
use deino_domain::{Clock, MeetingId};
use deino_recording::{Playhead, RecordingService};
use deino_screen::{
    Position, RecordingClock, ScreenCapture, ScreenCaptureParts, ScreenError, SnapshotRecord,
    SnapshotStore,
};
use tauri::{AppHandle, Manager, Runtime};
use tokio::runtime::Handle;

use super::attach::Phase;

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
    app.manage(Arc::new(ScreenService::new(
        repo,
        control,
        Arc::clone(clock),
    )));
    app.manage(capture);
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
