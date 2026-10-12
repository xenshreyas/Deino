//! Merging two people, or two companies, into one.
//!
//! The rules are pure ([`PersonMerge::plan`], [`CompanyMerge::plan`]): the
//! survivor keeps everything it has and fills its gaps from the absorbed
//! entry (address, title, company, avatar, domain, logo, summary); both sets
//! of notes are kept. The absorbed entry stays as a hidden row pointing at
//! the survivor (`merged_into`), so its address or domain still resolves to
//! the survivor when a calendar names it again.
//!
//! [`EntityMerger`] applies a plan in one transaction and moves every link:
//! for people, meeting attendance, transcript speakers (and with them the
//! voices diarization stored on them), voiceprints and voice suggestions,
//! action-item assignments, @mentions in notes and earlier merges; for companies, their people and earlier
//! merges.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;

use crate::companies::Company;
use crate::people::Person;

/// What merging one person into another writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonMerge {
    /// The person who remains, with the gaps filled.
    pub survivor: Person,
    /// The person merged away, now pointing at the survivor.
    pub absorbed: Person,
}

impl PersonMerge {
    /// Merges `absorbed` into `survivor` at `at`.
    pub fn plan(mut survivor: Person, mut absorbed: Person, at: DateTime<Utc>) -> Self {
        if survivor.email.is_none() {
            survivor.email = absorbed.email.take();
        }
        survivor.title = survivor.title.or_else(|| absorbed.title.clone());
        survivor.company_id = survivor.company_id.or(absorbed.company_id);
        survivor.avatar_path = survivor
            .avatar_path
            .or_else(|| absorbed.avatar_path.clone());
        survivor.notes_md = joined_notes(&survivor.notes_md, &absorbed.notes_md);
        if survivor.summary_md.is_none() {
            survivor.summary_md.clone_from(&absorbed.summary_md);
            survivor.summary_updated_at = absorbed.summary_updated_at;
        }
        survivor.is_me |= absorbed.is_me;
        survivor.updated_at = at;
        absorbed.is_me = false;
        absorbed.merged_into = Some(survivor.id);
        absorbed.updated_at = at;
        Self { survivor, absorbed }
    }
}

/// What merging one company into another writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanyMerge {
    /// The company that remains, with the gaps filled.
    pub survivor: Company,
    /// The company merged away, now pointing at the survivor.
    pub absorbed: Company,
}

impl CompanyMerge {
    /// Merges `absorbed` into `survivor` at `at`.
    pub fn plan(mut survivor: Company, mut absorbed: Company, at: DateTime<Utc>) -> Self {
        if survivor.domain.is_none() {
            survivor.domain = absorbed.domain.take();
        }
        if survivor.logo_path.is_none() {
            survivor.logo_path.clone_from(&absorbed.logo_path);
            survivor.logo_checked_at = absorbed.logo_checked_at;
        }
        survivor.notes_md = joined_notes(&survivor.notes_md, &absorbed.notes_md);
        if survivor.summary_md.is_none() {
            survivor.summary_md.clone_from(&absorbed.summary_md);
            survivor.summary_updated_at = absorbed.summary_updated_at;
        }
        survivor.updated_at = at;
        absorbed.merged_into = Some(survivor.id);
        absorbed.updated_at = at;
        Self { survivor, absorbed }
    }
}

/// Both sets of notes, the survivor's first, separated by a blank line.
fn joined_notes(kept: &str, absorbed: &str) -> String {
    match (kept.trim().is_empty(), absorbed.trim().is_empty()) {
        (_, true) => kept.to_owned(),
        (true, false) => absorbed.to_owned(),
        (false, false) => format!("{}\n\n{}", kept.trim_end(), absorbed.trim_start()),
    }
}

/// Applies merges, each in one transaction with every link it moves.
#[async_trait]
pub trait EntityMerger: Send + Sync {
    /// Writes both people and moves every link from the absorbed person to
    /// the survivor (see the module docs).
    async fn merge_people(&self, merge: &PersonMerge) -> Result<(), DbError>;

