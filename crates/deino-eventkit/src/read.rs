//! Reading the store: accounts, calendars and events in a window, as the
//! core's plain [`SourceSnapshot`] values.
//!
//! Every `EventKit` getter is `unsafe` in objc2 (they are plain property
//! reads on live objects); this module and `store.rs` are the only places
//! they are made. The meaning of each raw value is decided in `convert.rs`.

#![allow(
    unsafe_code,
    reason = "objc2 marks every EventKit property read unsafe; each is a getter on a live, retained object"
)]

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use deino_core::calendar::{
    CalendarColor, Participant, SourceAccount, SourceCalendar, SourceEvent, SourceSnapshot,
    SyncWindow,
};
use objc2::rc::Retained;
use objc2_app_kit::NSColorSpace;
use objc2_event_kit::{EKCalendar, EKEntityType, EKEvent, EKEventStore, EKParticipant};
use objc2_foundation::{NSDate, NSString};

use crate::convert;

/// Every account, calendar and event of `store` overlapping `window`.
pub(crate) fn snapshot(store: &EKEventStore, window: SyncWindow) -> SourceSnapshot {
    // SAFETY: a live store. It fetches stale remote calendars in the
    // background; their changes arrive as a change notification.
    unsafe { store.refreshSourcesIfNecessary() };
    let mut calendars = calendars(store);
    for event in events(store, window) {
        // SAFETY: a getter on an event the store just returned.
        let Some(calendar) = (unsafe { event.calendar() }) else {
            continue;
        };
        // SAFETY: a getter on a live calendar.
        let id = unsafe { calendar.calendarIdentifier() }.to_string();
        if let Some((_, entry)) = calendars.get_mut(&id) {
            entry.events.extend(read_event(&event));
        }
    }
    let mut accounts: BTreeMap<String, Vec<SourceCalendar>> = BTreeMap::new();
    for (account, calendar) in calendars.into_values() {
        accounts.entry(account).or_default().push(calendar);
    }
    let accounts = accounts
        .into_iter()
        .map(|(name, calendars)| SourceAccount { name, calendars })
        .collect();
    SourceSnapshot { accounts }
}

