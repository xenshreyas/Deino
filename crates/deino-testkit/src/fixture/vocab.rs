//! The small value types fixture meetings are made of: times, emails and the
//! closed sets (channel, platform, role, ...) that would otherwise be strings.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A duration or offset in milliseconds, measured from the start of the
/// recording when it is a timestamp.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Millis(pub u64);

impl Millis {
    /// The value in seconds, as the legacy `transcripts` columns store it.
    #[allow(
        clippy::cast_precision_loss,
        reason = "fixture offsets stay far below 2^52 ms, where f64 is exact"
    )]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

/// A validated email address: one `@`, non-empty local part and domain,
/// lower case. Parsing it is part of the fixture schema check.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Email(String);

impl Email {
    /// The part after the `@`, which identifies the attendee's company.
    pub fn domain(&self) -> &str {
        self.0.split_once('@').map_or("", |(_, domain)| domain)
    }

    /// The address as written in the fixture.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A string that is not a lower-case email address.
#[derive(Debug, thiserror::Error)]
#[error("{0:?} is not a lower-case email address")]
pub struct InvalidEmail(String);

impl TryFrom<String> for Email {
    type Error = InvalidEmail;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let valid = value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.contains('@')
        });
        if valid && value == value.to_lowercase() {
            Ok(Self(value))
        } else {
            Err(InvalidEmail(value))
        }
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Which capture track a segment was heard on (PARITY_PLAN §3C keeps them apart).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// The user's microphone: the user, and anyone in the room with them.
    Mic,
    /// System audio: everyone on the call who is not in the room.
    System,
}

impl Channel {
    /// The name the legacy `transcripts.speaker` column and the audio files use.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::System => "system",
        }
    }
}

/// The kind of meeting, one per fixture, so tests can pick a scenario by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingKind {
    /// A weekly manager and report conversation.
    OneOnOne,
    /// A short daily team status round.
    Standup,
    /// A first call with an external prospect.
    SalesDiscovery,
    /// A hiring interview with an external candidate.
    Interview,
    /// A quarterly update to the board, with external board members.
    BoardUpdate,
    /// An escalation call with an existing customer.
    CustomerSupport,
    /// An in-person session where everyone shares one microphone.
    Brainstorm,
    /// A long, many-person planning meeting.
    Planning,
}

/// Where a calendar event takes place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// Google Meet.
    GoogleMeet,
    /// Zoom.
    Zoom,
    /// Microsoft Teams.
    Teams,
    /// A room, with no conference link.
    InPerson,
}

/// How a calendar event repeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    /// A one-off event.
    None,
    /// Every working day.
    Daily,
    /// Every week.
    Weekly,
}

/// An attendee's part in the invite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendeeRole {
    /// Owns the calendar event.
    Organizer,
    /// Was invited.
    Invitee,
}

/// An attendee's answer to the invite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rsvp {
    /// Said yes.
    Accepted,
    /// Said maybe.
    Tentative,
    /// Said no; such attendees never speak in a fixture.
    Declined,
    /// Never answered.
    NeedsAction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_must_be_lower_case_with_a_dotted_domain() {
        let valid = Email::try_from("maya@deino.dev".to_owned()).unwrap();
        assert_eq!(valid.domain(), "deino.dev");

        for invalid in ["Maya@deino.dev", "maya@deino", "@deino.dev", "maya"] {
            assert!(Email::try_from(invalid.to_owned()).is_err(), "{invalid}");
        }
    }

    #[test]
    fn millis_convert_to_the_seconds_the_legacy_schema_stores() {
        assert!((Millis(1_250).as_secs_f64() - 1.25).abs() < f64::EPSILON);
    }
}
