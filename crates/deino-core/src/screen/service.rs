//! [`ScreenService`]: the per-meeting off switch and the snapshot list,
//! over the rows and the running capture.

use std::path::PathBuf;
use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{Clock, MeetingId};

use super::{SNAPSHOTS_DIR, ScreenRepo, Snapshot};

/// The running screen capture (the app's `deino_screen::ScreenCapture`).
pub trait CaptureControl: Send + Sync {
    /// Stops capturing `meeting`; false when it was not being captured.
    fn turn_off(&self, meeting: &MeetingId) -> bool;
}

/// Screen capture's rules over its rows (see the module docs).
pub struct ScreenService {
    repo: Arc<dyn ScreenRepo>,
    control: Arc<dyn CaptureControl>,
    clock: Arc<dyn Clock>,
    recordings: PathBuf,
}

impl ScreenService {
    /// The service over the rows, the running capture, the clock and the
    /// folder holding every meeting's recording folder.
    pub fn new(
        repo: Arc<dyn ScreenRepo>,
        control: Arc<dyn CaptureControl>,
        clock: Arc<dyn Clock>,
        recordings: PathBuf,
    ) -> Self {
        Self {
            repo,
            control,
            clock,
            recordings,
        }
    }

    /// "Turn off for this meeting": stops a running capture of `meeting`
    /// (what it captured is kept) and keeps capture off for it, so a
    /// resumed recording does not restart it. True when a capture stopped.
    pub async fn turn_off(&self, meeting: &MeetingId) -> Result<bool, DbError> {
        self.repo
            .set_capture(meeting, false, self.clock.now())
            .await?;
        Ok(self.control.turn_off(meeting))
    }

    /// Whether the user turned capture off for `meeting`. A read failure
    /// counts as off: capturing against the user's choice is worse than
    /// missing a capture.
    pub async fn turned_off(&self, meeting: &MeetingId) -> bool {
        match self.repo.capture_choice(meeting).await {
            Ok(choice) => choice == Some(false),
            Err(error) => {
                log::warn!(target: "screen", "could not read the screen capture choice of {meeting}: {error}");
                true
            }
        }
    }

    /// The snapshots of `meeting`, in time order.
    pub async fn snapshots(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        self.repo.snapshots(meeting).await
    }

    /// Deletes snapshot `id` of `meeting`: its row, then its PNG. False
    /// when there was no such snapshot. A PNG that cannot be removed is
    /// logged; the row is gone either way, so the picture is no longer shown.
    pub async fn delete_snapshot(&self, meeting: &MeetingId, id: i64) -> Result<bool, DbError> {
        let Some(name) = self.repo.delete_snapshot(meeting, id).await? else {
            return Ok(false);
        };
        let path = self
            .recordings
            .join(meeting.as_str())
            .join(SNAPSHOTS_DIR)
            .join(name);
        match tokio::fs::remove_file(&path).await {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                log::warn!(target: "screen", "could not remove snapshot {id} of {meeting}: {error}");
            }
            Ok(()) | Err(_) => {}
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;
    use crate::screen::FakeScreenRepo;

    /// A capture of one meeting, until it is turned off.
    #[derive(Default)]
    struct Running(Mutex<Option<MeetingId>>);

    impl CaptureControl for Running {
        fn turn_off(&self, meeting: &MeetingId) -> bool {
            let mut running = self.0.lock().unwrap();
            let stopped = running.as_ref() == Some(meeting);
            if stopped {
                *running = None;
            }
            stopped
        }
    }

    fn service(running: Option<&str>) -> ScreenService {
        let control = Running(Mutex::new(running.map(MeetingId::new)));
        ScreenService::new(
            Arc::new(FakeScreenRepo::default()),
            Arc::new(control),
            Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
            PathBuf::from("no-recordings"),
        )
    }

    #[tokio::test]
    async fn turning_a_meeting_off_stops_it_and_keeps_it_off() {
        let service = service(Some("meeting-a"));
        let meeting = MeetingId::new("meeting-a");
        assert!(!service.turned_off(&meeting).await);

        assert!(service.turn_off(&meeting).await.unwrap());

        assert!(service.turned_off(&meeting).await);
        assert!(
            !service.turn_off(&meeting).await.unwrap(),
            "already stopped"
        );
    }

    #[tokio::test]
    async fn a_meeting_that_is_not_recording_can_still_be_turned_off() {
        let service = service(None);
        let meeting = MeetingId::new("meeting-b");

        assert!(!service.turn_off(&meeting).await.unwrap());
        assert!(service.turned_off(&meeting).await);
        assert!(service.snapshots(&meeting).await.unwrap().is_empty());
    }
}
