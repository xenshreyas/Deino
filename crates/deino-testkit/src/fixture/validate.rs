//! The second half of the fixture schema check: rules that span fields.
//!
//! Deserializing proves each field has the right shape; [`validate`] proves
//! the meeting is coherent: every speaker is invited and speaks, the user is
//! present and on the microphone, segments and words are in order inside the
//! recording, owners are attendees, and every ground-truth `evidence` quote
//! appears in the transcript. AI evals rely on all of it.

use std::collections::HashSet;

use chrono::TimeDelta;

use super::model::{FixtureMeeting, GroundTruth, Segment, USER_EMAIL};
use super::vocab::{AttendeeRole, Channel, Email, Millis, Platform, Rsvp};

/// Every fixture ID starts with this, so fixture rows are easy to recognise.
pub const ID_PREFIX: &str = "meeting-fixture-";
/// The longest recording a fixture may describe.
const MAX_DURATION: Millis = Millis(4 * 60 * 60 * 1000);
/// How long after the scheduled start recording may begin.
const MAX_LATE_START: TimeDelta = TimeDelta::minutes(15);

/// A rule a fixture meeting breaks. Messages name indices and labels only.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The ID lacks the fixture prefix.
    #[error("the id must start with {ID_PREFIX:?}")]
    IdPrefix,
    /// The calendar event ends before it starts, or recording starts outside it.
    #[error("the calendar event times or the recording start are inconsistent")]
    EventTiming,
    /// A call has no link, or an in-person meeting has no room.
    #[error("the calendar event's platform, link and location disagree")]
    EventVenue,
    /// The duration is zero or longer than four hours.
    #[error("the duration must be between 1 ms and 4 hours")]
    Duration,
    /// The user is not an attendee.
    #[error("{USER_EMAIL} must attend every fixture meeting")]
    MissingUser,
    /// Two attendees share an email.
    #[error("attendee {0} is listed twice")]
    DuplicateAttendee(Email),
    /// The organizer role and the event's organizer email disagree.
    #[error("exactly one attendee, the event organizer, must have the organizer role")]
    Organizer,
    /// A company is listed twice, or no attendee belongs to it.
    #[error("company domain {0} is duplicated or has no attendee")]
    Company(String),
    /// A speaker label is not `Speaker N` in order of first appearance.
    #[error("speaker {0} is not labelled in order of first appearance")]
    SpeakerOrder(String),
    /// A speaker is not an attendee who accepted, or two speakers share one.
    #[error("speaker {0} is not a distinct, attending attendee")]
    SpeakerAttendee(String),
    /// The user speaks on the system channel.
    #[error("the user can only be heard on the microphone")]
    UserOnSystem,
    /// A segment names an unknown speaker or the wrong channel.
    #[error("segment {0} has an unknown speaker or the wrong channel")]
    SegmentSpeaker(usize),
    /// A segment is empty, reversed, overlaps the previous one or runs past the end.
    #[error("segment {0} is empty, out of order or outside the recording")]
    SegmentTiming(usize),
    /// A segment's words do not spell its text or fall outside it.
    #[error("the words of segment {0} do not match its text or timing")]
    Words(usize),
    /// An action item's owner is not an attendee.
    #[error("action item {0} is assigned to someone who was not invited")]
    Assignee(usize),
    /// A ground-truth quote is not in the transcript.
    #[error("ground-truth evidence {0} is not quoted from any segment")]
    Evidence(usize),
    /// The audio clip ends after the recording.
    #[error("the audio clip ends after the recording")]
    AudioClip,
}

/// Checks every cross-field rule of `meeting`.
pub fn validate(meeting: &FixtureMeeting) -> Result<(), Problem> {
    check_header(meeting)?;
    check_attendees(meeting)?;
    check_companies(meeting)?;
    check_speakers(meeting)?;
    meeting
        .segments
        .iter()
        .enumerate()
        .try_for_each(|(index, segment)| check_segment(meeting, index, segment))?;
    check_ground_truth(meeting, &meeting.ground_truth)?;
    match &meeting.audio {
        Some(audio) if audio.clip_end_ms > meeting.duration => Err(Problem::AudioClip),
        _ => Ok(()),
    }
}

fn check_header(meeting: &FixtureMeeting) -> Result<(), Problem> {
    let event = &meeting.calendar_event;
    if !meeting.id.as_str().starts_with(ID_PREFIX) {
        return Err(Problem::IdPrefix);
    }
    let late_by = meeting.started_at - event.start_at;
    if event.end_at <= event.start_at || late_by < TimeDelta::zero() || late_by > MAX_LATE_START {
        return Err(Problem::EventTiming);
    }
    let in_person = event.platform == Platform::InPerson;
    if in_person == event.conference_url.is_some() || in_person != event.location.is_some() {
        return Err(Problem::EventVenue);
    }
    if meeting.duration == Millis(0) || meeting.duration > MAX_DURATION {
        return Err(Problem::Duration);
    }
    Ok(())
}

