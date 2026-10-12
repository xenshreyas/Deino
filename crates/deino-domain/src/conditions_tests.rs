//! The condition rule engine, tabled.

use super::*;

const SALES_TAG: &str = "01920000-0000-7000-8000-000000000001";
const OTHER_TAG: &str = "01920000-0000-7000-8000-000000000002";
const CUSTOMERS: &str = "01920000-0000-7000-8000-000000000003";
const ALICE: &str = "01920000-0000-7000-8000-000000000004";
const BOB: &str = "01920000-0000-7000-8000-000000000005";
const WORK: &str = "01920000-0000-7000-8000-000000000006";

fn tag(id: &str) -> TagId {
    id.parse().unwrap()
}

fn folder(id: &str) -> FolderId {
    id.parse().unwrap()
}

fn person(id: &str) -> PersonId {
    id.parse().unwrap()
}

fn calendar(id: &str) -> CalendarId {
    id.parse().unwrap()
}

fn email(address: &str) -> Email {
    Email::parse(address).unwrap()
}

/// A discovery call with Acme: external, tagged Sales, in Customers.
fn discovery() -> MeetingFacts {
    MeetingFacts {
        title: "Acme discovery call".into(),
        template_id: Some(TemplateId::new("sales_discovery")),
        tag_ids: BTreeSet::from([tag(SALES_TAG)]),
        folder_ids: BTreeSet::from([folder(CUSTOMERS)]),
        participant_ids: BTreeSet::from([person(ALICE)]),
        invitee_emails: BTreeSet::from([email("wile@acme.com"), email("me@deino.dev")]),
        attendee_domains: BTreeSet::from(["acme.com".to_owned(), "deino.dev".to_owned()]),
        has_external: true,
        duration_s: Some(45 * 60),
        calendar_id: Some(calendar(WORK)),
    }
}

fn title(text: &str) -> Condition {
    Condition::TitleContains { text: text.into() }
}

fn template(id: &str) -> Condition {
    Condition::TemplateIs {
        template_id: TemplateId::new(id),
    }
}

fn has_tag(id: &str) -> Condition {
    Condition::HasTag { tag_id: tag(id) }
}

fn in_folder(id: &str) -> Condition {
    Condition::InFolder {
        folder_id: folder(id),
    }
}

fn domain(name: &str) -> Condition {
    Condition::AttendeeDomain {
        domain: name.into(),
    }
}

#[test]
fn each_condition_tests_its_own_fact() {
    let cases: [(Condition, bool); 12] = [
        (template("sales_discovery"), true),
        (template("one_on_one"), false),
        (has_tag(SALES_TAG), true),
        (has_tag(OTHER_TAG), false),
        (in_folder(CUSTOMERS), true),
        (in_folder(OTHER_TAG), false),
        (domain("acme.com"), true),
        (domain("globex.com"), false),
        (Condition::External { external: true }, true),
        (Condition::External { external: false }, false),
        (title("DISCOVERY"), true),
        (title("standup"), false),
    ];

    for (condition, expected) in cases {
        assert_eq!(condition.holds(&discovery()), expected, "{condition:?}");
    }
}

#[test]
fn a_meeting_without_a_template_matches_no_template_condition() {
    let facts = MeetingFacts {
        template_id: None,
        ..discovery()
    };

    let condition = Condition::TemplateIs {
        template_id: TemplateId::new("sales_discovery"),
    };
    assert!(!condition.holds(&facts));
}

#[test]
fn groups_are_and_within_and_or_across_and_none_match_everything() {
    let external = Condition::External { external: true };
    let internal = Condition::External { external: false };
    let cases = [
        (vec![], true),
        (vec![vec![external.clone(), title("acme")]], true),
        (vec![vec![external.clone(), title("standup")]], false),
        (vec![vec![internal.clone()], vec![title("acme")]], true),
        (vec![vec![internal], vec![title("standup")]], false),
    ];

    for (groups, expected) in cases {
        let groups = ConditionGroups(groups);
        assert_eq!(groups.matches(&discovery()), expected, "{groups:?}");
    }
    assert!(ConditionGroups::every_meeting().matches(&MeetingFacts::default()));
}

#[test]
fn normalising_cleans_domains_and_titles_and_refuses_what_cannot_match() {
    let domain = Condition::AttendeeDomain {
        domain: " @Acme.COM ".into(),
    };
    assert_eq!(
        domain.normalized(),
        Ok(Condition::AttendeeDomain {
            domain: "acme.com".into()
        })
    );
    assert_eq!(title("  QBR ").normalized(), Ok(title("QBR")));
    assert_eq!(title("   ").normalized(), Err(InvalidCondition::TitleText));
    assert_eq!(
        title(&"x".repeat(MAX_TITLE_TEXT_CHARS + 1)).normalized(),
        Err(InvalidCondition::TitleText)
    );
    let invalid = Condition::AttendeeDomain {
        domain: "not a domain".into(),
    };
    assert!(matches!(
        invalid.normalized(),
        Err(InvalidCondition::Domain(_))
    ));
}

#[test]
fn normalising_groups_drops_empty_ones_and_fails_on_any_invalid_condition() {
    let groups = ConditionGroups(vec![vec![], vec![title(" acme ")]]);
    assert_eq!(
        groups.normalized(),
        Ok(ConditionGroups(vec![vec![title("acme")]]))
    );

    let invalid = ConditionGroups(vec![vec![title("acme"), title("")]]);
    assert_eq!(invalid.normalized(), Err(InvalidCondition::TitleText));
}

