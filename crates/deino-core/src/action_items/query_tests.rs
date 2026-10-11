//! Filters resolved into query parameters: scopes, statuses and the day
//! windows across week boundaries and time zones.

use chrono::{NaiveDate, TimeZone, Utc};
use deino_domain::{ActionItemStatus, FixedZone, PersonId};

use super::*;

fn day(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

fn me() -> PersonId {
    "0199c6a0-0000-7000-8000-000000000001".parse().unwrap()
}

fn context<'a>(zone: &'a FixedZone, today: &str, me: Option<PersonId>) -> QueryContext<'a> {
    QueryContext {
        me,
        today: day(today),
        zone,
    }
}

fn resolve(filter: &ActionItemFilter, zone: FixedZone, today: &str) -> ItemQuery {
    ItemQuery::resolve(filter, context(&zone, today, Some(me()))).unwrap()
}

#[test]
fn the_default_view_reads_my_open_items_through_the_person_index() {
    let query = resolve(&ActionItemFilter::default(), FixedZone::utc(), "2026-10-09");

    assert_eq!(query.person, Some(me()));
    assert_eq!(query.statuses, [ActionItemStatus::Open; 2]);
    assert_eq!(query.access_path(), AccessPath::Person);
}

#[test]
fn my_items_match_nothing_until_there_is_a_me_person() {
    let zone = FixedZone::utc();

    assert_eq!(
        ItemQuery::resolve(
            &ActionItemFilter::default(),
            context(&zone, "2026-10-09", None)
        ),
        None
    );
}

#[test]
fn others_are_assigned_items_that_are_not_mine() {
    let filter = ActionItemFilter {
        assignee: AssigneeScope::Others,
        status: StatusFilter::Any,
        ..ActionItemFilter::default()
    };

    let query = resolve(&filter, FixedZone::utc(), "2026-10-09");

    assert!(query.assigned_only);
    assert_eq!(query.not_person, Some(me()));
    assert_eq!(query.person, None);
    assert_eq!(
        query.statuses,
        [ActionItemStatus::Open, ActionItemStatus::Done]
    );
    assert_eq!(query.access_path(), AccessPath::Status);
}

#[test]
fn a_meeting_filter_reads_through_the_meeting_index() {
    let filter = ActionItemFilter::meeting(deino_domain::MeetingId::new("meeting-a"));

    let query = resolve(&filter, FixedZone::utc(), "2026-10-09");

    assert_eq!(query.access_path(), AccessPath::Meeting);
    assert_eq!(query.person, None);
}

#[test]
fn overdue_means_open_and_due_before_local_midnight() {
    let pacific = FixedZone::hours(-7).unwrap();
    let filter = ActionItemFilter {
        status: StatusFilter::Any,
        due: Some(DueFilter::Overdue),
        ..ActionItemFilter::default()
    };

    let query = resolve(&filter, pacific, "2026-10-09");

    assert_eq!(query.statuses, [ActionItemStatus::Open; 2]);
    assert_eq!(query.due_from, None);
    assert_eq!(
        query.due_before,
        Some(Utc.with_ymd_and_hms(2026, 10, 9, 7, 0, 0).unwrap())
    );
}

#[test]
fn today_is_the_local_day_not_the_utc_one() {
    let tokyo = FixedZone::hours(9).unwrap();
    let filter = ActionItemFilter {
        due: Some(DueFilter::Today),
        ..ActionItemFilter::default()
    };

    let query = resolve(&filter, tokyo, "2026-10-09");

    assert_eq!(
        (query.due_from, query.due_before),
        (
            Some(Utc.with_ymd_and_hms(2026, 10, 8, 15, 0, 0).unwrap()),
            Some(Utc.with_ymd_and_hms(2026, 10, 9, 15, 0, 0).unwrap())
        )
    );
}

#[test]
fn this_week_runs_from_today_to_sunday_night() {
    let filter = ActionItemFilter {
        due: Some(DueFilter::ThisWeek),
        ..ActionItemFilter::default()
    };
    let zone = FixedZone::utc();

    let friday = resolve(&filter, zone, "2026-10-09");
    let sunday = resolve(&filter, zone, "2026-10-11");
    let monday = resolve(&filter, zone, "2026-10-12");

    let monday_after = Some(Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap());
    assert_eq!(friday.due_before, monday_after);
    assert_eq!(sunday.due_before, monday_after);
    assert_eq!(
        monday.due_before,
        Some(Utc.with_ymd_and_hms(2026, 10, 19, 0, 0, 0).unwrap())
    );
}

#[test]
fn a_date_range_covers_both_end_days_in_local_time() {
    let pacific = FixedZone::hours(-7).unwrap();
    let filter = ActionItemFilter {
        date_range: Some(DateRange {
            from: Some(day("2026-09-28")),
            to: Some(day("2026-10-02")),
        }),
        ..ActionItemFilter::default()
    };

    let query = resolve(&filter, pacific, "2026-10-09");

    assert_eq!(
        (query.held_from, query.held_before),
        (
            Some(Utc.with_ymd_and_hms(2026, 9, 28, 7, 0, 0).unwrap()),
            Some(Utc.with_ymd_and_hms(2026, 10, 3, 7, 0, 0).unwrap())
        )
    );
}

#[test]
fn next_monday_crosses_month_and_year_ends() {
    assert_eq!(next_monday(day("2026-10-31")), day("2026-11-02"));
    assert_eq!(next_monday(day("2026-12-28")), day("2027-01-04"));
    assert_eq!(next_monday(day("2027-01-03")), day("2027-01-04"));
}
