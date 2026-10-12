//! The rule matrix: table tests over audience, organiser, RSVP, size and
//! the user's choices.

use deino_domain::{Email, Rsvp};

use super::policy::{
    Audience, Choices, EventFacts, Organizer, RecordDecision, RecordReason, StartTrigger, decide,
    starts_recording,
};
use crate::calendar::test_events::{ME, stored};
use crate::profile::InternalDomains;
use crate::settings::AutoRecordSettings;

use Audience::{External, Internal};
use Organizer::{Me, Someone};
use RecordReason::{
    AudienceOff, Declined, EventChoice, NotAMeeting, NotAccepted, OrganizedByOthers, Rules,
    SeriesChoice,
};

const ALL_ON: AutoRecordSettings = AutoRecordSettings {
    internal: true,
    external: true,
    organized_by_others: true,
    not_accepted: true,
};
const OFF: AutoRecordSettings = AutoRecordSettings {
    internal: false,
    external: false,
    organized_by_others: false,
    not_accepted: false,
};
const EXTERNAL_ONLY: AutoRecordSettings = AutoRecordSettings {
    external: true,
    organized_by_others: true,
    ..OFF
};
const MINE_ONLY: AutoRecordSettings = AutoRecordSettings {
    internal: true,
    external: true,
    ..OFF
};
const NONE: Choices = Choices {
    event: None,
    series: None,
};

fn facts(audience: Audience, organizer: Organizer, rsvp: Option<Rsvp>) -> EventFacts {
    EventFacts {
        audience,
        organizer,
        rsvp,
        attendees: 3,
        all_day: false,
    }
}

const fn yes(reason: RecordReason) -> RecordDecision {
    RecordDecision {
        record: true,
        reason,
    }
}

const fn no(reason: RecordReason) -> RecordDecision {
    RecordDecision {
        record: false,
        reason,
    }
}

fn assert_cases(cases: &[(AutoRecordSettings, EventFacts, RecordDecision)]) {
    for (index, (rules, facts, expected)) in cases.iter().enumerate() {
        assert_eq!(
            decide(rules, facts, NONE),
            *expected,
            "case {index}: {facts:?}"
        );
    }
}

#[test]
fn the_audience_switches_decide_first() {
    let accepted = Some(Rsvp::Accepted);
    let tentative = Some(Rsvp::Tentative);
    let defaults = AutoRecordSettings::default();
    assert_cases(&[
        (OFF, facts(Internal, Me, accepted), no(AudienceOff)),
        (OFF, facts(External, Me, accepted), no(AudienceOff)),
        (ALL_ON, facts(Internal, Someone, None), yes(Rules)),
        (ALL_ON, facts(External, Someone, tentative), yes(Rules)),
        (
            EXTERNAL_ONLY,
            facts(Internal, Me, accepted),
            no(AudienceOff),
        ),
        (defaults, facts(Internal, Me, accepted), no(AudienceOff)),
        (
            defaults,
            facts(External, Someone, accepted),
            no(AudienceOff),
        ),
    ]);
}

#[test]
fn then_the_organiser_and_the_rsvp() {
    let accepted = Some(Rsvp::Accepted);
    let tentative = Some(Rsvp::Tentative);
    assert_cases(&[
        (
            EXTERNAL_ONLY,
            facts(External, Someone, accepted),
            yes(Rules),
        ),
        (
            EXTERNAL_ONLY,
            facts(External, Someone, tentative),
            no(NotAccepted),
        ),
        (EXTERNAL_ONLY, facts(External, Me, None), yes(Rules)),
        (MINE_ONLY, facts(Internal, Me, accepted), yes(Rules)),
        (
            MINE_ONLY,
            facts(Internal, Someone, accepted),
            no(OrganizedByOthers),
        ),
        (
            MINE_ONLY,
            facts(External, Someone, None),
            no(OrganizedByOthers),
        ),
    ]);
}

#[test]
fn declined_and_non_meetings_never_record_on_their_own() {
    let declined = facts(External, Someone, Some(Rsvp::Declined));
    let alone = EventFacts {
        attendees: 1,
        ..facts(Internal, Me, None)
    };
    let all_day = EventFacts {
        all_day: true,
        ..facts(Internal, Me, None)
    };
    assert_eq!(decide(&ALL_ON, &declined, NONE), no(Declined));
    assert_eq!(decide(&ALL_ON, &alone, NONE), no(NotAMeeting));
    assert_eq!(decide(&ALL_ON, &all_day, NONE), no(NotAMeeting));
}

#[test]
fn choices_win_this_one_over_the_series_over_the_switches() {
    let declined = facts(External, Someone, Some(Rsvp::Declined));
    let event_yes = Choices {
        event: Some(true),
        series: Some(false),
    };
    let series_no = Choices {
        event: None,
        series: Some(false),
    };
    let series_yes = Choices {
        event: None,
        series: Some(true),
    };
    let event_no = Choices {
        event: Some(false),
        series: Some(true),
    };
    let cases = [
        (OFF, declined, event_yes, yes(EventChoice)),
        (
            ALL_ON,
            facts(Internal, Me, None),
            series_no,
            no(SeriesChoice),
        ),
        (
            OFF,
            facts(Internal, Someone, None),
            series_yes,
            yes(SeriesChoice),
        ),
        (ALL_ON, facts(External, Me, None), event_no, no(EventChoice)),
    ];
    for (index, (rules, facts, choices, expected)) in cases.into_iter().enumerate() {
        assert_eq!(decide(&rules, &facts, choices), expected, "case {index}");
    }
}

#[test]
fn the_start_time_needs_the_note_open_and_the_mic_does_not() {
    let record = yes(Rules);
    let skip = no(AudienceOff);
    let open = StartTrigger::StartTime { note_open: true };
    let closed = StartTrigger::StartTime { note_open: false };
    assert!(starts_recording(record, StartTrigger::MicInUse));
    assert!(starts_recording(record, open));
    assert!(!starts_recording(record, closed));
    assert!(!starts_recording(skip, StartTrigger::MicInUse));
    assert!(!starts_recording(skip, open));
}

#[test]
fn facts_read_the_organiser_from_the_users_addresses() {
    let own = InternalDomains::from_emails(&[Email::parse(ME).unwrap()]);
    let mut event = stored("Sync", 1);
    assert_eq!(
        EventFacts::of(&event, &own).organizer,
        Me,
        "the user is flagged"
    );

    event.organizer_email = Some("ada@example.com".into());
    event.attendees[0].email = Some("ada@example.com".into());
    assert_eq!(EventFacts::of(&event, &own).organizer, Someone);

    event.organizer_email = Some("Maya@Deino.dev".into());
    assert_eq!(
        EventFacts::of(&event, &own).organizer,
        Me,
        "addresses ignore case"
    );

    event.organizer_email = None;
    assert_eq!(
        EventFacts::of(&event, &own).organizer,
        Someone,
        "flagged attendee"
    );
    event.attendees[0].is_organizer = false;
    assert_eq!(
        EventFacts::of(&event, &own).organizer,
        Me,
        "no organiser named"
    );
}
