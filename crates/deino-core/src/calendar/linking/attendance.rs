//! [`EventAttendance`]: a linked event's invitees as the meeting's
//! attendees.
//!
//! Each invitee who did not decline becomes a `meeting_attendees` row: the
//! user's own address is the "Me" person, anyone else is found or added
//! through the calendar's [`AttendeeDirectory`] (keyed by email, with their
//! company). A broadcast invitation (over
//! [`MAX_ATTENDEES_FOR_PEOPLE`]) adds no new people, only attaches those
//! already known. Re-linking takes the previous event's invitees off the
//! meeting unless the new event has them too; people added another way
//! (a named speaker) stay.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{AttendeeRole, Email, MeetingId, PersonId};

use crate::calendar::entities::invitee;
use crate::calendar::{AttendeeDirectory, CalendarEvent, MAX_ATTENDEES_FOR_PEOPLE};
use crate::people::{Attendee, AttendeeRepo, MeRepo};
use crate::settings::SettingsService;

/// How many attendees a link added and removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttendeeChange {
    /// Rows added for people not yet on the meeting.
    pub added: u32,
    /// The previous event's invitees taken off.
    pub removed: u32,
}

/// Who the user is in an invitation: their person and their addresses.
struct Own {
    me: Option<PersonId>,
    emails: BTreeSet<Email>,
}

/// Turns an event's invitees into a meeting's attendees.
pub struct EventAttendance {
    directory: AttendeeDirectory,
    attendees: Arc<dyn AttendeeRepo>,
    me: Arc<dyn MeRepo>,
    settings: Arc<SettingsService>,
}

impl EventAttendance {
    /// Attendance over the people directory, the attendee rows, the "Me"
    /// person and the profile's addresses (Settings → Profile).
    pub fn new(
        directory: AttendeeDirectory,
        attendees: Arc<dyn AttendeeRepo>,
        me: Arc<dyn MeRepo>,
        settings: Arc<SettingsService>,
    ) -> Self {
        Self {
            directory,
            attendees,
            me,
            settings,
        }
    }

    /// Replaces the attendees `previous` brought to `meeting` with the
    /// invitees of `next` (none to unlink).
    pub async fn replace(
        &self,
        meeting: &MeetingId,
        previous: Option<&CalendarEvent>,
        next: Option<&CalendarEvent>,
    ) -> Result<AttendeeChange, DbError> {
        let own = self.own().await?;
        let wanted = match next {
            Some(event) => self.attendees_of(meeting, event, &own).await?,
            None => Vec::new(),
        };
        let stale = match previous {
            Some(event) => self.known_invitees(event, &own).await?,
            None => BTreeSet::new(),
        };
        let current = self.roles(meeting).await?;
        let added = self.add(&wanted, &current).await?;
        let keep: BTreeSet<PersonId> = wanted.iter().map(|row| row.person_id).collect();
        let removed = self
            .remove(meeting, stale.difference(&keep), &current)
            .await?;
        Ok(AttendeeChange { added, removed })
    }

    /// The user's person and every address that is theirs.
    async fn own(&self) -> Result<Own, DbError> {
        let me = self.me.me().await?;
        let mut emails: BTreeSet<Email> = self
            .settings
            .get()
            .await
            .profile
            .emails
            .into_iter()
            .collect();
        emails.extend(me.as_ref().and_then(|person| person.email.clone()));
        Ok(Own {
            me: me.map(|person| person.id),
            emails,
        })
    }

    /// The attendee rows `event` gives `meeting`, adding new people unless
    /// the invitation is a broadcast.
    async fn attendees_of(
        &self,
        meeting: &MeetingId,
        event: &CalendarEvent,
        own: &Own,
    ) -> Result<Vec<Attendee>, DbError> {
        let create = event.attendees.len() <= MAX_ATTENDEES_FOR_PEOPLE;
        let mut rows = Vec::new();
        for attendee in &event.attendees {
            let Some(found) = invitee(attendee) else {
                continue;
            };
            let Some(person_id) = self.person(found, own, create).await? else {
                continue;
            };
            rows.push(Attendee {
                meeting_id: meeting.clone(),
                person_id,
                role: role(attendee.is_organizer),
                rsvp: attendee.rsvp,
            });
        }
        Ok(rows)
    }

    /// The people of `event`'s invitees who exist, without adding anyone.
    async fn known_invitees(
        &self,
        event: &CalendarEvent,
        own: &Own,
    ) -> Result<BTreeSet<PersonId>, DbError> {
        let mut people = BTreeSet::new();
        for found in event.attendees.iter().filter_map(invitee) {
            people.extend(self.person(found, own, false).await?);
        }
        Ok(people)
    }

    /// The person behind an invitee's address: the user, or someone in the
    /// directory (added when `create`).
    async fn person(
        &self,
        (email, name): (Email, Option<String>),
        own: &Own,
        create: bool,
    ) -> Result<Option<PersonId>, DbError> {
        if own.emails.contains(&email) {
            return Ok(own.me);
        }
        self.directory.person_for(&email, name, create).await
    }

    /// Each current attendee's role.
    async fn roles(
        &self,
        meeting: &MeetingId,
    ) -> Result<BTreeMap<PersonId, AttendeeRole>, DbError> {
        let rows = self.attendees.attendees(meeting).await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.person_id, row.role))
            .collect())
    }

    /// Writes `wanted`; counts the people who were not on the meeting.
    async fn add(
        &self,
        wanted: &[Attendee],
        current: &BTreeMap<PersonId, AttendeeRole>,
    ) -> Result<u32, DbError> {
        let mut added = 0;
        for row in wanted {
            self.attendees.upsert_attendee(row).await?;
            added += u32::from(!current.contains_key(&row.person_id));
        }
        Ok(added)
    }

    /// Takes `stale` invitees off the meeting; a person who only spoke, or
    /// is not on it, is left alone.
    async fn remove(
        &self,
        meeting: &MeetingId,
        stale: impl Iterator<Item = &PersonId>,
        current: &BTreeMap<PersonId, AttendeeRole>,
    ) -> Result<u32, DbError> {
        let mut removed = 0;
        for person in stale {
            if current
                .get(person)
                .is_some_and(|role| *role != AttendeeRole::SpeakerOnly)
            {
                removed += u32::from(self.attendees.remove_attendee(meeting, person).await?);
            }
        }
        Ok(removed)
    }
}

/// The role an invitee has.
fn role(is_organizer: bool) -> AttendeeRole {
    if is_organizer {
        AttendeeRole::Organizer
    } else {
        AttendeeRole::Invitee
    }
}
