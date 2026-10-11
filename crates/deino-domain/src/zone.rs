//! The `LocalZone` port: the user's time zone, beside [`Clock`](crate::Clock).
//!
//! Everything Deino stores is UTC; days ("due today", "overdue", 09:00
//! reminders, "by Friday") are the user's local days. Services take an
//! `Arc<dyn LocalZone>` so tests pin a zone instead of reading the Mac's.
//! [`SystemZone`] asks the operating system for the offset at a given
//! instant, which keeps daylight-saving changes right for past and future
//! dates; [`FixedZone`] is one offset, for tests and the mock.

use std::fmt::Debug;

use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveTime, Offset, TimeZone, Utc};

/// The user's time zone.
pub trait LocalZone: Debug + Send + Sync {
    /// The zone's offset from UTC at `at`.
    fn offset_at(&self, at: DateTime<Utc>) -> FixedOffset;

    /// `at` as the user's local time.
    fn local(&self, at: DateTime<Utc>) -> DateTime<FixedOffset> {
        at.with_timezone(&self.offset_at(at))
    }

    /// The user's calendar day at `at`.
    fn day_of(&self, at: DateTime<Utc>) -> NaiveDate {
        self.local(at).date_naive()
    }

    /// The instant the local clock reads `time` on `day`. Uses the offset
    /// in force at that day's noon, so a daylight-saving change in the early
    /// hours cannot move it onto another day.
    fn at(&self, day: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
        let noon_utc = day.and_time(NOON).and_utc();
        let offset = self.offset_at(noon_utc);
        (day.and_time(time) - offset).and_utc()
    }

    /// The first instant of `day`, local time.
    fn start_of(&self, day: NaiveDate) -> DateTime<Utc> {
        self.at(day, NaiveTime::MIN)
    }
}

/// Midday, the anchor for day arithmetic and for stored due dates.
pub const NOON: NaiveTime = match NaiveTime::from_hms_opt(12, 0, 0) {
    Some(noon) => noon,
    None => NaiveTime::MIN,
};

/// The Mac's time zone, as `chrono::Local` reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemZone;

impl LocalZone for SystemZone {
    fn offset_at(&self, at: DateTime<Utc>) -> FixedOffset {
        Local.offset_from_utc_datetime(&at.naive_utc()).fix()
    }
}

/// One fixed offset all year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedZone(pub FixedOffset);

impl FixedZone {
    /// UTC.
    pub fn utc() -> Self {
        Self(Utc.fix())
    }

    /// A zone `hours` east of UTC (negative for the Americas), if valid.
    pub fn hours(hours: i32) -> Option<Self> {
        FixedOffset::east_opt(hours * 3_600).map(Self)
    }
}

impl LocalZone for FixedZone {
    fn offset_at(&self, _at: DateTime<Utc>) -> FixedOffset {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;

    fn pacific() -> FixedZone {
        FixedZone::hours(-7).unwrap()
    }

    #[test]
    fn the_local_day_differs_from_the_utc_day_late_in_the_evening() {
        let late_sunday = Utc.with_ymd_and_hms(2026, 10, 12, 5, 30, 0).unwrap();

        assert_eq!(
            pacific().day_of(late_sunday),
            NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()
        );
        assert_eq!(
            FixedZone::utc().day_of(late_sunday),
            NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
        );
    }

    #[test]
    fn start_of_day_is_local_midnight_in_utc() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();

        assert_eq!(
            pacific().start_of(day),
            Utc.with_ymd_and_hms(2026, 10, 9, 7, 0, 0).unwrap()
        );
        assert_eq!(
            pacific().at(day, NOON),
            Utc.with_ymd_and_hms(2026, 10, 9, 19, 0, 0).unwrap()
        );
    }

    #[test]
    fn the_system_zone_round_trips_a_day() {
        let day = NaiveDate::from_ymd_opt(2026, 3, 29).unwrap();

        assert_eq!(SystemZone.day_of(SystemZone.at(day, NOON)), day);
    }
}
