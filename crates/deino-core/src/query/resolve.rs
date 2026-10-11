//! [`Resolver`]: what a caller typed (an ID, a name, an email, a domain)
//! turned into the tag, folder, person or company it means.
//!
//! A reference matches, in order: an ID exactly; any name ignoring case
//! (a person's email, a company's domain); one name containing it. More
//! than one match at a step is ambiguous, and the error lists them.

use deino_domain::{CompanyId, FolderId, PersonId, TagId};

use super::{EntityKind, QueryError, ReadPorts};

/// The most candidates an ambiguity error names.
const MAX_CANDIDATES: usize = 5;

/// Something a reference can match.
struct Candidate<T> {
    /// What the reference resolves to.
    value: T,
    /// Its ID as text.
    id: String,
    /// What it is called in messages.
    label: String,
    /// Other names it answers to exactly (an email, a domain).
    aliases: Vec<String>,
}

impl<T> Candidate<T> {
    fn named(&self, reference: &str) -> bool {
        self.label.eq_ignore_ascii_case(reference)
            || self
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(reference))
    }

    fn contains(&self, needle: &str) -> bool {
        self.label.to_lowercase().contains(needle)
    }
}

/// How a reference may match a candidate, strictest first.
#[derive(Clone, Copy)]
enum Step {
    /// The ID, exactly.
    Id,
    /// A whole name or alias, ignoring case.
    Name,
    /// Part of the name, ignoring case.
    Part,
}

impl Step {
    const ORDER: [Self; 3] = [Self::Id, Self::Name, Self::Part];

    fn matches<T>(self, candidate: &Candidate<T>, reference: &str, needle: &str) -> bool {
        match self {
            Self::Id => candidate.id == reference,
            Self::Name => candidate.named(reference),
            Self::Part => candidate.contains(needle),
        }
    }
}

/// Turns references into IDs over the library's repositories.
pub struct Resolver<'a> {
    ports: &'a ReadPorts,
}

impl<'a> Resolver<'a> {
    /// A resolver over `ports`.
    pub fn new(ports: &'a ReadPorts) -> Self {
        Self { ports }
    }

    /// The tag `reference` names.
    pub async fn tag(&self, reference: &str) -> Result<TagId, QueryError> {
        let tags = self.ports.tags.tags().await?;
        let candidates = tags.into_iter().map(|tag| Candidate {
            value: tag.id,
            id: tag.id.to_string(),
            label: tag.name,
            aliases: Vec::new(),
        });
        pick(EntityKind::Tag, reference, candidates.collect())
    }

    /// The folder `reference` names.
    pub async fn folder(&self, reference: &str) -> Result<FolderId, QueryError> {
        let folders = self.ports.folders.folders().await?;
        let candidates = folders.into_iter().map(|folder| Candidate {
            value: folder.id,
            id: folder.id.to_string(),
            label: folder.name,
            aliases: Vec::new(),
        });
        pick(EntityKind::Folder, reference, candidates.collect())
    }

    /// The person `reference` names (an ID, name or email).
    pub async fn person(&self, reference: &str) -> Result<PersonId, QueryError> {
        let people = self.ports.meeting.people.list().await?;
        let candidates = people.into_iter().map(|person| Candidate {
            value: person.id,
            id: person.id.to_string(),
            label: person.name,
            aliases: person
                .email
                .map(|email| email.as_str().to_owned())
                .into_iter()
                .collect(),
        });
        pick(EntityKind::Person, reference, candidates.collect())
    }

    /// The company `reference` names (an ID, name or domain).
    pub async fn company(&self, reference: &str) -> Result<CompanyId, QueryError> {
        let companies = self.ports.companies.list().await?;
        let candidates = companies.into_iter().map(|company| Candidate {
            value: company.id,
            id: company.id.to_string(),
            label: company.name,
            aliases: company
                .domain
                .map(|domain| domain.as_str().to_owned())
                .into_iter()
                .collect(),
        });
        pick(EntityKind::Company, reference, candidates.collect())
    }
}

/// The one candidate `reference` matches.
fn pick<T>(
    kind: EntityKind,
    reference: &str,
    candidates: Vec<Candidate<T>>,
) -> Result<T, QueryError> {
    let reference = reference.trim();
    let needle = reference.to_lowercase();
    let mut candidates = candidates;
    for step in Step::ORDER {
        let (mut matched, rest): (Vec<_>, Vec<_>) = candidates
            .into_iter()
            .partition(|candidate| step.matches(candidate, reference, &needle));
        if matched.len() > 1 {
            return Err(ambiguous(kind, reference, &matched));
        }
        if let Some(found) = matched.pop() {
            return Ok(found.value);
        }
        candidates = rest;
    }
    Err(QueryError::NotFound {
        kind,
        reference: reference.to_owned(),
    })
}

fn ambiguous<T>(kind: EntityKind, reference: &str, matched: &[Candidate<T>]) -> QueryError {
    QueryError::Ambiguous {
        kind,
        reference: reference.to_owned(),
        candidates: matched
            .iter()
            .take(MAX_CANDIDATES)
            .map(|candidate| candidate.label.clone())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn people() -> Vec<Candidate<&'static str>> {
        [
            ("p1", "Maya Chen", "maya@deino.dev"),
            ("p2", "Marco Rossi", "marco@acme.com"),
            ("p3", "Maya", "maya@other.dev"),
        ]
        .into_iter()
        .map(|(id, name, email)| Candidate {
            value: id,
            id: id.to_owned(),
            label: name.to_owned(),
            aliases: vec![email.to_owned()],
        })
        .collect()
    }

    #[test]
    fn an_id_wins_over_names() {
        assert_eq!(pick(EntityKind::Person, "p2", people()).unwrap(), "p2");
    }

    #[test]
    fn an_exact_name_or_alias_wins_over_a_partial_one() {
        assert_eq!(pick(EntityKind::Person, " maya ", people()).unwrap(), "p3");
        assert_eq!(
            pick(EntityKind::Person, "MARCO@acme.com", people()).unwrap(),
            "p2"
        );
    }

    #[test]
    fn a_unique_part_of_a_name_matches() {
        assert_eq!(pick(EntityKind::Person, "ross", people()).unwrap(), "p2");
    }

    #[test]
    fn several_partial_matches_are_ambiguous_and_listed() {
        let error = pick(EntityKind::Person, "ma", people()).unwrap_err();

        assert!(matches!(
            error,
            QueryError::Ambiguous { ref candidates, .. } if candidates.len() == 3
        ));
    }

    #[test]
    fn nothing_matching_is_not_found() {
        let error = pick(EntityKind::Person, "zed", people()).unwrap_err();

        assert!(matches!(
            error,
            QueryError::NotFound {
                kind: EntityKind::Person,
                ..
            }
        ));
    }
}
