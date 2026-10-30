//! [`EnhanceService`]: writes a meeting's enhanced notes.
//!
//! One run: gather the [`Material`], pick the template (NOT-04's
//! `auto_choice`: the user's choice, a rule, the default, the classifier,
//! General), write with [`NotesWriter`] (one pass or map-reduce), check and
//! resolve the answer ([`assemble`]), convert it to a TipTap document
//! ([`to_doc`]) and store it as the meeting's next version, with the basis
//! of its inputs. Earlier versions stay (the version history); a newest
//! version the user edited is only replaced with their consent.
//!
//! Reading, editing and restoring versions are in `history.rs`.

use std::sync::Arc;

use async_trait::async_trait;
use deino_doc::Flavor;
use deino_domain::{Clock, MeetingId, TemplateId};
use deino_llm::Gateway;

use super::assemble::{Checks, Draft, assemble};
use super::citations::SegmentIndex;
use super::convert::to_doc;
use super::live::LiveDraft;
use super::material::Material;
use super::progress::{NoProgress, NotesDraft, NotesProgress};
use super::runs::{Run, Runs};
use super::sources::MaterialSources;
use super::user_lines::{UserLine, user_headings, user_lines};
use super::writer::{NotesWriter, WriteJob};
use super::{
    EnhanceBasis, EnhanceError, EnhancedNotesReader, EnhancedNotesView, EnhancedNotesWriter,
    NewVersion,
};
use crate::templates::{MeetingTemplateService, Template, TemplateError};

/// Shown when the pipeline has nothing to write notes from.
pub const NOTHING_TO_WRITE: &str = "The meeting has no transcript or notes to write from.";
/// Shown when the pipeline leaves notes the user edited alone.
pub const KEPT_EDITS: &str = "Your edited notes were kept. Regenerate them from the Notes tab.";
/// Shown when nothing changed since the notes were written.
pub const UP_TO_DATE: &str = "The notes are up to date.";
/// Shown when retention deleted the transcript the notes came from (PRV-01).
pub const TRANSCRIPT_DELETED: &str = "The transcript was deleted; the notes were kept.";

/// Which template a meeting's notes are written with.
#[async_trait]
pub trait TemplateSource: Send + Sync {
    /// The template for `meeting` (may ask the LLM classifier).
    async fn template_for(&self, meeting: &MeetingId) -> Result<Template, TemplateError>;
}

#[async_trait]
impl TemplateSource for MeetingTemplateService {
    async fn template_for(&self, meeting: &MeetingId) -> Result<Template, TemplateError> {
        Ok(self.auto_choice(meeting).await?.template)
    }
}

/// What to do about a newest version the user edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overwrite {
    /// Refuse with [`EnhanceError::WouldReplaceEdits`].
    KeepEdits,
    /// The user confirmed: write a new version over their edits (the edited
    /// version stays in the history).
    ReplaceEdits,
}

/// How a pipeline run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineRun {
    /// A new version was written.
    Written(u32),
    /// Nothing was written, for this user-safe reason.
    Skipped(&'static str),
}

/// What the service is built from.
pub struct EnhancePorts {
    /// Where the material comes from.
    pub sources: MaterialSources,
    /// Stored versions.
    pub reader: Arc<dyn EnhancedNotesReader>,
    /// Where versions are written.
    pub writer: Arc<dyn EnhancedNotesWriter>,
    /// The meeting's template.
    pub templates: Arc<dyn TemplateSource>,
    /// The LLM gateway (`notes.enhance`).
    pub gateway: Arc<Gateway>,
}

/// Shown when the user stopped the notes the pipeline was writing.
pub const STOPPED: &str = "You stopped the notes. Use Enhance on the Notes tab to write them.";

/// Writes, reads and edits enhanced notes.
pub struct EnhanceService {
    pub(super) ports: EnhancePorts,
    pub(super) clock: Arc<dyn Clock>,
    writer: NotesWriter,
    runs: Runs,
}

impl EnhanceService {
    /// The service over its ports, reporting runs nowhere.
    pub fn new(ports: EnhancePorts, clock: Arc<dyn Clock>) -> Self {
        Self {
            writer: NotesWriter::new(Arc::clone(&ports.gateway)),
            ports,
            clock,
            runs: Runs::new(Arc::new(NoProgress)),
        }
    }

    /// Reports every run's drafts and end to `progress` (streamed notes).
    #[must_use]
    pub fn with_progress(mut self, progress: Arc<dyn NotesProgress>) -> Self {
        self.runs = Runs::new(progress);
        self
    }

