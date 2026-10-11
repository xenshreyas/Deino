//! Wiring for people and companies (ORG-04): the entity services the
//! `people` and `companies` plugins call, the summariser on the gateway's
//! `entity.summary` route, company logos over the `favicons` network service
//! cached under `<data>/logos`, and the updater behind the pipeline's
//! `update_entities` stage.

use std::sync::Arc;

use deino_core::entities::{
    AttendanceService, CompanyService, EntityPorts, EntitySummarizer, EntityUpdatePorts,
    EntityUpdateService, EntityUpdater, FaviconSource, FsLogoFiles, LogoPorts, LogoService,
    PersonService,
};
use deino_core::segments::SqliteSegmentRepo;
use deino_db::Database;
use deino_domain::{Clock, ServiceId};
use deino_llm::Gateway;
use deino_net::Net;
use tauri::{AppHandle, Manager, Runtime};

use crate::paths::AppPaths;

/// Builds and manages the entity services, and returns the updater for the
/// pipeline. Without a gateway summaries are skipped (`no_model`); without
/// the network or the app folders there are no logos.
pub(super) fn attach_entities(
    app: &AppHandle,
    database: &Database,
    gateway: Option<Arc<Gateway>>,
    clock: &Arc<dyn Clock>,
) -> Arc<dyn EntityUpdater> {
    let ports = EntityPorts::sqlite(database, Arc::clone(clock));
    let summarizer = Arc::new(EntitySummarizer::new(ports.clone(), gateway));
    let logos = logo_service(app, &ports);
    app.manage(Arc::new(PersonService::new(ports.clone())));
    app.manage(Arc::new(CompanyService::new(ports.clone())));
    app.manage(Arc::new(AttendanceService::new(ports.clone())));
    app.manage(Arc::clone(&summarizer));
    if let Some(logos) = &logos {
        app.manage(Arc::clone(logos));
    }
    Arc::new(EntityUpdateService::new(EntityUpdatePorts {
        entities: ports,
        speakers: Arc::new(SqliteSegmentRepo::new(database.clone())),
        summarizer,
        logos,
    }))
}

/// Company logos over a client scoped to `favicons`, so the user's switch
/// and offline mode apply; `None` (logged) without the network or folders.
fn logo_service<R: Runtime>(app: &AppHandle<R>, ports: &EntityPorts) -> Option<Arc<LogoService>> {
    let Some(net) = app.try_state::<Net>() else {
        log::warn!(target: "entities", "the network is not wired; company logos are off");
        return None;
    };
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            log::error!(target: "entities", "no app folders, company logos are off: {error}");
            return None;
        }
    };
    Some(Arc::new(LogoService::new(LogoPorts {
        companies: Arc::clone(&ports.companies),
        directory: Arc::clone(&ports.directory),
        writes: Arc::clone(&ports.writes),
        source: Arc::new(FaviconSource::new(Arc::new(
            net.scoped(ServiceId::Favicons),
        ))),
        files: Arc::new(FsLogoFiles::new(paths.logos())),
        clock: Arc::clone(&ports.clock),
    })))
}
