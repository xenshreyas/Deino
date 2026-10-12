//! The people side of an import (CAP-12): attendees become people through
//! ORG-04's [`EntityDirectory`] (keyed by email, else by name; companies from
//! work addresses) and join the meeting; action items are written through
//! AI-05's repository as the source's AI made them.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, AttendeeRole, Clock, Email, MeetingId,
    PersonId,
};

use super::{ImportActionItem, ImportAttendee, TranscriptImport, TranscriptImportError};
use crate::action_items::{ActionItem, ActionItemRepo, MAX_DESCRIPTION_CHARS, MAX_TITLE_CHARS};
use crate::entities::{EntityDirectory, PersonSeed};
use crate::people::{Attendee, AttendeeRepo};

/// What an importer writes attendees and action items through. Without it
/// (DEV-03's API importer), both are left out.
pub struct ImportPeoplePorts {
    /// Finds or adds each attendee and assignee as a person.
    pub directory: Arc<EntityDirectory>,
    /// Puts people on the meeting.
    pub attendees: Arc<dyn AttendeeRepo>,
    /// The meeting's action items.
    pub action_items: Arc<dyn ActionItemRepo>,
}

impl ImportPeoplePorts {
    /// Writes `import`'s attendees and action items onto meeting `id`.
    pub(super) async fn write(
        &self,
        id: &MeetingId,
        import: &TranscriptImport,
        clock: &dyn Clock,
    ) -> Result<(), TranscriptImportError> {
        for attendee in &import.attendees {
            let Some(person_id) = self.person(attendee).await? else {
                continue;
            };
            let attendee = Attendee {
                meeting_id: id.clone(),
                person_id,
                role: role(attendee),
                rsvp: None,
            };
            self.attendees.upsert_attendee(&attendee).await?;
        }
        for item in &import.action_items {
            self.add_item(id, item, clock).await?;
        }
        Ok(())
    }

    /// The person `attendee` names, added if new; `None` without a name or
    /// a valid address.
    async fn person(&self, attendee: &ImportAttendee) -> Result<Option<PersonId>, DbError> {
        let email = attendee
            .email
            .as_deref()
            .and_then(|email| Email::parse(email.trim()).ok());
        let seed = PersonSeed {
            email,
            name: attendee.name.clone(),
        };
        Ok(self
            .directory
            .upsert_person(&seed)
            .await?
            .map(|upserted| upserted.id))
    }

    async fn add_item(
        &self,
        id: &MeetingId,
        item: &ImportActionItem,
        clock: &dyn Clock,
    ) -> Result<(), TranscriptImportError> {
        let title = clipped(&item.title, MAX_TITLE_CHARS);
        if title.is_empty() {
            return Ok(());
        }
        let assignee = match &item.assignee {
            Some(assignee) => self.person(assignee).await?,
            None => None,
        };
        let now = clock.now();
        let (status, completed_at) = if item.done {
            (ActionItemStatus::Done, Some(now))
        } else {
            (ActionItemStatus::Open, None)
        };
        let row = ActionItem {
            id: ActionItemId::generate(clock),
            meeting_id: Some(id.clone()),
            title,
            description: clipped(&item.description, MAX_DESCRIPTION_CHARS),
            assignee_person_id: assignee,
            due_at: None,
            status,
            priority: None,
            source_segment_id: None,
            completed_at,
            created_by: ActionItemOrigin::Ai,
            created_at: now,
            updated_at: now,
        };
        self.action_items.create(&row).await?;
        Ok(())
    }
}

/// How the attendee took part, by the calendar's flags.
fn role(attendee: &ImportAttendee) -> AttendeeRole {
    if attendee.is_calendar_event_organizer {
        AttendeeRole::Organizer
    } else if attendee.is_calendar_invitee {
        AttendeeRole::Invitee
    } else {
        AttendeeRole::SpeakerOnly
    }
}

/// `text` trimmed and cut to `max` characters.
fn clipped(text: &str, max: usize) -> String {
    text.trim()
        .chars()
        .take(max)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_clipped_and_roles_follow_the_flags() {
        assert_eq!(clipped("  abc  ", 2), "ab");
        assert_eq!(clipped("a b", 2), "a");
        let organiser = ImportAttendee {
            is_calendar_event_organizer: true,
            ..ImportAttendee::default()
        };
        assert_eq!(role(&organiser), AttendeeRole::Organizer);
        assert_eq!(role(&ImportAttendee::default()), AttendeeRole::SpeakerOnly);
    }
}
