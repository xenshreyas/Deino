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

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use deino_doc::Flavor;
use deino_domain::{Clock, MeetingId};
use deino_llm::Gateway;

use super::assemble::{Checks, assemble};
use super::citations::SegmentIndex;
use super::convert::to_doc;
use super::material::Material;
use super::sources::MaterialSources;
use super::user_lines::{user_headings, user_lines};
use super::writer::NotesWriter;
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

/// Writes, reads and edits enhanced notes.
pub struct EnhanceService {
    pub(super) ports: EnhancePorts,
    pub(super) clock: Arc<dyn Clock>,
    writer: NotesWriter,
    running: Mutex<HashSet<MeetingId>>,
}

impl EnhanceService {
    /// The service over its ports.
    pub fn new(ports: EnhancePorts, clock: Arc<dyn Clock>) -> Self {
        Self {
            writer: NotesWriter::new(Arc::clone(&ports.gateway)),
            ports,
            clock,
            running: Mutex::new(HashSet::new()),
        }
    }

    /// Writes the meeting's notes as a new version and returns it. Fails
    /// with [`EnhanceError::WouldReplaceEdits`] when the newest version has
    /// the user's edits and `overwrite` does not allow replacing them.
    pub async fn enhance(
        &self,
        meeting: &MeetingId,
        overwrite: Overwrite,
    ) -> Result<EnhancedNotesView, EnhanceError> {
        let _running = self.start(meeting)?;
        let latest = self.ports.reader.latest(meeting).await?;
        if overwrite == Overwrite::KeepEdits && latest.is_some_and(|notes| notes.is_user_edited) {
            return Err(EnhanceError::WouldReplaceEdits);
        }
        let material = self.material(meeting).await?;
        if material.is_empty() {
            return Err(EnhanceError::NothingToEnhance);
        }
        let version = self.write(&material).await?;
        self.view(meeting, Some(version))
            .await?
            .ok_or(EnhanceError::VersionNotFound { version })
    }

    /// The pipeline's run (`enhance_notes`): writes notes unless there is
    /// nothing to write from, the user edited the notes, or nothing changed
    /// since they were written.
    pub async fn run_for_pipeline(&self, meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
        let material = self.material(meeting).await?;
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
        let _running = self.start(meeting)?;
        Ok(PipelineRun::Written(self.write(&material).await?))
    }

    /// The material of `meeting`, or [`EnhanceError::MeetingNotFound`].
    pub(super) async fn material(&self, meeting: &MeetingId) -> Result<Material, EnhanceError> {
        self.ports
            .sources
            .gather(meeting)
            .await?
            .ok_or_else(|| EnhanceError::MeetingNotFound(meeting.clone()))
    }

    /// Writes, checks and stores one version; returns its number.
    async fn write(&self, material: &Material) -> Result<u32, EnhanceError> {
        let meeting = &material.meeting.id;
        let template = self.ports.templates.template_for(meeting).await?;
        let index = SegmentIndex::new(&material.transcript);
        let written = self.writer.write(material, &template, &index).await?;
        let (lines, headings) = match &material.notes {
            Some(notes) => (user_lines(&notes.doc), user_headings(&notes.doc)),
            None => (Vec::new(), Vec::new()),
        };
        let checks = Checks {
            lines: &lines,
            headings: &headings,
            index: &index,
        };
        let draft = assemble(&written.answer, checks);
        if draft.sections.is_empty() {
            return Err(EnhanceError::EmptyNotes);
        }
        let doc = to_doc(&draft);
        let version = NewVersion {
            meeting_id: meeting.clone(),
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            template_id: Some(template.id),
            model: written.model,
            basis: Some(EnhanceBasis::of(material)),
            is_user_edited: false,
            at: self.clock.now(),
        };
        let number = self.ports.writer.insert(&version).await?;
        log::info!(target: "enhance", "{meeting}: stored notes version {number} ({} sections)", draft.sections.len());
        Ok(number)
    }

    /// Marks `meeting` as being written, or fails when it already is.
    fn start(&self, meeting: &MeetingId) -> Result<Running<'_>, EnhanceError> {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if !running.insert(meeting.clone()) {
            return Err(EnhanceError::AlreadyRunning);
        }
        Ok(Running {
            set: &self.running,
            meeting: meeting.clone(),
        })
    }
}

/// While alive, its meeting counts as being written.
struct Running<'a> {
    set: &'a Mutex<HashSet<MeetingId>>,
    meeting: MeetingId,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.set
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.meeting);
    }
}
