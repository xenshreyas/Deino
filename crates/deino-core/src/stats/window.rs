//! The instants a period starts at, in the user's time zone.
//!
//! Periods are whole local days ending today, so "last 30 days" at 09:00
//! includes everything since midnight 29 days ago, on any daylight-saving
//! schedule ([`LocalZone::start_of`]).

use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Utc};
use deino_domain::LocalZone;

use super::StatsRange;

/// Days the heatmap goes back before today's week: 52 full weeks.
const HEATMAP_DAYS: u64 = 364;

/// Where `range` starts when it is `now`; `None` for all time.
pub fn since(range: StatsRange, now: DateTime<Utc>, zone: &dyn LocalZone) -> Option<DateTime<Utc>> {
    let today = zone.day_of(now);
    let first = match range {
        StatsRange::Last30Days => today.checked_sub_days(Days::new(29)),
        StatsRange::Last90Days => today.checked_sub_days(Days::new(89)),
        StatsRange::Last12Months => today
            .checked_sub_months(Months::new(12))
            .and_then(|day| day.checked_add_days(Days::new(1))),
        StatsRange::AllTime => return None,
    };
    Some(zone.start_of(first.unwrap_or(NaiveDate::MIN)))
}

/// The heatmap's first and last day: from the Monday 52 weeks before this
/// week's Monday, to today.
pub fn heatmap_days(now: DateTime<Utc>, zone: &dyn LocalZone) -> (NaiveDate, NaiveDate) {
    let today = zone.day_of(now);
    let back = HEATMAP_DAYS + u64::from(today.weekday().num_days_from_monday());
    let first = today.checked_sub_days(Days::new(back)).unwrap_or(today);
    (first, today)
}

/// The local calendar year `year`, as `[start, end)`.
pub fn year_bounds(year: i32, zone: &dyn LocalZone) -> (DateTime<Utc>, DateTime<Utc>) {
    let first = NaiveDate::from_yo_opt(year, 1).unwrap_or(NaiveDate::MIN);
    let next = NaiveDate::from_yo_opt(year.saturating_add(1), 1).unwrap_or(NaiveDate::MAX);
    (zone.start_of(first), zone.start_of(next))
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, Weekday};
    use deino_domain::FixedZone;
    use deino_testkit::at;

    use super::*;

    fn kolkata() -> FixedZone {
        FixedZone(FixedOffset::east_opt(5 * 3600 + 1800).unwrap())
    }

    #[test]
    fn periods_start_at_local_midnight() {
        // 2026-10-11 02:00 in Kolkata is still 2026-10-10 in UTC.
        let now = at("2026-10-10T20:30:00Z");
        let zone = kolkata();

        assert_eq!(
            since(StatsRange::Last30Days, now, &zone),
            Some(at("2026-09-11T18:30:00Z"))
        );
        assert_eq!(
            since(StatsRange::Last90Days, now, &zone),
            Some(at("2026-07-13T18:30:00Z"))
        );
        assert_eq!(
            since(StatsRange::Last12Months, now, &zone),
            Some(at("2025-10-11T18:30:00Z"))
        );
        assert_eq!(since(StatsRange::AllTime, now, &zone), None);
    }

    #[test]
    fn the_heatmap_starts_on_a_monday_and_ends_today() {
        let (first, last) = heatmap_days(at("2026-10-11T12:00:00Z"), &FixedZone::utc());

        assert_eq!(first.weekday(), Weekday::Mon);
        assert_eq!(last, NaiveDate::from_ymd_opt(2026, 10, 11).unwrap());
        assert_eq!((last - first).num_days(), 370);
    }

    #[test]
    fn a_year_is_bounded_by_local_new_years() {
        let (start, end) = year_bounds(2026, &kolkata());

        assert_eq!(start, at("2025-12-31T18:30:00Z"));
        assert_eq!(end, at("2026-12-31T18:30:00Z"));
    }
}