    /// Writes the meeting's notes as a new version and returns it. Fails
    /// with [`EnhanceError::WouldReplaceEdits`] when the newest version has
    /// the user's edits and `overwrite` does not allow replacing them, and
    /// with [`EnhanceError::Stopped`] when the user stopped it.
    pub async fn enhance(
        &self,
        meeting: &MeetingId,
        overwrite: Overwrite,
    ) -> Result<EnhancedNotesView, EnhanceError> {
        let run = self.runs.start(meeting)?;
        let latest = self.ports.reader.latest(meeting).await?;
        if overwrite == Overwrite::KeepEdits && latest.is_some_and(|notes| notes.is_user_edited) {
            return Err(EnhanceError::WouldReplaceEdits);
        }
        let material = self.material(meeting).await?;
        if material.transcript_deleted {
            return Err(EnhanceError::TranscriptDeleted);
        }
        if material.is_empty() {
            return Err(EnhanceError::NothingToEnhance);
        }
        let written = self.write(&material, &run).await;
        run.finish(&written);
        let version = written?;
        self.view(meeting, Some(version))
            .await?
            .ok_or(EnhanceError::VersionNotFound { version })
    }

    /// The pipeline's run (`enhance_notes`): writes notes unless there is
    /// nothing to write from, the user edited the notes, or nothing changed
    /// since they were written. A run the user stops is a skip.
    pub async fn run_for_pipeline(&self, meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
        let material = self.material(meeting).await?;
        if material.transcript_deleted {
            return Ok(PipelineRun::Skipped(TRANSCRIPT_DELETED));
        }
        if material.is_empty() {
            return Ok(PipelineRun::Skipped(NOTHING_TO_WRITE));
        }
        if let Some(latest) = self.ports.reader.latest(meeting).await? {
            if latest.is_user_edited {
                return Ok(PipelineRun::Skipped(KEPT_EDITS));
            }
            if latest
                .basis
                .is_some_and(|basis| basis.reasons(&material).is_empty())
            {
                return Ok(PipelineRun::Skipped(UP_TO_DATE));
            }
        }
        let run = self.runs.start(meeting)?;
        let written = self.write(&material, &run).await;
        run.finish(&written);
        match written {
            Ok(version) => Ok(PipelineRun::Written(version)),
            Err(EnhanceError::Stopped) => Ok(PipelineRun::Skipped(STOPPED)),
            Err(error) => Err(error),
        }
    }

    /// The notes of `meeting` as far as they are written, while they are.
    pub fn draft(&self, meeting: &MeetingId) -> Option<NotesDraft> {
        self.runs.draft(meeting)
    }

    /// Stops writing `meeting`'s notes; nothing is saved. False when they
    /// were not being written.
    pub fn stop(&self, meeting: &MeetingId) -> bool {
        self.runs.stop(meeting)
    }

    /// The material of `meeting`, or [`EnhanceError::MeetingNotFound`].
    pub(super) async fn material(&self, meeting: &MeetingId) -> Result<Material, EnhanceError> {
        self.ports
            .sources
            .gather(meeting)
            .await?
            .ok_or_else(|| EnhanceError::MeetingNotFound(meeting.clone()))
    }

    /// Writes, checks and stores one version, showing it to `run` as it is
    /// written; returns its number.
    async fn write(&self, material: &Material, run: &Run<'_>) -> Result<u32, EnhanceError> {
        let template = self
            .ports
            .templates
            .template_for(&material.meeting.id)
            .await?;
        let index = SegmentIndex::new(&material.transcript);
        let (lines, headings) = typed(material);
        run.keeps(lines.len());
        let checks = Checks {
            lines: &lines,
            headings: &headings,
            index: &index,
        };
        let live = LiveDraft::new(run, checks);
        let job = WriteJob {
            material,
            template: &template,
            index: &index,
            report: &live,
        };
        let written = run.abortable(self.writer.write(&job)).await?;
        live.saving();
        let answer = Answer {
            draft: assemble(&written.answer, checks),
            template: template.id,
            model: written.model,
        };
        self.store(material, answer).await
    }

    /// Stores the checked answer as the meeting's next version.
    async fn store(&self, material: &Material, answer: Answer) -> Result<u32, EnhanceError> {
        let meeting = &material.meeting.id;
        let draft = answer.draft;
        if draft.sections.is_empty() {
            return Err(EnhanceError::EmptyNotes);
        }
        let doc = to_doc(&draft);
        let version = NewVersion {
            meeting_id: meeting.clone(),
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            template_id: Some(answer.template),
            model: answer.model,
            basis: Some(EnhanceBasis::of(material)),
            is_user_edited: false,
            at: self.clock.now(),
        };
        let number = self.ports.writer.insert(&version).await?;
        log::info!(target: "enhance", "{meeting}: stored notes version {number} ({} sections)", draft.sections.len());
        Ok(number)
    }
}

/// A checked answer and what wrote it.
struct Answer {
    draft: Draft,
    template: TemplateId,
    model: String,
}

/// The lines and headings the user typed in their notes.
fn typed(material: &Material) -> (Vec<UserLine>, Vec<String>) {
    match &material.notes {
        Some(notes) => (user_lines(&notes.doc), user_headings(&notes.doc)),
        None => (Vec::new(), Vec::new()),
    }
}
