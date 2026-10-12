//! Wires the meeting player (TRN-03, ADR 0040), clips (TRN-04) and comments
//! (TRN-06, ADR 0048): `PlayerService` over the meetings, the recordings
//! folder (the same folder the asset protocol is scoped to) and the
//! Rust-side save dialog; `ClipService` over the `clips` table, the player's
//! view of each recording, that folder, the `AVFoundation` cutter and Finder;
//! `CommentService` over its tables.

use std::sync::Arc;

use deino_core::clips::{ClipDeps, ClipService, FsClipFiles, SqliteClipRepo};
use deino_core::comments::{CommentService, SqliteCommentRepo};
use deino_core::export::SaveDialog;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::player::{FsMediaFiles, PlayerDeps, PlayerService};
use deino_db::Database;
use deino_domain::Clock;
use tauri::{AppHandle, Manager};

use super::attach::Phase;
use crate::features::clips::platform::{AvClipCutter, OpenFinder};
use crate::features::export::TauriDialogs;
use crate::paths::AppPaths;

super::attach::attach!(
    "player",
    50,
    Phase::Local(|cx| attach_player(cx.app, cx.database, cx.clock))
);

/// Manages `Arc<PlayerService>`, `Arc<ClipService>` and
/// `Arc<CommentService>`. Without the app's folders only comments are
/// wired, and the error is logged.
pub(super) fn attach_player(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let comments = CommentService::new(
        Arc::new(SqliteCommentRepo::new(database.clone())),
        Arc::clone(clock),
    );
    app.manage(Arc::new(comments));
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            log::error!(target: "startup", "no recordings folder, so no player or clips: {error}");
            return;
        }
    };
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let dialog: Arc<dyn SaveDialog> = Arc::new(TauriDialogs::new(app.clone()));
    let player = Arc::new(PlayerService::new(PlayerDeps {
        lifecycle: meetings.clone(),
        meetings,
        files: Arc::new(FsMediaFiles::new(paths.recordings())),
        dialog,
    }));
    let clips = ClipService::new(ClipDeps {
        repo: Arc::new(SqliteClipRepo::new(database.clone())),
        sources: player.clone(),
        files: Arc::new(FsClipFiles::new(paths.recordings())),
        cutter: Arc::new(AvClipCutter),
        finder: Arc::new(OpenFinder),
        clock: Arc::clone(clock),
    });
    app.manage(player);
    app.manage(Arc::new(clips));
}
