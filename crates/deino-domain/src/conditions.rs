//! Meeting conditions: the one typed model of "which meetings does this
//! apply to?", shared by AI-06 (when an insight runs on its own) and AUT-01
//! (when an automation runs).
//!
//! A [`Condition`] is one test of a meeting's [`MeetingFacts`].
//! [`ConditionGroups`] combine them the way Circleback's automations do:
//! every condition of a group must hold (AND), and any group may match
//! (OR). No groups at all matches every meeting.
//!
//! Evaluation is pure, so the services that load the facts and the tests
//! that table them share one rule engine. A new condition is a variant here,
//! its arm in `holds.rs`, whatever [`MeetingFacts`] needs for it, and its
//! normalisation in `normalize.rs`.

mod holds;
mod normalize;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{CalendarId, Email, FolderId, PersonId, TagId, TemplateId};

/// The longest title text a condition may look for, in characters.
pub const MAX_TITLE_TEXT_CHARS: usize = 200;

/// The most domains one "invitee domains are all" condition may list.
pub const MAX_CONDITION_DOMAINS: usize = 20;

/// One test of a meeting.
///
/// Variant names and fields are stored inside insight and automation
/// definitions: rename them only with a data migration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    /// The meeting's notes were written with this template.
    TemplateIs {
        /// The template.
        #[serde(rename = "templateId")]
        template_id: TemplateId,
    },
    /// The meeting carries this tag, whoever applied it.
    HasTag {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
    },
    /// The meeting does not carry this tag.
    LacksTag {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
    },
    /// The meeting sits in this folder.
    InFolder {
        /// The folder.
        #[serde(rename = "folderId")]
        folder_id: FolderId,
    },
    /// This person took part (invited, or heard speaking).
    HasParticipant {
        /// The person.
        #[serde(rename = "personId")]
        person_id: PersonId,
    },
    /// This person did not take part.
    LacksParticipant {
        /// The person.
        #[serde(rename = "personId")]
        person_id: PersonId,
    },
    /// An attendee's email is at this domain (`acme.com`).
    AttendeeDomain {
        /// The domain, lower case, without `@`.
        domain: String,
    },
    /// An invitee has exactly this address.
    InviteeEmail {
        /// The address.
        email: Email,
    },
    /// Every invitee's domain is one of these ("internal only"). A meeting
    /// without invitees has no evidence either way and does not match.
    DomainsAreAll {
        /// The domains, lower case, without `@`.
        domains: Vec<String>,
    },
    /// Some invitee's domain is none of these ("someone external"). A meeting
    /// without invitees does not match.
    DomainsNotAll {
        /// The domains, lower case, without `@`.
        domains: Vec<String>,
    },
    /// How many invitees the meeting had, compared with `count`.
    InviteeCount {
        /// Equal to, more than or fewer than.
        comparison: Comparison,
        /// The number to compare with.
        count: u32,
    },
    /// The meeting has (`true`) or has no (`false`) external attendees:
    /// people outside the user's own email domains.
    External {
        /// Whether external attendees must be present.
        external: bool,
    },
    /// The title is exactly this text, ignoring case.
    TitleIs {
        /// The text.
        text: String,
    },
    /// The title is anything but this text, ignoring case.
    TitleIsNot {
        /// The text.
        text: String,
    },
    /// The title contains this text, ignoring case.
    TitleContains {
        /// The text to look for.
        text: String,
    },
    /// The title does not contain this text, ignoring case.
    TitleLacks {
        /// The text to look for.
        text: String,
    },
    /// The title matches this regular expression (Rust `regex` syntax).
    TitleMatches {
        /// The pattern.
        pattern: String,
    },
    /// The meeting recorded for longer than this many minutes.
    DurationOver {
        /// The minutes.
        minutes: u32,
    },
    /// The meeting's calendar event is on this calendar.
    CalendarIs {
        /// The calendar.
        #[serde(rename = "calendarId")]
        calendar_id: CalendarId,
    },
}

/// How a number in a condition compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    /// Exactly.
    Equal,
    /// Strictly more than.
    Greater,
    /// Strictly fewer than.
    Less,
}

impl Comparison {
    /// True when `value` compares with `target` this way.
    pub const fn holds(self, value: u32, target: u32) -> bool {
        match self {
            Self::Equal => value == target,
            Self::Greater => value > target,
            Self::Less => value < target,
        }
    }
}

/// A condition that cannot be stored as given.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCondition {
    /// The attendee domain is not an email domain.
    #[error("{0:?} is not an email domain")]
    Domain(String),
    /// A domain list is empty or too long.
    #[error("list 1 to {MAX_CONDITION_DOMAINS} domains")]
    DomainCount,
    /// The title text is empty or too long.
    #[error("the title text must be 1 to {MAX_TITLE_TEXT_CHARS} characters")]
    TitleText,
    /// The title pattern is not a regular expression.
    #[error("the title pattern is not a valid regular expression")]
    Pattern,
}

/// Conditions in groups: all of a group (AND), any group (OR); no groups
/// matches every meeting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct ConditionGroups(pub Vec<Vec<Condition>>);

impl ConditionGroups {
    /// Every meeting.
    pub const fn every_meeting() -> Self {
        Self(Vec::new())
    }

    /// True when a meeting with `facts` matches.
    pub fn matches(&self, facts: &MeetingFacts) -> bool {
        self.0.is_empty()
            || self
                .0
                .iter()
                .any(|group| group.iter().all(|condition| condition.holds(facts)))
    }

    /// True when any condition tests for `tag` being on the meeting: a tag
    /// applied to it may have made the groups match (AUT-01's tag trigger).
    pub fn mentions_tag(&self, tag: &TagId) -> bool {
        self.0
            .iter()
            .flatten()
            .any(|condition| matches!(condition, Condition::HasTag { tag_id } if tag_id == tag))
    }

    /// The groups in canonical form, empty groups dropped (an empty group
    /// would match every meeting, which "no groups" already says).
    pub fn normalized(self) -> Result<Self, InvalidCondition> {
        let mut groups = Vec::with_capacity(self.0.len());
        for group in self.0.into_iter().filter(|group| !group.is_empty()) {
            let group: Result<Vec<Condition>, InvalidCondition> =
                group.into_iter().map(Condition::normalized).collect();
            groups.push(group?);
        }
        Ok(Self(groups))
    }
}

/// What conditions know about one meeting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingFacts {
    /// Its title.
    pub title: String,
    /// The template its notes were written with, else the one chosen for it.
    pub template_id: Option<TemplateId>,
    /// The tags on it, by anyone.
    pub tag_ids: BTreeSet<TagId>,
    /// The folders it sits in.
    pub folder_ids: BTreeSet<FolderId>,
    /// The people who took part: invitees and identified speakers.
    pub participant_ids: BTreeSet<PersonId>,
    /// The invitees' addresses.
    pub invitee_emails: BTreeSet<Email>,
    /// The email domains of its attendees.
    pub attendee_domains: BTreeSet<String>,
    /// True when an attendee is outside the user's own domains.
    pub has_external: bool,
    /// How long it recorded, in seconds, when known.
    pub duration_s: Option<u32>,
    /// The calendar its event is on, when it has one.
    pub calendar_id: Option<CalendarId>,
}

#[cfg(test)]
#[path = "conditions_tests.rs"]
mod tests;
