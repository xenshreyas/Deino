//! Typed field validation, tabled: what each field type accepts, repairs
//! and drops, and how whole answers are kept, merged and cut.

use std::collections::BTreeMap;

use super::*;
use crate::insights::wire::WireEvidence;

fn text(value: &str) -> WireScalar {
    WireScalar::Text(value.into())
}

fn number(value: f64) -> WireScalar {
    WireScalar::Number(value)
}

fn day(year: i32, month: u32, date: u32) -> FieldValue {
    FieldValue::Date(NaiveDate::from_ymd_opt(year, month, date).unwrap())
}

fn health() -> FieldKind {
    FieldKind::Enum {
        options: vec!["Red".into(), "Yellow".into(), "Green".into()],
    }
}

/// Coerces every raw value of `cases` as `kind` and checks the outcome.
fn check(kind: &FieldKind, cases: Vec<(WireScalar, Option<FieldValue>)>) {
    for (raw, expected) in cases {
        assert_eq!(coerce(kind, &raw), expected, "{kind:?} {raw:?}");
    }
}

fn as_text(value: &str) -> FieldValue {
    FieldValue::Text(value.into())
}

fn as_number(value: f64) -> FieldValue {
    FieldValue::Number(value)
}

fn as_choice(value: &str) -> FieldValue {
    FieldValue::Choice(value.into())
}

#[test]
fn text_is_trimmed_numbers_and_flags_are_written_out_and_lists_joined() {
    let list = WireScalar::List(vec![text("slow"), text(""), text("manual")]);
    check(
        &FieldKind::Text,
        vec![
            (text("  Okta SSO "), Some(as_text("Okta SSO"))),
            (number(120.0), Some(as_text("120"))),
            (WireScalar::Bool(true), Some(as_text("Yes"))),
            (list, Some(as_text("slow; manual"))),
            (text("   "), None),
        ],
    );
}

#[test]
fn numbers_repair_currency_separators_suffixes_and_percents() {
    check(
        &FieldKind::Number,
        vec![
            (number(4.0), Some(as_number(4.0))),
            (text("$40,000"), Some(as_number(40_000.0))),
            (text("40k"), Some(as_number(40_000.0))),
            (text("1.5M"), Some(as_number(1_500_000.0))),
            (text("12.5%"), Some(as_number(12.5))),
            (text("~120"), Some(as_number(120.0))),
            (text("four"), None),
            (text("4/5"), None),
            (WireScalar::Bool(true), None),
            (number(f64::INFINITY), None),
        ],
    );
}

#[test]
fn yes_no_reads_booleans_one_and_zero_and_words() {
    let flag = |value| Some(FieldValue::Boolean(value));
    check(
        &FieldKind::Boolean,
        vec![
            (WireScalar::Bool(false), flag(false)),
            (text("Yes."), flag(true)),
            (text("FALSE"), flag(false)),
            (number(1.0), flag(true)),
            (number(2.0), None),
            (text("maybe"), None),
        ],
    );
}

#[test]
fn dates_read_iso_date_times_and_written_out_days() {
    check(
        &FieldKind::Date,
        vec![
            (text("2026-10-08"), Some(day(2026, 10, 8))),
            (text("2026-10-08T09:30:00Z"), Some(day(2026, 10, 8))),
            (text("October 8, 2026"), Some(day(2026, 10, 8))),
            (text("8 Oct 2026"), Some(day(2026, 10, 8))),
            (text("next Friday"), None),
            (text("2026-02-30"), None),
            (number(20_261_008.0), None),
        ],
    );
}

#[test]
fn options_match_as_spelled_ignoring_case_and_punctuation() {
    check(
        &health(),
        vec![
            (text("Green"), Some(as_choice("Green"))),
            (text("yellow"), Some(as_choice("Yellow"))),
            (text(" red. "), Some(as_choice("Red"))),
            (text("Amber"), None),
            (number(1.0), None),
        ],
    );
}

#[test]
fn long_text_is_cut_to_the_limit() {
    let long = "x".repeat(MAX_VALUE_CHARS + 10);

    let Some(FieldValue::Text(kept)) = coerce(&FieldKind::Text, &text(&long)) else {
        panic!("long text is kept");
    };

    assert_eq!(kept.chars().count(), MAX_VALUE_CHARS);
}

fn field(name: &str, kind: FieldKind) -> InsightField {
    InsightField {
        name: name.into(),
        kind,
    }
}

fn scorecard() -> Vec<InsightField> {
    vec![
        field("Budget", FieldKind::Number),
        field("Health", health()),
    ]
}

fn instance(fields: &[(&str, Option<WireScalar>)], evidence: &str) -> WireInstance {
    WireInstance {
        value: None,
        fields: fields
            .iter()
            .map(|(name, raw)| ((*name).to_owned(), raw.clone()))
            .collect::<BTreeMap<_, _>>(),
        evidence: Some(WireEvidence::One(evidence.into())),
    }
}

#[test]
fn field_answers_keep_valid_values_and_leave_invalid_ones_empty() {
    let wire = WireAnswer {
        answers: vec![instance(
            &[
                ("budget ", Some(text("$40k"))),
                ("Health", Some(text("Amber"))),
            ],
            "s12",
        )],
    };

    let answers = validate(&wire, &scorecard(), Cardinality::Auto);

    assert_eq!(
        answers,
        [Validated {
            value: InsightValue::Fields {
                fields: vec![
                    FieldEntry {
                        name: "Budget".into(),
                        value: Some(FieldValue::Number(40_000.0)),
                    },
                    FieldEntry {
                        name: "Health".into(),
                        value: None,
                    },
                ],
            },
            references: vec!["s12".into()],
        }]
    );
}

#[test]
fn answers_with_nothing_valid_and_repeats_are_dropped_and_the_rest_cut_to_the_count() {
    let budget = |raw: &str| instance(&[("Budget", Some(text(raw)))], "s1");
    let wire = WireAnswer {
        answers: vec![
            instance(&[("Budget", Some(text("lots"))), ("Health", None)], "s1"),
            budget("10"),
            budget("10"),
            budget("20"),
            budget("30"),
        ],
    };

    let auto = validate(&wire, &scorecard(), Cardinality::Auto);
    let two = validate(&wire, &scorecard(), Cardinality::Fixed { count: 2 });

    let budgets = |answers: &[Validated]| -> Vec<Option<FieldValue>> {
        answers
            .iter()
            .map(|answer| answer.value.field("Budget").cloned())
            .collect()
    };
    assert_eq!(
        budgets(&auto),
        [10.0, 20.0, 30.0].map(|value| Some(FieldValue::Number(value)))
    );
    assert_eq!(
        budgets(&two),
        [10.0, 20.0].map(|value| Some(FieldValue::Number(value)))
    );
}

#[test]
fn free_text_answers_need_a_value() {
    let wire = WireAnswer {
        answers: vec![
            WireInstance {
                value: Some(text("How is pricing structured?")),
                ..WireInstance::default()
            },
            WireInstance {
                value: Some(text("  ")),
                ..WireInstance::default()
            },
            WireInstance::default(),
        ],
    };

    let answers = validate(&wire, &[], Cardinality::Auto);

    assert_eq!(
        answers,
        [Validated {
            value: InsightValue::Text {
                text: "How is pricing structured?".into()
            },
            references: Vec::new(),
        }]
    );
}
