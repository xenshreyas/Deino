//! The small value types fixture meetings are made of: times, emails and the
//! closed sets (channel, platform, role, ...) that would otherwise be strings.
//!
//! [`Millis`], [`Email`], [`Channel`], [`AttendeeRole`] and [`Rsvp`] are the
//! domain's own types, re-exported; the rest only exist in fixtures.

use serde::{Deserialize, Serialize};

// The values the app's schema shares with the fixtures come from the domain,
// so a fixture can only hold what the database accepts (FND-07).
pub use deino_domain::{AttendeeRole, Channel, Email, InvalidEmail, Millis, Rsvp};

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
