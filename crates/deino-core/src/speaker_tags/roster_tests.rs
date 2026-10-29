use deino_domain::{CallApp, Clock, FakeClock, SegmentId};

use super::*;

fn listed(name: &str, (start, end): (u32, u32)) -> TagSpan {
    TagSpan {
        start: Millis(start),
        end: Millis(end),
        name: DisplayName::parse(name).unwrap(),
        app: CallApp::Zoom,
    }
}

fn segment(clock: &FakeClock, (start, end): (u32, u32)) -> Timed {
    Timed {
        id: SegmentId::generate(clock as &dyn Clock),
        start: Millis(start),
        end: Millis(end),
    }
}

fn names(wins: &[Win]) -> Vec<&str> {
    wins.iter().map(|win| win.name.as_str()).collect()
}

#[test]
fn the_user_is_dropped_by_name_or_a_longer_name_starting_with_it() {
    let roster = vec![
        listed("Mehrdad Oveisi", (0, 10_000)),
        listed("Shreyas Goyal", (0, 10_000)),
        listed("Shreyash Rao", (0, 10_000)),
    ];

    let kept = without_me(roster.clone(), Some("Shreyas"));
    let kept: Vec<&str> = kept.iter().map(|span| span.name.as_str()).collect();

    assert_eq!(kept, ["Mehrdad Oveisi", "Shreyash Rao"]);
    assert_eq!(
        without_me(roster, None).len(),
        3,
        "without a profile name nothing goes"
    );
}

#[test]
fn a_segment_is_won_only_where_one_remote_person_was_listed() {
    let clock = FakeClock::new(chrono::DateTime::UNIX_EPOCH);
    let roster = [
        listed("Mehrdad", (0, 40_000)),
        listed("Mehrdad", (60_000, 120_000)),
        listed("Jordon", (90_000, 120_000)),
    ];
    let segments = [
        segment(&clock, (5_000, 9_000)),
        segment(&clock, (45_000, 50_000)),
        segment(&clock, (62_000, 70_000)),
        segment(&clock, (95_000, 99_000)),
    ];

    let wins = listed_alone(&roster, &segments);

    assert_eq!(
        names(&wins),
        ["Mehrdad", "Mehrdad"],
        "unwatched and shared moments win nothing"
    );
    assert_eq!(wins[1].segment, segments[2].id);
}

#[test]
fn the_one_to_one_rule_needs_the_roster_to_have_watched_most_of_the_call() {
    let clock = FakeClock::new(chrono::DateTime::UNIX_EPOCH);
    let segments = [
        segment(&clock, (0, 40_000)),
        segment(&clock, (40_000, 90_000)),
        segment(&clock, (90_000, 100_000)),
    ];

    assert!(covers_call(&[listed("Mehrdad", (0, 95_000))], &segments));
    assert!(!covers_call(&[listed("Mehrdad", (0, 39_000))], &segments));
    assert!(!covers_call(&[listed("Mehrdad", (0, 39_000))], &[]));
}
