//! Running an insight on meetings the user picks: "Run on past meetings"
//! and the Insights tab's "Run".
//!
//! A run is a pending row plus the meeting's `insights` stage scheduled in
//! the post-meeting pipeline (FND-08), which owns retries, restarts and
//! progress events. The pending rows are the progress the UI shows.

use deino_domain::{InsightId, MeetingId};

use super::{InsightError, InsightService, RunProgress};

/// The most meetings one request may run on.
pub const MAX_RUN_MEETINGS: usize = 500;

impl InsightService {
    /// Runs `insight` on each of `meetings` in the background; returns the
    /// insight's progress with them queued.
    pub async fn run(
        &self,
        insight: &InsightId,
        meetings: &[MeetingId],
    ) -> Result<RunProgress, InsightError> {
        self.get(insight).await?;
        let meetings = unique(meetings);
        if meetings.is_empty() || meetings.len() > MAX_RUN_MEETINGS {
            return Err(InsightError::invalid("meetings", "must number 1 to 500"));
        }
        for meeting in &meetings {
            if self.ports.records.record(meeting).await?.is_none() {
                return Err(InsightError::MeetingNotFound(meeting.clone()));
            }
        }
        let now = self.ports.clock.now();
        self.ports.results.request(insight, &meetings, now).await?;
        self.schedule(&meetings).await?;
        log::info!(target: "deino_core::insights", "queued insight {insight} on {} meetings", meetings.len());
        Ok(self.ports.results.progress(insight).await?)
    }

    /// Schedules each meeting's stage. One failure does not stop the rest
    /// (their rows stay pending until the meeting's pipeline runs); the
    /// first is returned when every meeting failed.
    async fn schedule(&self, meetings: &[MeetingId]) -> Result<(), InsightError> {
        let mut first_error = None;
        let mut scheduled = 0_usize;
        for meeting in meetings {
            match self.ports.scheduler.schedule(meeting).await {
                Ok(()) => scheduled += 1,
                Err(error) => {
                    log::warn!(target: "deino_core::insights", "could not schedule meeting {meeting}: {error}");
                    first_error.get_or_insert(error);
                }
            }
        }
        match first_error {
            Some(error) if scheduled == 0 => Err(InsightError::Schedule(error.message)),
            _ => Ok(()),
        }
    }
}

/// `meetings` without repeats, in order.
fn unique(meetings: &[MeetingId]) -> Vec<MeetingId> {
    let mut kept: Vec<MeetingId> = Vec::with_capacity(meetings.len());
    for meeting in meetings {
        if !kept.contains(meeting) {
            kept.push(meeting.clone());
        }
    }
    kept
}
