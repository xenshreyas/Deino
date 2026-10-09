//! Auto-add rules: which future meetings a folder collects (ORG-05).
//!
//! A [`FolderRule`] matches a recurring series, a tag or an attendee's email
//! domain. [`folders_matching`] is the rule engine: given every rule and the
//! [`MeetingFacts`] of one meeting, it names the folders the meeting belongs
//! in. It is pure, so the `auto_folder` stage and its tests share it.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use deino_domain::{Email, FolderId, FolderRuleId, FolderRuleKind, SeriesId, TagId};
use serde::{Deserialize, Serialize};

use super::FolderError;

/// What a folder's auto-add rule matches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FolderRule {
    /// Future meetings of a recurring calendar series ("Auto-add future
    /// meetings in this series?").
    Series {
        /// The series, as `meetings.series_id` stores it.
        #[serde(rename = "seriesId")]
        series_id: SeriesId,
    },
    /// Meetings carrying a tag, including tags the auto-tagger applies.
    Tag {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
    },
    /// Meetings with an attendee whose email is at a domain (`acme.com`).
    AttendeeDomain {
        /// The domain, lower case, without `@`.
        domain: String,
    },
}

impl FolderRule {
    /// The rule's kind, as `folder_rules.kind` stores it.
    pub fn kind(&self) -> FolderRuleKind {
        match self {
            Self::Series { .. } => FolderRuleKind::Series,
            Self::Tag { .. } => FolderRuleKind::Tag,
            Self::AttendeeDomain { .. } => FolderRuleKind::AttendeeDomain,
        }
    }

    /// The rule in canonical form: a domain typed as `@Acme.com ` becomes
    /// `acme.com`, and anything that is not a domain is refused.
    pub fn normalized(self) -> Result<Self, FolderError> {
        match self {
            Self::AttendeeDomain { domain } => Ok(Self::AttendeeDomain {
                domain: parse_domain(&domain)?,
            }),
            rule => Ok(rule),
        }
    }

    /// True when a meeting with `facts` belongs in the rule's folder.
    pub fn matches(&self, facts: &MeetingFacts) -> bool {
        match self {
            Self::Series { series_id } => facts.series_id.as_ref() == Some(series_id),
            Self::Tag { tag_id } => facts.tag_ids.contains(tag_id),
            Self::AttendeeDomain { domain } => facts.attendee_domains.contains(domain),
        }
    }
}

/// `input` as an email domain, exactly as [`Email::domain`] reports one.
fn parse_domain(input: &str) -> Result<String, FolderError> {
    let domain = input.trim().trim_start_matches('@');
    Email::parse(&format!("someone@{domain}"))
        .map(|email| email.domain().to_owned())
        .map_err(|_| FolderError::InvalidDomain)
}

/// A rule to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewFolderRule {
    /// The new rule's ID.
    pub id: FolderRuleId,
    /// The folder it files meetings into.
    pub folder_id: FolderId,
    /// What it matches, in canonical form ([`FolderRule::normalized`]).
    pub rule: FolderRule,
    /// When it was added.
    pub created_at: DateTime<Utc>,
}

/// A stored auto-add rule, as the folder page shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderRuleEntry {
    /// The rule's ID.
    pub id: FolderRuleId,
    /// The folder it files meetings into.
    pub folder_id: FolderId,
    /// What it matches.
    pub rule: FolderRule,
    /// When it was added.
    pub created_at: DateTime<Utc>,
    /// For a series rule, the title of the series' latest meeting, so the
    /// page can say which series it is; `None` otherwise.
    pub series_title: Option<String>,
}

/// What the rule engine knows about one meeting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingFacts {
    /// The recurring series it belongs to.
    pub series_id: Option<SeriesId>,
    /// The tags on it, by anyone.
    pub tag_ids: BTreeSet<TagId>,
    /// The email domains of its attendees.
    pub attendee_domains: BTreeSet<String>,
}

