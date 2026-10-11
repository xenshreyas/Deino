//! In-memory ports for tests of insights and of the stage that runs them.
//!
//! [`FakeInsightStore`] honours the same contracts as [`SqliteInsightRepo`]
//! (oldest-first lists, pending requests keeping answers,
//! progress over meetings it knows), so service tests read like the real
//! thing; the repository tests cover the SQL.
//!
//! [`SqliteInsightRepo`]: super::SqliteInsightRepo

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{AppError, ErrorCode, InsightId, InsightStatus, MeetingId, SegmentId};
use deino_llm::LlmError;

use super::{
    InsightDef, InsightModel, InsightRepo, InsightRequest, InsightResultRepo, InsightRun,
    MeetingRecord, MeetingRecords, MeetingRun, OwnDomains, PastMeeting, RunAnswers, RunProgress,
    RunScheduler, SpeakerNames, WireAnswer,
};
use crate::profile::InternalDomains;
use crate::testing::lock;

/// Definitions, runs, meetings and speaker names, in memory.
#[derive(Default)]
pub struct FakeInsightStore {
    defs: Mutex<Vec<InsightDef>>,
    runs: Mutex<Vec<InsightRun>>,
    meetings: Mutex<Vec<(PastMeeting, MeetingRecord)>>,
    names: Mutex<HashMap<SegmentId, String>>,
}

impl FakeInsightStore {
    /// Adds a finished meeting with `record`'s facts.
    pub fn add_meeting(&self, meeting: PastMeeting, record: MeetingRecord) {
        lock(&self.meetings).push((meeting, record));
    }

    /// Names the speaker of `segment`.
    pub fn name_speaker(&self, segment: SegmentId, name: &str) {
        lock(&self.names).insert(segment, name.to_owned());
    }

    /// Every stored run.
    pub fn runs(&self) -> Vec<InsightRun> {
        lock(&self.runs).clone()
    }

    fn upsert(
        &self,
        insight: &InsightId,
        meeting: &MeetingId,
        at: DateTime<Utc>,
        apply: impl FnOnce(&mut InsightRun),
    ) {
        let mut runs = lock(&self.runs);
        let index = runs
            .iter()
            .position(|run| run.insight_id == *insight && run.meeting_id == *meeting)
            .unwrap_or_else(|| {
                runs.push(InsightRun {
                    insight_id: *insight,
                    meeting_id: meeting.clone(),
                    status: InsightStatus::Pending,
                    answers: Vec::new(),
                    revision: None,
                    error: None,
                    requested_at: at,
                    generated_at: None,
                });
                runs.len() - 1
            });
        if let Some(run) = runs.get_mut(index) {
            apply(run);
        }
    }

    fn meeting(&self, id: &MeetingId) -> Option<(PastMeeting, MeetingRecord)> {
        lock(&self.meetings)
            .iter()
            .find(|(meeting, _)| meeting.id == *id)
            .cloned()
    }
}

#[async_trait]
impl InsightRepo for FakeInsightStore {
    async fn list(&self) -> Result<Vec<InsightDef>, DbError> {
        let mut defs = lock(&self.defs).clone();
        defs.sort_by_key(|def| (def.created_at, def.id));
        Ok(defs)
    }

    async fn get(&self, id: &InsightId) -> Result<Option<InsightDef>, DbError> {
        Ok(lock(&self.defs).iter().find(|def| def.id == *id).cloned())
    }

    async fn create(&self, def: &InsightDef) -> Result<(), DbError> {
        lock(&self.defs).push(def.clone());
        Ok(())
    }

    async fn update(&self, def: &InsightDef) -> Result<bool, DbError> {
        let mut defs = lock(&self.defs);
        let Some(stored) = defs.iter_mut().find(|other| other.id == def.id) else {
            return Ok(false);
        };
        *stored = def.clone();
        Ok(true)
    }

    async fn delete(&self, id: &InsightId) -> Result<bool, DbError> {
        let mut defs = lock(&self.defs);
        let before = defs.len();
        defs.retain(|def| def.id != *id);
        lock(&self.runs).retain(|run| run.insight_id != *id);
        Ok(defs.len() < before)
    }
}

#[async_trait]
impl InsightResultRepo for FakeInsightStore {
    async fn request(
        &self,
        insight: &InsightId,
        meetings: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        for meeting in meetings {
            self.upsert(insight, meeting, at, |run| {
                run.status = InsightStatus::Pending;
                run.error = None;
                run.requested_at = at;
            });
        }
        Ok(())
    }

    async fn for_meeting(&self, meeting: &MeetingId) -> Result<Vec<InsightRun>, DbError> {
        Ok(lock(&self.runs)
            .iter()
            .filter(|run| run.meeting_id == *meeting)
            .cloned()
            .collect())
    }

