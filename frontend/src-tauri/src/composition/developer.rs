//! Wiring for Settings → Developer: the command-line setup (DEV-02/04) over
//! the real file system, the sidecar beside the app's executable and the
//! user's home folder, and the fixture seeder of debug builds (FND-04).

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::cli_setup::{CliSetupService, Entry, FsLinks, Links};
use tauri::{AppHandle, Manager, Runtime};

/// The CLI sidecar's file name (`externalBin` `binaries/cli`). Tauri puts
/// sidecars beside the main executable: `Deino.app/Contents/MacOS/cli` in a
/// bundle, `target/<profile>/cli` in a dev build (the copy tauri-build makes
/// of the staged `frontend/src-tauri/binaries/cli-<target triple>`).
const SIDECAR: &str = "cli";

/// Manages `Arc<CliSetupService>`. Without a home folder there is nowhere
/// to install, so the service is left out and the commands fail as
/// unavailable.
pub(crate) fn attach_developer<R: Runtime>(app: &AppHandle<R>) {
    let home = match app.path().home_dir() {
        Ok(home) => home,
        Err(error) => {
            log::warn!(target: "developer", "no home folder for the CLI install: {error}");
            return;
        }
    };
    let links = Arc::new(FsLinks);
    let bundled = sidecar(links.as_ref());
    app.manage(Arc::new(CliSetupService::new(links, bundled, home)));
}

/// The bundled CLI, when this build has one beside its executable.
fn sidecar(links: &dyn Links) -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let path = executable.parent()?.join(SIDECAR);
    (links.inspect(&path).ok()? == Entry::File).then_some(path)
}

#[cfg(debug_assertions)]
super::attach::attach!(
    "fixture-seeder",
    70,
    super::attach::Phase::Local(|cx| {
        attach_fixture_seeder(cx.app, cx.database);
    })
);

/// Debug builds only: gives the `dev` plugin a seeder that writes the fixture
/// meetings into the app's database (Settings → Developer).
#[cfg(debug_assertions)]
pub(super) fn attach_fixture_seeder(app: &AppHandle, database: &deino_db::Database) {
    let seeder = deino_testkit::FixtureSeeder::new(database.clone());
    app.manage(Arc::new(seeder));
}
