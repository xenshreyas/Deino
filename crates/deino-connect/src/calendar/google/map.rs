//! Google's JSON as the core's plain source values. Rules stay in the core;
//! this only translates Google's words (event types, response statuses,
//! transparency, all-day dates).

use chrono::{DateTime, Utc};
use deino_core::calendar::{
    Availability, CalendarColor, EventKind, EventStatus, Participant, ParticipantKind,
    SourceCalendar, SourceEvent,
};
use deino_domain::Rsvp;

use super::wire::{Attendee, CalendarEntry, Event, Person, When};
use crate::calendar::api::local_midnight;

/// A calendar without its events.
pub(super) fn calendar(entry: &CalendarEntry) -> SourceCalendar {
    let title = entry.summary_override.as_ref().or(entry.summary.as_ref());
    SourceCalendar {
        provider_id: entry.id.clone(),
        title: title.cloned().unwrap_or_default(),
        color: entry
            .background_color
            .as_deref()
            .and_then(|color| CalendarColor::parse(color).ok()),
        events: Vec::new(),
    }
}

/// One event; `None` for birthdays and events without usable times.
pub(super) fn event(event: Event) -> Option<SourceEvent> {
    let kind = kind_of(event.kind.as_deref())?;
    let (start, all_day) = instant(event.start.as_ref()?)?;
    let (end, _) = instant(event.end.as_ref()?)?;
    let conference_url = event.hangout_link.clone().or_else(|| video_link(&event));
    Some(SourceEvent {
        series_key: event
            .recurring_event_id
            .as_ref()
            .map(|series| event.ical_uid.clone().unwrap_or_else(|| series.clone())),
        provider_event_id: event.id,
        ical_uid: event.ical_uid,
        title: event.summary.unwrap_or_default(),
        notes: event.description,
        location: event.location,
        url: None,
        conference_url,
        start,
        end,
        all_day,
        status: status(event.status.as_deref()),
        kind: Some(kind),
        availability: if event.transparency.as_deref() == Some("transparent") {
            Availability::Free
        } else {
            Availability::Busy
        },
        organizer: event.organizer.map(organizer),
        attendees: event.attendees.into_iter().map(attendee).collect(),
    })
}

/// Google's event type; `None` for the contacts' birthdays.
fn kind_of(event_type: Option<&str>) -> Option<EventKind> {
    match event_type {
        Some("birthday") => None,
        Some("outOfOffice") => Some(EventKind::OutOfOffice),
        Some("focusTime") => Some(EventKind::Focus),
        Some("workingLocation") => Some(EventKind::WorkingLocation),
        _ => Some(EventKind::Default),
    }
}

/// When it starts or ends, and whether it is an all-day date.
fn instant(when: &When) -> Option<(DateTime<Utc>, bool)> {
    if let Some(instant) = when.date_time {
        return Some((instant.with_timezone(&Utc), false));
    }
    Some((local_midnight(when.date?)?, true))
}

/// The first video entry point of the conference data.
fn video_link(event: &Event) -> Option<String> {
    let points = &event.conference_data.as_ref()?.entry_points;
    points
        .iter()
        .find(|point| point.entry_point_type.as_deref() == Some("video"))
        .and_then(|point| point.uri.clone())
}

fn status(status: Option<&str>) -> EventStatus {
    match status {
        Some("cancelled") => EventStatus::Cancelled,
        Some("tentative") => EventStatus::Tentative,
        _ => EventStatus::Confirmed,
    }
}

fn rsvp(status: Option<&str>) -> Option<Rsvp> {
    match status? {
        "accepted" => Some(Rsvp::Accepted),
        "declined" => Some(Rsvp::Declined),
        "tentative" => Some(Rsvp::Tentative),
        "needsAction" => Some(Rsvp::NeedsAction),
        _ => None,
    }
}

fn attendee(attendee: Attendee) -> Participant {
    Participant {
        rsvp: rsvp(attendee.response_status.as_deref()),
        email: attendee.email,
        name: attendee.display_name,
        kind: if attendee.resource {
            ParticipantKind::Room
        } else {
            ParticipantKind::Person
        },
        is_self: attendee.is_self,
    }
}

fn organizer(person: Person) -> Participant {
    Participant {
        email: person.email,
        name: person.display_name,
        rsvp: None,
        kind: ParticipantKind::Person,
        is_self: person.is_self,
    }
}
