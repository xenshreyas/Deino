//! Moving notes from a draft meeting into the meeting a recording saved.
//!
//! Until FND-05b creates the meeting row when recording starts, the record
//! screen's notepad writes into a `scheduled` draft (so every keystroke is
//! already on disk), and the legacy stop path saves the recording as a new
//! meeting. [`NoteService::adopt`] then moves the notes and images across
//! and deletes the draft. FND-05b removes the need for this.

use deino_doc::Doc;
use deino_domain::{MeetingId, MeetingStatus};

use super::{NoteService, NotesError, SavedNotes};
use crate::meetings::MeetingState;

impl NoteService {
    /// Moves the scheduled draft `draft`'s notes into `into` (after any notes
    /// `into` already has), moves its images, and deletes the draft.
    ///
    /// Images move first: if the database step fails, the draft still
    /// exists and adopting again finishes the job.
    pub async fn adopt(
        &self,
        draft: &MeetingId,
        into: &MeetingId,
    ) -> Result<SavedNotes, NotesError> {
        let drafted_state = self.meeting(draft).await?;
        if drafted_state.status != MeetingStatus::Scheduled || draft == into {
            return Err(NotesError::NotADraft(draft.clone()));
        }
        self.carry_template(&drafted_state, into).await?;
        let drafted = self.ports.notes.get(draft).await?.map(|notes| notes.doc);
        let existing = self.ports.notes.get(into).await?.map(|notes| notes.doc);
        self.ports
            .images
            .relocate(draft, into)
            .await
            .map_err(NotesError::Files)?;
        let notes = self.prepare(into, merge(existing, drafted))?;
        if !self.ports.notes.transfer(&notes, draft).await? {
            return Err(NotesError::NotADraft(draft.clone()));
        }
        log::info!(target: "notes", "moved the notes of draft {draft} into {into}");
        Ok(self.announce(&notes))
    }

    /// A template chosen on the draft (`/template`) becomes the meeting's,
    /// unless the meeting already has one.
    async fn carry_template(
        &self,
        draft: &MeetingState,
        into: &MeetingId,
    ) -> Result<(), NotesError> {
        let target = self.meeting(into).await?;
        if let (None, Some(template)) = (&target.template_id, &draft.template_id) {
            let now = self.clock.now();
            self.ports
                .meetings
                .set_template(into, Some(template), now)
                .await?;
        }
        Ok(())
    }
}

/// The meeting's own notes, then the draft's; blank documents add nothing.
fn merge(existing: Option<Doc>, drafted: Option<Doc>) -> Doc {
    let content: Vec<_> = [existing, drafted]
        .into_iter()
        .flatten()
        .filter(|doc| !doc.is_blank())
        .flat_map(|doc| doc.content)
        .collect();
    if content.is_empty() {
        Doc::empty()
    } else {
        Doc { content }
    }
}

#[cfg(test)]
mod tests {
    use deino_doc::{Block, Inline};

    use super::*;

    fn note(text: &str) -> Doc {
        Doc {
            content: vec![Block::paragraph(vec![Inline::text(text, vec![])])],
        }
    }

    #[test]
    fn merging_keeps_the_meetings_notes_first_and_skips_blank_ones() {
        let merged = merge(Some(note("agenda")), Some(note("during")));
        assert_eq!(merged.content.len(), 2);
        assert_eq!(merged.content[0], note("agenda").content[0]);

        assert_eq!(
            merge(Some(Doc::empty()), Some(note("during"))),
            note("during")
        );
        assert_eq!(merge(None, None), Doc::empty());
    }
}
