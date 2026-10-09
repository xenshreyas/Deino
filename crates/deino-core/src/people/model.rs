//! People, their email addresses, and who attended which meeting.

use chrono::{DateTime, Utc};
use deino_domain::{AttendeeRole, CompanyId, MeetingId, PersonId, Rsvp};
use serde::{Deserialize, Serialize};

/// Someone the user meets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    /// The person's ID.
    pub id: PersonId,
    /// Their display name.
    pub name: String,
    /// Their email address; unique, so calendar attendees upsert onto one person.
    pub email: Option<EmailAddress>,
    /// Where they work.
    pub company_id: Option<CompanyId>,
    /// Their job title.
    pub title: Option<String>,
    /// A locally cached avatar.
    pub avatar_path: Option<String>,
    /// The user's own notes, in Markdown.
    pub notes_md: String,
    /// The AI summary of who they are and where things stand (ORG-04).
    pub summary_md: Option<String>,
    /// When `summary_md` was last refreshed.
    pub summary_updated_at: Option<DateTime<Utc>>,
    /// True for the user; at most one person is.
    pub is_me: bool,
    /// The person this one was merged into; merged people are hidden from lists.
    pub merged_into: Option<PersonId>,
    /// When the person was added.
    pub created_at: DateTime<Utc>,
    /// When the person last changed.
    pub updated_at: DateTime<Utc>,
}

/// An email address: trimmed, lower case, with something on both sides of
/// one `@` and no whitespace. The schema checks the same shape, and the
/// lower case makes the UNIQUE constraint case-insensitive.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// Normalises `input` into an email address, or `None` if it is not one.
    pub fn parse(input: &str) -> Option<Self> {
        let email = input.trim().to_lowercase();
        let (local, domain) = email.split_once('@')?;
        let looks_valid = !local.is_empty()
            && !domain.is_empty()
            && !domain.contains('@')
            && !email.contains(char::is_whitespace);
        looks_valid.then_some(Self(email))
    }

    /// The address as stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part after the `@`, for matching a company by domain.
    pub fn domain(&self) -> &str {
        self.0.split_once('@').map_or("", |(_, domain)| domain)
    }
}

/// A person's place in a meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The person.
    pub person_id: PersonId,
    /// How they took part.
    pub role: AttendeeRole,
    /// Their answer to the invitation; `None` when they were not invited or
    /// the calendar did not say.
    pub rsvp: Option<Rsvp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_addresses_are_trimmed_and_lower_cased() {
        let email = EmailAddress::parse(" Ada@Example.COM ").unwrap();
        assert_eq!(email.as_str(), "ada@example.com");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn things_that_are_not_email_addresses_are_rejected() {
        for input in [
            "",
            "ada",
            "@example.com",
            "ada@",
            "a@b@c",
            "ada lovelace@example.com",
        ] {
            assert_eq!(EmailAddress::parse(input), None, "{input:?}");
        }
    }
}
