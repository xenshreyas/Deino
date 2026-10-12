//! Wiring for insights (AI-06): the service the `insights` plugin calls and
//! the generator behind the pipeline's `insights` stage (the gateway's
//! `extract.insights` route), over the SQLite repository, the settings
//! (the user's own domains), the system clock and the Mac's time zone.

use std::sync::Arc;

use deino_core::enhance::MaterialSources;
use deino_core::export::{FsVault, SystemZone};
use deino_core::insights::{
    GatewayInsightModel, GenerateDeps, GenerateService, InsightFiles, InsightGenerator,
    InsightModel, InsightPorts, InsightService, NoInsightModel, SqliteInsightRepo,
};
use deino_db::Database;
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;
use crate::features::export::TauriDialogs;
use crate::features::insights::PipelineScheduler;

/// Builds and manages the insights service, and returns the generator for
/// the pipeline.
pub(super) fn attach_insights(
    app: &AppHandle,
    database: &Database,
    wiring: &ActionItemWiring,
) -> Arc<dyn InsightGenerator> {
    let repo = Arc::new(SqliteInsightRepo::new(database.clone()));
    let service = InsightService::new(InsightPorts {
        defs: repo.clone(),
        results: repo.clone(),
        records: repo.clone(),
        domains: wiring.settings.clone(),
        names: repo.clone(),
        scheduler: Arc::new(PipelineScheduler::new(app.clone())),
        files: InsightFiles {
            save_dialog: Arc::new(TauriDialogs::new(app.clone())),
            vault: Arc::new(FsVault),
            zone: Arc::new(SystemZone),
        },
        clock: wiring.clock.clone(),
    });
    app.manage(Arc::new(service));
    let model: Arc<dyn InsightModel> = match &wiring.gateway {
        Some(gateway) => Arc::new(GatewayInsightModel::new(gateway.clone())),
        None => Arc::new(NoInsightModel),
    };
    Arc::new(GenerateService::new(GenerateDeps {
        defs: repo.clone(),
        results: repo.clone(),
        records: repo,
        domains: wiring.settings.clone(),
        material: MaterialSources::sqlite(database),
        model,
        clock: wiring.clock.clone(),
        zone: Arc::new(SystemZone),
    }))
}
