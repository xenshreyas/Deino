//! The auto-record policy: one pure function from an event's facts, the
//! user's switches and their choices to "record or not, and why".
//!
//! Order of precedence, first match wins:
//!
//! 1. the user's choice for this one event ("just this one");
//! 2. their choice for the whole series ("all occurrences");
//! 3. events the user declined never record;
//! 4. events that are not meetings (all day, or fewer than
//!    [`MIN_MEETING_ATTENDEES`] people) never record;
//! 5. the global switches in [`AutoRecordSettings`]: the audience
//!    (internal or external), then who organised it, then the RSVP.

use deino_domain::{Email, Rsvp};
use serde::Serialize;

use crate::calendar::CalendarEvent;
use crate::profile::InternalDomains;
use crate::settings::AutoRecordSettings;

/// The fewest invitees, the user included, that make an event a meeting
/// (Granola notifies only for events with other people in them).
pub const MIN_MEETING_ATTENDEES: usize = 2;

/// Who a meeting is with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    /// Only people from the user's own domains.
    Internal,
    /// Someone from outside them.
    External,
}

/// Who organised a meeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Organizer {
    /// The user.
    Me,
    /// Someone else.
    Someone,
}

/// What the policy needs to know about an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventFacts {
    /// Internal or external.
    pub audience: Audience,
    /// Who organised it.
    pub organizer: Organizer,
    /// The user's answer, if they gave one.
    pub rsvp: Option<Rsvp>,
    /// How many people are invited, the user included.
    pub attendees: usize,
    /// True for an all-day event.
    pub all_day: bool,
}

impl EventFacts {
    /// The facts of a synced `event`, with `own` telling the user's addresses.
    pub fn of(event: &CalendarEvent, own: &InternalDomains) -> Self {
        Self {
            audience: if event.is_external {
                Audience::External
            } else {
                Audience::Internal
            },
            organizer: organizer_of(event, own),
            rsvp: event.my_rsvp,
            attendees: event.attendees.len(),
            all_day: event.is_all_day,
        }
    }

    /// True when the user organised it or said yes.
    fn accepted(&self) -> bool {
        self.organizer == Organizer::Me || self.rsvp == Some(Rsvp::Accepted)
    }
}

/// The organiser: the user when the organiser's address, or the address
/// of the attendee flagged as organiser, is theirs; also the user when no
/// one is named at all (an event made on this Mac without invitations).
fn organizer_of(event: &CalendarEvent, own: &InternalDomains) -> Organizer {
    let named = event.organizer_email.as_deref();
    let mine = named
        .and_then(|email| Email::parse(email).ok())
        .is_some_and(|email| own.is_own(&email));
    let nobody = named.is_none() && !event.attendees.iter().any(|it| it.is_organizer);
    if mine || nobody || own_organizer_flag(event, own) {
        Organizer::Me
    } else {
        Organizer::Someone
    }
}

/// True when the attendee flagged as organiser has one of the user's addresses.
fn own_organizer_flag(event: &CalendarEvent, own: &InternalDomains) -> bool {
    event
        .attendees
        .iter()
        .filter(|attendee| attendee.is_organizer)
        .filter_map(|attendee| attendee.email.as_deref())
        .filter_map(|email| Email::parse(email).ok())
        .any(|email| own.is_own(&email))
}

/// The user's explicit choices that apply to one event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Choices {
    /// "Just this one": `Some(true)` always records, `Some(false)` never.
    pub event: Option<bool>,
    /// "All occurrences" of its series.
    pub series: Option<bool>,
}

/// Why an event will or will not record; the UI words it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RecordReason {
    /// The user's choice for this event.
    EventChoice,
    /// The user's choice for its series.
    SeriesChoice,
    /// The user declined it.
    Declined,
    /// All day, or nobody else is invited.
    NotAMeeting,
    /// Recording this audience (internal or external) is off.
    AudienceOff,
    /// Someone else organised it and those are off.
    OrganizedByOthers,
    /// The user has not accepted it and those are off.
    NotAccepted,
    /// It passes every switch.
    Rules,
}

/// Whether an event records on its own, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecordDecision {
    /// True when it records.
    pub record: bool,
    /// The rule that decided it.
    pub reason: RecordReason,
}

impl RecordDecision {
    const fn yes(reason: RecordReason) -> Self {
        Self {
            record: true,
            reason,
        }
    }

    const fn no(reason: RecordReason) -> Self {
        Self {
            record: false,
            reason,
        }
    }

    const fn by_choice(record: bool, reason: RecordReason) -> Self {
        Self { record, reason }
    }
}

/// Decides whether an event with `facts` records on its own.
pub fn decide(rules: &AutoRecordSettings, facts: &EventFacts, choices: Choices) -> RecordDecision {
    if let Some(record) = choices.event {
        return RecordDecision::by_choice(record, RecordReason::EventChoice);
    }
    if let Some(record) = choices.series {
        return RecordDecision::by_choice(record, RecordReason::SeriesChoice);
    }
    if facts.rsvp == Some(Rsvp::Declined) {
        return RecordDecision::no(RecordReason::Declined);
    }
    if facts.all_day || facts.attendees < MIN_MEETING_ATTENDEES {
        return RecordDecision::no(RecordReason::NotAMeeting);
    }
    switches(*rules, facts)
}

/// Step 5: the global switches, audience first.
fn switches(rules: AutoRecordSettings, facts: &EventFacts) -> RecordDecision {
    let audience_on = match facts.audience {
        Audience::Internal => rules.internal,
        Audience::External => rules.external,
    };
    if !audience_on {
        return RecordDecision::no(RecordReason::AudienceOff);
    }
    if facts.organizer == Organizer::Someone && !rules.organized_by_others {
        return RecordDecision::no(RecordReason::OrganizedByOthers);
    }
    if !facts.accepted() && !rules.not_accepted {
        return RecordDecision::no(RecordReason::NotAccepted);
    }
    RecordDecision::yes(RecordReason::Rules)
}

/// What can start a recording on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartTrigger {
    /// The meeting app started using the microphone (CAP-06's detection).
    MicInUse,
    /// The event's start time came; `note_open` when the user has its note open.
    StartTime {
        /// True when the event's note is on screen.
        note_open: bool,
    },
}

/// True when `trigger` should start recording an event decided `decision`.
/// The start time alone is not enough: the user must have the note open,
/// so a meeting they skip is never recorded from an empty room.
pub fn starts_recording(decision: RecordDecision, trigger: StartTrigger) -> bool {
    decision.record
        && match trigger {
            StartTrigger::MicInUse => true,
            StartTrigger::StartTime { note_open } => note_open,
        }
}
