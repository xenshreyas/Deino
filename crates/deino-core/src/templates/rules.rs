//! Auto rules: when a template is chosen for a meeting without asking.
//!
//! A template has a list of [`AutoRule`]s and matches a meeting when any one
//! of them does; a rule matches when every condition it sets holds (title
//! pattern, attendee domain, internal or external, attendee count). Rules are
//! data (stored as JSON on the template) and validated on the way in;
//! [`RuleEngine`](super::RuleEngine) compiles and evaluates them.

use deino_domain::Audience;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use super::TemplateError;

/// The most rules a template can have.
pub const MAX_AUTO_RULES: usize = 10;
/// The longest rule label, in characters.
pub const MAX_RULE_LABEL_CHARS: usize = 60;
/// The longest title pattern, in characters.
pub const MAX_TITLE_PATTERN_CHARS: usize = 200;
/// The most attendee domains one rule can list.
pub const MAX_RULE_DOMAINS: usize = 10;
/// Compiled pattern size cap, so a pathological pattern cannot eat memory.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// An inclusive range of attendee counts; either end may be open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CountRange {
    /// At least this many attendees.
    pub min: Option<u32>,
    /// At most this many attendees.
    pub max: Option<u32>,
}

impl CountRange {
    /// True when `count` is within the range.
    pub fn contains(self, count: usize) -> bool {
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        self.min.is_none_or(|min| count >= min) && self.max.is_none_or(|max| count <= max)
    }
}

/// One way a template can be chosen automatically. Unset conditions are
/// ignored; a rule must set at least one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoRule {
    /// What the user sees when this rule chose the template, for example
    /// "external discovery call". Generated from the conditions when blank.
    pub label: String,
    /// A regular expression the meeting title must match, ignoring case.
    pub title_pattern: Option<String>,
    /// At least one attendee's email must be at one of these domains (or a
    /// subdomain), for example a customer's.
    pub attendee_domains: Vec<String>,
    /// The meeting must be internal, or external.
    pub audience: Option<Audience>,
    /// The number of attendees, the user included.
    pub attendee_count: Option<CountRange>,
}

impl AutoRule {
    /// The rule trimmed and lower-cased where it matters, or the first broken
    /// rule as an error naming the rule's 1-based `position`.
    pub fn validated(self, position: usize) -> Result<Self, TemplateError> {
        let invalid = |rule| TemplateError::InvalidRule { position, rule };
        let label = self.label.trim().to_owned();
        if label.chars().count() > MAX_RULE_LABEL_CHARS {
            return Err(invalid("has a label that is too long"));
        }
        let title_pattern = validated_pattern(self.title_pattern).map_err(invalid)?;
        let attendee_domains = validated_domains(self.attendee_domains).map_err(invalid)?;
        if let Some(range) = self.attendee_count {
            validate_range(range).map_err(invalid)?;
        }
        let rule = Self {
            label,
            title_pattern,
            attendee_domains,
            ..self
        };
        if rule.condition_count() == 0 {
            return Err(invalid("needs at least one condition"));
        }
        Ok(rule)
    }

    /// How many conditions the rule sets; more conditions, more specific.
    pub fn condition_count(&self) -> usize {
        usize::from(self.title_pattern.is_some())
            + usize::from(!self.attendee_domains.is_empty())
            + usize::from(self.audience.is_some())
            + usize::from(self.attendee_count.is_some())
    }

    /// The label, or a plain description of the conditions when it is blank.
    pub fn describe(&self) -> String {
        if !self.label.is_empty() {
            return self.label.clone();
        }
        let mut parts: Vec<String> = Vec::new();
        if let Some(audience) = self.audience {
            parts.push(format!("{audience} meeting"));
        }
        if let Some(range) = self.attendee_count {
            parts.push(describe_range(range));
        }
        if !self.attendee_domains.is_empty() {
            parts.push(format!(
                "attendee from {}",
                self.attendee_domains.join(" or ")
            ));
        }
        if let Some(pattern) = &self.title_pattern {
            parts.push(format!("title matches “{pattern}”"));
        }
        parts.join(", ")
    }
}

/// Compiles a title pattern the way rules match it: case-insensitively.
pub(super) fn compile_pattern(pattern: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
}

