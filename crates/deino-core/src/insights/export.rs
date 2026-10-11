//! The meeting's insights for exports: EXP-01's Insights section, read
//! through [`InsightExports`] by the export assembler.

use std::collections::HashMap;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{InsightStatus, MeetingId, SegmentId};
use deino_export::{ExportField, InsightAnswerExport, InsightExport, InsightExportValue};

use super::{
    InsightAnswer, InsightDef, InsightRepo, InsightResultRepo, InsightRun, InsightValue,
    SpeakerNames, SqliteInsightRepo,
};

/// Reads a meeting's insights as exports show them.
#[async_trait]
pub trait InsightExports: Send + Sync {
    /// The meeting's answered insights, in the Insights page's order, with
    /// speakers by their current names.
    async fn insights_of(&self, meeting: &MeetingId) -> Result<Vec<InsightExport>, DbError>;
}

#[async_trait]
impl InsightExports for SqliteInsightRepo {
    async fn insights_of(&self, meeting: &MeetingId) -> Result<Vec<InsightExport>, DbError> {
        let runs = self.for_meeting(meeting).await?;
        let segments: Vec<SegmentId> = runs
            .iter()
            .flat_map(|run| run.answers.iter().filter_map(|answer| answer.segment_id))
            .collect();
        let names = self.names(&segments).await?;
        let defs = self.list().await?;
        Ok(defs
            .iter()
            .filter_map(|def| {
                let run = runs.iter().find(|run| run.insight_id == def.id)?;
                export_of(def, run, &names)
            })
            .collect())
    }
}

/// `run` as an export section entry; `None` when it has no answers to show.
pub fn export_of(
    def: &InsightDef,
    run: &InsightRun,
    names: &HashMap<SegmentId, String>,
) -> Option<InsightExport> {
    if run.status == InsightStatus::Pending && run.answers.is_empty() {
        return None;
    }
    let answers: Vec<InsightAnswerExport> = run
        .answers
        .iter()
        .map(|answer| answer_export(answer, names))
        .collect();
    (!answers.is_empty()).then(|| InsightExport {
        name: def.name.clone(),
        answers,
    })
}

fn answer_export(
    answer: &InsightAnswer,
    names: &HashMap<SegmentId, String>,
) -> InsightAnswerExport {
    let speaker = answer
        .segment_id
        .and_then(|segment| names.get(&segment).cloned())
        .or_else(|| answer.speaker.clone());
    InsightAnswerExport {
        value: value_export(&answer.value),
        speaker,
        at: answer.timestamp_ms,
    }
}

fn value_export(value: &InsightValue) -> InsightExportValue {
    match value {
        InsightValue::Text { text } => InsightExportValue::Text { text: text.clone() },
        InsightValue::Fields { fields } => InsightExportValue::Fields {
            fields: fields
                .iter()
                .filter_map(|entry| {
                    let value = entry.value.as_ref()?;
                    Some(ExportField {
                        name: entry.name.clone(),
                        value: value.display(),
                    })
                })
                .collect(),
        },
    }
}