/// The folders whose rules match `facts`, each once, in rule order.
pub fn folders_matching(rules: &[FolderRuleEntry], facts: &MeetingFacts) -> Vec<FolderId> {
    let mut folders: Vec<FolderId> = Vec::new();
    for entry in rules.iter().filter(|entry| entry.rule.matches(facts)) {
        if !folders.contains(&entry.folder_id) {
            folders.push(entry.folder_id);
        }
    }
    folders
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    fn clock() -> FakeClock {
        FakeClock::new(at("2026-10-09T09:00:00Z"))
    }

    fn entry(folder_id: FolderId, rule: FolderRule) -> FolderRuleEntry {
        FolderRuleEntry {
            id: FolderRuleId::generate(&clock()),
            folder_id,
            rule,
            created_at: at("2026-10-09T09:00:00Z"),
            series_title: None,
        }
    }

    fn standup_facts(tag: TagId) -> MeetingFacts {
        MeetingFacts {
            series_id: Some(SeriesId::new("standup@google.com")),
            tag_ids: BTreeSet::from([tag]),
            attendee_domains: BTreeSet::from(["deino.dev".to_owned(), "acme.com".to_owned()]),
        }
    }

    #[test]
    fn each_kind_matches_only_its_own_fact() {
        let tag = TagId::generate(&clock());
        let facts = standup_facts(tag);
        let series = |id: &str| FolderRule::Series {
            series_id: SeriesId::new(id),
        };
        let domain = |domain: &str| FolderRule::AttendeeDomain {
            domain: domain.into(),
        };

        assert!(series("standup@google.com").matches(&facts));
        assert!(!series("planning@google.com").matches(&facts));
        assert!(FolderRule::Tag { tag_id: tag }.matches(&facts));
        assert!(
            !FolderRule::Tag {
                tag_id: TagId::generate(&clock())
            }
            .matches(&facts)
        );
        assert!(domain("acme.com").matches(&facts));
        assert!(!domain("globex.com").matches(&facts));
        assert!(!series("standup@google.com").matches(&MeetingFacts::default()));
    }

    #[test]
    fn the_engine_names_each_matching_folder_once_in_rule_order() {
        let tag = TagId::generate(&clock());
        let (clients, standups, other) = (
            FolderId::generate(&clock()),
            FolderId::generate(&clock()),
            FolderId::generate(&clock()),
        );
        let rules = [
            entry(
                standups,
                FolderRule::Series {
                    series_id: SeriesId::new("standup@google.com"),
                },
            ),
            entry(
                other,
                FolderRule::AttendeeDomain {
                    domain: "globex.com".into(),
                },
            ),
            entry(
                clients,
                FolderRule::AttendeeDomain {
                    domain: "acme.com".into(),
                },
            ),
            entry(standups, FolderRule::Tag { tag_id: tag }),
        ];

        assert_eq!(
            folders_matching(&rules, &standup_facts(tag)),
            [standups, clients]
        );
        assert!(folders_matching(&rules, &MeetingFacts::default()).is_empty());
    }

    #[test]
    fn domains_are_normalised_or_refused() {
        let domain = |input: &str| {
            FolderRule::AttendeeDomain {
                domain: input.into(),
            }
            .normalized()
        };

        assert_eq!(
            domain(" @Acme.COM ").unwrap(),
            FolderRule::AttendeeDomain {
                domain: "acme.com".into()
            }
        );
        for invalid in ["", "acme", "a b.com", "x@acme.com", ".com"] {
            assert!(
                matches!(domain(invalid), Err(FolderError::InvalidDomain)),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn rules_cross_ipc_tagged_by_kind_with_camel_case_fields() {
        let rule = FolderRule::Series {
            series_id: SeriesId::new("s-1"),
        };

        let json = serde_json::to_value(&rule).unwrap();

        assert_eq!(
            json,
            serde_json::json!({ "kind": "series", "seriesId": "s-1" })
        );
        let domain: FolderRule = serde_json::from_value(
            serde_json::json!({ "kind": "attendee_domain", "domain": "acme.com" }),
        )
        .unwrap();
        assert_eq!(domain.kind(), FolderRuleKind::AttendeeDomain);
    }
}
