//! The condition rule engine, tabled.

use super::*;

const SALES_TAG: &str = "01920000-0000-7000-8000-000000000001";
const OTHER_TAG: &str = "01920000-0000-7000-8000-000000000002";
const CUSTOMERS: &str = "01920000-0000-7000-8000-000000000003";

fn tag(id: &str) -> TagId {
    id.parse().unwrap()
}

fn folder(id: &str) -> FolderId {
    id.parse().unwrap()
}

/// A discovery call with Acme: external, tagged Sales, in Customers.
fn discovery() -> MeetingFacts {
    MeetingFacts {
        title: "Acme discovery call".into(),
        template_id: Some(TemplateId::new("sales_discovery")),
        tag_ids: BTreeSet::from([tag(SALES_TAG)]),
        folder_ids: BTreeSet::from([folder(CUSTOMERS)]),
        attendee_domains: BTreeSet::from(["acme.com".to_owned(), "deino.dev".to_owned()]),
        has_external: true,
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
