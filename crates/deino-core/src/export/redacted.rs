//! Exports with secrets `[redacted]` (NOT-09): the notes through the
//! [`RedactedNotes`] decorator, the transcript and action items through
//! [`redact_utterances`] and [`redact_items`]. Every reader of the
//! [`ExportPorts`](super::ExportPorts) (exports, auto-export, the `deino` CLI
//! and its MCP server) gets the same text.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::MeetingId;
use deino_export::{ActionItemExport, Utterance};

use super::{MeetingNotes, NoteRenderer};
use crate::redaction::TextRedactor;

/// [`NoteRenderer`] whose Markdown has its secrets redacted.
pub struct RedactedNotes {
    inner: Arc<dyn NoteRenderer>,
    redactor: Arc<dyn TextRedactor>,
}

impl RedactedNotes {
    /// `inner`'s notes, through `redactor`.
    pub fn new(inner: Arc<dyn NoteRenderer>, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { inner, redactor }
    }
}

#[async_trait]
impl NoteRenderer for RedactedNotes {
    async fn notes(&self, meeting: &MeetingId) -> Result<MeetingNotes, DbError> {
        let notes = self.inner.notes(meeting).await?;
        let texts = vec![
            notes.enhanced.clone().unwrap_or_default(),
            notes.mine.clone().unwrap_or_default(),
        ];
        let mut texts = self.redactor.redact(texts).await.into_iter();
        let (enhanced, mine) = (texts.next(), texts.next());
        Ok(MeetingNotes {
            enhanced: notes.enhanced.and(enhanced),
            mine: notes.mine.and(mine),
        })
    }
}

/// `utterances` with their secrets redacted, in one batch.
pub(super) async fn redact_utterances(
    redactor: &dyn TextRedactor,
    utterances: Vec<Utterance>,
) -> Vec<Utterance> {
    let texts = utterances
        .iter()
        .map(|utterance| utterance.text.clone())
        .collect();
    let texts = redactor.redact(texts).await;
    utterances
        .into_iter()
        .zip(texts)
        .map(|(utterance, text)| Utterance { text, ..utterance })
        .collect()
}

/// `items` with secrets redacted from their titles and descriptions.
pub(super) async fn redact_items(
    redactor: &dyn TextRedactor,
    items: Vec<ActionItemExport>,
) -> Vec<ActionItemExport> {
    let texts = items
        .iter()
        .flat_map(|item| [item.title.clone(), item.description.clone()])
        .collect();
    let mut texts = redactor.redact(texts).await.into_iter();
    items
        .into_iter()
        .map(|item| ActionItemExport {
            title: texts.next().unwrap_or_default(),
            description: texts.next().unwrap_or_default(),
            ..item
        })
        .collect()
}
