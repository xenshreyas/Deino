//! Wiring for onboarding (FND-11): the permission rules over this app's
//! operating-system permissions, and the sample meeting's importer over the
//! SQLite repositories.

use std::sync::Arc;

use deino_core::action_items::SqliteActionItemRepo;
use deino_core::enhance::SqliteEnhancedNotesRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::permissions::PermissionService;
use deino_core::sample::{SampleImporter, SamplePorts};
use deino_core::segments::SqliteSegmentRepo;
use deino_db::Database;
use deino_domain::Clock;
use tauri::{AppHandle, Manager, Runtime};

use super::attach::Phase;
use crate::permissions::SystemPermissions;

/// Manages `Arc<PermissionService>` over the real permissions.
pub(crate) fn attach_permissions<R: Runtime>(app: &AppHandle<R>) {
    let gate = Arc::new(SystemPermissions::new(app.clone()));
    app.manage(Arc::new(PermissionService::new(gate)));
}

super::attach::attach!(
    "sample",
    40,
    Phase::Local(|cx| attach_sample(cx.app, cx.database, cx.clock))
);

/// Manages `Arc<SampleImporter>` for the `onboarding` plugin.
pub(crate) fn attach_sample<R: Runtime>(
    app: &AppHandle<R>,
    database: &Database,
    clock: &Arc<dyn Clock>,
) {
    app.manage(Arc::new(sample_importer(database, clock)));
}

/// The sample meeting's importer over `database`'s repositories.
pub(crate) fn sample_importer(database: &Database, clock: &Arc<dyn Clock>) -> SampleImporter {
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    SampleImporter::new(SamplePorts {
        meetings: meetings.clone(),
        trash: meetings,
        speakers: segments.clone(),
        segments,
        notes: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
        action_items: Arc::new(SqliteActionItemRepo::new(database.clone())),
        clock: Arc::clone(clock),
    })
}
