//! The rule engine: matching, specificity and tie-breaks.

use chrono::DateTime;
use deino_domain::{Audience, Email, SectionFormat, TemplateId};

use super::*;

fn template(id: &str, is_builtin: bool, rules: Vec<AutoRule>) -> Template {
    Template {
        id: TemplateId::new(id),
        is_builtin,
        content: TemplateContent {
            name: id.into(),
            description: String::new(),
            meeting_context: String::new(),
            sections: vec![TemplateSection {
                title: "Notes".into(),
                instructions: String::new(),
                format: SectionFormat::Bullets,
            }],
            auto_rules: rules,
        },
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    }
}

fn titled(pattern: &str) -> AutoRule {
    AutoRule {
        title_pattern: Some(pattern.into()),
        ..AutoRule::default()
    }
}

fn emails(addresses: &[&str]) -> Vec<Email> {
    addresses.iter().map(|a| Email::parse(a).unwrap()).collect()
}

fn signals<'a>(
    title: &'a str,
    attendees: &'a [Email],
    audience: Option<Audience>,
) -> MeetingSignals<'a> {
    MeetingSignals {
        title,
        attendee_emails: attendees,
        attendee_count: attendees.len(),
        audience,
    }
}

fn chosen(templates: &[Template], meeting: &MeetingSignals<'_>) -> Option<String> {
    RuleEngine::new(templates)
        .best_match(meeting)
        .map(|matched| matched.template_id.to_string())
}

#[test]
fn titles_match_case_insensitively_and_every_condition_must_hold() {
    let external_demo = AutoRule {
        audience: Some(Audience::External),
        ..titled(r"\bdemo\b")
    };
    let templates = [template("demo", true, vec![external_demo])];
    let nobody: Vec<Email> = Vec::new();

    assert_eq!(
        chosen(
            &templates,
            &signals("Acme DEMO", &nobody, Some(Audience::External))
        )
        .as_deref(),
        Some("demo")
    );
    assert_eq!(
        chosen(
            &templates,
            &signals("Acme demo", &nobody, Some(Audience::Internal))
        ),
        None
    );
    assert_eq!(
        chosen(&templates, &signals("Acme demo", &nobody, None)),
        None,
        "unknown audience"
    );
    assert_eq!(
        chosen(
            &templates,
            &signals("Demolition", &nobody, Some(Audience::External))
        ),
        None
    );
}

#[test]
fn domains_match_attendees_and_their_subdomains_and_counts_are_inclusive() {
    let acme = AutoRule {
        attendee_domains: vec!["acme.com".into()],
        attendee_count: Some(CountRange {
            min: Some(2),
            max: Some(3),
        }),
        ..AutoRule::default()
    };
    let templates = [template("acme", false, vec![acme])];
    let three = emails(&["me@deino.dev", "a@eu.acme.com", "b@deino.dev"]);
    let four = emails(&["me@deino.dev", "a@acme.com", "b@deino.dev", "c@deino.dev"]);

    assert_eq!(
        chosen(&templates, &signals("Sync", &three, None)).as_deref(),
        Some("acme")
    );
    assert_eq!(chosen(&templates, &signals("Sync", &four, None)), None);
}

#[test]
fn the_most_specific_rule_wins_then_users_templates_then_list_order() {
    let broad = template("broad", true, vec![titled("sync")]);
    let specific = template(
        "specific",
        true,
        vec![AutoRule {
            audience: Some(Audience::Internal),
            ..titled("sync")
        }],
    );
    let mine = template("mine", false, vec![titled("sync")]);
    let nobody: Vec<Email> = Vec::new();
    let meeting = signals("Weekly sync", &nobody, Some(Audience::Internal));

    assert_eq!(
        chosen(&[broad.clone(), specific], &meeting).as_deref(),
        Some("specific")
    );
    assert_eq!(
        chosen(&[broad.clone(), mine], &meeting).as_deref(),
        Some("mine")
    );
    let other = template("other", true, vec![titled("weekly")]);
    assert_eq!(chosen(&[broad, other], &meeting).as_deref(), Some("broad"));
}

#[test]
fn the_match_names_the_rule_by_its_label_or_its_conditions() {
    let labelled = AutoRule {
        label: "external sales".into(),
        ..titled("discovery")
    };
    let nobody: Vec<Email> = Vec::new();
    let meeting = signals("Discovery call", &nobody, None);

    let by_label = RuleEngine::new(&[template("a", true, vec![labelled])]).best_match(&meeting);
    let by_conditions =
        RuleEngine::new(&[template("b", true, vec![titled("discovery")])]).best_match(&meeting);

    assert_eq!(by_label.unwrap().rule, "external sales");
    assert_eq!(by_conditions.unwrap().rule, "title matches “discovery”");
}

#[test]
fn templates_without_rules_or_with_a_broken_pattern_never_match() {
    let broken = template("broken", false, vec![titled("(unclosed")]);
    let ruleless = template("general", true, Vec::new());
    let nobody: Vec<Email> = Vec::new();

    assert_eq!(
        chosen(&[broken, ruleless], &signals("(unclosed", &nobody, None)),
        None
    );
}

#[test]
fn every_built_in_pattern_compiles() {
    let templates: Vec<Template> = CATALOG
        .iter()
        .map(|builtin| template(builtin.id, true, builtin.content().auto_rules))
        .collect();
    let compiled = templates
        .iter()
        .flat_map(|template| &template.content.auto_rules)
        .filter_map(|rule| rule.title_pattern.as_deref())
        .all(|pattern| super::rules::compile_pattern(pattern).is_ok());

    assert!(compiled);
}
