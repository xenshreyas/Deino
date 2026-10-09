//! `EventKit`'s raw values as Deino's: status codes, availability, roles,
//! instants and occurrence IDs.
//!
//! Pure and platform-independent (`EventKit`'s enums are plain integers), so
//! every rule here is unit-tested on any machine; `read.rs` only fetches the
//! values from the objects.

use chrono::{DateTime, Utc};
use deino_core::calendar::{Availability, CalendarAccess, EventStatus, ParticipantKind};
use deino_domain::Rsvp;

/// `EKAuthorizationStatus` as Deino's access state.
pub(crate) fn access(status: isize) -> CalendarAccess {
    match status {
        0 => CalendarAccess::NotDetermined,
        1 => CalendarAccess::Restricted,
        3 => CalendarAccess::Granted,
        4 => CalendarAccess::WriteOnly,
        // 2 (denied), and anything a later macOS adds, reads nothing.
        _ => CalendarAccess::Denied,
    }
}

/// `EKEventStatus`: whether the event takes place.
pub(crate) fn status(status: isize) -> EventStatus {
    match status {
        2 => EventStatus::Tentative,
        3 => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    }
}

/// `EKEventAvailability`: how the event marks the user's time.
pub(crate) fn availability(availability: isize) -> Availability {
    match availability {
        0 => Availability::Busy,
        1 => Availability::Free,
        2 => Availability::Tentative,
        3 => Availability::Unavailable,
        _ => Availability::Unknown,
    }
}

/// `EKParticipantStatus` as an RSVP; `None` when unknown or not an answer.
pub(crate) fn rsvp(status: isize) -> Option<Rsvp> {
    match status {
        1 => Some(Rsvp::NeedsAction),
        2 => Some(Rsvp::Accepted),
        // Delegated: someone else goes in their place.
        3 | 5 => Some(Rsvp::Declined),
        4 => Some(Rsvp::Tentative),
        _ => None,
    }
}

/// `EKParticipantType`: a person unless `EventKit` says otherwise.
pub(crate) fn participant_kind(kind: isize) -> ParticipantKind {
    match kind {
        2 => ParticipantKind::Room,
        3 => ParticipantKind::Resource,
        4 => ParticipantKind::Group,
        _ => ParticipantKind::Person,
    }
}

/// `EKCalendarType` 4: the contacts' birthdays, which are never meetings.
pub(crate) fn is_birthday_calendar(kind: isize) -> bool {
    kind == 4
}

/// An `NSDate`'s seconds since 1970 as an instant; `None` for a value no
/// calendar holds (not finite, or out of range).
pub(crate) fn instant(seconds: f64) -> Option<DateTime<Utc>> {
    if !seconds.is_finite() {
        return None;
    }
    let millis = (seconds * 1000.0).round();
    // i64 milliseconds cover ±292 million years; anything beyond is garbage.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "range-checked against i64 just below"
    )]
    let millis = (millis.abs() < 9.0e15).then_some(millis as i64)?;
    DateTime::from_timestamp_millis(millis)
}

/// An instant as `NSDate` seconds since 1970.
#[allow(
    clippy::cast_precision_loss,
    reason = "calendar instants are millisecond-exact far inside f64's 2^53"
)]
pub(crate) fn seconds(instant: DateTime<Utc>) -> f64 {
    instant.timestamp_millis() as f64 / 1000.0
}

/// The ID of one occurrence. `EventKit` gives every occurrence of a recurring
/// event the same `eventIdentifier`, so an occurrence adds its original
/// start (`occurrenceDate`), which stays put when that one occurrence is
/// moved.
pub(crate) fn occurrence_id(event_id: &str, occurrence: Option<DateTime<Utc>>) -> String {
    match occurrence {
        Some(start) => format!("{event_id}/{}", start.timestamp()),
        None => event_id.to_owned(),
    }
}

/// The address in a participant's `mailto:` URL, if it has one.
pub(crate) fn mailto_address(url: &str) -> Option<&str> {
    let scheme = url.get(..7)?;
    scheme
        .eq_ignore_ascii_case("mailto:")
        .then(|| url.get(7..))
        .flatten()
        .map(|address| address.split('?').next().unwrap_or(address))
        .filter(|address| address.contains('@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_codes_map_to_access() {
        assert_eq!(access(0), CalendarAccess::NotDetermined);
        assert_eq!(access(1), CalendarAccess::Restricted);
        assert_eq!(access(2), CalendarAccess::Denied);
        assert_eq!(access(3), CalendarAccess::Granted);
        assert_eq!(access(4), CalendarAccess::WriteOnly);
        assert_eq!(access(9), CalendarAccess::Denied);
    }

    #[test]
    fn participant_answers_map_to_rsvps() {
        assert_eq!(rsvp(0), None);
        assert_eq!(rsvp(1), Some(Rsvp::NeedsAction));
        assert_eq!(rsvp(2), Some(Rsvp::Accepted));
        assert_eq!(rsvp(3), Some(Rsvp::Declined));
        assert_eq!(rsvp(4), Some(Rsvp::Tentative));
        assert_eq!(rsvp(5), Some(Rsvp::Declined));
        assert_eq!(rsvp(6), None);
    }

    #[test]
    fn statuses_availability_and_kinds_map() {
        assert_eq!(status(3), EventStatus::Cancelled);
        assert_eq!(status(0), EventStatus::Confirmed);
        assert_eq!(availability(3), Availability::Unavailable);
        assert_eq!(availability(-1), Availability::Unknown);
        assert_eq!(participant_kind(2), ParticipantKind::Room);
        assert_eq!(participant_kind(0), ParticipantKind::Person);
        assert!(is_birthday_calendar(4));
        assert!(!is_birthday_calendar(1));
    }

    #[test]
    fn instants_round_trip_through_nsdate_seconds() {
        let start = DateTime::parse_from_rfc3339("2026-10-09T09:30:00.250Z")
            .unwrap()
            .to_utc();
        assert_eq!(instant(seconds(start)), Some(start));
        assert_eq!(instant(f64::NAN), None);
        assert_eq!(instant(1.0e300), None);
    }

    #[test]
    fn occurrences_of_a_recurring_event_get_their_own_ids() {
        let start = instant(1_791_451_800.0);
        assert_eq!(occurrence_id("ABC:123", start), "ABC:123/1791451800");
        assert_eq!(occurrence_id("ABC:123", None), "ABC:123");
    }

    #[test]
    fn mailto_urls_yield_their_address() {
        assert_eq!(mailto_address("mailto:ada@acme.com"), Some("ada@acme.com"));
        assert_eq!(
            mailto_address("MAILTO:ada@acme.com?subject=x"),
            Some("ada@acme.com")
        );
        assert_eq!(mailto_address("urn:uuid:1234"), None);
        assert_eq!(mailto_address("mailto:room-4"), None);
    }
}