/// Every event calendar except birthdays, keyed by its identifier, with its
/// account's name.
fn calendars(store: &EKEventStore) -> BTreeMap<String, (String, SourceCalendar)> {
    // SAFETY: a query on a live store for a valid entity type.
    let calendars = unsafe { store.calendarsForEntityType(EKEntityType::Event) };
    calendars
        .to_vec()
        .iter()
        .filter(|calendar| {
            // SAFETY: a getter on a live calendar.
            !convert::is_birthday_calendar(unsafe { calendar.r#type() }.0)
        })
        .map(|calendar| {
            let source = read_calendar(calendar);
            (source.1.provider_id.clone(), source)
        })
        .collect()
}

/// A calendar and the name of its account (its `EKSource`).
fn read_calendar(calendar: &EKCalendar) -> (String, SourceCalendar) {
    // SAFETY: getters on a live calendar and its source.
    let (account, provider_id, title) = unsafe {
        let account = calendar.source().map(|source| source.title().to_string());
        (account, calendar.calendarIdentifier(), calendar.title())
    };
    let calendar = SourceCalendar {
        provider_id: provider_id.to_string(),
        title: title.to_string(),
        color: color_of(calendar),
        events: Vec::new(),
    };
    (account.unwrap_or_else(|| "Other".to_owned()), calendar)
}

/// The calendar's colour in sRGB.
fn color_of(calendar: &EKCalendar) -> Option<CalendarColor> {
    // SAFETY: a getter on a live calendar; the colour is converted, not changed.
    let color = unsafe { calendar.color() };
    let srgb = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    Some(CalendarColor::from_unit_rgb(
        srgb.redComponent(),
        srgb.greenComponent(),
        srgb.blueComponent(),
    ))
}

/// The events of every calendar overlapping `window`.
fn events(store: &EKEventStore, window: SyncWindow) -> Vec<Retained<EKEvent>> {
    let start = NSDate::dateWithTimeIntervalSince1970(convert::seconds(window.start));
    let end = NSDate::dateWithTimeIntervalSince1970(convert::seconds(window.end));
    // SAFETY: the predicate comes from this store's own factory, as
    // `eventsMatchingPredicate` requires; `None` means every calendar.
    unsafe {
        let predicate = store.predicateForEventsWithStartDate_endDate_calendars(&start, &end, None);
        store.eventsMatchingPredicate(&predicate).to_vec()
    }
}

/// Which occurrence an event is and when: what identifies it in a sync.
struct Identity {
    provider_event_id: String,
    ical_uid: Option<String>,
    series_key: Option<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

/// One event (one occurrence of a recurring one); `None` without valid times.
fn read_event(event: &EKEvent) -> Option<SourceEvent> {
    Some(describe(event, identity(event)?))
}

/// The event's occurrence ID, iCalendar UID, series and times. Recurring
/// occurrences share the series key: the iCalendar UID, which Google and
/// Microsoft report for the same series too.
fn identity(event: &EKEvent) -> Option<Identity> {
    // SAFETY: getters on a live event.
    let (id, uid, recurring, times) = unsafe {
        let id = event
            .eventIdentifier()
            .unwrap_or_else(|| event.calendarItemIdentifier());
        let uid = event
            .calendarItemExternalIdentifier()
            .map(|uid| uid.to_string());
        let recurring = event.hasRecurrenceRules() || event.isDetached();
        (
            id.to_string(),
            uid,
            recurring,
            (event.startDate(), event.endDate(), event.occurrenceDate()),
        )
    };
    let (start, end, occurrence) = times;
    let occurrence = occurrence
        .filter(|_| recurring)
        .and_then(|date| convert::instant(date.timeIntervalSince1970()));
    Some(Identity {
        provider_event_id: convert::occurrence_id(&id, occurrence),
        series_key: recurring.then(|| uid.clone().unwrap_or(id)),
        ical_uid: uid,
        start: convert::instant(start.timeIntervalSince1970())?,
        end: convert::instant(end.timeIntervalSince1970())?,
    })
}

/// The event as the core reads it: its identity, text fields, state and
/// participants.
fn describe(event: &EKEvent, identity: Identity) -> SourceEvent {
    // SAFETY: getters on a live event.
    unsafe {
        SourceEvent {
            provider_event_id: identity.provider_event_id,
            ical_uid: identity.ical_uid,
            series_key: identity.series_key,
            title: event.title().to_string(),
            notes: event.notes().as_deref().map(NSString::to_string),
            location: event.location().as_deref().map(NSString::to_string),
            url: event
                .URL()
                .and_then(|url| url.absoluteString())
                .as_deref()
                .map(NSString::to_string),
            conference_url: None,
            start: identity.start,
            end: identity.end,
            all_day: event.isAllDay(),
            status: convert::status(event.status().0),
            kind: None,
            availability: convert::availability(event.availability().0),
            organizer: event.organizer().as_deref().map(participant),
            attendees: event
                .attendees()
                .map(|attendees| attendees.to_vec().iter().map(|p| participant(p)).collect())
                .unwrap_or_default(),
        }
    }
}

/// One participant.
fn participant(participant: &EKParticipant) -> Participant {
    // SAFETY: getters on a live participant.
    unsafe {
        let url = participant
            .URL()
            .absoluteString()
            .map(|url| url.to_string());
        Participant {
            email: url
                .as_deref()
                .and_then(convert::mailto_address)
                .map(str::to_owned),
            name: participant.name().as_deref().map(NSString::to_string),
            rsvp: convert::rsvp(participant.participantStatus().0),
            kind: convert::participant_kind(participant.participantType().0),
            is_self: participant.isCurrentUser(),
        }
    }
}
