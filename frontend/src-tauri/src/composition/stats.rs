//! Wiring for stats and the year in review (ORG-09): the services the
//! `stats` plugin calls, over the SQLite repositories, the profile, the
//! Mac's time zone and the gateway's `recap` route.

use std::sync::Arc;

use deino_core::stats::{
    RecapParts, RecapService, RecapWindow, SqliteRecapRepo, StatsPorts, StatsService,
};
use deino_db::Database;
use deino_domain::SystemZone;
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;

/// The debug flag that opens the year in review outside December, for
/// testing: `DEINO_RECAP_ANYTIME=1 pnpm run tauri:dev`.
const RECAP_ANYTIME_VAR: &str = "DEINO_RECAP_ANYTIME";

/// Builds and manages the stats and recap services.
pub(super) fn attach_stats(app: &AppHandle, database: &Database, wiring: &ActionItemWiring) {
    let ports = StatsPorts::sqlite(
        database,
        Arc::clone(&wiring.settings),
        Arc::new(SystemZone),
        Arc::clone(&wiring.clock),
    );
    let recap = RecapService::new(
        ports.clone(),
        RecapParts {
            recaps: Arc::new(SqliteRecapRepo::new(database.clone())),
            gateway: wiring.gateway.clone(),
            window: recap_window(),
        },
    );
    app.manage(Arc::new(StatsService::new(ports)));
    app.manage(Arc::new(recap));
}

/// December, unless the debug flag is set to `1`.
fn recap_window() -> RecapWindow {
    if std::env::var(RECAP_ANYTIME_VAR).is_ok_and(|value| value == "1") {
        log::info!(target: "stats", "{RECAP_ANYTIME_VAR} is set: the year in review is open");
        RecapWindow::Anytime
    } else {
        RecapWindow::December
    }
}

/// Wires the services that read the library for the user: stats (ORG-09) and
/// pre-meeting briefs (NOT-08).
pub(super) fn attach_read_models(app: &AppHandle, database: &Database, wiring: &ActionItemWiring) {
    attach_stats(app, database, wiring);
    super::briefs::attach_briefs(app, database, wiring);
}
