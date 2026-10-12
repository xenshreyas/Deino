//! Wiring for pre-meeting briefs (NOT-08): the `BriefService` the `briefs`
//! plugin and ORG-02's event note read, over SQLite, the Mac's time zone and
//! the gateway's `brief` route, with redaction when the setting is on
//! (NOT-09); and the background job, ticked every minute over the visible
//! calendar events. Needs the calendar service, so it runs after
//! `attach_calendar`.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::briefs::{
    BriefDeps, BriefScheduler, BriefService, BriefSources, BriefWriter, GatewayBriefWriter,
    SchedulerDeps, SqliteBriefRepo,
};
use deino_core::calendar::{CalendarService, SqliteCalendarRepo};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::redaction::{LocalModelCheck, Redactor, SettingRedactor};
use deino_db::Database;
use deino_domain::{LocalZone, SystemZone};
use deino_llm::brief::BriefAnswer;
use deino_llm::{Gateway, LlmError, Prompt, ProviderId};
use tauri::{AppHandle, Manager};

use super::action_items::ActionItemWiring;
use crate::features::briefs::start_brief_job;

/// Manages `Arc<BriefService>` and, with a gateway and the calendar wired,
/// starts the brief job.
pub(super) fn attach_briefs(app: &AppHandle, database: &Database, wiring: &ActionItemWiring) {
    let zone: Arc<dyn LocalZone> = Arc::new(SystemZone);
    let mut sources = BriefSources::sqlite(database, Arc::clone(&zone));
    let writer: Arc<dyn BriefWriter> = match &wiring.gateway {
        Some(gateway) => {
            sources = sources.with_redactor(redactor(wiring, gateway));
            Arc::new(GatewayBriefWriter::new(Arc::clone(gateway)))
        }
        None => Arc::new(Unwired),
    };
    let service = Arc::new(BriefService::new(BriefDeps {
        store: Arc::new(SqliteBriefRepo::new(database.clone())),
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        sources,
        writer,
        clock: Arc::clone(&wiring.clock),
    }));
    app.manage(Arc::clone(&service));
    let calendar = app
        .try_state::<Arc<CalendarService>>()
        .map(|calendar| Arc::clone(&calendar));
    let (Some(calendar), Some(_)) = (calendar, &wiring.gateway) else {
        return log::info!(target: "briefs", "no model or calendar; the brief job is off");
    };
    start_brief_job(Arc::new(BriefScheduler::new(SchedulerDeps {
        service,
        events: calendar,
        switch: wiring.settings.clone(),
        clock: Arc::clone(&wiring.clock),
        zone,
    })));
}

/// Hides secrets when "Hide sensitive information" is on, as for notes.
fn redactor(
    wiring: &ActionItemWiring,
    gateway: &Arc<Gateway>,
) -> Arc<dyn deino_core::redaction::TextRedactor> {
    let check = Arc::new(LocalModelCheck::new(Arc::clone(gateway)));
    Arc::new(SettingRedactor::new(
        wiring.settings.clone(),
        Arc::new(Redactor::with_check(check)),
    ))
}

/// No model could be wired at startup: writing a brief fails as unavailable
/// (the job is not started, so only a direct call gets here).
struct Unwired;

#[async_trait]
impl BriefWriter for Unwired {
    async fn write(&self, _prompt: Prompt) -> Result<BriefAnswer, LlmError> {
        Err(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "no model is set up for briefs".to_owned(),
        })
    }
}