fn validated_pattern(pattern: Option<String>) -> Result<Option<String>, &'static str> {
    let Some(pattern) = pattern.map(|pattern| pattern.trim().to_owned()) else {
        return Ok(None);
    };
    if pattern.is_empty() {
        return Ok(None);
    }
    if pattern.chars().count() > MAX_TITLE_PATTERN_CHARS {
        return Err("has a title pattern that is too long");
    }
    compile_pattern(&pattern)
        .map_err(|_| "has a title pattern that is not a valid regular expression")?;
    Ok(Some(pattern))
}

fn validated_domains(domains: Vec<String>) -> Result<Vec<String>, &'static str> {
    let domains: Vec<String> = domains
        .into_iter()
        .map(|domain| domain.trim().trim_start_matches('@').to_lowercase())
        .filter(|domain| !domain.is_empty())
        .collect();
    if domains.len() > MAX_RULE_DOMAINS {
        return Err("lists too many domains");
    }
    let well_formed = |domain: &String| {
        domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains(|c: char| c == '@' || c.is_whitespace())
    };
    if !domains.iter().all(well_formed) {
        return Err("has a domain that is not like example.com");
    }
    Ok(domains)
}

fn validate_range(range: CountRange) -> Result<(), &'static str> {
    match (range.min, range.max) {
        (None, None) => Err("has an attendee count with no bounds"),
        (Some(0), _) | (_, Some(0)) => Err("has an attendee count below 1"),
        (Some(min), Some(max)) if min > max => {
            Err("has an attendee count whose minimum exceeds its maximum")
        }
        _ => Ok(()),
    }
}

fn describe_range(range: CountRange) -> String {
    match (range.min, range.max) {
        (Some(min), Some(max)) if min == max => format!("{min} attendees"),
        (Some(min), Some(max)) => format!("{min}–{max} attendees"),
        (Some(min), None) => format!("{min}+ attendees"),
        (None, Some(max)) => format!("up to {max} attendees"),
        (None, None) => "any number of attendees".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titled(pattern: &str) -> AutoRule {
        AutoRule {
            title_pattern: Some(pattern.into()),
            ..AutoRule::default()
        }
    }

    #[test]
    fn rules_are_normalised_on_the_way_in() {
        let rule = AutoRule {
            label: "  Acme calls ".into(),
            attendee_domains: vec![" @Acme.COM ".into(), " ".into()],
            ..titled("  demo  ")
        };

        let rule = rule.validated(1).unwrap();

        assert_eq!(rule.label, "Acme calls");
        assert_eq!(rule.title_pattern.as_deref(), Some("demo"));
        assert_eq!(rule.attendee_domains, ["acme.com"]);
    }

    #[test]
    fn broken_rules_are_named_by_position() {
        let empty = AutoRule::default().validated(2).unwrap_err();
        let bad_regex = titled("(unclosed").validated(1).unwrap_err();
        let bad_domain = AutoRule {
            attendee_domains: vec!["acme".into()],
            ..AutoRule::default()
        };
        let backwards = AutoRule {
            attendee_count: Some(CountRange {
                min: Some(5),
                max: Some(2),
            }),
            ..AutoRule::default()
        };

        assert_eq!(
            empty.to_string(),
            "auto rule 2 needs at least one condition"
        );
        assert!(
            bad_regex
                .to_string()
                .contains("not a valid regular expression")
        );
        assert!(bad_domain.validated(1).is_err());
        assert!(backwards.validated(1).is_err());
    }

    #[test]
    fn counts_and_descriptions_read_naturally() {
        let one_on_one = AutoRule {
            audience: Some(Audience::Internal),
            attendee_count: Some(CountRange {
                min: Some(2),
                max: Some(2),
            }),
            ..AutoRule::default()
        };

        assert!(
            CountRange {
                min: Some(2),
                max: None
            }
            .contains(9)
        );
        assert!(
            !CountRange {
                min: None,
                max: Some(3)
            }
            .contains(4)
        );
        assert_eq!(one_on_one.condition_count(), 2);
        assert_eq!(one_on_one.describe(), "internal meeting, 2 attendees");
        assert_eq!(titled("demo").describe(), "title matches “demo”");
    }
}
