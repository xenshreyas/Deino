//! Starting runs: by hand (Run on selected, Rerun, Test on latest meeting)
//! and when a tag is applied.
//!
//! A run is a stored request plus the meeting's `automations` stage
//! scheduled in the post-meeting pipeline (FND-08), which owns retries,
//! restarts and progress. A tag applied to a meeting that has not happened
//! (or is still being processed) **arms** the run instead: the stage runs
//! it once the meeting is processed.

use deino_domain::{
    AutomationId, AutomationRunId, AutomationRunStatus, MeetingId, MeetingStatus, TagId,
};

use super::{AutomationError, AutomationService, RunRequest, Trigger};

/// The most meetings one request may run on.
pub const MAX_RUN_MEETINGS: usize = 500;

impl AutomationService {
    /// Runs `automation` on each of `meetings` now, whatever its conditions
    /// and switches say; an earlier run on a meeting is replaced.
    pub async fn run(
        &self,
        automation: &AutomationId,
        meetings: &[MeetingId],
    ) -> Result<(), AutomationError> {
        self.get(automation).await?;
        let mut unique: Vec<MeetingId> = Vec::with_capacity(meetings.len());
        for meeting in meetings {
            if !unique.contains(meeting) {
                unique.push(meeting.clone());
            }
        }
        if unique.is_empty() || unique.len() > MAX_RUN_MEETINGS {
            return Err(AutomationError::invalid("meetings", "must number 1 to 500"));
        }
        for meeting in &unique {
            if self.ports.facts.facts(meeting).await?.is_none() {
                return Err(AutomationError::MeetingNotFound(meeting.clone()));
            }
        }
        let manual = (Trigger::Manual, AutomationRunStatus::Pending);
        for meeting in &unique {
            let request = self.new_request(automation, meeting, manual);
            self.ports.runs.request(&request, true).await?;
            self.schedule(meeting).await?;
        }
        log::info!(target: "automations", "queued automation {automation} on {} meetings", unique.len());
        Ok(())
    }

    /// Runs `automation` on the newest finished meeting; returns that meeting.
    pub async fn test_on_latest(
        &self,
        automation: &AutomationId,
    ) -> Result<MeetingId, AutomationError> {
        let latest = self
            .ports
            .facts
            .past_meetings()
            .await?
            .into_iter()
            .next()
            .ok_or(AutomationError::NoMeeting)?;
        self.run(automation, std::slice::from_ref(&latest.id))
            .await?;
        Ok(latest.id)
    }

    /// A tag landed on `meeting`: every automation that runs on its own,
    /// tests for the tag and now matches gets a run, unless it already has
    /// one there. Returns how many runs were added.
    pub async fn tag_applied(
        &self,
        meeting: &MeetingId,
        tag: &TagId,
    ) -> Result<usize, AutomationError> {
        let Some(facts) = self.ports.facts.facts(meeting).await? else {
            return Ok(0);
        };
        let ready = self.is_processed(meeting).await?;
        let status = if ready {
            AutomationRunStatus::Pending
        } else {
            AutomationRunStatus::Armed
        };
        let ask = (Trigger::TagApplied { tag_id: *tag }, status);
        let mut added = 0;
        for automation in self.list().await? {
            if !automation.waits_for_tag(tag) || !automation.conditions.matches(&facts) {
                continue;
            }
            let request = self.new_request(&automation.id, meeting, ask);
            added += usize::from(self.ports.runs.request(&request, false).await?);
        }
        if added > 0 && ready {
            self.schedule(meeting).await?;
        }
        Ok(added)
    }

    /// True when the meeting is past its pipeline (ready, or failed), so a
    /// new run must schedule the stage itself.
    async fn is_processed(&self, meeting: &MeetingId) -> Result<bool, AutomationError> {
        let state = self.ports.meetings.state(meeting).await?;
        Ok(state.is_some_and(|state| {
            matches!(state.status, MeetingStatus::Ready | MeetingStatus::Failed)
        }))
    }

    /// A request for `automation` on `meeting`, started by `trigger`.
    fn new_request(
        &self,
        automation: &AutomationId,
        meeting: &MeetingId,
        (trigger, status): (Trigger, AutomationRunStatus),
    ) -> RunRequest {
        RunRequest {
            id: AutomationRunId::generate(self.ports.clock.as_ref()),
            automation_id: *automation,
            meeting_id: meeting.clone(),
            trigger,
            status,
            at: self.ports.clock.now(),
        }
    }

    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AutomationError> {
        self.ports
            .scheduler
            .schedule(meeting)
            .await
            .map_err(|error| AutomationError::Schedule(error.message))
    }
}
