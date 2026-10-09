//! Wiring for the notepad (NOT-01): the notes service over SQLite and the
//! meetings folder, the people search behind @mentions, and the record of
//! files dropped on the window.

use std::sync::Arc;

use deino_core::meetings::SqliteMeetingRepo;
use deino_core::notes::{FsNoteImages, NotePorts, NoteService, SqliteNotesRepo};
use deino_core::people::{PeopleService, SqlitePeopleRepo};
use deino_db::Database;
use deino_domain::Clock;
use tauri::{AppHandle, Manager, Runtime};

use crate::features::notes::{DroppedFiles, TauriNotesEvents};
use crate::paths::AppPaths;

/// The notes and people services over `database`, with note images under
/// `paths.meetings()`.
pub(crate) fn notes_services<R: Runtime>(
    app: &AppHandle<R>,
    database: &Database,
    clock: &Arc<dyn Clock>,
    paths: &AppPaths,
) -> (Arc<NoteService>, Arc<PeopleService>) {
    let ports = NotePorts {
        notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        images: Arc::new(FsNoteImages::new(paths.meetings())),
        events: Arc::new(TauriNotesEvents::new(app.clone())),
    };
    let notes = Arc::new(NoteService::new(ports, Arc::clone(clock)));
    let people = Arc::new(PeopleService::new(Arc::new(SqlitePeopleRepo::new(
        database.clone(),
    ))));
    (notes, people)
}

/// Builds and manages the notes and people services and the dropped-files
/// record. The paths come from Tauri's resolver; without them (a broken
/// install) the notepad is unavailable and the rest of the app still runs.
pub(crate) fn attach_notepad(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            log::error!(target: "notes", "no app folders, the notepad is unavailable: {error}");
            return;
        }
    };
    let (notes, people) = notes_services(app, database, clock, &paths);
    app.manage(notes);
    app.manage(people);
    app.manage(Arc::new(DroppedFiles::default()));
}
