//! What the UI shows of runs: a meeting's ⋯ → Automations menu and an
//! automation's "Matching meetings" tab.

use deino_domain::{AutomationId, MeetingId};
use serde::Serialize;

use super::{Automation, AutomationError, AutomationRun, AutomationService};
use crate::insights::PastMeeting;

/// One automation in a meeting's ⋯ → Automations menu.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAutomation {
    /// The automation.
    pub automation: Automation,
    /// True when its conditions match the meeting (it may still be off).
    pub matches: bool,
    /// Its latest run on the meeting.
    pub run: Option<AutomationRun>,
}

/// One past meeting on an automation's "Matching meetings" tab.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MatchingMeeting {
    /// The meeting.
    pub meeting: PastMeeting,
    /// The automation's latest run on it.
    pub run: Option<AutomationRun>,
}

impl AutomationService {
    /// Every automation with whether it matches `meeting` and its run
    /// there; matching ones first, then by name.
    pub async fn for_meeting(
        &self,
        meeting: &MeetingId,
    ) -> Result<Vec<MeetingAutomation>, AutomationError> {
        let facts = self
            .ports
            .facts
            .facts(meeting)
            .await?
            .ok_or_else(|| AutomationError::MeetingNotFound(meeting.clone()))?;
        let mut runs = self.ports.runs.for_meeting(meeting).await?;
        let mut menu: Vec<MeetingAutomation> = self
            .list()
            .await?
            .into_iter()
            .map(|automation| MeetingAutomation {
                matches: automation.conditions.matches(&facts),
                run: take_run(&mut runs, |run| run.automation_id == automation.id),
                automation,
            })
            .collect();
        menu.sort_by(|a, b| {
            b.matches.cmp(&a.matches).then_with(|| {
                a.automation
                    .name
                    .to_lowercase()
                    .cmp(&b.automation.name.to_lowercase())
            })
        });
        Ok(menu)
    }

    /// The finished meetings the automation's conditions match, newest
    /// first, each with its run there.
    pub async fn matching(
        &self,
        automation: &AutomationId,
    ) -> Result<Vec<MatchingMeeting>, AutomationError> {
        let automation = self.get(automation).await?;
        let mut runs = self.ports.runs.for_automation(&automation.id).await?;
        let mut matching = Vec::new();
        for meeting in self.ports.facts.past_meetings().await? {
            let Some(facts) = self.ports.facts.facts(&meeting.id).await? else {
                continue;
            };
            if automation.conditions.matches(&facts) {
                let run = take_run(&mut runs, |run| run.meeting_id == meeting.id);
                matching.push(MatchingMeeting { meeting, run });
            }
        }
        Ok(matching)
    }
}

/// Removes and returns the first run `select` picks.
fn take_run(
    runs: &mut Vec<AutomationRun>,
    select: impl Fn(&AutomationRun) -> bool,
) -> Option<AutomationRun> {
    let index = runs.iter().position(select)?;
    Some(runs.swap_remove(index))
}