#[test]
fn conditions_serialize_as_tagged_camel_case_objects() {
    let groups = ConditionGroups(vec![vec![
        Condition::HasTag {
            tag_id: tag(SALES_TAG),
        },
        Condition::External { external: true },
    ]]);

    let json = serde_json::to_value(&groups).unwrap();

    assert_eq!(
        json,
        serde_json::json!([[
            { "kind": "has_tag", "tagId": SALES_TAG },
            { "kind": "external", "external": true }
        ]])
    );
    assert_eq!(
        serde_json::from_value::<ConditionGroups>(json).unwrap(),
        groups
    );
}

fn text_condition(kind: fn(String) -> Condition, text: &str) -> Condition {
    kind(text.to_owned())
}

fn domains(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn count(comparison: Comparison, count: u32) -> Condition {
    Condition::InviteeCount { comparison, count }
}

/// AUT-01's conditions, tabled against the discovery call: two invitees
/// (acme.com, deino.dev), Alice taking part, 45 minutes, the Work calendar.
#[test]
fn every_automation_condition_tests_its_own_fact() {
    let is = |text: &str| text_condition(|text| Condition::TitleIs { text }, text);
    let is_not = |text: &str| text_condition(|text| Condition::TitleIsNot { text }, text);
    let lacks = |text: &str| text_condition(|text| Condition::TitleLacks { text }, text);
    let matches = |text: &str| text_condition(|pattern| Condition::TitleMatches { pattern }, text);
    let cases: [(Condition, bool); 30] = [
        (
            Condition::LacksTag {
                tag_id: tag(OTHER_TAG),
            },
            true,
        ),
        (
            Condition::LacksTag {
                tag_id: tag(SALES_TAG),
            },
            false,
        ),
        (
            Condition::HasParticipant {
                person_id: person(ALICE),
            },
            true,
        ),
        (
            Condition::HasParticipant {
                person_id: person(BOB),
            },
            false,
        ),
        (
            Condition::LacksParticipant {
                person_id: person(BOB),
            },
            true,
        ),
        (
            Condition::LacksParticipant {
                person_id: person(ALICE),
            },
            false,
        ),
        (is("acme DISCOVERY call"), true),
        (is("Acme discovery"), false),
        (is_not("Weekly sync"), true),
        (is_not("Acme discovery call"), false),
        (lacks("standup"), true),
        (lacks("acme"), false),
        (matches(r"^acme\s+\w+ call$"), true),
        (matches(r"^\d{4}-"), false),
        (
            Condition::InviteeEmail {
                email: email("wile@acme.com"),
            },
            true,
        ),
        (
            Condition::InviteeEmail {
                email: email("road@acme.com"),
            },
            false,
        ),
        (
            Condition::DomainsAreAll {
                domains: domains(&["acme.com", "deino.dev"]),
            },
            true,
        ),
        (
            Condition::DomainsAreAll {
                domains: domains(&["deino.dev"]),
            },
            false,
        ),
        (
            Condition::DomainsNotAll {
                domains: domains(&["deino.dev"]),
            },
            true,
        ),
        (
            Condition::DomainsNotAll {
                domains: domains(&["acme.com", "deino.dev"]),
            },
            false,
        ),
        (count(Comparison::Equal, 2), true),
        (count(Comparison::Equal, 3), false),
        (count(Comparison::Greater, 1), true),
        (count(Comparison::Greater, 2), false),
        (count(Comparison::Less, 3), true),
        (count(Comparison::Less, 2), false),
        (Condition::DurationOver { minutes: 30 }, true),
        (Condition::DurationOver { minutes: 45 }, false),
        (
            Condition::CalendarIs {
                calendar_id: calendar(WORK),
            },
            true,
        ),
        (
            Condition::CalendarIs {
                calendar_id: calendar(CUSTOMERS),
            },
            false,
        ),
    ];

    for (condition, expected) in cases {
        assert_eq!(condition.holds(&discovery()), expected, "{condition:?}");
    }
}

#[test]
fn a_meeting_without_invitees_duration_or_calendar_matches_none_of_their_tests() {
    let bare = MeetingFacts {
        title: "Notes".into(),
        ..MeetingFacts::default()
    };
    let conditions = [
        Condition::DomainsAreAll {
            domains: domains(&["deino.dev"]),
        },
        Condition::DomainsNotAll {
            domains: domains(&["deino.dev"]),
        },
        Condition::DurationOver { minutes: 0 },
        Condition::CalendarIs {
            calendar_id: calendar(WORK),
        },
        count(Comparison::Greater, 0),
    ];

    for condition in conditions {
        assert!(!condition.holds(&bare), "{condition:?}");
    }
    assert!(count(Comparison::Equal, 0).holds(&bare));
}

#[test]
fn normalising_refuses_bad_patterns_and_cleans_domain_lists() {
    let bad = Condition::TitleMatches {
        pattern: "(unclosed".into(),
    };
    assert_eq!(bad.normalized(), Err(InvalidCondition::Pattern));
    let list = Condition::DomainsAreAll {
        domains: domains(&[" @Deino.dev", "acme.com", "deino.dev"]),
    };
    assert_eq!(
        list.normalized(),
        Ok(Condition::DomainsAreAll {
            domains: domains(&["acme.com", "deino.dev"])
        })
    );
    let empty = Condition::DomainsNotAll {
        domains: Vec::new(),
    };
    assert_eq!(empty.normalized(), Err(InvalidCondition::DomainCount));
}

#[test]
fn groups_know_when_a_tag_could_have_made_them_match() {
    let groups = ConditionGroups(vec![vec![has_tag(SALES_TAG)], vec![title("acme")]]);

    assert!(groups.mentions_tag(&tag(SALES_TAG)));
    assert!(!groups.mentions_tag(&tag(OTHER_TAG)));
}
