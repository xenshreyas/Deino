//! The services that run on their own once the pipeline is up.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::Clock;
use tauri::AppHandle;

/// Wires the services that run on their own once the pipeline is up: speaker
/// tags (CAP-05), the live recorder with its auto-stop, the transcript edits
/// (their re-index queue is search's), calendar sync (its scheduler starts
/// here) and the integrations service.
pub(super) fn attach_background(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    super::speaker_tags::attach_speaker_tags(app, database);
    super::recording::attach_recording(app, database, clock);
    super::transcript::attach_transcript_edits(app, database, clock);
    super::calendar::attach_calendar(app, database, clock);
    super::integrations::attach_integrations(app, database, clock);
}
