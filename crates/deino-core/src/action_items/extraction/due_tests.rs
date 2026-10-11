//! Due phrases across week, month and year boundaries and time zones. The
//! fixture meetings' own phrases are the first group (their ground truth).

use chrono::{NaiveDate, TimeZone, Utc};
use deino_domain::{FixedZone, LocalZone};

use super::*;

fn day(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

fn due(phrase: &str, meeting_day: &str) -> Option<NaiveDate> {
    resolve_due(Some(phrase), None, day(meeting_day))
}

#[test]
fn the_fixture_meetings_phrases_resolve_to_their_ground_truth() {
    // (phrase, meeting day, ground-truth due day)
    let cases = [
        ("by Thursday", "2026-09-28", "2026-10-01"),
        ("by Friday", "2026-09-28", "2026-10-02"),
        ("by the ninth of October", "2026-09-28", "2026-10-09"),
        ("by the twenty-third of October", "2026-09-28", "2026-10-23"),
        ("tonight", "2026-09-28", "2026-09-28"),
        ("by Friday the ninth of October", "2026-09-29", "2026-10-09"),
        ("by the end of October", "2026-09-29", "2026-10-31"),
        ("by Thursday next week", "2026-10-01", "2026-10-08"),
        ("by Monday", "2026-10-01", "2026-10-05"),
        ("Tuesday the thirteenth", "2026-10-01", "2026-10-13"),
        ("by Wednesday, October seventh", "2026-10-02", "2026-10-07"),
        ("by end of day today", "2026-10-02", "2026-10-02"),
        ("this week", "2026-10-05", "2026-10-09"),
        ("by Wednesday", "2026-10-05", "2026-10-07"),
        ("this afternoon", "2026-10-07", "2026-10-07"),
        ("tomorrow", "2026-10-07", "2026-10-08"),
        ("by next Friday", "2026-10-07", "2026-10-16"),
        ("by Monday", "2026-10-07", "2026-10-12"),
        ("by end of day tomorrow", "2026-10-08", "2026-10-09"),
        ("on Monday", "2026-10-08", "2026-10-12"),
        ("at four o'clock today", "2026-10-08", "2026-10-08"),
    ];
    for (phrase, meeting, expected) in cases {
        assert_eq!(
            due(phrase, meeting),
            Some(day(expected)),
            "{phrase:?} on {meeting}"
        );
    }
}

#[test]
fn a_bare_weekday_is_never_the_meeting_day_itself() {
    // Friday 9 October: "by Friday" is next week's, "this Friday" is today.
    assert_eq!(due("by Friday", "2026-10-09"), Some(day("2026-10-16")));
    assert_eq!(due("this Friday", "2026-10-09"), Some(day("2026-10-09")));
}

#[test]
fn on_a_sunday_the_following_week_starts_tomorrow() {
    // Sunday 11 October ends its week: "next Monday" and a bare "Monday"
    // are both tomorrow, and "next Friday" is that week's.
    assert_eq!(due("next Monday", "2026-10-11"), Some(day("2026-10-12")));
    assert_eq!(due("next Friday", "2026-10-11"), Some(day("2026-10-16")));
    assert_eq!(due("Monday", "2026-10-11"), Some(day("2026-10-12")));
    assert_eq!(due("next week", "2026-10-11"), Some(day("2026-10-16")));
}

#[test]
fn the_week_ends_on_its_friday_or_on_the_weekend_day_itself() {
    assert_eq!(
        due("by the end of the week", "2026-10-06"),
        Some(day("2026-10-09"))
    );
    assert_eq!(due("this week", "2026-10-10"), Some(day("2026-10-10")));
}

#[test]
fn dates_roll_over_month_and_year_ends() {
    assert_eq!(due("by Friday", "2026-12-30"), Some(day("2027-01-01")));
    assert_eq!(due("the second", "2026-10-30"), Some(day("2026-11-02")));
    assert_eq!(due("January 4th", "2026-12-20"), Some(day("2027-01-04")));
    assert_eq!(
        due("end of the month", "2026-02-10"),
        Some(day("2026-02-28"))
    );
    assert_eq!(
        due("end of next month", "2026-12-10"),
        Some(day("2027-01-31"))
    );
    assert_eq!(due("in two weeks", "2026-12-28"), Some(day("2027-01-11")));
}

#[test]
fn an_explicit_year_is_kept_when_plausible() {
    assert_eq!(
        due("by 3 March 2027", "2026-10-09"),
        Some(day("2027-03-03"))
    );
    assert_eq!(due("by 3 March 2025", "2026-10-09"), None);
}

#[test]
fn the_models_date_is_the_fallback_for_unrecognised_wording() {
    let meeting = day("2026-10-08");

    assert_eq!(
        resolve_due(Some("before the board call"), Some("2026-10-12"), meeting),
        Some(day("2026-10-12"))
    );
    assert_eq!(
        resolve_due(None, Some("2026-10-12"), meeting),
        Some(day("2026-10-12"))
    );
    assert_eq!(resolve_due(None, Some("next sprint"), meeting), None);
    assert_eq!(resolve_due(None, Some("2026-09-01"), meeting), None);
    assert_eq!(resolve_due(None, Some("2031-01-01"), meeting), None);
    assert_eq!(resolve_due(None, None, meeting), None);
}

#[test]
fn the_phrase_wins_over_a_model_date_that_slipped_a_week() {
    // A Wednesday meeting: "next Friday" is the 16th, not the 9th.
    assert_eq!(
        resolve_due(
            Some("by next Friday"),
            Some("2026-10-09"),
            day("2026-10-07")
        ),
        Some(day("2026-10-16"))
    );
}

#[test]
fn ordinary_words_are_not_dates() {
    for phrase in [
        "I may look at it",
        "after the march",
        "once Maya is back",
        "soon",
    ] {
        assert_eq!(parse_phrase(phrase, day("2026-10-08")), None, "{phrase:?}");
    }
}

#[test]
fn tomorrow_counts_from_the_local_meeting_day_not_the_utc_one() {
    // 22:30 on Sunday 11 October in California is already Monday in UTC.
    let started = Utc.with_ymd_and_hms(2026, 10, 12, 5, 30, 0).unwrap();
    let pacific = FixedZone::hours(-7).unwrap();

    assert_eq!(
        due("tomorrow", &pacific.day_of(started).to_string()),
        Some(day("2026-10-12"))
    );
    assert_eq!(
        due("tomorrow", &FixedZone::utc().day_of(started).to_string()),
        Some(day("2026-10-13"))
    );
}
