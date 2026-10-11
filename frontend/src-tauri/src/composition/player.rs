//! Wires the meeting player (TRN-03, ADR 0038): `PlayerService` over the
//! meetings, the recordings folder (the same folder the asset protocol is
//! scoped to) and the Rust-side save dialog.

use std::sync::Arc;

use deino_core::export::SaveDialog;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::player::{FsMediaFiles, PlayerDeps, PlayerService};
use deino_db::Database;
use tauri::{AppHandle, Manager};

use crate::features::export::TauriDialogs;
use crate::paths::AppPaths;

/// Manages `Arc<PlayerService>`; logs and skips it when the app's folders
/// cannot be resolved.
pub(super) fn attach_player(app: &AppHandle, database: &Database) {
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            log::error!(target: "startup", "no recordings folder, so no player: {error}");
            return;
        }
    };
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let dialog: Arc<dyn SaveDialog> = Arc::new(TauriDialogs::new(app.clone()));
    let service = PlayerService::new(PlayerDeps {
        lifecycle: meetings.clone(),
        meetings,
        files: Arc::new(FsMediaFiles::new(paths.recordings())),
        dialog,
    });
    app.manage(Arc::new(service));
}
