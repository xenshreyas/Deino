//! The FND-04 fixture meetings as [`MeetingExport`]s, for golden tests.
//!
//! The fixtures have no AI notes, tags or folders, so this builds them the
//! way the product would show them: the notes are the ground truth's
//! decisions and facts, the action items its expected items, and each
//! meeting is tagged with its kind. Times are shown at UTC−07:00 so the
//! goldens prove renderers use the local time they are given.

use chrono::FixedOffset;
use deino_domain::Seconds;
use deino_testkit::fixture::{FixtureMeeting, Fixtures, Rsvp, USER_EMAIL};

use crate::{
    ActionItemExport, InsightAnswerExport, InsightExport, InsightExportValue, MeetingExport,
    Participant, Utterance,
};

/// Pacific daylight time, the zone the goldens are written in.
const ZONE_SECONDS: i32 = -7 * 3_600;

/// Every fixture meeting as an export.
pub(crate) fn fixture_exports() -> Vec<MeetingExport> {
    let fixtures = Fixtures::load().expect("the fixtures load");
    fixtures.meetings().iter().map(export_of).collect()
}

/// The fixture meeting whose ID ends in `name` (`"standup"`).
pub(crate) fn fixture_export(name: &str) -> MeetingExport {
    fixture_exports()
        .into_iter()
        .find(|export| export.id.as_str().ends_with(name))
        .expect("a fixture with that name")
}

fn export_of(meeting: &FixtureMeeting) -> MeetingExport {
    let zone = FixedOffset::east_opt(ZONE_SECONDS).expect("a valid offset");
    MeetingExport {
        id: meeting.id.clone(),
        title: meeting.title.clone(),
        started_at: meeting.started_at.with_timezone(&zone),
        duration: Some(Seconds(meeting.duration.get() / 1_000)),
        attendees: meeting
            .attendees
            .iter()
            .filter(|attendee| attendee.rsvp != Rsvp::Declined)
            .map(|attendee| Participant {
                name: attendee.name.clone(),
                email: Some(attendee.email.as_str().to_owned()),
                is_me: attendee.email.as_str() == USER_EMAIL,
            })
            .collect(),
        tags: vec![format!("{:?}", meeting.kind)],
        folders: vec!["Meetings".to_owned()],
        notes: Some(notes(meeting)),
        my_notes: meeting.user_notes.clone(),
        action_items: action_items(meeting),
        insights: insights(meeting),
        transcript: transcript(meeting),
    }
}

/// A "Decisions" insight from the ground truth's decisions, the first one
/// attributed to the meeting's first line, so goldens show both forms.
fn insights(meeting: &FixtureMeeting) -> Vec<InsightExport> {
    let transcript = transcript(meeting);
    let first = transcript.first();
    let answers: Vec<InsightAnswerExport> = meeting
        .ground_truth
        .decisions
        .iter()
        .enumerate()
        .map(|(index, decision)| InsightAnswerExport {
            value: InsightExportValue::Text {
                text: decision.clone(),
            },
            speaker: first
                .filter(|_| index == 0)
                .map(|line| line.speaker.clone()),
            at: first.filter(|_| index == 0).map(|line| line.start),
        })
        .collect();
    if answers.is_empty() {
        return Vec::new();
    }
    vec![InsightExport {
        name: "Decisions".to_owned(),
        answers,
    }]
}

fn notes(meeting: &FixtureMeeting) -> String {
    let truth = &meeting.ground_truth;
    let decisions: Vec<String> = truth
        .decisions
        .iter()
        .map(|decision| format!("- {decision}"))
        .collect();
    let facts: Vec<String> = truth
        .facts
        .iter()
        .map(|fact| format!("- **{}** {}", fact.question, fact.answer))
        .collect();
    format!(
        "## Decisions\n\n{}\n\n## Key facts\n\n{}\n",
        decisions.join("\n"),
        facts.join("\n")
    )
}

fn action_items(meeting: &FixtureMeeting) -> Vec<ActionItemExport> {
    meeting
        .ground_truth
        .action_items
        .iter()
        .enumerate()
        .map(|(index, item)| ActionItemExport {
            title: item.title.clone(),
            description: String::new(),
            assignee: item
                .assignee
                .as_ref()
                .map(|email| name_of(meeting, email.as_str())),
            due: item.due,
            done: index == 0,
        })
        .collect()
}

fn transcript(meeting: &FixtureMeeting) -> Vec<Utterance> {
    meeting
        .segments
        .iter()
        .map(|segment| {
            let attendee = meeting
                .speaker(&segment.speaker)
                .and_then(|speaker| speaker.attendee.clone());
            Utterance {
                speaker: attendee.as_ref().map_or_else(
                    || segment.speaker.clone(),
                    |email| name_of(meeting, email.as_str()),
                ),
                is_me: attendee.is_some_and(|email| email.as_str() == USER_EMAIL),
                start: segment.start_ms,
                end: segment.end_ms,
                text: segment.text.clone(),
            }
        })
        .collect()
}

fn name_of(meeting: &FixtureMeeting, email: &str) -> String {
    meeting
        .attendees
        .iter()
        .find(|attendee| attendee.email.as_str() == email)
        .map_or_else(|| email.to_owned(), |attendee| attendee.name.clone())
}
