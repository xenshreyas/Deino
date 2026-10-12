//! [`Condition::normalized`]: conditions in canonical form, or refused.

use regex::RegexBuilder;

use super::holds::PATTERN_SIZE_LIMIT;
use super::{Condition, InvalidCondition, MAX_CONDITION_DOMAINS, MAX_TITLE_TEXT_CHARS};
use crate::Email;

impl Condition {
    /// The condition in canonical form: a domain typed as `@Acme.com `
    /// becomes `acme.com`, title text is trimmed; anything that cannot match
    /// as intended is refused.
    pub fn normalized(self) -> Result<Self, InvalidCondition> {
        match self {
            Self::AttendeeDomain { domain } => Ok(Self::AttendeeDomain {
                domain: parse_domain(&domain)?,
            }),
            Self::DomainsAreAll { domains } => Ok(Self::DomainsAreAll {
                domains: parse_domains(&domains)?,
            }),
            Self::DomainsNotAll { domains } => Ok(Self::DomainsNotAll {
                domains: parse_domains(&domains)?,
            }),
            Self::TitleIs { text } => title_text(&text).map(|text| Self::TitleIs { text }),
            Self::TitleIsNot { text } => title_text(&text).map(|text| Self::TitleIsNot { text }),
            Self::TitleContains { text } => {
                title_text(&text).map(|text| Self::TitleContains { text })
            }
            Self::TitleLacks { text } => title_text(&text).map(|text| Self::TitleLacks { text }),
            Self::TitleMatches { pattern } => {
                parse_pattern(&pattern).map(|pattern| Self::TitleMatches { pattern })
            }
            condition => Ok(condition),
        }
    }
}

/// `input` trimmed, if it is 1 to [`MAX_TITLE_TEXT_CHARS`] characters.
fn title_text(input: &str) -> Result<String, InvalidCondition> {
    let text = input.trim().to_owned();
    let length = text.chars().count();
    if length == 0 || length > MAX_TITLE_TEXT_CHARS {
        return Err(InvalidCondition::TitleText);
    }
    Ok(text)
}

/// `input` trimmed, if it compiles within the size limit.
fn parse_pattern(input: &str) -> Result<String, InvalidCondition> {
    let pattern = title_text(input).map_err(|_| InvalidCondition::Pattern)?;
    RegexBuilder::new(&pattern)
        .size_limit(PATTERN_SIZE_LIMIT)
        .build()
        .map_err(|_| InvalidCondition::Pattern)?;
    Ok(pattern)
}

/// `input` as an email domain, exactly as [`Email::domain`] reports one.
fn parse_domain(input: &str) -> Result<String, InvalidCondition> {
    let domain = input.trim().trim_start_matches('@');
    Email::parse(&format!("someone@{domain}"))
        .map(|email| email.domain().to_owned())
        .map_err(|_| InvalidCondition::Domain(input.to_owned()))
}

/// Every domain parsed, sorted, without repeats; 1 to
/// [`MAX_CONDITION_DOMAINS`] of them.
fn parse_domains(inputs: &[String]) -> Result<Vec<String>, InvalidCondition> {
    let mut domains = inputs
        .iter()
        .map(|input| parse_domain(input))
        .collect::<Result<Vec<_>, _>>()?;
    domains.sort();
    domains.dedup();
    if domains.is_empty() || domains.len() > MAX_CONDITION_DOMAINS {
        return Err(InvalidCondition::DomainCount);
    }
    Ok(domains)
}
