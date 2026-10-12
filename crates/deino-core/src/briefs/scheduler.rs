//! [`BriefScheduler`]: the background `brief` job. The app ticks it about
//! once a minute without any UI; each tick prepares the briefs whose
//! 06:00 or one-hour-before run is due ([`due_trigger`]).
//!
//! It is idempotent: a run whose inputs are unchanged writes nothing and
//! calls no model (the `briefs.inputs_hash` fingerprint), so the in-memory
//! record of the runs done only saves reading the inputs again; after a
//! restart each due run looks once more and finds nothing to do.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{CalendarEventId, Clock, LocalZone};

use super::{BriefError, BriefService, BriefTrigger, due_trigger, due_window, qualifies};
use crate::calendar::{CalendarError, CalendarEvent, CalendarService, TimeRange};
use crate::settings::{BriefSettings, SettingsService};

/// The upcoming events the job may brief.
#[async_trait]
pub trait UpcomingEvents: Send + Sync {
    /// The visible events starting from `from` until `to`, by start time.
    async fn starting(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, CalendarError>;
}

#[async_trait]
impl UpcomingEvents for CalendarService {
    async fn starting(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        let events = self
            .upcoming(TimeRange {
                start: from,
                end: to,
            })
            .await?;
        Ok(events
            .into_iter()
            .filter(|event| event.start_at >= from)
            .collect())
    }
}

/// The user's brief settings.
#[async_trait]
pub trait BriefSwitch: Send + Sync {
    /// The current settings.
    async fn brief_settings(&self) -> BriefSettings;
}

#[async_trait]
impl BriefSwitch for SettingsService {
    async fn brief_settings(&self) -> BriefSettings {
        self.get().await.briefs
    }
}

/// What the job needs.
#[derive(Clone)]
pub struct SchedulerDeps {
    /// Prepares each brief.
    pub service: Arc<BriefService>,
    /// The calendar.
    pub events: Arc<dyn UpcomingEvents>,
    /// The user's settings.
    pub switch: Arc<dyn BriefSwitch>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The user's time zone.
    pub zone: Arc<dyn LocalZone>,
}

/// The background job that keeps upcoming briefs prepared.
pub struct BriefScheduler {
    deps: SchedulerDeps,
    done: Mutex<BTreeSet<(CalendarEventId, BriefTrigger)>>,
}

impl BriefScheduler {
    /// A job that has run nothing yet.
    pub fn new(deps: SchedulerDeps) -> Self {
        Self {
            deps,
            done: Mutex::new(BTreeSet::new()),
        }
    }

    /// Runs every due, not yet run trigger of a qualifying event; returns
    /// the events it prepared. A failing event is logged and retried at
    /// its next trigger, without stopping the others.
    pub async fn tick(&self) -> Result<Vec<CalendarEventId>, BriefError> {
        let settings = self.deps.switch.brief_settings().await;
        if !settings.enabled {
            return Ok(Vec::new());
        }
        let now = self.deps.clock.now();
        let (from, to) = due_window(now, self.deps.zone.as_ref());
        let events = self.deps.events.starting(from, to).await?;
        self.forget_all_but(&events);
        let mut prepared = Vec::new();
        for event in events.iter().filter(|e| qualifies(e, settings.audience)) {
            let Some(trigger) = due_trigger(event.start_at, now, self.deps.zone.as_ref()) else {
                continue;
            };
            if !self.first_run(event.id, trigger) {
                continue;
            }
            match self.deps.service.prepare(event).await {
                Ok(_) => prepared.push(event.id),
                Err(error) => {
                    log::warn!(target: "briefs", "brief for event {} failed: {error}", event.id);
                }
            }
        }
        Ok(prepared)
    }

    /// Marks `trigger` of `event` as run; false if it already was.
    fn first_run(&self, event: CalendarEventId, trigger: BriefTrigger) -> bool {
        let mut done = self.done.lock().unwrap_or_else(PoisonError::into_inner);
        done.insert((event, trigger))
    }

    /// Drops the record of events no longer coming up, so it stays small.
    fn forget_all_but(&self, events: &[CalendarEvent]) {
        let upcoming: BTreeSet<CalendarEventId> = events.iter().map(|event| event.id).collect();
        let mut done = self.done.lock().unwrap_or_else(PoisonError::into_inner);
        done.retain(|(event, _)| upcoming.contains(event));
    }
}
