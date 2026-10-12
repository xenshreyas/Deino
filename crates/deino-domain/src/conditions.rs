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
//! its arm in [`Condition::holds`], whatever [`MeetingFacts`] needs for it,
//! and its normalisation in [`Condition::normalized`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Email, FolderId, TagId, TemplateId};

/// The longest title text a condition may look for, in characters.
pub const MAX_TITLE_TEXT_CHARS: usize = 200;

/// One test of a meeting.
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
    /// The meeting sits in this folder.
    InFolder {
        /// The folder.
        #[serde(rename = "folderId")]
        folder_id: FolderId,
    },
    /// An attendee's email is at this domain (`acme.com`).
    AttendeeDomain {
        /// The domain, lower case, without `@`.
        domain: String,
    },
    /// The meeting has (`true`) or has no (`false`) external attendees:
    /// people outside the user's own email domains.
    External {
        /// Whether external attendees must be present.
        external: bool,
    },
    /// The title contains this text, ignoring case.
    TitleContains {
        /// The text to look for.
        text: String,
    },
}

/// A condition that cannot be stored as given.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCondition {
    /// The attendee domain is not an email domain.
    #[error("{0:?} is not an email domain")]
    Domain(String),
    /// The title text is empty or too long.
    #[error("the title text must be 1 to {MAX_TITLE_TEXT_CHARS} characters")]
    TitleText,
}

impl Condition {
    /// The condition in canonical form: a domain typed as `@Acme.com `
    /// becomes `acme.com`, title text is trimmed; anything that cannot match
    /// as intended is refused.
    pub fn normalized(self) -> Result<Self, InvalidCondition> {
        match self {
            Self::AttendeeDomain { domain } => Ok(Self::AttendeeDomain {
                domain: parse_domain(&domain)?,
            }),
            Self::TitleContains { text } => {
                let text = text.trim().to_owned();
                let length = text.chars().count();
                if length == 0 || length > MAX_TITLE_TEXT_CHARS {
                    return Err(InvalidCondition::TitleText);
                }
                Ok(Self::TitleContains { text })
            }
            condition => Ok(condition),
        }
    }

    /// True when a meeting with `facts` passes the test.
    pub fn holds(&self, facts: &MeetingFacts) -> bool {
        match self {
            Self::TemplateIs { template_id } => facts.template_id.as_ref() == Some(template_id),
            Self::HasTag { tag_id } => facts.tag_ids.contains(tag_id),
            Self::InFolder { folder_id } => facts.folder_ids.contains(folder_id),
            Self::AttendeeDomain { domain } => facts.attendee_domains.contains(domain),
            Self::External { external } => facts.has_external == *external,
            Self::TitleContains { text } => contains_ignoring_case(&facts.title, text),
        }
    }
}

/// `input` as an email domain, exactly as [`Email::domain`] reports one.
fn parse_domain(input: &str) -> Result<String, InvalidCondition> {
    let domain = input.trim().trim_start_matches('@');
    Email::parse(&format!("someone@{domain}"))
        .map(|email| email.domain().to_owned())
        .map_err(|_| InvalidCondition::Domain(input.to_owned()))
}

fn contains_ignoring_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
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
    /// The email domains of its attendees.
    pub attendee_domains: BTreeSet<String>,
    /// True when an attendee is outside the user's own domains.
    pub has_external: bool,
}

#[cfg(test)]
#[path = "conditions_tests.rs"]
mod tests;
