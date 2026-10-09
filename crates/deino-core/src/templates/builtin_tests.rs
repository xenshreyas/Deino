//! The built-in catalogue: every template is valid, and the catalogue keeps
//! its promises (30 templates, stable slugs, legacy successors).

use std::collections::BTreeSet;

use super::*;

#[test]
fn all_thirty_built_ins_validate_unchanged() {
    assert_eq!(CATALOG.len(), 30);
    for builtin in &CATALOG {
        let content = builtin.content();
        let validated = content.clone().validated();
        assert!(validated.is_ok(), "{}: {:?}", builtin.id, validated.err());
        assert_eq!(
            validated.unwrap(),
            content,
            "{} is not already normalised",
            builtin.id
        );
    }
}

#[test]
fn ids_are_unique_slugs_and_names_are_unique() {
    let ids: BTreeSet<&str> = CATALOG.iter().map(|builtin| builtin.id).collect();
    let names: BTreeSet<String> = CATALOG
        .iter()
        .map(|builtin| builtin.name.to_lowercase())
        .collect();

    assert_eq!(ids.len(), CATALOG.len());
    assert_eq!(names.len(), CATALOG.len());
    for id in ids {
        assert!(
            id.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{id} is not a slug"
        );
    }
}

#[test]
fn every_built_in_is_described_and_versioned() {
    for builtin in &CATALOG {
        assert!(builtin.version >= 1, "{}", builtin.id);
        assert!(!builtin.description.is_empty(), "{}", builtin.id);
        assert!(!builtin.meeting_context.is_empty(), "{}", builtin.id);
        assert!(
            builtin
                .sections
                .iter()
                .all(|section| !section.instructions.is_empty()),
            "{} has a section without instructions",
            builtin.id
        );
    }
}

#[test]
fn the_card_library_is_complete() {
    let names: Vec<&str> = CATALOG.iter().map(|builtin| builtin.name).collect();
    for expected in [
        "General",
        "1:1",
        "Stand-up",
        "Sales discovery",
        "Therapy / SOAP",
        "Circleback style",
        "Investor pitch (VC side)",
        "Vendor evaluation",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
    assert_eq!(CATALOG[0].id, GENERAL_TEMPLATE_ID);
}

#[test]
fn circleback_style_puts_action_items_first_then_the_overview() {
    let circleback = CATALOG.iter().find(|b| b.id == "circleback_style").unwrap();

    let titles: Vec<&str> = circleback.sections.iter().map(|s| s.title).collect();

    assert_eq!(titles, ["Action Items", "Overview", "Topics"]);
    assert_eq!(
        circleback.sections[0].format,
        deino_domain::SectionFormat::Todo
    );
}

#[test]
fn the_six_pre_not04_templates_have_successors_in_the_catalogue() {
    for legacy in [
        "standard_meeting",
        "daily_standup",
        "project_sync",
        "retrospective",
        "sales_marketing_client_call",
        "psychatric_session",
    ] {
        let successor = successor_of_legacy(legacy).unwrap();
        assert!(
            CATALOG.iter().any(|b| b.id == successor),
            "{legacy} → {successor}"
        );
    }
    assert_eq!(
        successor_of_legacy("psychatric_session"),
        Some("therapy_soap")
    );
    assert_eq!(successor_of_legacy("nope"), None);
}
