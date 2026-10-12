use deino_domain::{ActionItemOrigin, ActionItemStatus, FakeClock, FixedZone, MeetingId};
use deino_testkit::at;

use super::*;

const NOW: &str = "2026-10-12T13:30:00Z";

fn pacific() -> FixedZone {
    FixedZone::hours(-7).unwrap()
}

fn digest(summary: Option<&str>, notes: Option<&str>) -> EntityDigest {
    EntityDigest {
        meeting_id: MeetingId::new("meeting-1"),
        title: " Kickoff ".into(),
        at: at("2026-10-01T16:00:00Z"),
        summary: summary.map(String::from),
        notes: notes.map(String::from),
    }
}

#[test]
fn a_meeting_reads_its_summary_else_the_notes_with_an_absolute_date() {
    let zone = pacific();

    let summarised = meeting_source(&digest(Some("Agreed."), Some("raw")), false, &zone);
    let noted = meeting_source(&digest(Some(" "), Some("raw notes")), true, &zone);

    assert_eq!(
        summarised.heading,
        "Meeting: Kickoff (Thursday 1 October 2026)"
    );
    assert_eq!(summarised.body, "Agreed.");
    assert_eq!(
        noted.heading,
        "Meeting: Kickoff (Thursday 1 October 2026, the last meeting of this series)"
    );
    assert_eq!(noted.body, "raw notes");
}

fn item(meeting: Option<&str>, segment: bool) -> ActionItem {
    let clock = FakeClock::new(at(NOW));
    ActionItem {
        id: deino_domain::ActionItemId::generate(&clock),
        meeting_id: meeting.map(MeetingId::new),
        title: " Send the security questionnaire ".into(),
        description: String::new(),
        assignee_person_id: None,
        due_at: Some(at("2026-10-16T17:00:00Z")),
        status: ActionItemStatus::Open,
        priority: None,
        source_segment_id: segment.then(|| deino_domain::SegmentId::generate(&clock)),
        completed_at: None,
        created_by: ActionItemOrigin::Ai,
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

#[test]
fn an_item_cites_its_moment_its_meeting_or_itself() {
    let names = BTreeMap::new();

    let moment = item_source(&item(Some("meeting-1"), true), &names);
    let meeting = item_source(&item(Some("meeting-1"), false), &names);
    let alone = item_source(&item(None, false), &names);

    assert_eq!(
        moment.heading,
        "Open action item for someone, due 16 October 2026: Send the security questionnaire"
    );
    assert!(matches!(moment.citation, BriefCitation::Moment { .. }));
    assert!(matches!(meeting.citation, BriefCitation::Meeting { .. }));
    assert!(matches!(alone.citation, BriefCitation::ActionItem { .. }));
}

#[test]
fn long_texts_are_clipped_and_an_empty_invitation_is_no_source() {
    assert_eq!(clip("  short  ", 10), "short");
    assert_eq!(clip("abcdef ghij", 7), "abcdef…");
    assert_eq!(clip("", 3), "");
}