fn check_attendees(meeting: &FixtureMeeting) -> Result<(), Problem> {
    let mut seen = HashSet::new();
    if let Some(duplicate) = meeting.attendees.iter().find(|a| !seen.insert(&a.email)) {
        return Err(Problem::DuplicateAttendee(duplicate.email.clone()));
    }
    if !meeting
        .attendees
        .iter()
        .any(|a| a.email.as_str() == USER_EMAIL)
    {
        return Err(Problem::MissingUser);
    }
    let mut organizers = meeting
        .attendees
        .iter()
        .filter(|a| a.role == AttendeeRole::Organizer);
    match (organizers.next(), organizers.next()) {
        (Some(organizer), None) if organizer.email == meeting.calendar_event.organizer_email => {
            Ok(())
        }
        _ => Err(Problem::Organizer),
    }
}

fn check_companies(meeting: &FixtureMeeting) -> Result<(), Problem> {
    let mut seen = HashSet::new();
    for company in &meeting.companies {
        let has_attendee = meeting
            .attendees
            .iter()
            .any(|a| a.email.domain() == company.domain);
        if !seen.insert(&company.domain) || !has_attendee {
            return Err(Problem::Company(company.domain.clone()));
        }
    }
    Ok(())
}

fn check_speakers(meeting: &FixtureMeeting) -> Result<(), Problem> {
    let mut first_heard: Vec<&str> = Vec::new();
    for segment in &meeting.segments {
        if !first_heard.contains(&segment.speaker.as_str()) {
            first_heard.push(&segment.speaker);
        }
    }
    let mut attendees = HashSet::new();
    for (index, speaker) in meeting.speakers.iter().enumerate() {
        let label = || speaker.label.clone();
        let expected = format!("Speaker {}", index + 1);
        if speaker.label != expected || first_heard.get(index) != Some(&expected.as_str()) {
            return Err(Problem::SpeakerOrder(label()));
        }
        let Some(email) = &speaker.attendee else {
            continue;
        };
        let attending = meeting
            .attendee(email)
            .is_some_and(|a| a.rsvp != Rsvp::Declined);
        if !attending || !attendees.insert(email) {
            return Err(Problem::SpeakerAttendee(label()));
        }
        if email.as_str() == USER_EMAIL && speaker.channel != Channel::Mic {
            return Err(Problem::UserOnSystem);
        }
    }
    Ok(())
}

fn check_segment(meeting: &FixtureMeeting, index: usize, segment: &Segment) -> Result<(), Problem> {
    let speaker = meeting.speaker(&segment.speaker);
    if speaker.is_none_or(|speaker| speaker.channel != segment.channel) {
        return Err(Problem::SegmentSpeaker(index));
    }
    let previous_end = index
        .checked_sub(1)
        .and_then(|previous| meeting.segments.get(previous))
        .map_or(Millis(0), |previous| previous.end_ms);
    let ordered = previous_end <= segment.start_ms
        && segment.start_ms < segment.end_ms
        && segment.end_ms <= meeting.duration;
    if !ordered || segment.text.trim().is_empty() {
        return Err(Problem::SegmentTiming(index));
    }
    if words_match(segment) {
        Ok(())
    } else {
        Err(Problem::Words(index))
    }
}

/// True when the words spell the text, in order, inside the segment.
fn words_match(segment: &Segment) -> bool {
    let words: Vec<&str> = segment.words.iter().map(|w| w.text.as_str()).collect();
    let text: Vec<&str> = segment.text.split_whitespace().collect();
    let mut cursor = segment.start_ms;
    for word in &segment.words {
        if word.start_ms < cursor || word.end_ms <= word.start_ms || word.end_ms > segment.end_ms {
            return false;
        }
        cursor = word.end_ms;
    }
    words == text
}

fn check_ground_truth(meeting: &FixtureMeeting, truth: &GroundTruth) -> Result<(), Problem> {
    for (index, item) in truth.action_items.iter().enumerate() {
        if item
            .assignee
            .as_ref()
            .is_some_and(|a| meeting.attendee(a).is_none())
        {
            return Err(Problem::Assignee(index));
        }
    }
    let quotes = truth.action_items.iter().map(|item| &item.evidence);
    let quotes = quotes.chain(truth.facts.iter().map(|fact| &fact.evidence));
    for (index, quote) in quotes.enumerate() {
        let in_transcript = meeting
            .segments
            .iter()
            .any(|s| s.text.contains(quote.as_str()));
        if quote.trim().is_empty() || !in_transcript {
            return Err(Problem::Evidence(index));
        }
    }
    Ok(())
}
