//! Microsoft Graph's JSON as the core's plain source values. Graph does not
//! mark the user's own entry among the attendees, so the signed-in address
//! (`/me`) and the event's `responseStatus` stand in for it.

use chrono::{DateTime, NaiveDateTime, Utc};
use deino_core::calendar::{
    Availability, CalendarColor, EventKind, EventStatus, Participant, ParticipantKind,
    SourceCalendar, SourceEvent,
};
use deino_domain::Rsvp;

use super::wire::{Attendee, Calendar, DateTimeZone, Event, Recipient, ResponseStatus};
use crate::calendar::api::local_midnight;

/// Graph's wall-clock format (`2026-10-09T09:00:00.0000000`).
const GRAPH_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S%.f";

/// A calendar without its events.
pub(super) fn calendar(calendar: &Calendar) -> SourceCalendar {
    SourceCalendar {
        provider_id: calendar.id.clone(),
        title: calendar.name.clone().unwrap_or_default(),
        color: calendar
            .hex_color
            .as_deref()
            .and_then(|color| CalendarColor::parse(color).ok()),
        events: Vec::new(),
    }
}

/// One event of `me`'s calendar; `None` without usable times.
pub(super) fn event(event: Event, me: &str) -> Option<SourceEvent> {
    let start = instant(event.start.as_ref()?, event.is_all_day)?;
    let end = instant(event.end.as_ref()?, event.is_all_day)?;
    let (kind, availability) = show_as(event.show_as.as_deref());
    let conference_url = event
        .online_meeting
        .as_ref()
        .and_then(|meeting| meeting.join_url.clone())
        .or(event.online_meeting_url.clone());
    let attendees = participants(&event, me);
    Some(SourceEvent {
        provider_event_id: event.id,
        // Occurrences have their own iCalUId; the master's ID ties them.
        series_key: event.series_master_id,
        ical_uid: event.ical_uid,
        title: event.subject.unwrap_or_default(),
        notes: event.body.and_then(|body| body.content),
        location: event.location.and_then(|location| location.display_name),
        url: None,
        conference_url,
        start,
        end,
        all_day: event.is_all_day,
        status: if event.is_cancelled {
            EventStatus::Cancelled
        } else {
            EventStatus::Confirmed
        },
        kind,
        availability,
        organizer: event
            .organizer
            .as_ref()
            .map(|organizer| person(organizer, event.is_organizer)),
        attendees,
    })
}

/// When it starts or ends: UTC (the request prefers it); an all-day event's
/// date at the user's midnight.
fn instant(when: &DateTimeZone, all_day: bool) -> Option<DateTime<Utc>> {
    let wall = NaiveDateTime::parse_from_str(&when.date_time, GRAPH_DATE_TIME).ok()?;
    if all_day {
        return local_midnight(wall.date());
    }
    Some(wall.and_utc())
}

/// Graph's `showAs`: out of office is a block, the rest only availability.
fn show_as(show_as: Option<&str>) -> (Option<EventKind>, Availability) {
    match show_as {
        Some("oof") => (Some(EventKind::OutOfOffice), Availability::Unavailable),
        Some("free") => (Some(EventKind::Default), Availability::Free),
        Some("tentative") => (Some(EventKind::Default), Availability::Tentative),
        Some("busy" | "workingElsewhere") => (Some(EventKind::Default), Availability::Busy),
        _ => (Some(EventKind::Default), Availability::Unknown),
    }
}

/// The attendees, with the user's entry marked (and added, with their own
/// answer, when Graph lists them only through `responseStatus`).
fn participants(event: &Event, me: &str) -> Vec<Participant> {
    let mut participants: Vec<Participant> =
        event.attendees.iter().map(|a| attendee(a, me)).collect();
    let listed = participants.iter().any(|participant| participant.is_self);
    if !listed && !participants.is_empty() {
        participants.push(Participant {
            email: Some(me.to_owned()),
            name: None,
            rsvp: answer(event.response_status.as_ref()),
            kind: ParticipantKind::Person,
            is_self: true,
        });
    }
    participants
}

fn attendee(attendee: &Attendee, me: &str) -> Participant {
    let address = attendee.email_address.as_ref();
    let email = address.and_then(|address| address.address.clone());
    Participant {
        is_self: email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(me)),
        email,
        name: address.and_then(|address| address.name.clone()),
        rsvp: answer(attendee.status.as_ref()),
        kind: if attendee.kind.as_deref() == Some("resource") {
            ParticipantKind::Resource
        } else {
            ParticipantKind::Person
        },
    }
}

fn person(recipient: &Recipient, is_self: bool) -> Participant {
    let address = recipient.email_address.as_ref();
    Participant {
        email: address.and_then(|address| address.address.clone()),
        name: address.and_then(|address| address.name.clone()),
        rsvp: None,
        kind: ParticipantKind::Person,
        is_self,
    }
}

fn answer(status: Option<&ResponseStatus>) -> Option<Rsvp> {
    match status?.response.as_deref()? {
        "accepted" | "organizer" => Some(Rsvp::Accepted),
        "declined" => Some(Rsvp::Declined),
        "tentativelyAccepted" => Some(Rsvp::Tentative),
        "notResponded" => Some(Rsvp::NeedsAction),
        _ => None,
    }
}
