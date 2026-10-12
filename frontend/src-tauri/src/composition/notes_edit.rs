//! Wiring for editing notes by asking (NOT-06): the `NoteEditService`
//! behind the `notes-edit` plugin, over the enhanced-notes repository, the
//! transcript, the gateway's `notes.edit` route and, when "Hide sensitive
//! information" is on, NOT-09's redactor (patterns, then the built-in local
//! model's check, as for the notes themselves).

use std::sync::Arc;

use deino_core::enhance::{MaterialSources, SqliteEnhancedNotesRepo};
use deino_core::notes::edit::{NoteEditPorts, NoteEditService};
use deino_core::redaction::{
    LocalModelCheck, NoCheck, Redactor, SettingRedactor, SpanCheck, TextRedactor,
};
use deino_db::Database;
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;
use super::attach::Phase;

super::attach::attach!(
    "notes-edit",
    30,
    Phase::ReadModel(|cx, ai| attach_notes_edit(cx.app, cx.database, ai))
);

/// Manages `Arc<NoteEditService>`; without a gateway only renames work.
pub(super) fn attach_notes_edit(app: &AppHandle, database: &Database, wiring: &ActionItemWiring) {
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let service = NoteEditService::new(
        NoteEditPorts {
            reader: repo.clone(),
            writer: repo,
            transcript: Arc::new(MaterialSources::sqlite(database)),
            redactor: redactor(wiring),
            gateway: wiring.gateway.clone(),
        },
        Arc::clone(&wiring.clock),
    );
    app.manage(Arc::new(service));
}

/// Hides secrets when the setting is on; the likely ones are checked by
/// the built-in model when a gateway is wired.
fn redactor(wiring: &ActionItemWiring) -> Arc<dyn TextRedactor> {
    let check: Arc<dyn SpanCheck> = match &wiring.gateway {
        Some(gateway) => Arc::new(LocalModelCheck::new(Arc::clone(gateway))),
        None => Arc::new(NoCheck),
    };
    Arc::new(SettingRedactor::new(
        wiring.settings.clone(),
        Arc::new(Redactor::with_check(check)),
    ))
}
