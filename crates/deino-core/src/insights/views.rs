//! What the Insights tab and the Insights page show: a meeting's runs, one
//! insight's runs across meetings, and the past meetings it could run on.
//! Speakers are read back by their current names.

use std::collections::HashMap;

use deino_domain::{InsightId, InsightStatus, MeetingId, SegmentId};
use serde::Serialize;

use super::{
    InsightDef, InsightError, InsightRun, InsightService, MeetingRun, PastMeeting, RunProgress,
};

/// One insight's run on the meeting.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingInsight {
    /// The insight.
    pub def: InsightDef,
    /// Its latest run here.
    pub run: InsightRun,
    /// True when the answers predate the insight's latest edit.
    pub stale: bool,
}

/// A meeting's Insights tab.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingInsights {
    /// The insights that ran (or are running) here, in the Insights page's order.
    pub entries: Vec<MeetingInsight>,
    /// The other insights, which the user can run here.
    pub others: Vec<InsightDef>,
}

/// One insight's table on the Insights page.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightTable {
    /// The insight.
    pub def: InsightDef,
    /// Its runs, newest meeting first.
    pub rows: Vec<MeetingRun>,
    /// Its runs by state.
    pub progress: RunProgress,
}

/// A past meeting, as "Run on past meetings" offers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunCandidate {
    /// The meeting.
    pub meeting: PastMeeting,
    /// True when the insight's conditions match it (always false for a
    /// manual insight), so the dialog can pre-select it.
    pub matches: bool,
    /// The insight's run there, if any.
    pub status: Option<InsightStatus>,
}

impl InsightService {
    /// The meeting's Insights tab.
    pub async fn for_meeting(&self, meeting: &MeetingId) -> Result<MeetingInsights, InsightError> {
        if self.ports.records.record(meeting).await?.is_none() {
            return Err(InsightError::MeetingNotFound(meeting.clone()));
        }
        let mut runs = self.ports.results.for_meeting(meeting).await?;
        self.rename_speakers(runs.iter_mut()).await?;
        let mut entries = Vec::new();
        let mut others = Vec::new();
        for def in self.ports.defs.list().await? {
            match runs.iter().position(|run| run.insight_id == def.id) {
                Some(index) => {
                    let run = runs.swap_remove(index);
                    let stale = run.is_stale(def.revision);
                    entries.push(MeetingInsight { def, run, stale });
                }
                None => others.push(def),
            }
        }
        Ok(MeetingInsights { entries, others })
    }

    /// The insight's runs across meetings.
    pub async fn table(&self, id: &InsightId) -> Result<InsightTable, InsightError> {
        let def = self.get(id).await?;
        let mut rows = self.ports.results.for_insight(id).await?;
        self.rename_speakers(rows.iter_mut().map(|row| &mut row.run))
            .await?;
        let progress = self.ports.results.progress(id).await?;
        Ok(InsightTable {
            def,
            rows,
            progress,
        })
    }

    /// The past meetings the insight can run on, newest first.
    pub async fn candidates(&self, id: &InsightId) -> Result<Vec<RunCandidate>, InsightError> {
        let def = self.get(id).await?;
        let statuses: HashMap<MeetingId, InsightStatus> = self
            .ports
            .results
            .for_insight(id)
            .await?
            .into_iter()
            .map(|row| (row.run.meeting_id, row.run.status))
            .collect();
        let domains = self.ports.domains.internal_domains().await;
        let mut candidates = Vec::new();
        for meeting in self.ports.records.past_meetings().await? {
            let facts = self.ports.records.record(&meeting.id).await?;
            let matches = match (def.automatic_conditions(), facts) {
                (Some(conditions), Some(record)) => conditions.matches(&record.facts(&domains)),
                _ => false,
            };
            let status = statuses.get(&meeting.id).copied();
            candidates.push(RunCandidate {
                meeting,
                matches,
                status,
            });
        }
        Ok(candidates)
    }

    /// Replaces each answer's stored speaker with the cited segment's
    /// current speaker name, so renames (CAP-04) reach insights.
    async fn rename_speakers<'a>(
        &self,
        runs: impl Iterator<Item = &'a mut InsightRun>,
    ) -> Result<(), InsightError> {
        let runs: Vec<&mut InsightRun> = runs.collect();
        let segments: Vec<SegmentId> = runs
            .iter()
            .flat_map(|run| run.answers.iter().filter_map(|answer| answer.segment_id))
            .collect();
        let names = self.ports.names.names(&segments).await?;
        for run in runs {
            for answer in &mut run.answers {
                if let Some(name) = answer.segment_id.and_then(|id| names.get(&id)) {
                    answer.speaker = Some(name.clone());
                }
            }
        }
        Ok(())
    }
}
