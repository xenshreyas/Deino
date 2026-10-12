//! Companies and their web domains.

use chrono::{DateTime, Utc};
use deino_domain::CompanyId;
use serde::{Deserialize, Serialize};

/// A company the user's contacts work at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    /// The company's ID.
    pub id: CompanyId,
    /// Its display name.
    pub name: String,
    /// Its web domain; unique, so people with that email domain join it.
    pub domain: Option<CompanyDomain>,
    /// A locally cached logo (ORG-04 fetches the favicon once).
    pub logo_path: Option<String>,
    /// The user's own notes, in Markdown.
    pub notes_md: String,
    /// The AI summary of the relationship, in Markdown (ORG-04).
    pub summary_md: Option<String>,
    /// When `summary_md` was last refreshed.
    pub summary_updated_at: Option<DateTime<Utc>>,
    /// When the favicon was last asked for, found or not; logos are fetched
    /// once.
    pub logo_checked_at: Option<DateTime<Utc>>,
    /// The company this one was merged into; merged companies are hidden
    /// from lists and their domain resolves to the survivor.
    pub merged_into: Option<CompanyId>,
    /// When the company was added.
    pub created_at: DateTime<Utc>,
    /// When the company last changed.
    pub updated_at: DateTime<Utc>,
}

impl Company {
    /// A new company with no logo, notes or summary yet.
    pub fn new(
        id: CompanyId,
        name: String,
        domain: Option<CompanyDomain>,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            domain,
            logo_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            logo_checked_at: None,
            merged_into: None,
            created_at: at,
            updated_at: at,
        }
    }
}

/// A web domain such as `acme.com`: trimmed, lower case, with at least one
/// dot and no `@` or whitespace. The schema checks the same shape.
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
pub struct CompanyDomain(String);

impl CompanyDomain {
    /// Normalises `input` into a domain, or `None` if it is not one.
    pub fn parse(input: &str) -> Option<Self> {
        let domain = input.trim().to_lowercase();
        let looks_valid = domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains(|c: char| c == '@' || c.is_whitespace());
        looks_valid.then_some(Self(domain))
    }

    /// The domain as stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_trimmed_and_lower_cased() {
        let domain = CompanyDomain::parse("  Acme.COM ").unwrap();
        assert_eq!(domain.as_str(), "acme.com");
    }

    #[test]
    fn things_that_are_not_domains_are_rejected() {
        for input in ["", "acme", "me@acme.com", "acme .com", ".acme.com", "acme."] {
            assert_eq!(CompanyDomain::parse(input), None, "{input:?}");
        }
    }
}
