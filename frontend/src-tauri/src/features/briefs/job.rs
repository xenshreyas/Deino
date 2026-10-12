//! The background `brief` job in the app: ticks
//! `deino_core::briefs::BriefScheduler` once a minute for the app's
//! lifetime, with no UI. The rules (06:00, an hour before, which events,
//! idempotence) are the core's.

use std::sync::Arc;
use std::time::Duration;

use deino_core::briefs::BriefScheduler;

/// How often the job checks the clock: a minute keeps the 06:00 and
/// hour-before runs on time after sleep, at no measurable cost (a tick with
/// nothing due reads only the calendar).
const TICK: Duration = Duration::from_mins(1);

/// Ticks `job` once a minute until the app exits. Owned for the app's
/// lifetime: the task ends with the async runtime at exit.
pub(crate) fn start_brief_job(job: Arc<BriefScheduler>) {
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(TICK);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            match job.tick().await {
                Ok(prepared) if !prepared.is_empty() => {
                    log::info!(target: "briefs", "looked at {} upcoming briefs", prepared.len());
                }
                Ok(_) => {}
                Err(error) => log::warn!(target: "briefs", "the brief job failed: {error}"),
            }
        }
    });
}
