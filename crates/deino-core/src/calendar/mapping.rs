//! From what a source reports to what Deino stores: the same reading of an
//! event for every provider.
//!
//! [`to_batch`] drops what [`filter::keeps`] rejects, finds the join link
//! ([`find_conference`]), keeps only people as attendees (no rooms, no
//! resources), works out the user's answer and whether the event is
//! external ([`OwnAddresses`], from the user's own entries in the
//! invitations), and names its recurring series.

use std::collections::BTreeSet;

use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarId, CalendarProvider, Clock, Email, Rsvp, SeriesId,
};

use super::audience::OwnAddresses;
use super::batch::{SyncBatch, SyncedAccount, SyncedCalendar, SyncedEvent};
use super::conference::{Conference, ConferenceFields, find_conference};
use super::filter;
use super::source::{Participant, ParticipantKind, SourceEvent, SourceSnapshot, SyncWindow};
use super::{EventAttendee, SourceCalendar};

/// Domains Google uses for rooms and calendars invited like people.
const CALENDAR_RESOURCE_DOMAINS: &[&str] = &[
    "resource.calendar.google.com",
    "group.calendar.google.com",
    "group.v.calendar.google.com",
    "import.calendar.google.com",
];

/// The scheme some providers put in front of an attendee's address.
const MAILTO: &str = "mailto:";

/// What a batch is built for.
#[derive(Clone, Copy, Debug)]
pub struct BatchContext<'a> {
    /// The provider the snapshot came from.
    pub provider: CalendarProvider,
    /// The window it covers.
    pub window: SyncWindow,
    /// Stamps the sync and the candidate IDs.
    pub clock: &'a dyn Clock,
}

/// The batch to store for `snapshot`, and the user's own addresses it shows.
pub fn to_batch(
    snapshot: &SourceSnapshot,
    context: &BatchContext<'_>,
) -> (SyncBatch, OwnAddresses) {
    let own = own_addresses(snapshot);
    let accounts = snapshot
        .accounts
        .iter()
        .map(|account| SyncedAccount {
            id: CalendarAccountId::generate(context.clock),
            name: account.name.clone(),
            calendars: account
                .calendars
                .iter()
                .map(|calendar| to_calendar(calendar, &own, context.clock))
                .collect(),
        })
        .collect();
    let batch = SyncBatch {
        provider: context.provider,
        window: context.window,
        synced_at: context.clock.now(),
        accounts,
    };
    (batch, own)
}

/// Every address the snapshot marks as the user's.
fn own_addresses(snapshot: &SourceSnapshot) -> OwnAddresses {
    let participants = snapshot
        .accounts
        .iter()
        .flat_map(|account| &account.calendars)
        .flat_map(|calendar| &calendar.events)
        .flat_map(|event| event.attendees.iter().chain(&event.organizer));
    OwnAddresses::from_emails(
        participants
            .filter(|participant| participant.is_self)
            .filter_map(email_of),
    )
}

fn to_calendar(calendar: &SourceCalendar, own: &OwnAddresses, clock: &dyn Clock) -> SyncedCalendar {
    SyncedCalendar {
        id: CalendarId::generate(clock),
        provider_calendar_id: calendar.provider_id.clone(),
        title: calendar.title.trim().to_owned(),
        color: calendar.color.clone(),
        events: calendar
            .events
            .iter()
            .filter(|event| filter::keeps(event))
            .map(|event| to_event(event, own, clock))
            .collect(),
    }
}

fn to_event(event: &SourceEvent, own: &OwnAddresses, clock: &dyn Clock) -> SyncedEvent {
    let attendees = attendees_of(event);
    let emails: Vec<Email> = attendees
        .iter()
        .filter_map(|attendee| attendee.email.as_deref())
        .filter_map(|email| Email::parse(email).ok())
        .collect();
    let conference = find_conference(ConferenceFields {
        conference: event.conference_url.as_deref(),
        url: event.url.as_deref(),
        location: event.location.as_deref(),
        notes: event.notes.as_deref(),
    });
    SyncedEvent {
        id: CalendarEventId::generate(clock),
        provider_event_id: event.provider_event_id.clone(),
        ical_uid: event.ical_uid.clone(),
        series_id: event.series_key.clone().map(SeriesId::new),
        title: event.title.trim().to_owned(),
        description: event.notes.clone().unwrap_or_default(),
        start_at: event.start,
        end_at: event.end.max(event.start),
        is_all_day: event.all_day,
        platform: conference.as_ref().map(Conference::platform),
        conference_url: conference.map(|conference| conference.url),
        organizer_email: event
            .organizer
            .as_ref()
            .and_then(email_of)
            .map(String::from),
        my_rsvp: my_rsvp(event, own),
        is_external: own.any_external(&emails),
        attendees,
    }
}

/// The people invited, each address once, with the organiser added first
/// when the provider lists them apart from the invitees.
fn attendees_of(event: &SourceEvent) -> Vec<EventAttendee> {
    let organizer = event.organizer.as_ref().and_then(email_of);
    let mut seen: BTreeSet<Email> = BTreeSet::new();
    let mut attendees: Vec<EventAttendee> = event
        .attendees
        .iter()
        .filter(|participant| is_person(participant))
        .filter(|participant| email_of(participant).is_none_or(|email| seen.insert(email)))
        .map(|participant| {
            let email = email_of(participant);
            attendee(participant, email.is_some() && email == organizer)
        })
        .collect();
    if let (Some(participant), Some(email)) = (&event.organizer, &organizer) {
        let missing = !attendees.is_empty() && !seen.contains(email) && is_person(participant);
        if missing {
            attendees.insert(0, attendee(participant, true));
        }
    }
    attendees
}

/// `participant` as stored in `attendees_json`.
fn attendee(participant: &Participant, is_organizer: bool) -> EventAttendee {
    let rsvp = participant.rsvp.or(is_organizer.then_some(Rsvp::Accepted));
    EventAttendee {
        email: email_of(participant).map(String::from),
        name: participant
            .name
            .clone()
            .filter(|name| !name.trim().is_empty()),
        rsvp,
        is_organizer,
    }
}

/// The user's answer: their own entry's, accepted when they organised it,
/// or the answer of an attendee with one of their addresses.
fn my_rsvp(event: &SourceEvent, own: &OwnAddresses) -> Option<Rsvp> {
    if let Some(me) = event
        .attendees
        .iter()
        .find(|participant| participant.is_self)
    {
        return me.rsvp;
    }
    if event
        .organizer
        .as_ref()
        .is_some_and(|organizer| organizer.is_self)
    {
        return Some(Rsvp::Accepted);
    }
    event
        .attendees
        .iter()
        .find(|participant| email_of(participant).is_some_and(|email| own.is_own(&email)))
        .and_then(|participant| participant.rsvp)
}

/// True for a person: not a room, a resource, a list, or a Google calendar
/// invited like a person.
fn is_person(participant: &Participant) -> bool {
    participant.kind == ParticipantKind::Person
        && !email_of(participant)
            .is_some_and(|email| CALENDAR_RESOURCE_DOMAINS.contains(&email.domain()))
}

/// The participant's address in canonical form, if it is one.
fn email_of(participant: &Participant) -> Option<Email> {
    let raw = participant.email.as_deref()?.trim();
    let address = raw
        .get(..MAILTO.len())
        .filter(|scheme| scheme.eq_ignore_ascii_case(MAILTO))
        .and_then(|_| raw.get(MAILTO.len()..))
        .unwrap_or(raw);
    Email::parse(address).ok()
}

#[cfg(test)]
mod mapping_tests;
