//! [`LastMeetingService`]: "Review last meeting" on an upcoming event
//! (ORG-02). The meeting before it in its recurring series, or the latest
//! one with the same title and people ([`RelatedService`]'s rules), else the
//! latest one with exactly the same people, whatever its title.

use std::collections::BTreeSet;
use std::sync::Arc;

use deino_domain::{Email, MeetingId, PersonId};

use super::{LinkError, RelatedBy, RelatedMeeting, RelatedService, RelationKey};
use crate::calendar::CalendarEvent;
use crate::people::{AttendeeRepo, MeRepo, PersonRepo};

/// How many of an attendee's meetings, newest first, are compared by people.
pub const PEOPLE_SCAN: usize = 20;

/// Finds the meeting to review before an event.
pub struct LastMeetingService {
    related: Arc<RelatedService>,
    people: Arc<dyn PersonRepo>,
    attendance: Arc<dyn AttendeeRepo>,
    me: Arc<dyn MeRepo>,
}

impl LastMeetingService {
    /// A service over the related meetings and the people repositories.
    pub fn new(
        related: Arc<RelatedService>,
        people: Arc<dyn PersonRepo>,
        attendance: Arc<dyn AttendeeRepo>,
        me: Arc<dyn MeRepo>,
    ) -> Self {
        Self {
            related,
            people,
            attendance,
            me,
        }
    }

    /// The meeting to review before `event`, if there is one.
    pub async fn last_meeting(
        &self,
        event: &CalendarEvent,
    ) -> Result<Option<RelatedMeeting>, LinkError> {
        let people = self.people_of(event).await?;
        let key = RelationKey {
            // An ID no meeting has: the event's own note may be listed.
            meeting: MeetingId::new(event.id.to_string()),
            title: event.title.clone(),
            series_id: event.series_id.clone(),
            happened_at: event.start_at,
            attendees: people.clone(),
        };
        if let Some(previous) = self.related.related_to(&key).await?.previous {
            return Ok(Some(previous));
        }
        self.same_people(event, &people).await
    }

    /// The people invited to `event` whom Deino knows, merges followed.
    async fn people_of(&self, event: &CalendarEvent) -> Result<BTreeSet<PersonId>, LinkError> {
        let mut people = BTreeSet::new();
        let emails = event.attendees.iter().filter_map(|a| a.email.as_deref());
        for email in emails.filter_map(|email| Email::parse(email).ok()) {
            if let Some(person) = self.people.find_by_email(&email).await? {
                people.insert(person.merged_into.unwrap_or(person.id));
            }
        }
        Ok(people)
    }

    /// The latest meeting before `event` attended by exactly `people`, the
    /// user left out.
    async fn same_people(
        &self,
        event: &CalendarEvent,
        people: &BTreeSet<PersonId>,
    ) -> Result<Option<RelatedMeeting>, LinkError> {
        let me = self.me.me().await?.map(|person| person.id);
        let others = without(people.iter().copied(), me.as_ref());
        let Some(first) = others.first() else {
            return Ok(None);
        };
        let meetings = self.attendance.meetings_of(first).await?;
        let earlier = meetings
            .into_iter()
            .filter(|meeting| meeting.created_at < event.start_at);
        for meeting in earlier.take(PEOPLE_SCAN) {
            let attendees = self.attendance.attendees(&meeting.id).await?;
            let theirs = without(attendees.into_iter().map(|a| a.person_id), me.as_ref());
            if theirs == others {
                return Ok(Some(RelatedMeeting {
                    id: meeting.id,
                    title: meeting.title,
                    happened_at: meeting.created_at,
                    by: RelatedBy::People,
                }));
            }
        }
        Ok(None)
    }
}

/// `people` without the user.
fn without(people: impl Iterator<Item = PersonId>, me: Option<&PersonId>) -> BTreeSet<PersonId> {
    people.filter(|person| Some(person) != me).collect()
}

#[cfg(test)]
#[path = "last_meeting_tests.rs"]
mod tests;
