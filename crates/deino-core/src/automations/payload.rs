//! The JSON a step hands to the outside (a shell script's stdin today, a
//! webhook's body with DEV-01): the meeting with the fields the step chose,
//! and what earlier steps of the run produced.
//!
//! [`MeetingSource`] reads the meeting; [`AssembledMeetings`] implements it
//! with EXP-01's assembler, so the payload and the JSON export agree.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use deino_domain::{AutomationId, MeetingId, Seconds};
use deino_export::{ActionItemExport, InsightExport, MeetingExport, Participant, Utterance};
use serde::Serialize;

use super::{PayloadFields, StepContext, StepOutput, Trigger};
use crate::export::{ExportAssembler, ExportError};
use crate::meetings::MeetingLifecycleRepo;

/// The payload's format name.
pub const PAYLOAD_FORMAT: &str = "deino.automation";
/// Its version; bump it when a field changes meaning or goes away.
pub const PAYLOAD_VERSION: u32 = 1;

/// Reads what a payload or an exported file is made of.
#[async_trait]
pub trait MeetingSource: Send + Sync {
    /// The whole meeting, as exports see it.
    async fn export(&self, meeting: &MeetingId) -> Result<MeetingExport, ExportError>;

    /// Where its recording is (the mixed file, else the microphone track).
    async fn audio_path(&self, meeting: &MeetingId) -> Result<Option<String>, ExportError>;
}

/// [`MeetingSource`] over the export assembler and the meetings repository.
pub struct AssembledMeetings {
    assembler: Arc<ExportAssembler>,
    lifecycle: Arc<dyn MeetingLifecycleRepo>,
}

impl AssembledMeetings {
    /// The source over its ports.
    pub fn new(assembler: Arc<ExportAssembler>, lifecycle: Arc<dyn MeetingLifecycleRepo>) -> Self {
        Self {
            assembler,
            lifecycle,
        }
    }
}

#[async_trait]
impl MeetingSource for AssembledMeetings {
    async fn export(&self, meeting: &MeetingId) -> Result<MeetingExport, ExportError> {
        self.assembler.assemble(meeting).await
    }

    async fn audio_path(&self, meeting: &MeetingId) -> Result<Option<String>, ExportError> {
        let state = self
            .lifecycle
            .state(meeting)
            .await
            .map_err(ExportError::from)?;
        Ok(state.and_then(|state| state.audio.audio_path.or(state.audio.audio_mic_path)))
    }
}

/// The automation a payload came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadAutomation {
    /// Its ID.
    pub id: AutomationId,
    /// Its name.
    pub name: String,
}

/// The meeting's header, always included.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadMeeting {
    /// Its ID.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started, in the user's zone.
    pub started_at: DateTime<FixedOffset>,
    /// How long it recorded.
    pub duration: Option<Seconds>,
    /// Its tags, by name.
    pub tags: Vec<String>,
    /// Its folders, by name.
    pub folders: Vec<String>,
}

/// The JSON document a step sends.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPayload {
    /// [`PAYLOAD_FORMAT`].
    pub format: &'static str,
    /// [`PAYLOAD_VERSION`].
    pub version: u32,
    /// The automation.
    pub automation: PayloadAutomation,
    /// Why it ran.
    pub trigger: Trigger,
    /// The meeting's header.
    pub meeting: PayloadMeeting,
    /// The `deino://` link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// The AI notes, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// The action items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_items: Option<Vec<ActionItemExport>>,
    /// The transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Vec<Utterance>>,
    /// The insights with answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insights: Option<Vec<InsightExport>>,
    /// Who took part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<Participant>>,
    /// Where the recording is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// What earlier steps of the run produced (generated insights first of all).
    pub steps: Vec<StepOutput>,
}

/// Builds the payload for `ctx` with `fields` from `source`.
pub async fn build_payload(
    source: &dyn MeetingSource,
    ctx: StepContext<'_>,
    fields: PayloadFields,
) -> Result<AutomationPayload, ExportError> {
    let export = source.export(ctx.meeting).await?;
    let audio_path = if fields.audio_path {
        source.audio_path(ctx.meeting).await?
    } else {
        None
    };
    let link = fields.link.then(|| export.link());
    Ok(AutomationPayload {
        format: PAYLOAD_FORMAT,
        version: PAYLOAD_VERSION,
        automation: PayloadAutomation {
            id: ctx.automation.id,
            name: ctx.automation.name.clone(),
        },
        trigger: ctx.trigger,
        link,
        notes: export.notes.clone().filter(|_| fields.notes),
        action_items: fields.action_items.then(|| export.action_items.clone()),
        transcript: fields.transcript.then(|| export.transcript.clone()),
        insights: fields.insights.then(|| export.insights.clone()),
        attendees: fields.attendees.then(|| export.attendees.clone()),
        audio_path,
        steps: ctx.earlier.to_vec(),
        meeting: PayloadMeeting {
            id: export.id,
            title: export.title,
            started_at: export.started_at,
            duration: export.duration,
            tags: export.tags,
            folders: export.folders,
        },
    })
}