    async fn for_insight(&self, insight: &InsightId) -> Result<Vec<MeetingRun>, DbError> {
        let mut rows: Vec<MeetingRun> = lock(&self.runs)
            .iter()
            .filter(|run| run.insight_id == *insight)
            .filter_map(|run| {
                let (meeting, _) = self.meeting(&run.meeting_id)?;
                Some(MeetingRun {
                    meeting_title: meeting.title,
                    meeting_started_at: meeting.started_at,
                    run: run.clone(),
                })
            })
            .collect();
        rows.sort_by(|a, b| b.meeting_started_at.cmp(&a.meeting_started_at));
        Ok(rows)
    }

    async fn save_answers(&self, answers: &RunAnswers) -> Result<(), DbError> {
        self.upsert(
            &answers.insight_id,
            &answers.meeting_id,
            answers.at,
            |run| {
                run.status = InsightStatus::Done;
                run.answers.clone_from(&answers.answers);
                run.revision = Some(answers.revision);
                run.error = None;
                run.generated_at = Some(answers.at);
            },
        );
        Ok(())
    }

    async fn save_failure(
        &self,
        insight: &InsightId,
        meeting: &MeetingId,
        message: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        self.upsert(insight, meeting, at, |run| {
            run.status = InsightStatus::Failed;
            run.error = Some(message.to_owned());
        });
        Ok(())
    }

    async fn progress(&self, insight: &InsightId) -> Result<RunProgress, DbError> {
        let mut progress = RunProgress::default();
        for row in self.for_insight(insight).await? {
            match row.run.status {
                InsightStatus::Pending => progress.pending += 1,
                InsightStatus::Done => progress.done += 1,
                InsightStatus::Failed => progress.failed += 1,
            }
        }
        Ok(progress)
    }
}

#[async_trait]
impl MeetingRecords for FakeInsightStore {
    async fn record(&self, meeting: &MeetingId) -> Result<Option<MeetingRecord>, DbError> {
        Ok(self.meeting(meeting).map(|(_, record)| record))
    }

    async fn past_meetings(&self) -> Result<Vec<PastMeeting>, DbError> {
        let mut meetings: Vec<PastMeeting> = lock(&self.meetings)
            .iter()
            .map(|(meeting, _)| meeting.clone())
            .collect();
        meetings.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(meetings)
    }
}

#[async_trait]
impl SpeakerNames for FakeInsightStore {
    async fn names(&self, segments: &[SegmentId]) -> Result<HashMap<SegmentId, String>, DbError> {
        let names = lock(&self.names);
        Ok(segments
            .iter()
            .filter_map(|segment| names.get(segment).map(|name| (*segment, name.clone())))
            .collect())
    }
}

/// The user's own domains, fixed.
#[derive(Clone, Debug, Default)]
pub struct FixedDomains(pub InternalDomains);

#[async_trait]
impl OwnDomains for FixedDomains {
    async fn internal_domains(&self) -> InternalDomains {
        self.0.clone()
    }
}

/// A scheduler that records what it scheduled, failing for chosen meetings.
#[derive(Default)]
pub struct RecordedScheduler {
    scheduled: Mutex<Vec<MeetingId>>,
    refused: Mutex<Vec<MeetingId>>,
}

impl RecordedScheduler {
    /// Refuses to schedule `meeting`, as the pipeline does for a meeting
    /// that is still recording.
    pub fn refuse(&self, meeting: MeetingId) {
        lock(&self.refused).push(meeting);
    }

    /// The meetings scheduled, in order.
    pub fn scheduled(&self) -> Vec<MeetingId> {
        lock(&self.scheduled).clone()
    }
}

#[async_trait]
impl RunScheduler for RecordedScheduler {
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError> {
        if lock(&self.refused).contains(meeting) {
            return Err(AppError::new(
                ErrorCode::Invalid,
                "the meeting is still recording",
            ));
        }
        lock(&self.scheduled).push(meeting.clone());
        Ok(())
    }
}

/// A model that answers by insight name, and records what it was asked.
#[derive(Default)]
pub struct ScriptedInsightModel {
    answers: Mutex<BTreeMap<String, Vec<Result<WireAnswer, LlmError>>>>,
    asked: Mutex<Vec<InsightRequest>>,
}

impl ScriptedInsightModel {
    /// Answers the next request for the insight `name` with `answer`.
    pub fn answer(self, name: &str, answer: Result<WireAnswer, LlmError>) -> Self {
        lock(&self.answers)
            .entry(name.to_owned())
            .or_default()
            .push(answer);
        self
    }

    /// Every request, in order.
    pub fn asked(&self) -> Vec<InsightRequest> {
        lock(&self.asked).clone()
    }
}

#[async_trait]
impl InsightModel for ScriptedInsightModel {
    async fn answer(&self, request: &InsightRequest) -> Result<WireAnswer, LlmError> {
        lock(&self.asked).push(request.clone());
        let mut answers = lock(&self.answers);
        let queue = answers.entry(request.name.clone()).or_default();
        if queue.is_empty() {
            return Ok(WireAnswer::default());
        }
        queue.remove(0)
    }
}