    /// Writes both companies and moves the absorbed one's people and
    /// earlier merges to the survivor.
    async fn merge_companies(&self, merge: &CompanyMerge) -> Result<(), DbError>;
}

#[cfg(test)]
mod tests {
    use deino_domain::{Clock, CompanyId, Email, FakeClock, PersonId};
    use deino_testkit::at;

    use super::*;
    use crate::companies::CompanyDomain;

    fn person(name: &str) -> Person {
        let created_at = at("2026-10-08T09:00:00Z");
        Person {
            id: PersonId::generate(&FakeClock::new(created_at)),
            name: name.into(),
            email: None,
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at,
            updated_at: created_at,
        }
    }

    #[test]
    fn the_survivor_keeps_its_fields_and_fills_gaps_from_the_absorbed_person() {
        let mut kept = person("Priya Nair");
        kept.title = Some("Staff Engineer".into());
        kept.notes_md = "Owns the sync rewrite.".into();
        let mut absorbed = person("Priya");
        absorbed.email = Some(Email::parse("priya@deino.dev").unwrap());
        absorbed.title = Some("Engineer".into());
        absorbed.company_id = Some(CompanyId::generate(&FakeClock::new(at(
            "2026-10-08T09:00:00Z",
        ))));
        absorbed.notes_md = "Prefers async updates.".into();
        absorbed.is_me = true;
        let when = at("2026-10-10T09:00:00Z");

        let merge = PersonMerge::plan(kept.clone(), absorbed.clone(), when);

        let survivor = &merge.survivor;
        assert_eq!(survivor.name, "Priya Nair");
        assert_eq!(survivor.title.as_deref(), Some("Staff Engineer"));
        assert_eq!(survivor.email, absorbed.email);
        assert_eq!(survivor.company_id, absorbed.company_id);
        assert_eq!(
            survivor.notes_md,
            "Owns the sync rewrite.\n\nPrefers async updates."
        );
        assert!(survivor.is_me);
        assert_eq!(survivor.updated_at, when);
        assert_eq!(
            merge.absorbed.email, None,
            "the address moved to the survivor"
        );
        assert_eq!(merge.absorbed.merged_into, Some(kept.id));
        assert!(!merge.absorbed.is_me);
    }

    #[test]
    fn an_address_the_survivor_does_not_need_stays_on_the_absorbed_person() {
        let mut kept = person("Maya");
        kept.email = Some(Email::parse("maya@deino.dev").unwrap());
        let mut absorbed = person("Maya Chen");
        absorbed.email = Some(Email::parse("maya.chen@gmail.com").unwrap());

        let merge = PersonMerge::plan(kept.clone(), absorbed.clone(), at("2026-10-10T09:00:00Z"));

        assert_eq!(merge.survivor.email, kept.email);
        assert_eq!(merge.absorbed.email, absorbed.email, "kept as an alias");
    }

    #[test]
    fn a_company_takes_the_absorbed_domain_logo_and_summary_when_it_has_none() {
        let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
        let kept = Company::new(
            CompanyId::generate(&clock),
            "Acme".into(),
            None,
            clock.now(),
        );
        let mut absorbed = Company::new(
            CompanyId::generate(&clock),
            "Acme Corp".into(),
            CompanyDomain::parse("acme.com"),
            clock.now(),
        );
        absorbed.logo_path = Some("/logos/acme.png".into());
        absorbed.summary_md = Some("Renewing in Q1.".into());

        let merge = CompanyMerge::plan(kept.clone(), absorbed, at("2026-10-10T09:00:00Z"));

        assert_eq!(merge.survivor.name, "Acme");
        assert_eq!(merge.survivor.domain, CompanyDomain::parse("acme.com"));
        assert_eq!(merge.survivor.logo_path.as_deref(), Some("/logos/acme.png"));
        assert_eq!(
            merge.survivor.summary_md.as_deref(),
            Some("Renewing in Q1.")
        );
        assert_eq!(merge.absorbed.domain, None);
        assert_eq!(merge.absorbed.merged_into, Some(kept.id));
    }
}
