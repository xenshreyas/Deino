//! The core services every meeting screen reads: meetings and transcripts.

use std::sync::Arc;

use deino_core::meetings::{MeetingService, SqliteMeetingRepo};
use deino_core::segments::SqliteSegmentRepo;
use deino_core::transcript::TranscriptService;
use deino_db::Database;
use deino_domain::Clock;
use tauri::{Manager, Runtime};

/// The core services backed by the app's database.
pub(crate) struct Services {
    pub(super) meetings: Arc<MeetingService>,
    transcript: Arc<TranscriptService>,
}

impl Services {
    /// Builds every service over `database`, reading time from `clock`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let meeting_repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
        let transcript = TranscriptService::new(segments);
        Self {
            meetings: Arc::new(MeetingService::new(meeting_repo, Arc::clone(clock))),
            transcript: Arc::new(transcript),
        }
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.meetings);
        app.manage(self.transcript);
    }
}
