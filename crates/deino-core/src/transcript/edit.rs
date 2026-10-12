//! [`TranscriptEditService`]: the user's changes to a transcript (TRN-02).
//!
//! - **Edit** a line's text: the first edit keeps what the engine heard in
//!   `original_text`, every edit stamps `edited_at`.
//! - **Delete** a line (a soft delete, so it can be restored) and
//!   **restore** it. A deleted line leaves every reader at once: the
//!   transcript, the notes engine, exports, keyword search (ORG-08
//!   triggers) and the semantic index (the TRN-02 migration's trigger drops
//!   the windows that covered it).
//! - **Find and replace** across one meeting or all of them: a preview with
//!   counts first, then the same plan applied in one transaction.
//!
//! After each change the touched meetings go to the [`TranscriptIndexer`],
//! which re-embeds them in the background. The notes' "Transcript changed"
//! prompt needs nothing from here: `enhance::EnhanceBasis` sees `edited_at`
//! and missing segments.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId, SegmentId};

use super::pattern::Pattern;
use super::replace_plan::ReplacePlan;
use super::{ReplaceOutcome, ReplacePreview, ReplaceRequest, TranscriptError};
use crate::segments::{Segment, SegmentEditor, SegmentWriter};

/// The longest line an edit may leave, in characters.
pub const MAX_LINE_CHARS: usize = 10_000;
/// The longest text find and replace looks for, in characters.
pub const MAX_FIND_CHARS: usize = 500;

/// Re-indexes meetings whose transcript the user changed.
pub trait TranscriptIndexer: Send + Sync {
    /// Queues `meetings` for re-indexing and returns at once.
    fn changed(&self, meetings: &[MeetingId]);
}

/// What the edit service works with.
pub struct TranscriptEditDeps {
    /// Reads one line and applies batched edits.
    pub editor: Arc<dyn SegmentEditor>,
    /// Edits, deletes and restores one line.
    pub writer: Arc<dyn SegmentWriter>,
    /// Re-indexes the meetings it changes.
    pub indexer: Arc<dyn TranscriptIndexer>,
    /// Stamps `edited_at`.
    pub clock: Arc<dyn Clock>,
}

/// Applies the user's transcript edits.
pub struct TranscriptEditService {
    deps: TranscriptEditDeps,
}

impl TranscriptEditService {
    /// The service over its ports.
    pub fn new(deps: TranscriptEditDeps) -> Self {
        Self { deps }
    }

    /// Sets line `id`'s text to `text` (trimmed) and returns the line. The
    /// same text again changes nothing.
    pub async fn edit(&self, id: &SegmentId, text: &str) -> Result<Segment, TranscriptError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(TranscriptError::EmptyText);
        }
        within(text, MAX_LINE_CHARS)?;
        let segment = self.live_segment(id).await?;
        if segment.text == text {
            return Ok(segment);
        }
        self.deps
            .writer
            .edit_text(id, text, self.deps.clock.now())
            .await?;
        self.deps
            .indexer
            .changed(std::slice::from_ref(&segment.meeting_id));
        self.live_segment(id).await
    }

    /// Hides line `id` from the transcript and everything that reads it.
    pub async fn delete(&self, id: &SegmentId) -> Result<(), TranscriptError> {
        self.set_deleted(id, true).await
    }

    /// Brings a deleted line back.
    pub async fn restore(&self, id: &SegmentId) -> Result<(), TranscriptError> {
        self.set_deleted(id, false).await
    }

    /// What replacing `request.find` would change, without changing it.
    pub async fn preview(
        &self,
        request: &ReplaceRequest,
    ) -> Result<ReplacePreview, TranscriptError> {
        Ok(self.plan(request, "").await?.preview)
    }

    /// Replaces every match of `request.find` with `with`, in one
    /// transaction, and says what changed.
    pub async fn replace(
        &self,
        request: &ReplaceRequest,
        with: &str,
    ) -> Result<ReplaceOutcome, TranscriptError> {
        within(with, MAX_FIND_CHARS)?;
        let plan = self.plan(request, with).await?;
        if plan.edits.is_empty() {
            return Ok(plan.outcome());
        }
        let at = self.deps.clock.now();
        self.deps.editor.edit_many(&plan.edits, at).await?;
        self.deps.indexer.changed(&plan.meetings());
        Ok(plan.outcome())
    }

    /// The plan for `request`, replacing with `with`.
    async fn plan(
        &self,
        request: &ReplaceRequest,
        with: &str,
    ) -> Result<ReplacePlan, TranscriptError> {
        if request.find.trim().is_empty() {
            return Err(TranscriptError::EmptyFind);
        }
        within(&request.find, MAX_FIND_CHARS)?;
        let lines = self
            .deps
            .editor
            .matching(request.scope.meeting(), &request.find)
            .await?;
        let pattern = Pattern::new(&request.find, request.match_case);
        Ok(ReplacePlan::new(lines, pattern, with))
    }

    /// Soft-deletes or restores line `id`, then re-indexes its meeting.
    async fn set_deleted(&self, id: &SegmentId, deleted: bool) -> Result<(), TranscriptError> {
        let segment = self.segment(id).await?;
        if segment.is_deleted != deleted {
            self.deps.writer.set_deleted(id, deleted).await?;
            self.deps
                .indexer
                .changed(std::slice::from_ref(&segment.meeting_id));
        }
        Ok(())
    }

    /// Line `id`, deleted or not.
    async fn segment(&self, id: &SegmentId) -> Result<Segment, TranscriptError> {
        let found = self.deps.editor.segment(id).await?;
        found.ok_or(TranscriptError::SegmentNotFound)
    }

    /// Line `id`, which must not be deleted (a deleted line is not edited).
    async fn live_segment(&self, id: &SegmentId) -> Result<Segment, TranscriptError> {
        let segment = self.segment(id).await?;
        if segment.is_deleted {
            return Err(TranscriptError::SegmentNotFound);
        }
        Ok(segment)
    }
}

/// Rejects `text` longer than `max` characters.
fn within(text: &str, max: usize) -> Result<(), TranscriptError> {
    if text.chars().count() > max {
        return Err(TranscriptError::TooLong { max });
    }
    Ok(())
}
