//! [`NoteEditService`]: proposes an edit of the newest enhanced notes from
//! the user's words, and stores an accepted one as a new version.
//!
//! A plain rename is done here, without the AI. Anything else goes to the
//! `notes.edit` route with the notes as ID'd blocks and the transcript,
//! redacted first when the user turned redaction on (NOT-09); the answer is
//! a list of block changes, checked ([`plan`]) and applied in place. Nothing
//! is stored until the user accepts; accepting writes a new version marked
//! edited, so regenerating asks first and restoring the previous version
//! undoes the edit.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_doc::{Doc, Flavor};
use deino_domain::{Clock, MeetingId};
use deino_llm::{Gateway, LlmError, ProviderId, Route};

use super::EditError;
use super::apply::apply;
use super::blocks::text_blocks;
use super::diff::diff;
use super::model::{AcceptedEdit, EditMethod, EditProposal, EditRequest};
use super::plan::{Checks, plan, prompt_blocks, redact_blocks};
use super::prompt::{EditPrompt, PromptBlock};
use super::rename::Rename;
use super::rewrite::Refs;
use super::scope::Scope;
use super::wire::EditAnswer;
use crate::enhance::{
    EnhancedNotes, EnhancedNotesReader, EnhancedNotesWriter, MaterialSources, NewVersion,
    TranscriptLine, render_lines,
};
use crate::redaction::TextRedactor;

/// The transcript an edit may cite.
#[async_trait]
pub trait EditTranscript: Send + Sync {
    /// The meeting's transcript lines, numbered as the notes cite them.
    async fn lines(&self, meeting: &MeetingId) -> Result<Vec<TranscriptLine>, DbError>;
}

#[async_trait]
impl EditTranscript for MaterialSources {
    async fn lines(&self, meeting: &MeetingId) -> Result<Vec<TranscriptLine>, DbError> {
        self.transcript_of(meeting).await
    }
}

/// What the service is built from.
pub struct NoteEditPorts {
    /// Stored versions.
    pub reader: Arc<dyn EnhancedNotesReader>,
    /// Where accepted edits are written.
    pub writer: Arc<dyn EnhancedNotesWriter>,
    /// The transcript.
    pub transcript: Arc<dyn EditTranscript>,
    /// Hides secrets before the request leaves (NOT-09's setting).
    pub redactor: Arc<dyn TextRedactor>,
    /// The LLM gateway (`notes.edit`); `None` when none could be wired, so
    /// only renames work.
    pub gateway: Option<Arc<Gateway>>,
}

/// Edits enhanced notes by asking.
pub struct NoteEditService {
    ports: NoteEditPorts,
    clock: Arc<dyn Clock>,
}

impl NoteEditService {
    /// The service over its ports.
    pub fn new(ports: NoteEditPorts, clock: Arc<dyn Clock>) -> Self {
        Self { ports, clock }
    }

    /// The edit `request` asks for, applied to a copy of the newest notes.
    pub async fn propose(&self, request: &EditRequest) -> Result<EditProposal, EditError> {
        let instruction = request.instruction.trim();
        if instruction.is_empty() {
            return Err(EditError::EmptyInstruction);
        }
        let notes = self.newest(&request.meeting_id, request.version).await?;
        let blocks = text_blocks(&notes.doc);
        let scope = Scope::of(&request.selection, blocks.len());
        if let Some(rename) = Rename::parse(instruction) {
            let (doc, count) = rename.apply(&notes.doc, &scope);
            if count > 0 {
                log::debug!(target: "notes_edit", "{}: rename in {count} places", notes.meeting_id);
                return proposal(&notes, doc, EditMethod::Rename, Some(count));
            }
        }
        let doc = self.ask(&notes, &scope, instruction).await?;
        proposal(&notes, doc, EditMethod::Ai, None)
    }

