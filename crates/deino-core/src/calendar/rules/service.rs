//! [`AutoRecordService`]: the auto-record policy over stored events, the
//! user's switches and their choices; what Home's Coming up and the
//! auto-start triggers ask.

use std::collections::BTreeMap;
use std::sync::Arc;

use deino_domain::{CalendarEventId, Clock, SeriesId};
use serde::{Deserialize, Serialize};

use super::choice_repo::SeriesChoiceRepo;
use super::policy::{Choices, EventFacts, RecordDecision, StartTrigger, decide, starts_recording};
use crate::calendar::{
    CalendarError, CalendarEvent, CalendarEventRepo, CalendarService, TimeRange,
};
use crate::profile::InternalDomains;
use crate::settings::{Settings, SettingsService};

/// How far a choice reaches on a recurring event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ChoiceScope {
    /// "Just this one".
    ThisEvent,
    /// "All occurrences"; on a one-off event, the same as `ThisEvent`.
    AllOccurrences,
}

/// The user's record toggle on an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordChoice {
    /// `true` always records, `false` never; `None` follows the switches again.
    pub record: Option<bool>,
    /// This event or its whole series.
    pub scope: ChoiceScope,
}

/// An upcoming event and whether it records on its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlannedEvent {
    /// The event.
    pub event: CalendarEvent,
    /// The policy's answer for it.
    pub decision: RecordDecision,
}

/// What [`AutoRecordService`] reads and writes.
pub struct AutoRecordDeps {
    /// The visible upcoming events.
    pub calendar: Arc<CalendarService>,
    /// One event's choice.
    pub events: Arc<dyn CalendarEventRepo>,
    /// Series choices.
    pub series: Arc<dyn SeriesChoiceRepo>,
    /// The switches and the user's addresses.
    pub settings: Arc<SettingsService>,
    /// When a choice was made.
    pub clock: Arc<dyn Clock>,
}

/// Decides and changes which events record on their own.
pub struct AutoRecordService {
    deps: AutoRecordDeps,
}

/// What every decision in one call shares.
struct Context {
    settings: Settings,
    own: InternalDomains,
    series: BTreeMap<SeriesId, bool>,
}

impl Context {
    fn decide(&self, event: &CalendarEvent) -> RecordDecision {
        let choices = Choices {
            event: event.record_override,
            series: event
                .series_id
                .as_ref()
                .and_then(|id| self.series.get(id).copied()),
        };
        let facts = EventFacts::of(event, &self.own);
        decide(&self.settings.calendar.auto_record, &facts, choices)
    }
}

impl AutoRecordService {
    /// A service over `deps`.
    pub fn new(deps: AutoRecordDeps) -> Self {
        Self { deps }
    }

    /// The visible events overlapping `range`, each with its decision.
    pub async fn plan(&self, range: TimeRange) -> Result<Vec<PlannedEvent>, CalendarError> {
        let events = self.deps.calendar.upcoming(range).await?;
        let context = self.context().await?;
        Ok(events
            .into_iter()
            .map(|event| PlannedEvent {
                decision: context.decide(&event),
                event,
            })
            .collect())
    }

    /// Whether `event` records on its own.
    pub async fn decision(&self, event: &CalendarEvent) -> Result<RecordDecision, CalendarError> {
        Ok(self.context().await?.decide(event))
    }

    /// True when `trigger` should start recording event `id`: the meeting
    /// app took the microphone (CAP-06), or its start time came while its
    /// note is open (the meeting ticker).
    pub async fn starts_on(
        &self,
        id: CalendarEventId,
        trigger: StartTrigger,
    ) -> Result<bool, CalendarError> {
        let event = self.deps.calendar.event(id).await?;
        Ok(starts_recording(self.decision(&event).await?, trigger))
    }

    /// Applies the user's toggle on `id` and returns its new decision.
    pub async fn choose(
        &self,
        id: CalendarEventId,
        choice: RecordChoice,
    ) -> Result<RecordDecision, CalendarError> {
        let at = self.deps.clock.now();
        let event = self.deps.calendar.event(id).await?;
        match (choice.scope, event.series_id.as_ref()) {
            (ChoiceScope::AllOccurrences, Some(series)) => {
                self.deps
                    .series
                    .set_series_choice(series, choice.record, at)
                    .await?;
                if choice.record.is_none() {
                    self.deps.events.set_record_override(&id, None, at).await?;
                }
            }
            _ => {
                self.deps
                    .events
                    .set_record_override(&id, choice.record, at)
                    .await?;
            }
        }
        let event = self.deps.calendar.event(id).await?;
        self.decision(&event).await
    }

    async fn context(&self) -> Result<Context, CalendarError> {
        let settings = self.deps.settings.get().await;
        Ok(Context {
            own: InternalDomains::of(&settings.profile),
            series: self.deps.series.series_choices().await?,
            settings,
        })
    }
}
