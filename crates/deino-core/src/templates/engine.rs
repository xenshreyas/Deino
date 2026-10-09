//! [`RuleEngine`]: picks the template whose auto rules fit a meeting best.
//!
//! Pure and synchronous: it compiles every template's rules once and
//! evaluates them against [`MeetingSignals`]. When several templates match,
//! the most specific rule wins (the one with the most conditions); ties go to
//! the user's own templates over built-ins, then to list order (by name).

use std::cmp::Reverse;

use deino_domain::{Audience, Email, TemplateId};
use regex::Regex;

use super::rules::compile_pattern;
use super::{AutoRule, CountRange, Template};

/// What rules can test about a meeting.
#[derive(Clone, Copy, Debug)]
pub struct MeetingSignals<'a> {
    /// The meeting's title.
    pub title: &'a str,
    /// Its attendees' addresses.
    pub attendee_emails: &'a [Email],
    /// How many attendees it has.
    pub attendee_count: usize,
    /// Internal or external; `None` when it cannot be told.
    pub audience: Option<Audience>,
}

/// The template a rule chose, and the rule's description for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleMatch {
    /// The chosen template.
    pub template_id: TemplateId,
    /// The rule's label, or a description of its conditions.
    pub rule: String,
}

/// Every template's rules, compiled.
#[derive(Debug)]
pub struct RuleEngine {
    rules: Vec<CompiledRule>,
}

impl RuleEngine {
    /// Compiles the rules of `templates`. A pattern that no longer compiles
    /// (it was valid when saved) disables only its own rule.
    pub fn new(templates: &[Template]) -> Self {
        let rules = templates
            .iter()
            .enumerate()
            .flat_map(|(order, template)| {
                template
                    .content
                    .auto_rules
                    .iter()
                    .filter_map(move |rule| CompiledRule::new(template, order, rule))
            })
            .collect();
        Self { rules }
    }

    /// The best-matching template for `meeting`, if any rule matches.
    pub fn best_match(&self, meeting: &MeetingSignals<'_>) -> Option<RuleMatch> {
        self.rules
            .iter()
            .filter(|rule| rule.matches(meeting))
            .min_by_key(|rule| (Reverse(rule.conditions), rule.is_builtin, rule.order))
            .map(|rule| RuleMatch {
                template_id: rule.template_id.clone(),
                rule: rule.description.clone(),
            })
    }
}

/// One rule, ready to evaluate.
#[derive(Debug)]
struct CompiledRule {
    template_id: TemplateId,
    is_builtin: bool,
    order: usize,
    conditions: usize,
    description: String,
    title: Option<Regex>,
    domains: Vec<String>,
    audience: Option<Audience>,
    count: Option<CountRange>,
}

impl CompiledRule {
    fn new(template: &Template, order: usize, rule: &AutoRule) -> Option<Self> {
        let title = match &rule.title_pattern {
            Some(pattern) => Some(compile_pattern(pattern).ok()?),
            None => None,
        };
        Some(Self {
            template_id: template.id.clone(),
            is_builtin: template.is_builtin,
            order,
            conditions: rule.condition_count(),
            description: rule.describe(),
            title,
            domains: rule.attendee_domains.clone(),
            audience: rule.audience,
            count: rule.attendee_count,
        })
    }

    /// True when every condition the rule sets holds; a rule with none never matches.
    fn matches(&self, meeting: &MeetingSignals<'_>) -> bool {
        self.conditions > 0
            && self
                .title
                .as_ref()
                .is_none_or(|title| title.is_match(meeting.title))
            && self
                .audience
                .is_none_or(|audience| meeting.audience == Some(audience))
            && self
                .count
                .is_none_or(|count| count.contains(meeting.attendee_count))
            && (self.domains.is_empty() || self.has_attendee_from_domains(meeting))
    }

    fn has_attendee_from_domains(&self, meeting: &MeetingSignals<'_>) -> bool {
        meeting.attendee_emails.iter().any(|email| {
            let domain = email.domain();
            self.domains.iter().any(|wanted| {
                domain == wanted
                    || domain
                        .strip_suffix(wanted.as_str())
                        .is_some_and(|prefix| prefix.ends_with('.'))
            })
        })
    }
}