    /// Stores `doc`, an accepted proposal made from `base_version`, as the
    /// newest version.
    pub async fn accept(
        &self,
        meeting: &MeetingId,
        base_version: u32,
        doc: Doc,
    ) -> Result<AcceptedEdit, EditError> {
        let base = self.newest(meeting, base_version).await?;
        let doc = doc.normalized();
        deino_doc::validate(&doc).map_err(EditError::InvalidDoc)?;
        let version = NewVersion {
            meeting_id: meeting.clone(),
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            template_id: base.template_id,
            model: base.model,
            basis: base.basis,
            is_user_edited: true,
            at: self.clock.now(),
        };
        let number = self.ports.writer.insert(&version).await?;
        log::info!(target: "notes_edit", "{meeting}: stored the edit of version {base_version} as {number}");
        Ok(AcceptedEdit {
            version: number,
            previous_version: base_version,
        })
    }

    /// The newest notes, which must be `version`.
    async fn newest(&self, meeting: &MeetingId, version: u32) -> Result<EnhancedNotes, EditError> {
        let latest = self.ports.reader.latest(meeting).await?;
        let latest = latest.ok_or(EditError::NoNotes)?;
        if latest.version != version {
            return Err(EditError::NotLatest);
        }
        Ok(latest)
    }

    /// The model's edit of `notes`, checked and applied.
    async fn ask(
        &self,
        notes: &EnhancedNotes,
        scope: &Scope,
        instruction: &str,
    ) -> Result<Doc, EditError> {
        let blocks = &text_blocks(&notes.doc);
        let lines = self.ports.transcript.lines(&notes.meeting_id).await?;
        let refs = Refs::new(&lines);
        let mut shown = prompt_blocks(blocks, &refs, scope);
        let (instruction, transcript) = self.redact(&mut shown, instruction, lines).await;
        let selection = scope.describe();
        let prompt = EditPrompt {
            instruction: &instruction,
            selection: selection.as_deref(),
            blocks: &shown,
            transcript: &transcript,
        };
        let gateway = self.ports.gateway.as_ref().ok_or_else(no_model)?;
        let answer: EditAnswer = gateway
            .structured(Route::NotesEdit, prompt.render())
            .await?;
        let checks = Checks {
            blocks,
            shown: &shown,
            scope,
            refs: &refs,
        };
        let plan = plan(&answer, &checks);
        if plan.is_empty() {
            return Err(EditError::NoChange);
        }
        Ok(apply(&notes.doc, &plan))
    }

    /// Redacts the request, the blocks (locking those with a secret) and
    /// the transcript in one batch; returns the request and the transcript.
    async fn redact(
        &self,
        shown: &mut [PromptBlock],
        instruction: &str,
        lines: Vec<TranscriptLine>,
    ) -> (String, String) {
        let mut texts = vec![instruction.to_owned()];
        texts.extend(shown.iter().map(|block| block.text.clone()));
        texts.extend(lines.iter().map(|line| line.text.clone()));
        let mut redacted = self.ports.redactor.redact(texts).await.into_iter();
        let instruction = redacted.next().unwrap_or_default();
        let count = shown.len();
        redact_blocks(shown, redacted.by_ref().take(count).collect());
        let lines: Vec<TranscriptLine> = lines
            .into_iter()
            .zip(redacted)
            .map(|(line, text)| TranscriptLine { text, ..line })
            .collect();
        (instruction, render_lines(&lines))
    }
}

/// The failure of an AI edit when no gateway could be wired.
fn no_model() -> EditError {
    EditError::from(LlmError::Unavailable {
        provider: ProviderId::BuiltIn,
        reason: "no model is set up for editing notes".to_owned(),
    })
}

/// The proposal of `doc` over `notes`, or [`EditError::NoChange`].
fn proposal(
    notes: &EnhancedNotes,
    doc: Doc,
    method: EditMethod,
    occurrences: Option<u32>,
) -> Result<EditProposal, EditError> {
    let doc = doc.normalized();
    deino_doc::validate(&doc).map_err(EditError::InvalidDoc)?;
    let changes = diff(&notes.doc, &doc);
    if changes.is_empty() {
        return Err(EditError::NoChange);
    }
    Ok(EditProposal {
        meeting_id: notes.meeting_id.clone(),
        base_version: notes.version,
        method,
        occurrences,
        doc,
        changes,
    })
}
