//! [`Condition::holds`]: each condition's test of a meeting's facts.

use regex::RegexBuilder;

use super::{Condition, MeetingFacts};

/// The largest compiled title pattern, in bytes: a user's pattern can never
/// make evaluation slow or large.
pub(super) const PATTERN_SIZE_LIMIT: usize = 64 * 1024;

/// Seconds in a minute, for duration conditions.
const SECONDS_PER_MINUTE: u32 = 60;

impl Condition {
    /// True when a meeting with `facts` passes the test.
    pub fn holds(&self, facts: &MeetingFacts) -> bool {
        match self {
            Self::TemplateIs { template_id } => facts.template_id.as_ref() == Some(template_id),
            Self::HasTag { tag_id } => facts.tag_ids.contains(tag_id),
            Self::LacksTag { tag_id } => !facts.tag_ids.contains(tag_id),
            Self::InFolder { folder_id } => facts.folder_ids.contains(folder_id),
            Self::HasParticipant { person_id } => facts.participant_ids.contains(person_id),
            Self::LacksParticipant { person_id } => !facts.participant_ids.contains(person_id),
            Self::External { external } => facts.has_external == *external,
            Self::DurationOver { minutes } => facts
                .duration_s
                .is_some_and(|seconds| seconds > minutes.saturating_mul(SECONDS_PER_MINUTE)),
            Self::CalendarIs { calendar_id } => facts.calendar_id.as_ref() == Some(calendar_id),
            _ => self.holds_for_people(facts) || self.holds_for_title(&facts.title),
        }
    }

    /// The tests of who was invited; false for every other condition.
    fn holds_for_people(&self, facts: &MeetingFacts) -> bool {
        match self {
            Self::AttendeeDomain { domain } => facts.attendee_domains.contains(domain),
            Self::InviteeEmail { email } => facts.invitee_emails.contains(email),
            Self::DomainsAreAll { domains } => {
                !facts.attendee_domains.is_empty()
                    && facts
                        .attendee_domains
                        .iter()
                        .all(|seen| domains.contains(seen))
            }
            Self::DomainsNotAll { domains } => facts
                .attendee_domains
                .iter()
                .any(|seen| !domains.contains(seen)),
            Self::InviteeCount { comparison, count } => {
                let invitees = u32::try_from(facts.invitee_emails.len()).unwrap_or(u32::MAX);
                comparison.holds(invitees, *count)
            }
            _ => false,
        }
    }

    /// The tests of the title; false for every other condition.
    fn holds_for_title(&self, title: &str) -> bool {
        match self {
            Self::TitleIs { text } => same_ignoring_case(title, text),
            Self::TitleIsNot { text } => !same_ignoring_case(title, text),
            Self::TitleContains { text } => contains_ignoring_case(title, text),
            Self::TitleLacks { text } => !contains_ignoring_case(title, text),
            Self::TitleMatches { pattern } => RegexBuilder::new(pattern)
                .case_insensitive(true)
                .size_limit(PATTERN_SIZE_LIMIT)
                .build()
                .is_ok_and(|regex| regex.is_match(title)),
            _ => false,
        }
    }
}

fn same_ignoring_case(title: &str, text: &str) -> bool {
    title.trim().to_lowercase() == text.to_lowercase()
}

fn contains_ignoring_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
